// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented Player full-save coordinator.
//!
//! C++ `Player::SaveToDB` (`Player.cpp:19312`) and `Player::_SaveVoidStorage`
//! (`Player.cpp:20002`) own the complete operation: autosave timer, transfer
//! gate, money admission fence, durable Item-loot wait and completion
//! publication, detached payout reconciliation, exclusive money mutation
//! lock, capture, one-transaction persist, incarnation-bound acknowledgement,
//! unlock and the represented objective drain. This module owns that order;
//! the World session only selects the participants it lends through
//! [`PlayerSaveHostLikeCpp`] and supplies session publication/registry access.
//!
//! The request is fully owned before the port await. No canonical map access
//! or borrowed owner data crosses that boundary.

use std::future::Future;
use std::sync::Arc;

use tracing::{info, trace, warn};
use wow_core::ObjectGuidGenerator;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_persistence::PlayerTutorialsSaveLikeCpp;
use wow_persistence::{
    PersistenceOutcomeLikeCpp, PlayerCharacterCommittedGroupsLikeCpp,
    PlayerCharacterSaveRequestLikeCpp,
};
use wow_world_core::session::connection_identity::unix_now;
use wow_world_core::session::{
    AcknowledgedPlayerSaveLikeCpp, CapturedPlayerSaveLikeCpp, PlayerSaveOperationAccessLikeCpp,
    PlayerSaveOwnerAccessLikeCpp,
};
use wow_world_lifecycle::{
    PlayerMoneyCommitCancellationFenceLikeCpp, PlayerSaveOutcomeLikeCpp, SessionLifecycleState,
};

pub enum PlayerSavePersistenceResultLikeCpp {
    Applied {
        player_guid: wow_core::ObjectGuid,
        rows: u64,
        committed: PlayerCharacterCommittedGroupsLikeCpp,
        registry_sync_required: bool,
    },
    Failed {
        reason: String,
    },
    Unknown {
        reason: String,
    },
    SnapshotUnavailable,
    Unavailable,
    Quarantined {
        reason: &'static str,
    },
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
    let Some(port) = lifecycle.player_lifecycle_port_like_cpp().map(Arc::clone) else {
        warn!(
            account = account_id,
            player_guid = ?player_guid,
            "Skipping Player::SaveToDB represented save because lifecycle persistence is unavailable"
        );
        return PlayerSavePersistenceResultLikeCpp::Unavailable;
    };

    let tracker = lifecycle.durable_loot_money_persistence_tracker_like_cpp();
    if tracker.is_indeterminate_like_cpp() {
        let reason = "player persistence became indeterminate before the full-save semantic snapshot; aborting the entire save";
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
    let result = persist_player_save_request_like_cpp(lifecycle, owner, player_guid, request).await;
    match result {
        PlayerSavePersistenceResultLikeCpp::Applied {
            rows, committed, ..
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

/// The session-owned participants one save phase borrows.
///
/// The World adapter builds both halves from disjoint `WorldSession` fields;
/// the coordinator never learns how they were selected.
pub struct PlayerSaveParticipantsLikeCpp<'a> {
    /// Lifecycle owner of the autosave timer, the transfer gate and the fences.
    pub lifecycle: &'a mut SessionLifecycleState,
    /// Mutable canonical Player owner used by capture, commit and ACK.
    pub operation: PlayerSaveOperationAccessLikeCpp<'a>,
}

/// Session-only diagnostics of the skipped-save warning.
pub struct PlayerSaveDiagnosticsLikeCpp {
    pub account_id: u32,
    pub player_guid: Option<wow_core::ObjectGuid>,
    pub has_session_position: bool,
    pub has_canonical_map_manager: bool,
}

/// One capture from the legacy ownerless fixture oracle.
///
/// Ownerless sessions exist only in the World test harness: production always
/// reaches the save through a canonical Player incarnation.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct OwnerlessPlayerSaveCaptureLikeCpp {
    pub guid: wow_core::ObjectGuid,
    pub request: PlayerCharacterSaveRequestLikeCpp,
    pub receipt: OwnerlessPlayerSaveReceiptLikeCpp,
}

/// The values the ownerless fixture oracle needs to acknowledge its capture.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct OwnerlessPlayerSaveReceiptLikeCpp {
    pub expected: PlayerCharacterCommittedGroupsLikeCpp,
    pub tutorials: Option<PlayerTutorialsSaveLikeCpp>,
}

/// Session-owned capabilities of the represented full save.
///
/// Each method is invoked at the exact point C++ `Player::SaveToDB` performs
/// the step, so no stage value, ordering or resumption is needed. The World
/// session implements them by selecting its existing owners; the coordinator
/// never re-derives a session decision.
pub trait PlayerSaveHostLikeCpp {
    /// The two session owners the save borrows together: the lifecycle owner of
    /// the timer, transfer gate and money fences, and the canonical Player owner
    /// the capture, commit and acknowledgement phases use.
    fn player_save_participants_like_cpp(&mut self) -> PlayerSaveParticipantsLikeCpp<'_>;

