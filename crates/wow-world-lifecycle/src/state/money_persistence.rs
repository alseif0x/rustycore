#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::BTreeSet;
use std::sync::Arc;

use super::SessionLifecycleState;
use crate::{
    AbsolutePlayerMoneyCommitReconciliationLikeCpp, ExclusivePlayerMoneyPersistenceLikeCpp,
    PlayerMoneyCommitCancellationFenceLikeCpp, reconcile_absolute_player_money_commit_like_cpp,
};
use tracing::warn;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::HubRef;
use wow_world_core::session::{HubMut, PlayerMoneyTransactionSessionAccessLikeCpp};

impl SessionLifecycleState {
    pub fn durable_loot_money_persistence_tracker_like_cpp(
        &self,
    ) -> &Arc<wow_world_core::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp> {
        &self.durable_loot_money_persistence_like_cpp
    }

    pub fn reset_durable_loot_money_persistence_tracker_like_cpp(&mut self) {
        self.durable_loot_money_persistence_like_cpp = Arc::new(
            wow_world_core::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp::default(),
        );
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn loot_money_persistence_test_result_like_cpp(&self) -> Option<bool> {
        self.loot_money_persistence_test_result_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_loot_money_persistence_test_result_like_cpp(&mut self, value: Option<bool>) {
        self.loot_money_persistence_test_result_like_cpp = value;
    }

    /// Commit a trainer fee when the represented cast has no durable
    /// spell/skill mutation (for example, every acquisition effect was
    /// suppressed by target immunity). C++ charges and publishes its trainer
    /// visuals before that triggered cast resolves its hit effects.
    pub async fn commit_exclusive_trainer_money_only_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        money_before: u64,
        money_after: u64,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
        let mut access = hub.core.player_money_transaction_access_like_cpp();
        self.commit_exclusive_trainer_money_only_with_access_like_cpp(
            &mut access,
            money_persistence,
            money_before,
            money_after,
        )
        .await
    }

    /// Access-based form used by the trainer application while its single
    /// SessionCore capability remains borrowed through the persistence await.
    pub async fn commit_exclusive_trainer_money_only_with_access_like_cpp(
        &mut self,
        access: &mut PlayerMoneyTransactionSessionAccessLikeCpp<'_>,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        money_before: u64,
        money_after: u64,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(success) = self.loot_money_persistence_test_result_like_cpp {
            return success.then_some(money_persistence);
        }

        if money_before == money_after {
            return Some(money_persistence);
        }
        let guid = access.player_guid()?.counter() as u64;
        let port = self.player_lifecycle_port_like_cpp().map(Arc::clone)?;
        let request = wow_persistence::PlayerMoneyTransactionRequestLikeCpp {
            player_guid: guid,
            money_after,
            durability_repairs: Vec::new(),
        };
        self.await_exclusive_player_money_transaction_outcome_with_access_like_cpp(
            access,
            money_persistence,
            port.persist_money_transaction_like_cpp(request),
            money_before,
            money_after,
            "trainer fee without durable acquisition mutation",
        )
        .await
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_skill_non_durable_tombstones_like_cpp(&self, hub: HubRef<'_>) -> BTreeSet<u16> {
        hub.resolved_player_skill_non_durable_tombstones_like_cpp()
            .expect("test Player skill owner must resolve")
    }

    /// Await a typed adapter transaction while the cancellation fence and the
    /// Session-owned money exclusion remain active. The adapter observes the
    /// durable money marker; Session owns reconciliation and quarantine.
    pub async fn await_exclusive_player_money_transaction_outcome_like_cpp<F>(
        &mut self,
        hub: &mut HubMut<'_>,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        outcome_future: F,
        money_before: u64,
        money_after: u64,
        operation: &'static str,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp>
    where
        F: std::future::Future<Output = wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp>,
    {
        let mut access = hub.core.player_money_transaction_access_like_cpp();
        self.await_exclusive_player_money_transaction_outcome_with_access_like_cpp(
            &mut access,
            money_persistence,
            outcome_future,
            money_before,
            money_after,
            operation,
        )
        .await
    }

    /// Access-based outcome classifier; the capability is used only for
    /// session identity and immediate quarantine, never for Player storage.
    pub async fn await_exclusive_player_money_transaction_outcome_with_access_like_cpp<F>(
        &mut self,
        access: &mut PlayerMoneyTransactionSessionAccessLikeCpp<'_>,
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
                    access.quarantine_like_cpp(
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
}
