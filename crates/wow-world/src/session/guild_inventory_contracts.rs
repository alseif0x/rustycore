// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Compatibility paths for Session-owned guild inventory contracts.

pub(crate) use wow_world_inventory::RepresentedBankItemMoveLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_inventory::{
    RepresentedGuildBankInventoryMoveLikeCpp, RepresentedGuildBankListRequestLikeCpp,
    RepresentedGuildBankMoneyMoveLikeCpp, RepresentedGuildBankTabActionKindLikeCpp,
    RepresentedGuildBankTabActionLikeCpp, RepresentedGuildRepairBankStateLikeCpp,
    RepresentedGuildRepairBankWithdrawLikeCpp,
};
