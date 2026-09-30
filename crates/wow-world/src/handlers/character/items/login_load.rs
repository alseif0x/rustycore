// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Restore the canonical inventory and item-create snapshot during login.

use std::sync::Arc;

use super::*;
use crate::handlers::character::vendor::rules::{
    LoadedItemRefundDecision, loaded_item_refund_decision,
};
use crate::handlers::character::world_entry::is_represented_bag_slot;

pub(in crate::handlers::character) struct LoginInventorySnapshotLikeCpp {
    pub(in crate::handlers::character) visible_items: [(i32, u16, u16); 19],
    pub(in crate::handlers::character) inv_slots: [ObjectGuid; 141],
    pub(in crate::handlers::character) item_creates:
        Vec<wow_packet::packets::update::ItemCreateData>,
    pub(in crate::handlers::character) loaded_equipped_item_guids: Vec<ObjectGuid>,
    pub(in crate::handlers::character) loaded_item_time_updates:
        Vec<wow_entities::PlayerItemTimeUpdate>,
    pub(in crate::handlers::character) loaded_non_equipped_enchantment_updates:
        Vec<wow_entities::PlayerEnchantTimeUpdate>,
}

mod inventory;
