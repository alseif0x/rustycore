// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Money persistence contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, BTreeMap, DurableLootMoneyPersistenceTrackerLikeCpp};
use super::{DurableLootMoneySaveFenceLikeCpp, MAX_SPECIALIZATIONS_LIKE_CPP};

#[derive(Debug)]
pub(crate) enum LootMoneyPersistenceErrorLikeCpp {
    MissingPlayer,
    MissingCharacterDatabase,
    WorkerTerminated,
    Claim(wow_loot::LootClaimCommitError),
    Persistence(String),
    CommitOutcomeUnknownPersistence(String),
}

/// Owned exclusion held while an absolute character-money mutation derives
/// and persists its new value. Admission stays closed and the same
/// per-character serial lock used by group/stored loot remains held through
/// the caller's COMMIT.
#[must_use]
pub(crate) struct ExclusivePlayerMoneyPersistenceLikeCpp {
    pub(in crate::session) _save_fence: DurableLootMoneySaveFenceLikeCpp,
    pub(in crate::session) _mutation_lock: tokio::sync::OwnedMutexGuard<()>,
}

/// Pure post-reset snapshot used to make the durable talent reset and its
/// runtime publication describe the same state.
///
/// C++ `Player::ResetTalents` calls `RemoveTalent` for the active group, then
/// `_SaveTalents` rewrites every group. `RemoveTalent` calls
/// `RemoveSpell(..., disabled=true)`, and the complete C++ `_SaveSpells` path
/// rewrites those rows with their exact active/disabled/favorite state. Rust
/// does not retain the full `PlayerSpellMap` active/disabled/temporary state,
/// so this deliberately leaves `character_spell` and
/// `character_spell_favorite` untouched. A known non-dependent spell proves a
/// normal persisted row exists; it does not prove that the talent is its only
/// source. Deleting that row would lose normal ownership where C++ instead
/// preserves a disabled row. Recursive `RemoveSpell` persistence and exact
/// disabled/favorite preservation therefore remain a represented boundary,
/// while active `character_talent` rows can no longer survive a committed fee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) struct RepresentedTalentResetStatePlanLikeCpp {
    pub(in crate::session) active_group: u8,
    pub(in crate::session) active_talents: BTreeMap<u32, u8>,
    pub(in crate::session) post_talents: [BTreeMap<u32, u8>; MAX_SPECIALIZATIONS_LIKE_CPP],
}

/// A successfully committed reset whose covered runtime state still has to be
/// published synchronously before the money exclusion is released.
#[must_use]
pub(crate) struct CommittedRepresentedTalentResetLikeCpp {
    pub(in crate::session) money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
    pub(in crate::session) old_money: u64,
    pub(in crate::session) new_money: u64,
    pub(in crate::session) cost: u32,
    pub(in crate::session) reset_time_secs: u64,
    pub(in crate::session) state_plan: RepresentedTalentResetStatePlanLikeCpp,
}

/// Once a SQL transaction containing an absolute money write starts awaiting
/// COMMIT, cancellation is itself an unknown outcome. This synchronous drop
/// fence prevents a cancelled packet/shutdown future from reopening payout
/// admission and then letting disconnect-save overwrite a transaction whose
/// COMMIT reply was never observed.
pub(crate) struct PlayerMoneyCommitCancellationFenceLikeCpp {
    pub(in crate::session) tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    pub(in crate::session) armed: bool,
}

impl PlayerMoneyCommitCancellationFenceLikeCpp {
    pub(in crate::session) fn new(tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>) -> Self {
        Self {
            tracker,
            armed: true,
        }
    }

    /// Build a fence which is inert until the database layer is immediately
    /// about to await a COMMIT.  Multi-step sagas use this form so cancelling
    /// during pre-commit validation does not quarantine a session whose
    /// transaction was never submitted.
    pub(crate) fn new_disarmed_like_cpp(
        tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    ) -> Self {
        Self {
            tracker,
            armed: false,
        }
    }

    pub(crate) fn arm_like_cpp(&mut self) {
        self.armed = true;
    }

    pub(crate) fn disarm_like_cpp(&mut self) {
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
pub(in crate::session) enum AbsolutePlayerMoneyCommitReconciliationLikeCpp {
    Committed,
    RolledBack,
    Indeterminate,
}

/// Reconcile an ambiguous COMMIT using a money row whose value changed in the
/// transaction. Equal before/after values are deliberately not evidence: a
/// caller may have bundled other durable mutations whose outcome cannot be
/// inferred from an unchanged money column.
pub(in crate::session) fn reconcile_absolute_player_money_commit_like_cpp(
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
