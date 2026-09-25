// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player item storage, equipment and enchantment.
//!
//! Named `items`, not `inventory`: a module of the latter name shadows the
//! `inventory` crate namespace, which the handler-registration guard rejects.

use super::{
    EQUIPMENT_SLOT_BACK, EQUIPMENT_SLOT_BODY, EQUIPMENT_SLOT_CHEST, EQUIPMENT_SLOT_FEET,
    EQUIPMENT_SLOT_FINGER1, EQUIPMENT_SLOT_FINGER2, EQUIPMENT_SLOT_HANDS, EQUIPMENT_SLOT_HEAD,
    EQUIPMENT_SLOT_LEGS, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_NECK, EQUIPMENT_SLOT_OFFHAND,
    EQUIPMENT_SLOT_SHOULDERS, EQUIPMENT_SLOT_TRINKET1, EQUIPMENT_SLOT_TRINKET2,
    EQUIPMENT_SLOT_WAIST, EQUIPMENT_SLOT_WRISTS,
};
use wow_constants::InventoryType;
use wow_core::ObjectGuid;

pub fn represented_total_avg_equipment_slot_candidates_like_cpp(
    inventory_type: InventoryType,
    can_dual_wield: bool,
    can_titan_grip: bool,
) -> Vec<(u8, bool)> {
    match inventory_type {
        InventoryType::Head => vec![(EQUIPMENT_SLOT_HEAD, false)],
        InventoryType::Neck => vec![(EQUIPMENT_SLOT_NECK, false)],
        InventoryType::Shoulders => vec![(EQUIPMENT_SLOT_SHOULDERS, false)],
        InventoryType::Body => vec![(EQUIPMENT_SLOT_BODY, false)],
        InventoryType::Robe | InventoryType::Chest => vec![(EQUIPMENT_SLOT_CHEST, false)],
        InventoryType::Waist => vec![(EQUIPMENT_SLOT_WAIST, false)],
        InventoryType::Legs => vec![(EQUIPMENT_SLOT_LEGS, false)],
        InventoryType::Feet => vec![(EQUIPMENT_SLOT_FEET, false)],
        InventoryType::Wrists => vec![(EQUIPMENT_SLOT_WRISTS, false)],
        InventoryType::Hands => vec![(EQUIPMENT_SLOT_HANDS, false)],
        InventoryType::Cloak => vec![(EQUIPMENT_SLOT_BACK, false)],
        InventoryType::Finger => vec![
            (EQUIPMENT_SLOT_FINGER1, false),
            (EQUIPMENT_SLOT_FINGER2, true),
        ],
        InventoryType::Trinket => vec![
            (EQUIPMENT_SLOT_TRINKET1, false),
            (EQUIPMENT_SLOT_TRINKET2, true),
        ],
        InventoryType::Weapon => {
            let mut slots = vec![(EQUIPMENT_SLOT_MAINHAND, false)];
            if can_dual_wield {
                slots.push((EQUIPMENT_SLOT_OFFHAND, true));
            }
            slots
        }
        InventoryType::Weapon2Hand => {
            let mut slots = vec![(EQUIPMENT_SLOT_MAINHAND, false)];
            if can_dual_wield && can_titan_grip {
                slots.push((EQUIPMENT_SLOT_OFFHAND, true));
            }
            slots
        }
        InventoryType::Ranged | InventoryType::RangedRight | InventoryType::WeaponMainhand => {
            vec![(EQUIPMENT_SLOT_MAINHAND, false)]
        }
        InventoryType::Shield | InventoryType::Holdable | InventoryType::WeaponOffhand => {
            vec![(EQUIPMENT_SLOT_OFFHAND, false)]
        }
        InventoryType::NonEquip
        | InventoryType::Bag
        | InventoryType::Tabard
        | InventoryType::Ammo
        | InventoryType::Thrown
        | InventoryType::Quiver
        | InventoryType::Relic
        | InventoryType::ProfessionTool
        | InventoryType::ProfessionGear
        | InventoryType::EquipableSpellOffensive
        | InventoryType::EquipableSpellUtility
        | InventoryType::EquipableSpellDefensive
        | InventoryType::EquipableSpellMobility => Vec::new(),
    }
}

pub fn represented_avg_total_item_level_maybe_replace_slot_like_cpp(
    best_item_levels: &mut [(InventoryType, u32, ObjectGuid)],
    sum: &mut u32,
    slot: u8,
    inventory_type: InventoryType,
    item_level: u32,
    item_guid: ObjectGuid,
    check_duplicate_guid: bool,
) {
    if check_duplicate_guid
        && best_item_levels
            .iter()
            .any(|(_, _, existing_guid)| *existing_guid == item_guid)
    {
        return;
    }
    let slot_data = &mut best_item_levels[slot as usize];
    if item_level > slot_data.1 {
        *sum = sum.saturating_add(item_level.saturating_sub(slot_data.1));
        *slot_data = (inventory_type, item_level, item_guid);
    }
}

mod enchantment;
mod equipment;
mod storage;
mod storage_move;

pub use storage_move::{
    ExistingStorageStackUpdateLikeCpp, InventoryStorageMovePlanLikeCpp,
    plan_inventory_storage_move_like_cpp,
};
