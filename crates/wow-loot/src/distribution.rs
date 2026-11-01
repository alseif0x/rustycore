// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Per-player loot distribution, access and consumption over existing loot state.
//!
//! Relocated from the world adapters under #1233. These functions do not own a
//! Session, packet writer, persistence worker or object-generation authority.

use crate::{
    CreatureLoot, LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP, LOOT_METHOD_GROUP_LIKE_CPP,
    LOOT_METHOD_MASTER_LIKE_CPP, LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP,
    LOOT_METHOD_PERSONAL_LIKE_CPP, LOOT_METHOD_ROUND_ROBIN_LIKE_CPP, LootEntry, NotNormalLootItem,
};
use wow_core::ObjectGuid;

#[cfg(test)]
mod tests;

pub fn loot_is_looted_like_cpp(loot: &CreatureLoot) -> bool {
    loot.coins == 0 && loot.unlooted_count == 0
}

pub fn mark_loot_allowed_for_player_like_cpp(loot: &mut CreatureLoot, player_guid: ObjectGuid) {
    if !player_guid.is_empty() && !loot.allowed_looters.contains(&player_guid) {
        loot.allowed_looters.push(player_guid);
    }

    for entry in &mut loot.items {
        if entry.allowed_looters.is_empty() || entry.flags.freeforall {
            entry.add_allowed_looter_like_cpp(player_guid);
        }
    }

    let existing_ffa_item_ids: Vec<u8> = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .map(|(_, items)| items.iter().map(|item| item.loot_list_id).collect())
        .unwrap_or_default();
    let mut ffa_items = Vec::new();
    for entry in &mut loot.items {
        if entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
            && !existing_ffa_item_ids.contains(&entry.loot_list_id)
        {
            ffa_items.push(NotNormalLootItem {
                loot_list_id: entry.loot_list_id,
                is_looted: false,
            });
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        } else if !entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
            && !entry.flags.counted
        {
            entry.flags.counted = true;
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        }
    }

    if !ffa_items.is_empty() {
        match loot
            .player_ffa_items
            .iter_mut()
            .find(|(player, _)| *player == player_guid)
        {
            Some((_, existing)) => existing.extend(ffa_items),
            None => loot.player_ffa_items.push((player_guid, ffa_items)),
        }
    }
}

/// Completes the shared C++ `Loot::FillLoot` visibility/count state before the
/// generation is published through the object-owned authority. Applying this
/// only to the session cache after publication is unsafe: reconciliation would
/// immediately restore the older authoritative snapshot and lose the tap list.
pub fn prepare_represented_shared_loot_generation_like_cpp(
    loot: &mut CreatureLoot,
    allowed_looters: &[ObjectGuid],
) {
    for looter in allowed_looters {
        if !looter.is_empty() && !loot.allowed_looters.contains(looter) {
            loot.allowed_looters.push(*looter);
        }
    }
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
}

pub fn prepare_represented_shared_creature_loot_generation_like_cpp(
    loot: &mut CreatureLoot,
    allowed_looters: &[ObjectGuid],
) {
    for looter in allowed_looters {
        mark_loot_allowed_for_player_like_cpp(loot, *looter);
    }
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
}

pub fn rebuild_represented_personal_loot_counts_like_cpp(loot: &mut CreatureLoot) {
    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        entry.ffa_looted_by.clear();
        entry.flags.counted = false;

        if entry.flags.freeforall {
            for looter in &entry.allowed_looters {
                match loot
                    .player_ffa_items
                    .iter_mut()
                    .find(|(player, _)| player == looter)
                {
                    Some((_, existing)) => existing.push(NotNormalLootItem {
                        loot_list_id: entry.loot_list_id,
                        is_looted: false,
                    }),
                    None => loot.player_ffa_items.push((
                        *looter,
                        vec![NotNormalLootItem {
                            loot_list_id: entry.loot_list_id,
                            is_looted: false,
                        }],
                    )),
                }
                loot.unlooted_count = loot.unlooted_count.saturating_add(1);
            }
        } else if !entry.allowed_looters.is_empty() {
            entry.flags.counted = true;
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        }
    }
}

