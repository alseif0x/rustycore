// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free Player full-save lifecycle orchestration.
//!
//! Session snapshots represented Player state; the lifecycle adapter privately
//! owns statement decomposition and single-transaction execution (#286).

use std::sync::Arc;

mod deferred;
#[cfg(test)]
#[path = "../../../unit_tests/session/lifecycle/persistence/fixture_tests.rs"]
mod fixture_tests;
pub use deferred::PlayerSaveOutcomeLikeCpp;
mod prepared;

use tracing::{info, trace, warn};
use wow_persistence::{
    PlayerCharacterCommittedGroupsLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerTutorialsSaveLikeCpp,
};

use super::super::{
    ExclusivePlayerMoneyPersistenceLikeCpp, PlayerSaveToDbSnapshotLikeCpp, WorldSession, unix_now,
};

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

    pub(crate) async fn save_current_player_to_db_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> PlayerSaveOutcomeLikeCpp {
        // C++ `Player::SaveToDB` delays the next autosave for manual, code, and
        // autosave callers before it appends statements.
        self.lifecycle.reset_player_save_timer_like_cpp();
        if let Some(outcome) = self.defer_player_save_for_transfer_like_cpp() {
            return outcome;
        }

        let money_tracker = Arc::clone(
            self.lifecycle
                .durable_loot_money_persistence_tracker_like_cpp(),
        );
        let money_save_fence = money_tracker.close_admission_for_save_like_cpp();
        trace!(fence = "player.save.mutations_closed", "persistence fence");
        crate::session::cx_inventory_ref(self)
            .wait_for_durable_item_loot_persistence_like_cpp()
            .await;
        self.apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp(
            item_guid_generator,
            false,
        )
        .await;
        let money_state_is_determinate = self
            .reconcile_durable_loot_money_before_save_like_cpp()
            .await;
        if !money_state_is_determinate {
            // The same unknown transaction may also have committed talents,
            // reset metadata, inventory, or other absolute state. Do not let a
            // disconnect/autosave restore any pre-COMMIT runtime snapshot.
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
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
            self.kick(
                "player persistence became indeterminate while waiting for the full-save money lock; aborting the entire save",
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
            return PlayerSaveOutcomeLikeCpp::Quarantined;
        }

        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            let Some(prepared) = self.prepare_player_save_like_cpp(unix_now()) else {
                let outcome = self
                    .defer_player_save_for_transfer_like_cpp()
                    .unwrap_or(PlayerSaveOutcomeLikeCpp::Unavailable);
                warn!(
                    account = self.core.account_id,
                    player_guid = ?self.player_guid(),
                    has_session_position = crate::session::hub_ref(self).player_position_like_cpp().is_some(),
                    has_canonical_map_manager = self.core.canonical_map_manager.is_some(),
                    "Skipping Player::SaveToDB represented save because no coherent player snapshot is available"
                );
                drop(money_mutation_lock);
                drop(money_save_fence);
                self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                    item_guid_generator,
                )
                .await;
                return outcome;
            };
            let guid = prepared.header.guid;
            let talent_store = self.catalogs.talent_store().map(AsRef::as_ref);
            let spell_store = self
                .catalogs
                .spell_catalogs
                .spell_store()
                .map(AsRef::as_ref);
            let mut save_owner = self
                .core
                .player_save_operation_access_like_cpp(talent_store, spell_store);
            let result = wow_world_application::persist_player_save_request_like_cpp(
                &self.lifecycle,
                &mut save_owner,
                guid,
                prepared.request,
            )
            .await;
            drop(save_owner);
            let outcome = match result {
                wow_world_application::PlayerSavePersistenceResultLikeCpp::Applied {
                    rows,
                    committed,
                    ..
                } => {
                    prepared.receipt.acknowledge(self, &committed);
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
                wow_world_application::PlayerSavePersistenceResultLikeCpp::Failed { .. } => {
                    PlayerSaveOutcomeLikeCpp::Failed
                }
                wow_world_application::PlayerSavePersistenceResultLikeCpp::Unknown { .. }
                | wow_world_application::PlayerSavePersistenceResultLikeCpp::Quarantined {
                    ..
                } => PlayerSaveOutcomeLikeCpp::Quarantined,
                wow_world_application::PlayerSavePersistenceResultLikeCpp::Unavailable
                | wow_world_application::PlayerSavePersistenceResultLikeCpp::SnapshotUnavailable => {
                    PlayerSaveOutcomeLikeCpp::Unavailable
                }
            };
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
            return outcome;
        }

        let talent_store = self.catalogs.talent_store().map(AsRef::as_ref);
        let spell_store = self
            .catalogs
            .spell_catalogs
            .spell_store()
            .map(AsRef::as_ref);
        let mut save_owner = self
            .core
            .player_save_operation_access_like_cpp(talent_store, spell_store);
        let result = wow_world_application::save_canonical_player_like_cpp(
            &mut self.lifecycle,
            &mut save_owner,
            unix_now(),
        )
        .await;
        drop(save_owner);
        let outcome = match result {
            wow_world_application::PlayerSavePersistenceResultLikeCpp::Applied {
                player_guid,
                rows,
                registry_sync_required,
                ..
            } => {
                if registry_sync_required {
                    self.sync_player_registry_state_like_cpp();
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
            wow_world_application::PlayerSavePersistenceResultLikeCpp::Failed { .. } => {
                PlayerSaveOutcomeLikeCpp::Failed
            }
            wow_world_application::PlayerSavePersistenceResultLikeCpp::Unknown { .. } => {
                PlayerSaveOutcomeLikeCpp::Quarantined
            }
            wow_world_application::PlayerSavePersistenceResultLikeCpp::Unavailable => {
                PlayerSaveOutcomeLikeCpp::Unavailable
            }
            wow_world_application::PlayerSavePersistenceResultLikeCpp::Quarantined { .. } => {
                PlayerSaveOutcomeLikeCpp::Quarantined
            }
            wow_world_application::PlayerSavePersistenceResultLikeCpp::SnapshotUnavailable => {
                // Preserve the transfer check and warning after a failed
                // canonical capture, at the original post-fence phase.
                let outcome = self
                    .defer_player_save_for_transfer_like_cpp()
                    .unwrap_or(PlayerSaveOutcomeLikeCpp::Unavailable);
                warn!(
                    account = self.core.account_id,
                    player_guid = ?self.player_guid(),
                    has_session_position = crate::session::hub_ref(self).player_position_like_cpp().is_some(),
                    has_canonical_map_manager = self.core.canonical_map_manager.is_some(),
                    "Skipping Player::SaveToDB represented save because no coherent player snapshot is available"
                );
                drop(money_mutation_lock);
                drop(money_save_fence);
                self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                    item_guid_generator,
                )
                .await;
                return outcome;
            }
        };
        drop(money_mutation_lock);
        drop(money_save_fence);
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
        outcome
    }

    #[cfg(test)]
    pub(crate) async fn save_current_player_to_db_like_cpp(&mut self) -> PlayerSaveOutcomeLikeCpp {
        let generators = self.id_generators_for_test_like_cpp();
        self.save_current_player_to_db_with_generator_like_cpp(generators.item.as_ref())
            .await
    }
}
