// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free Player full-save lifecycle orchestration.
//!
//! Session snapshots represented Player state; the lifecycle adapter privately
//! owns statement decomposition and single-transaction execution (#286).

use std::sync::Arc;

#[cfg(test)]
mod fixture_tests;
mod prepared;
mod projection;

use tracing::{info, trace, warn};
use wow_persistence::{
    PlayerActionButtonSaveLikeCpp, PlayerActionButtonsSaveLikeCpp,
    PlayerCharacterCommittedGroupsLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSnapshotSaveLikeCpp, PlayerCufProfileSaveLikeCpp,
    PlayerCufProfileSlotSaveLikeCpp, PlayerEquipmentSetSaveLikeCpp, PlayerEquipmentSetStateLikeCpp,
    PlayerEquipmentSetTypeLikeCpp, PlayerFallbackSpellSaveLikeCpp, PlayerGlyphSaveLikeCpp,
    PlayerInstanceLockTimeSaveLikeCpp, PlayerPlayedTimeSaveLikeCpp, PlayerPositionSaveLikeCpp,
    PlayerReputationSaveLikeCpp, PlayerSkillSaveLikeCpp, PlayerSpellChargeSaveLikeCpp,
    PlayerSpellCooldownSaveLikeCpp, PlayerSpellSaveGroupLikeCpp, PlayerSpellSaveLikeCpp,
    PlayerSpellStateLikeCpp, PlayerTalentSaveLikeCpp, PlayerTutorialsSaveLikeCpp,
    PlayerVoidStorageSaveLikeCpp, PlayerVoidStorageSlotSaveLikeCpp,
};

use super::super::{
    AbsolutePlayerMoneyCommitReconciliationLikeCpp, ExclusivePlayerMoneyPersistenceLikeCpp,
    PlayerMoneyCommitCancellationFenceLikeCpp, PlayerSaveToDbSnapshotLikeCpp,
    RepresentedEquipmentSetTypeLikeCpp, RepresentedEquipmentSetUpdateStateLikeCpp,
    RepresentedPlayerSkillStateLikeCpp, RepresentedPlayerSpellStateLikeCpp, WorldSession,
    character_power_snapshot_values_like_cpp, reconcile_absolute_player_money_commit_like_cpp,
    unix_now,
};

impl WorldSession {
    /// Await a typed adapter transaction while the cancellation fence and the
    /// Session-owned money exclusion remain active. The adapter observes the
    /// durable money marker; Session owns reconciliation and quarantine.
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
        let mut cancellation_fence = PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(
            &self.durable_loot_money_persistence_like_cpp,
        ));
        match outcome_future.await {
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed => {
                cancellation_fence.disarm_like_cpp();
                Some(money_persistence)
            }
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
                reason,
            } => {
                cancellation_fence.disarm_like_cpp();
                warn!(error = %reason, operation, "player-money transaction definitely rolled back");
                None
            }
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
                reason,
                observed_money,
            } => match reconcile_absolute_player_money_commit_like_cpp(
                money_before,
                money_after,
                observed_money,
            ) {
                AbsolutePlayerMoneyCommitReconciliationLikeCpp::Committed => {
                    cancellation_fence.disarm_like_cpp();
                    warn!(
                        error = %reason,
                        operation,
                        money_before,
                        money_after,
                        "player-money COMMIT reply was lost but durable money proves the transaction committed"
                    );
                    Some(money_persistence)
                }
                AbsolutePlayerMoneyCommitReconciliationLikeCpp::RolledBack => {
                    cancellation_fence.disarm_like_cpp();
                    warn!(
                        error = %reason,
                        operation,
                        money_before,
                        money_after,
                        "player-money COMMIT reply was lost but durable money proves the transaction rolled back"
                    );
                    None
                }
                AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate => {
                    self.durable_loot_money_persistence_like_cpp
                        .mark_indeterminate_like_cpp();
                    cancellation_fence.disarm_like_cpp();
                    self.kick(
                        "player-money COMMIT outcome is unknown; relog required before another money mutation",
                    );
                    warn!(
                        error = %reason,
                        operation,
                        money_before,
                        money_after,
                        ?observed_money,
                        "player-money COMMIT outcome remains indeterminate; quarantined the session"
                    );
                    None
                }
            },
        }
    }

    pub(crate) async fn save_current_player_to_db_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        // C++ `Player::SaveToDB` delays the next autosave for manual, code, and
        // autosave callers before it appends statements.
        self.reset_player_save_timer_like_cpp();

        let money_tracker = Arc::clone(&self.durable_loot_money_persistence_like_cpp);
        let money_save_fence = money_tracker.close_admission_for_save_like_cpp();
        trace!(fence = "player.save.mutations_closed", "persistence fence");
        self.wait_for_durable_item_loot_persistence_like_cpp().await;
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
            return;
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
            return;
        }

        let Some(prepared) = self.prepare_player_save_like_cpp(unix_now()) else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                has_session_position = self.player_position_like_cpp().is_some(),
                has_canonical_map_manager = self.canonical_map_manager.is_some(),
                "Skipping Player::SaveToDB represented save because no coherent player snapshot is available"
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
            return;
        };
        let Some(player_lifecycle_port) = self.player_lifecycle_port_like_cpp().map(Arc::clone)
        else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping Player::SaveToDB represented save because lifecycle persistence is unavailable"
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
            return;
        };

        if money_tracker.is_indeterminate_like_cpp() {
            self.kick(
                "player persistence became indeterminate before the full-save semantic snapshot; aborting the entire save",
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
            return;
        }
        let mut cancellation_fence =
            PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(&money_tracker));
        let guid = prepared.header.guid;
        // Only the request crosses the asynchronous persistence boundary; the
        // receipt remains owned by this operation and contains no borrowed guard.
        let result = player_lifecycle_port
            .save_character_like_cpp(prepared.request)
            .await;
        match result.outcome {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows } => {
                cancellation_fence.disarm_like_cpp();
                prepared.receipt.acknowledge(self, &result.committed);
                trace!(
                    publication = "player.save.commit_confirmed",
                    "persistence publication"
                );
                info!(
                    guid = guid.counter(),
                    statement_count = rows,
                    "Player::SaveToDB represented save committed in one CharacterDatabase transaction"
                );
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => {
                cancellation_fence.disarm_like_cpp();
                warn!(
                    guid = guid.counter(),
                    "Failed to commit Player::SaveToDB represented transaction: {reason}"
                );
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                // The full save includes many absolute replacements. A money
                // row alone cannot establish whether that whole transaction
                // committed, so preserve dirty flags and force a reload before
                // any further money mutation can race an unknown durable base.
                money_tracker.mark_indeterminate_like_cpp();
                trace!(fence = "player.save.relogin_required", "persistence fence");
                cancellation_fence.disarm_like_cpp();
                self.kick(
                    "Player::SaveToDB COMMIT outcome is unknown; relog required before another money mutation",
                );
                warn!(
                    guid = guid.counter(),
                    "Player::SaveToDB represented transaction COMMIT outcome is unknown: {reason}"
                );
            }
        }
        drop(money_mutation_lock);
        drop(money_save_fence);
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn save_current_player_to_db_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        self.save_current_player_to_db_with_generator_like_cpp(generators.item.as_ref())
            .await;
    }
}