/// Rebuild a player-scoped authority pool without resurrecting entries already
/// consumed in the session view. The generation helper above intentionally
/// starts fresh; authority synchronization can also run during release, after
/// `taken`/`ffa_looted_by` have already changed.
pub fn rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(
    loot: &mut CreatureLoot,
) {
    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        if entry.flags.freeforall {
            entry.flags.counted = false;
            for looter in &entry.allowed_looters {
                let is_looted = entry.ffa_looted_by.contains(looter);
                let item = NotNormalLootItem {
                    loot_list_id: entry.loot_list_id,
                    is_looted,
                };
                match loot
                    .player_ffa_items
                    .iter_mut()
                    .find(|(player, _)| player == looter)
                {
                    Some((_, items)) => items.push(item),
                    None => loot.player_ffa_items.push((*looter, vec![item])),
                }
                if !is_looted {
                    loot.unlooted_count = loot.unlooted_count.saturating_add(1);
                }
            }
        } else {
            entry.flags.counted = !entry.allowed_looters.is_empty();
            if entry.flags.counted && !entry.taken {
                loot.unlooted_count = loot.unlooted_count.saturating_add(1);
            }
        }
    }
}

pub fn loot_player_has_unlooted_ffa_item_like_cpp(
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
    loot_list_id: u8,
) -> bool {
    loot.player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .is_some_and(|(_, items)| {
            items
                .iter()
                .any(|item| item.loot_list_id == loot_list_id && !item.is_looted)
        })
}

pub fn loot_item_is_looted_for_player_like_cpp(
    loot: &CreatureLoot,
    entry: &LootEntry,
    player_guid: ObjectGuid,
) -> bool {
    if entry.flags.freeforall {
        !loot_player_has_unlooted_ffa_item_like_cpp(loot, player_guid, entry.loot_list_id)
    } else {
        entry.taken
    }
}

pub fn mark_loot_item_looted_for_player_like_cpp(
    loot: &mut CreatureLoot,
    loot_list_id: u8,
    player_guid: ObjectGuid,
) {
    let should_decrement = loot
        .items
        .iter()
        .find(|entry| entry.loot_list_id == loot_list_id)
        .is_some_and(|entry| !loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid));

    if let Some(entry) = loot
        .items
        .iter_mut()
        .find(|entry| entry.loot_list_id == loot_list_id)
    {
        entry.mark_looted_for_player_like_cpp(player_guid);
        if entry.flags.freeforall {
            if let Some((_, items)) = loot
                .player_ffa_items
                .iter_mut()
                .find(|(player, _)| *player == player_guid)
                && let Some(item) = items
                    .iter_mut()
                    .find(|item| item.loot_list_id == loot_list_id)
            {
                item.is_looted = true;
            }
        }
        if should_decrement {
            loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
        }
    }
}

pub fn loot_can_be_opened_by_player_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    if loot_is_looted_like_cpp(loot) {
        return false;
    }

    loot_has_item_for_all_like_cpp(loot, player_guid)
        || loot_has_item_for_player_like_cpp(loot, player_guid)
}

/// Exact represented branch order of C++ `Player::isAllowedToLoot` for a
/// creature and the pool selected by `Creature::GetLootForPlayer`.
pub fn creature_loot_is_allowed_to_player_like_cpp(
    creature_is_dead: bool,
    player_has_pending_bind: bool,
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> bool {
    if !creature_is_dead || player_has_pending_bind || loot_is_looted_like_cpp(loot) {
        return false;
    }
    if !loot.allowed_looters.contains(&player_guid)
        || (!loot_has_item_for_all_like_cpp(loot, player_guid)
            && !loot_has_item_for_player_like_cpp(loot, player_guid))
    {
        return false;
    }

    match loot.loot_method {
        LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP | LOOT_METHOD_PERSONAL_LIKE_CPP => true,
        LOOT_METHOD_ROUND_ROBIN_LIKE_CPP => {
            loot.round_robin_player.is_empty()
                || loot.round_robin_player == player_guid
                || loot_has_item_for_player_like_cpp(loot, player_guid)
        }
        LOOT_METHOD_MASTER_LIKE_CPP
        | LOOT_METHOD_GROUP_LIKE_CPP
        | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP => {
            loot.round_robin_player.is_empty()
                || loot.round_robin_player == player_guid
                || loot_has_over_threshold_item_like_cpp(loot)
                || loot_has_item_for_player_like_cpp(loot, player_guid)
        }
        _ => false,
    }
}

pub fn loot_has_over_threshold_item_like_cpp(loot: &CreatureLoot) -> bool {
    loot.items
        .iter()
        .any(|entry| !entry.taken && entry.is_over_threshold_like_cpp())
}

fn loot_has_item_for_all_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    if loot.coins > 0 {
        return true;
    }

    loot.items.iter().any(|entry| {
        !entry.taken
            && entry.flags.follow_loot_rules
            && !entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
    })
}

fn loot_has_item_for_player_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    loot.items.iter().any(|entry| {
        !loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid)
            && entry.has_allowed_looter_like_cpp(player_guid)
            && (!entry.flags.follow_loot_rules || entry.flags.freeforall)
    })
}
