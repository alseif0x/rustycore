use std::sync::Arc;

use super::SessionLifecycleState;
use crate::{
    AbsolutePlayerMoneyCommitReconciliationLikeCpp, ExclusivePlayerMoneyPersistenceLikeCpp,
    PlayerMoneyCommitCancellationFenceLikeCpp, reconcile_absolute_player_money_commit_like_cpp,
};
use tracing::warn;
use wow_world_core::session::HubMut;

impl SessionLifecycleState {
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
                    hub.core.kick(
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