    /// Publishes the durable Item-loot completions that outlived their packet
    /// waiter. The money criteria stay queued for the post-unlock drain.
    fn apply_pending_durable_item_loot_completions_for_player_save_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send;

    /// Reconciles the completed detached payouts against the represented money
    /// before the absolute snapshot is captured.
    fn reconcile_durable_loot_money_before_player_save_like_cpp(
        &mut self,
    ) -> impl Future<Output = bool> + Send;

    /// C++ `Player::SaveToDB` quarantines the session when the money owner became
    /// indeterminate while the save waited for its mutation lock.
    fn quarantine_player_save_like_cpp(&mut self, reason: &str);

    /// Drains the represented quest objectives the save's committed work changed.
    /// The save runs it only after the fences are released.
    fn drain_represented_quest_objective_progress_for_player_save_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send;

    /// Re-publishes the broadcast registry state after an acknowledged
    /// spell/skill commit produced by the save.
    fn sync_player_save_registry_state_like_cpp(&mut self);

    /// The session diagnostics the skipped-save warning reports.
    fn player_save_diagnostics_like_cpp(&self) -> PlayerSaveDiagnosticsLikeCpp;

    /// Captures the legacy ownerless fixture session, if this host has one.
    ///
    /// Only the World test harness overrides this; a build without that harness
    /// reports no fixture capture and the save falls through to the canonical
    /// owner, which then reports the established `Unavailable` outcome.
    #[cfg(any(test, feature = "test-fixtures"))]
    fn capture_ownerless_player_save_like_cpp(
        &mut self,
        now_unix_secs: i64,
    ) -> Option<OwnerlessPlayerSaveCaptureLikeCpp> {
        let _ = now_unix_secs;
        None
    }

