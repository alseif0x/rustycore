//! Per-player item selection for a loot response, independent of packet DTOs.
//!
//! Target C++ anchors: Loot.cpp::LootItem::GetUiTypeForPlayer (140) and
//! Loot.cpp::Loot::BuildLootResponse (995). Serialization stays in application.

use crate::{
    CreatureLoot, loot_item_is_looted_for_player_like_cpp, loot_item_ui_type_for_player_like_cpp,
    loot_player_has_unlooted_ffa_item_like_cpp,
};
use wow_core::ObjectGuid;

/// The visible scalars needed to map one response item into a packet.
#[derive(Debug, PartialEq, Eq)]
pub struct LootItemView {
    pub item_id: u32,
    pub quantity: u32,
    pub loot_list_id: u8,
    pub ui_type: u8,
}

pub fn loot_response_item_views(
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> impl Iterator<Item = LootItemView> + '_ {
    loot.items.iter().filter_map(move |entry| {
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

        Some(LootItemView {
            item_id: entry.item_id,
            quantity: entry.quantity,
            loot_list_id: entry.loot_list_id,
            ui_type,
        })
    })
}

#[cfg(test)]
mod tests;
