// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory request contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

pub(crate) use wow_world_inventory::{
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedAutoUnequipOffhandReasonLikeCpp,
};
pub(crate) use wow_world_inventory::{
    MAX_EQUIPMENT_SET_INDEX_LIKE_CPP, RepresentedEquipmentSetSavedLikeCpp,
};
pub(in crate::session) use wow_world_inventory::represented_equipment_set_from_packet_like_cpp;

/// Detached inventory state already reserved by an earlier operation in the
/// same atomic storage plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectInventoryStorageOverlayLikeCpp {
    pub(crate) bag: u8,
    pub(crate) slot: u8,
    pub(crate) entry_id: u32,
    pub(crate) count: u32,
}

pub(crate) use wow_world_interaction::VendorItemCount;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_interaction::VendorBuyItemTestOverrideLikeCpp;
