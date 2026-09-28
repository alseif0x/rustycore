//! Item assembly for loot replies: stack merging, direct item count and the
//! represented response item list.
//!
//! Split out of `loot/mod.rs` under #584 (B5); items are unchanged.

use super::*;

pub fn direct_item_count_after_loot_release_like_cpp(
    current_count: u32,
    maximum_destroy_count: Option<u32>,
) -> u32 {
    let destroy_count = maximum_destroy_count
        .unwrap_or(current_count)
        .min(current_count);
    current_count.saturating_sub(destroy_count)
}

#[cfg(test)]
pub fn assign_represented_personal_loot_items_like_cpp<R: Rng + ?Sized>(
    loot: &mut CreatureLoot,
    tappers: &[ObjectGuid],
    rng: &mut R,
) {
    if tappers.is_empty() {
        return;
    }

    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        entry.allowed_looters.clear();
        entry.flags.counted = false;

        let chosen_tapper = tappers[rng.gen_range(0..tappers.len())];
        entry.add_allowed_looter_like_cpp(chosen_tapper);
    }

    rebuild_represented_personal_loot_counts_like_cpp(loot);
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

pub fn add_loot_item_stacks_like_cpp(
    loot_items: &mut Vec<LootEntry>,
    item_id: u32,
    mut count: u32,
    max_stack_size: u32,
    flags: LootEntryFlags,
) {
    while count > 0 && loot_items.len() < MAX_NR_LOOT_ITEMS_LIKE_CPP {
        let quantity = count.min(max_stack_size);
        loot_items.push(LootEntry {
            loot_list_id: loot_items.len() as u8,
            item_id,
            quantity,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags,
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        });
        count = count.saturating_sub(max_stack_size);
    }
}
