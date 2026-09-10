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
