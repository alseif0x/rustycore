// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session-free group-roll values (#1363 slice 5b; moved from the World
//! `handlers/loot/mod.rs`): the connected roll looters, the highest enchanting
//! skill among them, the `SMSG_START_LOOT_ROLL` packet and the stored item
//! context.

use wow_constants::ItemContext;
use wow_core::ObjectGuid;
use wow_packet::packets::item::ItemInstance;
use wow_packet::packets::loot::{LootEntry, LootItemData, StartLootRoll};
use wow_world_core::session::directory::PlayerRegistry;

use crate::contracts::LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP;

pub const LOOT_ROLL_TIMEOUT_MS_LIKE_CPP: u32 = 60_000;

pub fn connected_roll_looters_like_cpp(
    entry: &LootEntry,
    player_guid: ObjectGuid,
    current_map_id: u16,
    current_instance_id: u32,
    player_registry: Option<&PlayerRegistry>,
) -> Vec<ObjectGuid> {
    let mut looters = Vec::new();

    for looter in &entry.allowed_looters {
        if *looter == player_guid {
            looters.push(*looter);
            continue;
        }

        let Some(registry) = player_registry else {
            continue;
        };
        let Some(player) = registry.loot_presence(*looter) else {
            continue;
        };
        if player.map_id == current_map_id && player.instance_id == current_instance_id {
            looters.push(*looter);
        }
    }

    looters.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
    looters.dedup();
    looters
}

pub fn represented_max_enchanting_skill_like_cpp(
    looters: &[ObjectGuid],
    current_player_guid: ObjectGuid,
    current_player_enchanting_skill: Option<u16>,
    player_registry: Option<&PlayerRegistry>,
) -> u16 {
    looters
        .iter()
        .filter_map(|looter| {
            if *looter == current_player_guid {
                current_player_enchanting_skill
            } else {
                player_registry.and_then(|registry| registry.loot_enchanting_skill(*looter))
            }
        })
        .max()
        .unwrap_or(0)
}

pub fn start_loot_roll_packet_like_cpp(
    loot_obj: ObjectGuid,
    map_id: u16,
    loot_method: u8,
    entry: &LootEntry,
    valid_rolls: u8,
    dungeon_encounter_id: i32,
) -> StartLootRoll {
    StartLootRoll {
        loot_obj,
        map_id: map_id as i32,
        roll_time_ms: LOOT_ROLL_TIMEOUT_MS_LIKE_CPP,
        method: loot_method,
        valid_rolls,
        loot_roll_ineligible_reason: [0; 4],
        item: LootItemData {
            item_type: 0,
            ui_type: LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
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
        },
        dungeon_encounter_id,
    }
}

pub fn loot_item_context(context: u8) -> ItemContext {
    <ItemContext as num_traits::FromPrimitive>::from_u8(context).unwrap_or(ItemContext::None)
}
