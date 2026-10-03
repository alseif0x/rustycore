use std::sync::Arc;

use wow_world_core::loot_persistence::{
    DurableLootMoneyPersistenceTrackerLikeCpp, DurableLootMoneySaveFenceLikeCpp,
};

/// Owned exclusion held while an absolute money mutation derives and persists
/// its new value. Admission stays closed and the same per-character serial
/// lock used by group/stored loot remains held through the caller's COMMIT.
#[must_use]
pub struct ExclusivePlayerMoneyPersistenceLikeCpp {
    _save_fence: DurableLootMoneySaveFenceLikeCpp,
    _mutation_lock: tokio::sync::OwnedMutexGuard<()>,
}

impl ExclusivePlayerMoneyPersistenceLikeCpp {
    pub fn new(
        save_fence: DurableLootMoneySaveFenceLikeCpp,
        mutation_lock: tokio::sync::OwnedMutexGuard<()>,
    ) -> Self {
        Self {
            _save_fence: save_fence,
            _mutation_lock: mutation_lock,
        }
    }
}

/// Once a SQL transaction containing an absolute money write starts awaiting
/// COMMIT, cancellation is itself an unknown outcome. This synchronous drop
/// fence prevents a cancelled packet/shutdown future from reopening payout
/// admission and then letting disconnect-save overwrite a transaction whose
/// COMMIT reply was never observed.
pub struct PlayerMoneyCommitCancellationFenceLikeCpp {
    tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    armed: bool,
}

impl PlayerMoneyCommitCancellationFenceLikeCpp {
    pub fn new(tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>) -> Self {
        Self {
            tracker,
            armed: true,
        }
    }

    /// Build a fence which is inert until the database layer is immediately
    /// about to await a COMMIT. Multi-step sagas use this form so cancelling
    /// during pre-commit validation does not quarantine a session whose
    /// transaction was never submitted.
    pub fn new_disarmed_like_cpp(
        tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    ) -> Self {
        Self {
            tracker,
            armed: false,
        }
    }

    pub fn arm_like_cpp(&mut self) {
        self.armed = true;
    }

    pub fn disarm_like_cpp(&mut self) {
        self.armed = false;
    }
}

impl wow_persistence::BattlePetPurchaseCommitFenceLikeCpp
    for PlayerMoneyCommitCancellationFenceLikeCpp
{
    fn arm_like_cpp(&mut self) {
        PlayerMoneyCommitCancellationFenceLikeCpp::arm_like_cpp(self);
    }

    fn disarm_like_cpp(&mut self) {
        PlayerMoneyCommitCancellationFenceLikeCpp::disarm_like_cpp(self);
    }
}

impl Drop for PlayerMoneyCommitCancellationFenceLikeCpp {
    fn drop(&mut self) {
        if self.armed {
            self.tracker.mark_indeterminate_like_cpp();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbsolutePlayerMoneyCommitReconciliationLikeCpp {
    Committed,
    RolledBack,
    Indeterminate,
}

/// Reconcile an ambiguous COMMIT using a money row whose value changed in the
/// transaction. Equal before/after values are deliberately not evidence: a
/// caller may have bundled other durable mutations whose outcome cannot be
/// inferred from an unchanged money column.
pub fn reconcile_absolute_player_money_commit_like_cpp(
    money_before: u64,
    money_after: u64,
    observed_money: Option<u64>,
) -> AbsolutePlayerMoneyCommitReconciliationLikeCpp {
    if money_before == money_after {
        return AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate;
    }

    match observed_money {
        Some(observed) if observed == money_after => {
            AbsolutePlayerMoneyCommitReconciliationLikeCpp::Committed
        }
        Some(observed) if observed == money_before => {
            AbsolutePlayerMoneyCommitReconciliationLikeCpp::RolledBack
        }
        Some(_) | None => AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate,
    }
}
