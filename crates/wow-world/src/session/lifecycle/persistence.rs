// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free Player full-save lifecycle adapter.
//!
//! The complete save coordinator — autosave timer, transfer gate, money fences,
//! durable-work completion drain, capture, one-transaction persist,
//! incarnation-bound acknowledgement, unlock and objective drain — is owned by
//! `wow-world-application::player_save` (#1263 F4 remate). This adapter only
//! selects the session participants it lends and supplies the session
//! publication/registry seams; it holds no stage order of its own.
//!
//! Session snapshots represented Player state; the lifecycle adapter privately
//! owns statement decomposition and single-transaction execution (#286).

use std::future::Future;

mod deferred;
#[cfg(test)]
#[path = "../../../unit_tests/session/lifecycle/persistence/fixture_tests.rs"]
mod fixture_tests;
pub use deferred::PlayerSaveOutcomeLikeCpp;
// The ownerless fixture oracle is World-test evidence only: production captures
// through the application coordinator's canonical owner.
#[cfg(test)]
mod prepared;

#[cfg(test)]
use tracing::{info, warn};
#[cfg(test)]
use wow_persistence::{
    PlayerCharacterCommittedGroupsLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerTutorialsSaveLikeCpp,
};

#[cfg(test)]
use super::super::PlayerSaveToDbSnapshotLikeCpp;
use super::super::{ExclusivePlayerMoneyPersistenceLikeCpp, WorldSession};

impl WorldSession {
    pub(crate) async fn await_exclusive_player_money_transaction_outcome_like_cpp<F>(
        &mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        outcome_future: F,
        money_before: u64,
        money_after: u64,
        operation: &'static str,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp>
    where
        F: std::future::Future<Output = wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp>,
    {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .await_exclusive_player_money_transaction_outcome_like_cpp(
                &mut hub,
                money_persistence,
                outcome_future,
                money_before,
                money_after,
                operation,
            )
            .await
    }

    /// Existing ownerless/fixture regressions drive the moved coordinator through
    /// the same entry point as the production callers.
    #[cfg(test)]
    pub(crate) async fn save_current_player_to_db_like_cpp(&mut self) -> PlayerSaveOutcomeLikeCpp {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_application::save_current_player_to_db_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
        )
        .await
    }
}

/// The World shell's capabilities for the moved full-save coordinator.
///
/// Every method below is a step the coordinator cannot reach from the
/// application crate: the session's selected owners, its durable Item-loot
/// publication, its represented objective drain, its registry publication and
/// the legacy ownerless fixture oracle. The coordinator keeps the order; these
/// only lend the existing operation at the exact point C++ reaches it.
impl wow_world_application::PlayerSaveHostLikeCpp for WorldSession {
    fn player_save_participants_like_cpp(
        &mut self,
    ) -> wow_world_application::PlayerSaveParticipantsLikeCpp<'_> {
        let talent_store = self.catalogs.talent_store().map(AsRef::as_ref);
        let spell_store = self
            .catalogs
            .spell_catalogs
            .spell_store()
            .map(AsRef::as_ref);
        wow_world_application::PlayerSaveParticipantsLikeCpp {
            lifecycle: &mut self.lifecycle,
            operation: self
                .core
                .player_save_operation_access_like_cpp(talent_store, spell_store),
        }
    }

    fn apply_pending_durable_item_loot_completions_for_player_save_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        async move {
            // The save keeps its money fence, so this publication must not drain
            // the money criteria; the post-unlock objective drain owns them.
            self.apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp(
                item_guid_generator,
                false,
            )
            .await;
        }
    }

    fn reconcile_durable_loot_money_before_player_save_like_cpp(
        &mut self,
    ) -> impl Future<Output = bool> + Send {
        WorldSession::reconcile_durable_loot_money_before_save_like_cpp(self)
    }

    fn quarantine_player_save_like_cpp(&mut self, reason: &str) {
        self.kick(reason);
    }

    fn drain_represented_quest_objective_progress_for_player_save_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        async move {
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
        }
    }

    fn sync_player_save_registry_state_like_cpp(&mut self) {
        WorldSession::sync_player_registry_state_like_cpp(self);
    }

    fn player_save_diagnostics_like_cpp(
        &self,
    ) -> wow_world_application::PlayerSaveDiagnosticsLikeCpp {
        wow_world_application::PlayerSaveDiagnosticsLikeCpp {
            account_id: self.core.account_id,
            player_guid: self.player_guid(),
            has_session_position: crate::session::hub_ref(self)
                .player_position_like_cpp()
                .is_some(),
            has_canonical_map_manager: self.core.canonical_map_manager.is_some(),
        }
    }

    #[cfg(test)]
    fn capture_ownerless_player_save_like_cpp(
        &mut self,
        now_unix_secs: i64,
    ) -> Option<wow_world_application::OwnerlessPlayerSaveCaptureLikeCpp> {
        if self.core.player_handle_like_cpp.is_some() {
            return None;
        }
        let prepared_save = self.prepare_player_save_like_cpp(now_unix_secs)?;
        let prepared::SavedPlayerReceipt::Fixture {
            expected,
            tutorials,
        } = prepared_save.receipt
        else {
            return None;
        };
        Some(wow_world_application::OwnerlessPlayerSaveCaptureLikeCpp {
            guid: prepared_save.header.guid,
            request: prepared_save.request,
            receipt: wow_world_application::OwnerlessPlayerSaveReceiptLikeCpp {
                expected,
                tutorials,
            },
        })
    }

    #[cfg(test)]
    fn acknowledge_ownerless_player_save_like_cpp(
        &mut self,
        receipt: wow_world_application::OwnerlessPlayerSaveReceiptLikeCpp,
        committed: &PlayerCharacterCommittedGroupsLikeCpp,
    ) {
        let groups = prepared::committed_groups_like_cpp(&receipt.expected, committed);
        self.mark_current_player_save_to_db_committed_like_cpp(&groups);
        let _ = receipt.tutorials;
    }
}
