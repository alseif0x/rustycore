// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Persistence half of the represented Player full-save coordinator.
//!
//! The request is fully owned before the port await. No canonical map access
//! or borrowed owner data crosses that boundary.

use std::sync::Arc;

use tracing::{trace, warn};
use wow_persistence::{
    PersistenceOutcomeLikeCpp, PlayerCharacterCommittedGroupsLikeCpp,
    PlayerCharacterSaveRequestLikeCpp,
};
use wow_world_lifecycle::{
    PlayerMoneyCommitCancellationFenceLikeCpp, SessionLifecycleState,
};
use wow_world_core::session::{
    AcknowledgedPlayerSaveLikeCpp, CapturedPlayerSaveLikeCpp,
    PlayerSaveOperationAccessLikeCpp, PlayerSaveOwnerAccessLikeCpp,
};

pub enum PlayerSavePersistenceResultLikeCpp {
    Applied {
        player_guid: wow_core::ObjectGuid,
        rows: u64,
        committed: PlayerCharacterCommittedGroupsLikeCpp,
        registry_sync_required: bool,
    },
    Failed { reason: String },
    Unknown { reason: String },
    SnapshotUnavailable,
    Unavailable,
    Quarantined { reason: &'static str },
}

pub fn capture_player_save_request_like_cpp(
    owner: &PlayerSaveOwnerAccessLikeCpp<'_>,
    lifecycle: &SessionLifecycleState,
    now_unix_secs: i64,
) -> Option<CapturedPlayerSaveLikeCpp> {
    let inputs = lifecycle.player_save_session_inputs_like_cpp(now_unix_secs);
    owner.capture_like_cpp(inputs)
}

/// Publish session-owned save acknowledgements only when their captured values
/// still match the current session state. Returns whether registry state needs
/// synchronization after the canonical owner capability is released.
pub fn apply_player_save_acknowledgement_like_cpp(
    lifecycle: &mut SessionLifecycleState,
    acknowledged: Option<&AcknowledgedPlayerSaveLikeCpp>,
) -> bool {
    let Some(acknowledged) = acknowledged else {
        return false;
    };
    if let Some(saved) = &acknowledged.tutorials {
        if acknowledged.groups.tutorials_insert {
            lifecycle.set_tutorials_loaded_from_db_like_cpp(true);
        }
        if acknowledged.groups.tutorials_changed
            && lifecycle.tutorial_values_like_cpp() == &saved.tutorials
        {
            lifecycle.set_tutorials_changed_like_cpp(false);
        }
    }
    acknowledged.groups.player_spells || acknowledged.groups.player_skills
}

pub async fn persist_player_save_request_like_cpp(
    lifecycle: &SessionLifecycleState,
    owner: &mut PlayerSaveOperationAccessLikeCpp<'_>,
    player_guid: wow_core::ObjectGuid,
    request: PlayerCharacterSaveRequestLikeCpp,
) -> PlayerSavePersistenceResultLikeCpp {
    let account_id = request.account_id;
    let Some(port) = lifecycle
        .player_lifecycle_port_like_cpp()
        .map(Arc::clone)
    else {
        warn!(
            account = account_id,
            player_guid = ?player_guid,
            "Skipping Player::SaveToDB represented save because lifecycle persistence is unavailable"
        );
        return PlayerSavePersistenceResultLikeCpp::Unavailable;
    };

    let tracker = lifecycle.durable_loot_money_persistence_tracker_like_cpp();
    if tracker.is_indeterminate_like_cpp() {
        let reason =
            "player persistence became indeterminate before the full-save semantic snapshot; aborting the entire save";
        owner.quarantine_like_cpp(reason);
        return PlayerSavePersistenceResultLikeCpp::Quarantined { reason };
    }
    let mut cancellation_fence =
        PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(tracker));
    let result = port.save_character_like_cpp(request).await;
    match result.outcome {
        PersistenceOutcomeLikeCpp::Applied { rows } => {
            cancellation_fence.disarm_like_cpp();
            PlayerSavePersistenceResultLikeCpp::Applied {
                player_guid,
                rows,
                committed: result.committed,
                registry_sync_required: false,
            }
        }
        PersistenceOutcomeLikeCpp::Failed { reason } => {
            cancellation_fence.disarm_like_cpp();
            warn!(
                guid = player_guid.counter(),
                "Failed to commit Player::SaveToDB represented transaction: {reason}"
            );
            PlayerSavePersistenceResultLikeCpp::Failed { reason }
        }
        PersistenceOutcomeLikeCpp::Unknown { reason } => {
            tracker.mark_indeterminate_like_cpp();
            trace!(fence = "player.save.relogin_required", "persistence fence");
            cancellation_fence.disarm_like_cpp();
            owner.quarantine_like_cpp(
                "Player::SaveToDB COMMIT outcome is unknown; relog required before another money mutation",
            );
            warn!(
                guid = player_guid.counter(),
                "Player::SaveToDB represented transaction COMMIT outcome is unknown: {reason}"
            );
            PlayerSavePersistenceResultLikeCpp::Unknown { reason }
        }
    }
}

/// Run the canonical snapshot, one-transaction save, and incarnation-bound ACK.
/// Call after the save admission and money fences have been acquired.
pub async fn save_canonical_player_like_cpp(
    lifecycle: &mut SessionLifecycleState,
    owner: &mut PlayerSaveOperationAccessLikeCpp<'_>,
    now_unix_secs: i64,
) -> PlayerSavePersistenceResultLikeCpp {
    if lifecycle
        .durable_loot_money_persistence_tracker_like_cpp()
        .is_indeterminate_like_cpp()
    {
        return PlayerSavePersistenceResultLikeCpp::SnapshotUnavailable;
    }
    let inputs = lifecycle.player_save_session_inputs_like_cpp(now_unix_secs);
    let Some(captured) = owner.capture_like_cpp(inputs) else {
        return PlayerSavePersistenceResultLikeCpp::SnapshotUnavailable;
    };
    let (request, header, receipt) = captured.into_parts_like_cpp();
    let player_guid = header.guid;
    let result =
        persist_player_save_request_like_cpp(lifecycle, owner, player_guid, request).await;
    match result {
        PlayerSavePersistenceResultLikeCpp::Applied {
            rows,
            committed,
            ..
        } => {
            let acknowledged = owner.acknowledge_like_cpp(receipt, &committed);
            let registry_sync_required =
                apply_player_save_acknowledgement_like_cpp(lifecycle, acknowledged.as_ref());
            PlayerSavePersistenceResultLikeCpp::Applied {
                player_guid,
                rows,
                registry_sync_required,
                committed,
            }
        }
        other => other,
    }
}
