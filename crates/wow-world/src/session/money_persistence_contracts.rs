// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Money persistence contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

pub(crate) use wow_world_lifecycle::{
    ExclusivePlayerMoneyPersistenceLikeCpp, LootMoneyPersistenceErrorLikeCpp,
    PlayerMoneyCommitCancellationFenceLikeCpp,
};
pub(in crate::session) use wow_world_lifecycle::{
    AbsolutePlayerMoneyCommitReconciliationLikeCpp,
    RepresentedTalentResetStatePlanLikeCpp,
    reconcile_absolute_player_money_commit_like_cpp,
};

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
