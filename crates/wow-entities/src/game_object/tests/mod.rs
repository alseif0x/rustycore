//! GameObject template, loot and runtime state regression scenarios.
//!
//! Separated from the game_object.rs root under #636.

use super::*;
use wow_core::guid::HighGuid;

fn owned_loot_fixture_like_cpp(coins: u32, unlooted_count: u8) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
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
        items: Vec::new(),
        looted_by_player: false,
    }
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
