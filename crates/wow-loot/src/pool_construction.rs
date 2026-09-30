//! Materialization of generated entries and player-scoped loot pools.
//!
//! C++ anchors: Loot.cpp::LootItem::LootItem (42), Loot::FillLoot (758),
//! and Loot::AddItem (826). The combined-pool partition is the existing Rust
//! bridge; it is not a literal implementation of LootTemplate::ProcessPersonalLoot.
//! Allocation, player-money lookup and authority installation stay in application.

use std::collections::HashMap;

use crate::{
    CreatureLoot, GeneratedLootItem, LootEntry, LootEntryFlags, LootStoreItemContext,
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp,
};
use wow_core::ObjectGuid;

pub fn loot_entry_from_generated(
    item: GeneratedLootItem,
    follow_quest_loot_rules: bool,
) -> LootEntry {
    LootEntry {
        loot_list_id: item.loot_list_id as u8,
        item_id: item.item_id,
        quantity: item.count,
        random_properties_id: item.random_properties_id,
        random_properties_seed: item.random_properties_seed,
        item_context: item.context,
        flags: LootEntryFlags {
            follow_loot_rules: !item.needs_quest || follow_quest_loot_rules,
            freeforall: item.free_for_all,
            blocked: item.is_blocked,
            counted: item.is_counted,
            under_threshold: item.is_under_threshold,
            needs_quest: item.needs_quest,
        },
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: item.is_looted,
    }
}

pub fn shared_loot_entry_from_generated<FAllowed>(
    item: GeneratedLootItem,
    follow_quest_loot_rules: bool,
    allowed_looters: &[ObjectGuid],
    mut item_allowed_for_player: FAllowed,
) -> LootEntry
where
    FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
{
    let store_item_context = item.store_item_context;
    let mut entry = loot_entry_from_generated(item, follow_quest_loot_rules);
    for looter in allowed_looters {
        if item_allowed_for_player(store_item_context, *looter) {
            entry.add_allowed_looter_like_cpp(*looter);
        }
    }
    entry
}

/// Materialize shared or personal pools without installing an authority.
///
/// After each original pool clone, `resolve` receives its player and the
/// existing GUID for the first pool (`Some`), or an allocation request (`None`).
/// Application resolves the GUID before reading that player's money. Resolution
/// failure drops the partial result at the same point as the former application.
pub fn materialize_loot_pools<F>(
    loot: CreatureLoot,
    player_guid: ObjectGuid,
    personal: bool,
    mut resolve: F,
) -> Option<(Option<CreatureLoot>, HashMap<ObjectGuid, CreatureLoot>)>
where
    F: FnMut(ObjectGuid, Option<ObjectGuid>) -> Option<(ObjectGuid, u32)>,
{
    if !personal {
        return Some((Some(loot), HashMap::new()));
    }

    let mut looters = loot.allowed_looters.clone();
    if looters.is_empty() && !player_guid.is_empty() {
        looters.push(player_guid);
    }
    looters.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
    looters.dedup();

    let mut personal_loot = HashMap::new();
    for (index, looter) in looters.into_iter().enumerate() {
        let mut pool = loot.clone();
        let existing_guid = if index == 0 {
            Some(pool.loot_guid)
        } else {
            None
        };
        let (loot_guid, coins) = resolve(looter, existing_guid)?;
        if index != 0 {
            pool.loot_guid = loot_guid;
        }
        pool.coins = coins;
        pool.allowed_looters = vec![looter];
        pool.players_looting.retain(|viewer| *viewer == looter);
        pool.items.retain(|entry| {
            entry.allowed_looters.is_empty() || entry.allowed_looters.contains(&looter)
        });
        for entry in &mut pool.items {
            entry.allowed_looters = vec![looter];
        }
        rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut pool);
        personal_loot.insert(looter, pool);
    }

    Some((None, personal_loot))
}

#[cfg(test)]
mod tests;
