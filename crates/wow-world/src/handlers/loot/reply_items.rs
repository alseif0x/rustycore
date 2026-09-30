//! Item assembly for loot replies: direct item count and represented response items.
//!
//! Split out of `loot/mod.rs` under #584 (B5); items are unchanged.

use super::*;
use wow_loot::loot_response_item_views;

pub(in crate::handlers::loot) use wow_loot::remaining_source_item_count as direct_item_count_after_loot_release_like_cpp;

pub(in crate::handlers::loot) fn represented_loot_response_items_like_cpp(
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> Vec<LootItemData> {
    loot_response_item_views(loot, player_guid)
        .map(|view| LootItemData {
            item_type: 0,
            ui_type: view.ui_type,
            can_trade_to_tap_list: false,
            loot: ItemInstance {
                item_id: view.item_id as i32,
                ..ItemInstance::default()
            },
            loot_list_id: view.loot_list_id,
            quantity: view.quantity,
            loot_item_type: 0,
        })
        .collect()
}
