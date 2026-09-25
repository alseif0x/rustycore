// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player rules that read no player state.
//!
//! Moved out of the owner under #678. Every one was already receiver-free,
//! so it cannot read or write the owner's state: these are rules, not owner
//! behaviour. Bodies and signatures are unchanged.

use crate::*;
use wow_constants::item::InventoryType;
use wow_constants::rest::{
    REST_STATE_NORMAL_LIKE_CPP, REST_STATE_RAF_LINKED_LIKE_CPP, REST_STATE_RESTED_LIKE_CPP,
};
use wow_constants::unit::UnitStandStateType;

/// C++ `ItemTransmogrificationSlots` mapping.
pub fn item_transmogrification_slot_like_cpp(inventory_type: u8) -> Option<usize> {
    let slot = match inventory_type {
        x if x == InventoryType::Head as u8 => EQUIPMENT_SLOT_HEAD,
        x if x == InventoryType::Shoulders as u8 => EQUIPMENT_SLOT_SHOULDERS,
        x if x == InventoryType::Body as u8 => EQUIPMENT_SLOT_BODY,
        x if x == InventoryType::Chest as u8 => EQUIPMENT_SLOT_CHEST,
        x if x == InventoryType::Waist as u8 => EQUIPMENT_SLOT_WAIST,
        x if x == InventoryType::Legs as u8 => EQUIPMENT_SLOT_LEGS,
        x if x == InventoryType::Feet as u8 => EQUIPMENT_SLOT_FEET,
        x if x == InventoryType::Wrists as u8 => EQUIPMENT_SLOT_WRISTS,
        x if x == InventoryType::Hands as u8 => EQUIPMENT_SLOT_HANDS,
        x if x == InventoryType::Weapon as u8
            || x == InventoryType::Ranged as u8
            || x == InventoryType::Weapon2Hand as u8
            || x == InventoryType::WeaponMainhand as u8
            || x == InventoryType::WeaponOffhand as u8
            || x == InventoryType::RangedRight as u8 =>
        {
            EQUIPMENT_SLOT_MAINHAND
        }
        x if x == InventoryType::Shield as u8 || x == InventoryType::Holdable as u8 => {
            EQUIPMENT_SLOT_OFFHAND
        }
        x if x == InventoryType::Cloak as u8 => EQUIPMENT_SLOT_BACK,
        x if x == InventoryType::Tabard as u8 => EQUIPMENT_SLOT_TABARD,
        x if x == InventoryType::Robe as u8 => EQUIPMENT_SLOT_CHEST,
        _ => return None,
    };
    Some(slot as usize)
}

pub fn chair_stand_state_like_cpp(chair_height: u32) -> UnitStandStateType {
    let stand_state = 4_u32.saturating_add(chair_height);
    <UnitStandStateType as num_traits::FromPrimitive>::from_u32(stand_state)
        .unwrap_or(UnitStandStateType::Stand)
}

pub const fn apply_pct_modifier_to_u32_like_cpp(value: u32, pct: i32) -> u32 {
    let adjusted = (value as i64) + ((value as i64) * (pct as i64)) / 100;
    if adjusted < 0 {
        0
    } else if adjusted > u32::MAX as i64 {
        u32::MAX
    } else {
        adjusted as u32
    }
}

/// C++ group XP multiplier before the member-level normalization step.
pub const fn xp_in_group_rate_like_cpp(count: u32, is_raid: bool) -> f32 {
    if is_raid {
        0.99
    } else {
        match count {
            0..=2 => 1.0,
            3 => 1.166,
            4 => 1.3,
            _ => 1.4,
        }
    }
}

pub const fn sanitize_rest_bonus_like_cpp(rest_bonus: f32) -> f32 {
    if rest_bonus.is_finite() {
        rest_bonus
    } else {
        0.0
    }
}

pub const fn valid_player_rest_state_like_cpp(rest_state: u8) -> bool {
    matches!(
        rest_state,
        REST_STATE_RESTED_LIKE_CPP | REST_STATE_NORMAL_LIKE_CPP | REST_STATE_RAF_LINKED_LIKE_CPP
    )
}

pub(crate) const fn is_use_equipped_weapon(
    mainhand: bool,
    is_in_feral_form: bool,
    is_disarmed: bool,
) -> bool {
    !is_in_feral_form && (!mainhand || !is_disarmed)
}

pub fn is_using_two_handed_weapon_in_one_hand_template(
    main_template: Option<&ItemStorageTemplate>,
    off_template: Option<&ItemStorageTemplate>,
) -> bool {
    if off_template.is_some_and(|template| template.inventory_type == InventoryType::Weapon2Hand) {
        return true;
    }

    main_template.is_some_and(|template| template.inventory_type == InventoryType::Weapon2Hand)
        && off_template.is_some()
}

pub(crate) fn set_dynamic_update_mask_index(mask: &mut Option<Vec<u32>>, index: usize) {
    let block = index / 32;
    let bit = index % 32;
    let blocks = mask.get_or_insert_with(Vec::new);
    if blocks.len() <= block {
        blocks.resize(block + 1, 0);
    }
    blocks[block] |= 1 << bit;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_xp_rate_matches_cpp_table() {
        assert_eq!(xp_in_group_rate_like_cpp(2, false), 1.0);
        assert_eq!(xp_in_group_rate_like_cpp(3, false), 1.166);
        assert_eq!(xp_in_group_rate_like_cpp(4, false), 1.3);
        assert_eq!(xp_in_group_rate_like_cpp(5, false), 1.4);
        assert_eq!(xp_in_group_rate_like_cpp(1, true), 0.99);
    }

    #[test]
    fn rest_rules_fail_closed_for_non_finite_bonus_and_unknown_state() {
        assert_eq!(sanitize_rest_bonus_like_cpp(f32::NAN), 0.0);
        assert_eq!(sanitize_rest_bonus_like_cpp(f32::INFINITY), 0.0);
        assert!(valid_player_rest_state_like_cpp(1));
        assert!(valid_player_rest_state_like_cpp(2));
        assert!(valid_player_rest_state_like_cpp(6));
        assert!(!valid_player_rest_state_like_cpp(0));
    }

    #[test]
    fn transmogrification_slots_keep_weapon_and_armor_mapping() {
        assert_eq!(
            item_transmogrification_slot_like_cpp(InventoryType::Head as u8),
            Some(EQUIPMENT_SLOT_HEAD as usize)
        );
        assert_eq!(
            item_transmogrification_slot_like_cpp(InventoryType::Weapon2Hand as u8),
            Some(EQUIPMENT_SLOT_MAINHAND as usize)
        );
        assert_eq!(item_transmogrification_slot_like_cpp(0xff), None);
    }

    #[test]
    fn chair_height_uses_cpp_base_offset_and_fails_closed() {
        assert_eq!(
            chair_stand_state_like_cpp(0),
            UnitStandStateType::SitLowChair
        );
        assert_eq!(
            chair_stand_state_like_cpp(u32::MAX),
            UnitStandStateType::Stand
        );
    }

    #[test]
    fn pct_modifier_clamps_signed_result_to_u32() {
        assert_eq!(apply_pct_modifier_to_u32_like_cpp(100, 25), 125);
        assert_eq!(apply_pct_modifier_to_u32_like_cpp(100, -200), 0);
        assert_eq!(apply_pct_modifier_to_u32_like_cpp(u32::MAX, 100), u32::MAX);
    }
}