    /// Applies the ownerless fixture acknowledgement after its COMMIT.
    #[cfg(any(test, feature = "test-fixtures"))]
    fn acknowledge_ownerless_player_save_like_cpp(
        &mut self,
        receipt: OwnerlessPlayerSaveReceiptLikeCpp,
        committed: &PlayerCharacterCommittedGroupsLikeCpp,
    ) {
        let _ = (receipt, committed);
    }
}

/// C++ `Player::SaveToDB` (`Player.cpp:19312`): the complete represented save.
///
/// Stage order, preserved exactly: autosave timer → transfer gate → money
/// admission fence → durable Item-loot wait → Item-loot completion publication →
/// detached payout reconciliation → exclusive money mutation lock → capture →
/// one-transaction persist → incarnation-bound ACK → unlock → represented
/// objective drain. `Unavailable`, `Deferred` and the unknown-COMMIT quarantine
/// keep their existing meanings.
pub async fn save_current_player_to_db_with_generator_like_cpp<H>(
    host: &mut H,
    item_guid_generator: &ObjectGuidGenerator,
) -> PlayerSaveOutcomeLikeCpp
where
    H: PlayerSaveHostLikeCpp + Send,
{
    // The two fences are held for the whole capture/commit/ACK window, exactly
    // as before the move: the money admission fence and the mutation lock.
    let (money_save_fence, money_tracker) = {
        let PlayerSaveParticipantsLikeCpp {
            lifecycle,
            mut operation,
        } = host.player_save_participants_like_cpp();
        // C++ `Player::SaveToDB` delays the next autosave for manual, code, and
        // autosave callers before it appends statements.
        lifecycle.reset_player_save_timer_like_cpp();
        if let Some(outcome) =
            lifecycle.defer_player_save_for_transfer_with_owner_like_cpp(&mut operation)
        {
            return outcome;
        }
        let money_tracker = Arc::clone(lifecycle.durable_loot_money_persistence_tracker_like_cpp());
        let money_save_fence = money_tracker.close_admission_for_save_like_cpp();
        trace!(fence = "player.save.mutations_closed", "persistence fence");
        lifecycle
            .wait_for_durable_item_loot_persistence_like_cpp()
            .await;
        (money_save_fence, money_tracker)
    };
    host.apply_pending_durable_item_loot_completions_for_player_save_like_cpp(item_guid_generator)
        .await;
    let money_state_is_determinate = host
        .reconcile_durable_loot_money_before_player_save_like_cpp()
        .await;
    if !money_state_is_determinate {
        // The same unknown transaction may also have committed talents,
        // reset metadata, inventory, or other absolute state. Do not let a
        // disconnect/autosave restore any pre-COMMIT runtime snapshot.
        drop(money_save_fence);
        host.drain_represented_quest_objective_progress_for_player_save_like_cpp(
            item_guid_generator,
        )
        .await;
        return PlayerSaveOutcomeLikeCpp::Quarantined;
    }
    trace!(
        fence = "player.save.pending_durable_work_drained",
        "persistence fence"
    );
    let money_mutation_lock = money_tracker.lock_money_mutation_like_cpp().await;
    if money_tracker.is_indeterminate_like_cpp() {
        host.quarantine_player_save_like_cpp(
            "player persistence became indeterminate while waiting for the full-save money lock; aborting the entire save",
        );
        drop(money_mutation_lock);
        drop(money_save_fence);
        host.drain_represented_quest_objective_progress_for_player_save_like_cpp(
            item_guid_generator,
        )
        .await;
        return PlayerSaveOutcomeLikeCpp::Quarantined;
    }

    // Legacy ownerless fixture sessions never reach the canonical owner. The
    // World test harness supplies their capture and acknowledgement; an ownerless
    // production session has no fixture capture and falls through below.
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(captured) = host.capture_ownerless_player_save_like_cpp(unix_now()) {
        let guid = captured.guid;
        let result = {
            let PlayerSaveParticipantsLikeCpp {
                lifecycle,
                mut operation,
            } = host.player_save_participants_like_cpp();
            persist_player_save_request_like_cpp(lifecycle, &mut operation, guid, captured.request)
                .await
        };
        let outcome = match result {
            PlayerSavePersistenceResultLikeCpp::Applied {
                rows, committed, ..
            } => {
                host.acknowledge_ownerless_player_save_like_cpp(captured.receipt, &committed);
                trace!(
                    publication = "player.save.commit_confirmed",
                    "persistence publication"
                );
                info!(
                    guid = guid.counter(),
                    statement_count = rows,
                    "Player::SaveToDB represented save committed in one CharacterDatabase transaction"
                );
                PlayerSaveOutcomeLikeCpp::Applied
            }
            PlayerSavePersistenceResultLikeCpp::Failed { .. } => PlayerSaveOutcomeLikeCpp::Failed,
            PlayerSavePersistenceResultLikeCpp::Unknown { .. }
            | PlayerSavePersistenceResultLikeCpp::Quarantined { .. } => {
                PlayerSaveOutcomeLikeCpp::Quarantined
            }
            PlayerSavePersistenceResultLikeCpp::Unavailable
            | PlayerSavePersistenceResultLikeCpp::SnapshotUnavailable => {
                PlayerSaveOutcomeLikeCpp::Unavailable
            }
        };
        drop(money_mutation_lock);
        drop(money_save_fence);
        host.drain_represented_quest_objective_progress_for_player_save_like_cpp(
            item_guid_generator,
        )
        .await;
        return outcome;
    }

    let result = {
        let PlayerSaveParticipantsLikeCpp {
            lifecycle,
            mut operation,
        } = host.player_save_participants_like_cpp();
        save_canonical_player_like_cpp(lifecycle, &mut operation, unix_now()).await
    };
    let outcome = match result {
        PlayerSavePersistenceResultLikeCpp::Applied {
            player_guid,
            rows,
            registry_sync_required,
            ..
        } => {
            if registry_sync_required {
                host.sync_player_save_registry_state_like_cpp();
            }
            trace!(
                publication = "player.save.commit_confirmed",
                "persistence publication"
            );
            info!(
                guid = player_guid.counter(),
                statement_count = rows,
                "Player::SaveToDB represented save committed in one CharacterDatabase transaction"
            );
            PlayerSaveOutcomeLikeCpp::Applied
        }
        PlayerSavePersistenceResultLikeCpp::Failed { .. } => PlayerSaveOutcomeLikeCpp::Failed,
        PlayerSavePersistenceResultLikeCpp::Unknown { .. } => PlayerSaveOutcomeLikeCpp::Quarantined,
        PlayerSavePersistenceResultLikeCpp::Unavailable => PlayerSaveOutcomeLikeCpp::Unavailable,
        PlayerSavePersistenceResultLikeCpp::Quarantined { .. } => {
            PlayerSaveOutcomeLikeCpp::Quarantined
        }
        PlayerSavePersistenceResultLikeCpp::SnapshotUnavailable => {
            // Preserve the transfer check and warning after a failed
            // canonical capture, at the original post-fence phase.
            let outcome = {
                let PlayerSaveParticipantsLikeCpp {
                    lifecycle,
                    mut operation,
                } = host.player_save_participants_like_cpp();
                lifecycle.defer_player_save_for_transfer_with_owner_like_cpp(&mut operation)
            }
            .unwrap_or(PlayerSaveOutcomeLikeCpp::Unavailable);
            let diagnostics = host.player_save_diagnostics_like_cpp();
            warn!(
                account = diagnostics.account_id,
                player_guid = ?diagnostics.player_guid,
                has_session_position = diagnostics.has_session_position,
                has_canonical_map_manager = diagnostics.has_canonical_map_manager,
                "Skipping Player::SaveToDB represented save because no coherent player snapshot is available"
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            host.drain_represented_quest_objective_progress_for_player_save_like_cpp(
                item_guid_generator,
            )
            .await;
            return outcome;
        }
    };
    drop(money_mutation_lock);
    drop(money_save_fence);
    host.drain_represented_quest_objective_progress_for_player_save_like_cpp(item_guid_generator)
        .await;
    outcome
}
