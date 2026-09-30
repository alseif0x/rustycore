//! Loot release commands and roll lifecycle: reliable release queuing, connected
//! rollers and roll packets.
//!
//! Split out of `loot/mod.rs` under #584 (B5); items are unchanged.

use super::*;


pub(in crate::handlers::loot) fn connected_roll_looters_like_cpp(
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

pub(in crate::handlers::loot) fn start_loot_roll_packet_like_cpp(
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

pub(in crate::handlers::loot) fn loot_roll_broadcast_item_like_cpp(entry: &LootEntry, ui_type: u8) -> LootItemData {
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
