// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory request value contracts owned by Session inventory.

use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedAutoUnequipOffhandReasonLikeCpp {
    Forced,
    LostDualWield,
    InvalidTwoHandState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedAutoUnequipOffhandLikeCpp {
    pub item_guid: ObjectGuid,
    pub item_entry: u32,
    pub reason: RepresentedAutoUnequipOffhandReasonLikeCpp,
    pub stored_destination: Option<(u8, u8)>,
    pub needs_mail_fallback: bool,
}
