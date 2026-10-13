// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session-free `SMSG_LOOT_RESPONSE` values shared by the World loot requests
//! and the loot-source operations (#1363 slice 5a; moved from the World
//! `handlers/loot` module).

use wow_core::ObjectGuid;
use wow_loot::{
    loot_item_is_looted_for_player_like_cpp, loot_item_ui_type_for_player_like_cpp,
    loot_player_has_unlooted_ffa_item_like_cpp,
};
use wow_packet::packets::item::ItemInstance;
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_DISENCHANTING_LIKE_CPP, LOOT_TYPE_FISHING_JUNK_LIKE_CPP,
    LOOT_TYPE_FISHING_LIKE_CPP, LOOT_TYPE_FISHINGHOLE_LIKE_CPP, LOOT_TYPE_INSIGNIA_LIKE_CPP,
    LOOT_TYPE_MILLING_LIKE_CPP, LOOT_TYPE_PROSPECTING_LIKE_CPP, LOOT_TYPE_SKINNING_LIKE_CPP,
    LootItemData,
};

pub fn loot_type_for_client_like_cpp(loot_type: u8) -> u8 {
    match loot_type {
        LOOT_TYPE_PROSPECTING_LIKE_CPP | LOOT_TYPE_MILLING_LIKE_CPP => {
            LOOT_TYPE_DISENCHANTING_LIKE_CPP
        }
        LOOT_TYPE_INSIGNIA_LIKE_CPP => LOOT_TYPE_SKINNING_LIKE_CPP,
        LOOT_TYPE_FISHINGHOLE_LIKE_CPP | LOOT_TYPE_FISHING_JUNK_LIKE_CPP => {
            LOOT_TYPE_FISHING_LIKE_CPP
        }
        _ => loot_type,
    }
}

pub fn represented_loot_response_items_like_cpp(
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> Vec<LootItemData> {
    loot.items
        .iter()
        .filter_map(|entry| {
            let ui_type = loot_item_ui_type_for_player_like_cpp(
                player_guid,
                &entry.allowed_looters,
                loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid),
                entry.flags.freeforall,
                loot_player_has_unlooted_ffa_item_like_cpp(loot, player_guid, entry.loot_list_id),
                entry.flags.needs_quest,
                entry.flags.follow_loot_rules,
                loot.loot_method,
                loot.round_robin_player,
                loot.loot_master,
                entry.flags.under_threshold,
                entry.flags.blocked,
                entry.roll_winner,
            )?;

            Some(LootItemData {
                item_type: 0,
                ui_type,
                can_trade_to_tap_list: false,
                loot: ItemInstance {
                    item_id: entry.item_id as i32,
                    ..ItemInstance::default()
                },
                loot_list_id: entry.loot_list_id,
                quantity: entry.quantity,
                loot_item_type: 0,
            })
        })
        .collect()
}
