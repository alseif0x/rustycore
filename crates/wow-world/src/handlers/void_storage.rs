// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void-storage handlers.
//!
//! C++ source of truth:
//! `src/server/game/Handlers/VoidStorageHandler.cpp` and
//! `src/server/game/Entities/Player/Player.cpp::{_Load,_Save}VoidStorage`.
//!
//! `#1263 F5 deferred families`: the four registrations moved unchanged to
//! `wow_world_inventory::register_void_storage_handlers_like_cpp`; `host.rs`
//! lends that registrar these operations. #1363 slice 4c: their bodies moved
//! to `wow_world_application::void_storage_session`; `session_host.rs` lends
//! those bodies the remaining `WorldSession` operations.

// The void-storage unit tests name these through `super::*`.
#[cfg(test)]
use crate::session::{
    DirectInventoryStorageOverlayLikeCpp, InventoryItem, RepresentedVoidStorageItemLikeCpp,
    WorldSession,
};
#[cfg(test)]
use wow_constants::unit::NPCFlags1;
#[cfg(test)]
use wow_constants::{EnchantmentSlot, ItemContext, ItemModifier};
#[cfg(test)]
use wow_entities::INVENTORY_SLOT_BAG_0;
#[cfg(test)]
use wow_packet::WorldPacket;
#[cfg(test)]
use wow_packet::packets::update::UpdateObject;
#[cfg(test)]
use wow_world_application::void_withdrawal_item_create_data_like_cpp;
#[cfg(test)]
use wow_world_inventory::EffectiveVoidStorageRandomPropertiesLikeCpp;

#[cfg(test)]
#[path = "../../unit_tests/handlers/void_storage_tests/mod.rs"]
mod tests;

mod host;
mod session_host;
