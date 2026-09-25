// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Guild inventory contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildRepairBankStateLikeCpp {
    pub available_repair_money: u64,
    pub withdraw_repair_money_allowed: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildRepairBankWithdrawLikeCpp {
    pub amount: u64,
    pub repair: bool,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBankItemMoveLikeCpp {
    pub to_bank: bool,
    pub inv_update_items: Vec<(u8, u8)>,
    pub bag: u8,
    pub slot: u8,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildBankInventoryMoveLikeCpp {
    pub banker: ObjectGuid,
    pub guild_id: u64,
    pub to_char: bool,
    pub bank_tab: u8,
    pub bank_slot: u8,
    pub player_bag: u8,
    pub player_slot: u8,
    pub stack_count: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildBankListRequestLikeCpp {
    pub banker: ObjectGuid,
    pub guild_id: u64,
    pub tab: u8,
    pub full_update: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildBankMoneyMoveLikeCpp {
    pub banker: ObjectGuid,
    pub guild_id: u64,
    pub deposit: bool,
    pub money: u64,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedGuildBankTabActionLikeCpp {
    pub banker: Option<ObjectGuid>,
    pub guild_id: u64,
    pub tab: i32,
    pub action: RepresentedGuildBankTabActionKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepresentedGuildBankTabActionKindLikeCpp {
    Buy,
    Update { name: String, icon: String },
    LogQuery,
    TextQuery,
    SetText { text: String },
}
