// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared direct-loot and disenchant item-storage plan records.

use super::LootStoreRandomProperties;
use wow_core::ObjectGuid;
use wow_packet::packets::loot::LootEntry;

#[derive(Debug, Clone)]
pub(super) struct PlannedLootNewStack {
    pub(super) slot: u8,
    pub(super) entry_id: u32,
    pub(super) count: u32,
    pub(super) max_durability: u32,
    pub(super) dynamic_flags: u32,
    pub(super) random_properties_id: i32,
    pub(super) random_properties_seed: i32,
    pub(super) item_context: u8,
}

/// Everything needed to mirror C++ `Player::StoreLootItem`'s post-store wire
/// boundary. SQL and the object-owned claim are settled by the detached
/// worker; the session publishes the stored-item update before choosing the
/// direct-loot or disenchant-specific removal/`ItemPushResult` order.
#[derive(Debug, Clone, Copy)]
pub(super) struct LootItemClaimCommitContextLikeCpp {
    pub(super) owner_guid: ObjectGuid,
    pub(super) loot_obj: ObjectGuid,
    pub(super) loot_list_id: u8,
    pub(super) player_guid: ObjectGuid,
    pub(super) free_for_all: bool,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedDisenchantExistingStack {
    pub(super) slot: u8,
    pub(super) item_guid: ObjectGuid,
    pub(super) db_guid: u64,
    pub(super) new_count: u32,
    pub(super) dynamic_flags: u32,
    pub(super) flags_changed: bool,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedDirectLootExistingStack {
    pub(super) slot: u8,
    pub(super) item_guid: ObjectGuid,
    pub(super) db_guid: u64,
    pub(super) new_count: u32,
    pub(super) added_count: u32,
    pub(super) dynamic_flags: u32,
    pub(super) flags_changed: bool,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedDisenchantExistingPush {
    pub(super) slot: u8,
    pub(super) item_guid: ObjectGuid,
    pub(super) added_count: u32,
    pub(super) new_count: u32,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedDisenchantNewPush {
    pub(super) stack_index: usize,
    pub(super) added_count: u32,
    pub(super) new_count: u32,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedDisenchantGrant {
    pub(super) entry: LootEntry,
    pub(super) random_properties: LootStoreRandomProperties,
    pub(super) existing_pushes: Vec<PlannedDisenchantExistingPush>,
    pub(super) new_pushes: Vec<PlannedDisenchantNewPush>,
}
