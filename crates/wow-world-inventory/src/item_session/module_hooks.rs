// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `item.equipped`, the equip hook port of the item handlers (#1444 H4).
//!
//! AzerothCore `Player::EquipItem` calls `OnPlayerEquip` once the item sits in
//! its equipment or bag slot (`PlayerStorage.cpp:2940`). Here the equip paths
//! raise it after their move committed and the runtime position changed, with
//! no split borrow, map guard or await in scope, through the provided
//! [`ItemSessionHostLikeCpp`] methods; the World overrides them.

use wow_core::ObjectGuid;
use wow_entities::{PlayerInventoryItem as InventoryItem, is_equipment_pos};

use super::ItemSessionHostLikeCpp;

/// `item.equipped`: AzerothCore `OnPlayerEquip` (`PlayerScript.h:416`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemEquippedHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub item_entry: u32,
    pub bag: u8,
    pub slot: u8,
}

/// Raise `item.equipped` for `item`, now at `bag`/`slot`, when that is an
/// equipment or bag position (C++ `Player::IsEquipmentPos`, the positions
/// `EquipItem` serves) and a linked module wants it.
pub(super) fn raise_item_equipped_hook_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item: &InventoryItem,
    bag: u8,
    slot: u8,
) {
    if !is_equipment_pos(bag, slot) || !host.item_equipped_hook_wanted_like_cpp() {
        return;
    }
    let Some(guid) = host.player_guid() else {
        return;
    };
    host.item_equipped_hook_like_cpp(&ItemEquippedHookEventLikeCpp {
        guid,
        item_guid: item.guid,
        item_entry: item.entry_id,
        bag,
        slot,
    });
}
