use std::collections::HashMap;
use std::time::Instant;

use wow_core::{ObjectGuid, Position};
use wow_loot::{OwnedLootAuthority, OwnedLootScope, RepresentedLootRollVote};
use wow_packet::packets::{item::ItemInstance, loot::{LootEntry, LootItemData}};
use wow_world_core::session::mailbox::LootRollCommandIdentityLikeCpp;

pub const LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP: u8 = 0;
pub const LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP: u8 = 1;
pub const LOOT_SLOT_TYPE_LOCKED_LIKE_CPP: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootStoreRandomProperties {
    pub id: i32,
    pub seed: i32,
}

#[derive(Debug, Clone)]
pub struct RepresentedCreatureLootStateLikeCpp {
    pub is_alive: bool,
    pub position: Position,
    pub level: u8,
    pub entry: u32,
    pub loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub dungeon_encounter_id: u32,
    pub tappers: Vec<ObjectGuid>,
    pub loot_lifecycle_revision: u64,
}

#[derive(Debug, Clone)]
pub struct RepresentedLootRollState {
    pub owner_guid: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    pub authority_scope: OwnedLootScope,
    /// Exact C++ `LootRoll*` lifetime surrogate published to remote sessions.
    /// This must change even if a replacement reuses the same packet key,
    /// authority allocation, and authority generation.
    pub command_identity: LootRollCommandIdentityLikeCpp,
    pub end_time: Instant,
    pub voters: HashMap<ObjectGuid, RepresentedLootRollVote>,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedLootRollCriteriaEvent {
    RollAnyNeed {
        player_guid: ObjectGuid,
        quantity: u32,
    },
    RollAnyGreed {
        player_guid: ObjectGuid,
        quantity: u32,
    },
    RollNeed {
        player_guid: ObjectGuid,
        item_id: u32,
        roll_number: u8,
    },
    RollGreed {
        player_guid: ObjectGuid,
        item_id: u32,
        roll_number: u8,
    },
    Disenchant {
        player_guid: ObjectGuid,
        spell_id: u32,
    },
}

pub fn loot_roll_broadcast_item_like_cpp(entry: &LootEntry, ui_type: u8) -> LootItemData {
    LootItemData {
        item_type: 0,
        ui_type,
        can_trade_to_tap_list: entry.allowed_looters.len() > 1,
        loot: ItemInstance {
            item_id: entry.item_id as i32,
            random_properties_id: entry.random_properties_id,
            random_properties_seed: entry.random_properties_seed,
            ..ItemInstance::default()
        },
        loot_list_id: entry.loot_list_id,
        quantity: entry.quantity,
        loot_item_type: 0,
    }
}
