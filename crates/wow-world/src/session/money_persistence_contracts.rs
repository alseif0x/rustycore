// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Money persistence contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
pub(crate) use wow_world_lifecycle::PlayerMoneyCommitCancellationFenceLikeCpp;
pub(in crate::session) use wow_world_lifecycle::{
    AbsolutePlayerMoneyCommitReconciliationLikeCpp, reconcile_absolute_player_money_commit_like_cpp,
};
pub(crate) use wow_world_lifecycle::{
    ExclusivePlayerMoneyPersistenceLikeCpp, LootMoneyPersistenceErrorLikeCpp,
};
