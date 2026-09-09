//! Owned-loot authority, claims and leases regression scenarios.
//!
//! Separated from the authority.rs root under #642.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Barrier;
use wow_core::ObjectGuid;

use super::{
    CreatureLoot, LootClaimCommitError, LootClaimError, LootClaimPayload, LootEntry,
    LootEntryFlags, LootInstallOutcome, LootItemClaimMode, NotNormalLootItem, OwnedLootAuthority,
    OwnedLootAuthorityLifecycle, OwnedLootScope, ReserveAttempt, reserve_item_once,
};

fn player(counter: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, counter)
}

fn owner(counter: u32) -> ObjectGuid {
    ObjectGuid::create_creature_like_cpp(1, 1, counter, i64::from(counter))
}

fn entry(list_id: u8, free_for_all: bool, allowed: Vec<ObjectGuid>) -> LootEntry {
    LootEntry {
        loot_list_id: list_id,
        item_id: 1000 + u32::from(list_id),
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags {
            freeforall: free_for_all,
            counted: true,
            ..LootEntryFlags::default()
        },
        allowed_looters: allowed,
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    }
}

fn loot(owner_guid: ObjectGuid, coins: u32, items: Vec<LootEntry>) -> CreatureLoot {
    let unlooted_count = items.len() as u8;
    CreatureLoot {
        loot_guid: owner_guid,
        coins,
        unlooted_count,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items,
        looted_by_player: false,
    }
}

fn money_loot(
    owner_guid: ObjectGuid,
    coins: u32,
    allowed_looters: Vec<ObjectGuid>,
) -> CreatureLoot {
    let mut loot = loot(owner_guid, coins, Vec::new());
    loot.allowed_looters = allowed_looters;
    loot
}

mod scenarios_1;
mod scenarios_2;
