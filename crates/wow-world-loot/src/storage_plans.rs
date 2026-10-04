// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared direct-loot and disenchant item-storage plan records.
//!
//! Session-independent: moved out of `wow-world` under #1263 F5.

use crate::LootStoreRandomProperties;
use wow_core::ObjectGuid;
use wow_packet::packets::loot::LootEntry;

#[derive(Debug, Clone)]
pub struct PlannedLootNewStack {
    pub slot: u8,
    pub entry_id: u32,
    pub count: u32,
    pub max_durability: u32,
    pub dynamic_flags: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub item_context: u8,
}

/// Everything needed to mirror C++ `Player::StoreLootItem`'s post-store wire
/// boundary. SQL and the object-owned claim are settled by the detached
/// worker; the session publishes the stored-item update before choosing the
/// direct-loot or disenchant-specific removal/`ItemPushResult` order.
#[derive(Debug, Clone, Copy)]
pub struct LootItemClaimCommitContextLikeCpp {
    pub owner_guid: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub player_guid: ObjectGuid,
    pub free_for_all: bool,
}

#[derive(Debug, Clone)]
pub struct PlannedDisenchantExistingStack {
    pub slot: u8,
    pub item_guid: ObjectGuid,
    pub db_guid: u64,
    pub new_count: u32,
    pub dynamic_flags: u32,
    pub flags_changed: bool,
}

#[derive(Debug, Clone)]
pub struct PlannedDirectLootExistingStack {
    pub slot: u8,
    pub item_guid: ObjectGuid,
    pub db_guid: u64,
    pub new_count: u32,
    pub added_count: u32,
    pub dynamic_flags: u32,
    pub flags_changed: bool,
}

#[derive(Debug, Clone)]
pub struct PlannedDisenchantExistingPush {
    pub slot: u8,
    pub item_guid: ObjectGuid,
    pub added_count: u32,
    pub new_count: u32,
}

#[derive(Debug, Clone)]
pub struct PlannedDisenchantNewPush {
    pub stack_index: usize,
    pub added_count: u32,
    pub new_count: u32,
}

#[derive(Debug, Clone)]
pub struct PlannedDisenchantGrant {
    pub entry: LootEntry,
    pub random_properties: LootStoreRandomProperties,
    pub existing_pushes: Vec<PlannedDisenchantExistingPush>,
    pub new_pushes: Vec<PlannedDisenchantNewPush>,
}
