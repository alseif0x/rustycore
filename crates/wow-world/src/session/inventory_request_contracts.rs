// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory request contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::RepresentedEquipmentSetUpdateStateLikeCpp;
use super::{ObjectGuid, RepresentedEquipmentSetLikeCpp, RepresentedEquipmentSetTypeLikeCpp};

/// Detached inventory state already reserved by an earlier operation in the
/// same atomic storage plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectInventoryStorageOverlayLikeCpp {
    pub(crate) bag: u8,
    pub(crate) slot: u8,
    pub(crate) entry_id: u32,
    pub(crate) count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedAutoUnequipOffhandReasonLikeCpp {
    Forced,
    LostDualWield,
    InvalidTwoHandState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAutoUnequipOffhandLikeCpp {
    pub item_guid: ObjectGuid,
    pub item_entry: u32,
    pub reason: RepresentedAutoUnequipOffhandReasonLikeCpp,
    pub stored_destination: Option<(u8, u8)>,
    pub needs_mail_fallback: bool,
}

pub(crate) const MAX_EQUIPMENT_SET_INDEX_LIKE_CPP: u32 = 20;

pub(in crate::session) fn represented_equipment_set_from_packet_like_cpp(
    set: wow_packet::packets::misc::EquipmentSetDataLikeCpp,
    guid: u64,
    state: RepresentedEquipmentSetUpdateStateLikeCpp,
) -> Option<RepresentedEquipmentSetLikeCpp> {
    let set_type =
        RepresentedEquipmentSetTypeLikeCpp::handler_branch_from_i32_like_cpp(set.set_type)?;
    Some(RepresentedEquipmentSetLikeCpp {
        raw_set_type: set.set_type,
        set_type,
        guid,
        set_id: set.set_id,
        ignore_mask: set.ignore_mask,
        pieces: set.pieces,
        appearances: set.appearances,
        enchants: set.enchants,
        secondary_shoulder_appearance_id: set.secondary_shoulder_appearance_id,
        secondary_shoulder_slot: set.secondary_shoulder_slot,
        secondary_weapon_appearance_id: set.secondary_weapon_appearance_id,
        secondary_weapon_slot: set.secondary_weapon_slot,
        assigned_spec_index: set.assigned_spec_index,
        set_name: set.set_name,
        set_icon: set.set_icon,
        state,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedEquipmentSetSavedLikeCpp {
    pub(crate) guid: u64,
    pub(crate) set_type: RepresentedEquipmentSetTypeLikeCpp,
    pub(crate) raw_set_type: i32,
    pub(crate) set_id: u32,
    pub(crate) generated_new_guid: bool,
}

/// Current finite stock for a vendor item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VendorItemCount {
    pub count: u32,
    pub last_increment_time: u64,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VendorBuyItemTestOverrideLikeCpp {
    pub(crate) item_id: u32,
    pub(crate) item_type: i32,
    pub(crate) max_count: u32,
    pub(crate) incr_time: u32,
    pub(crate) player_condition_id: u32,
    pub(crate) has_vendor_conditions: bool,
    pub(crate) extended_cost: u32,
    pub(crate) buy_price: u64,
    pub(crate) max_durability: u32,
    pub(crate) buy_count: u32,
}
