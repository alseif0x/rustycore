//! Admission rules for represented direct loot and master/roll awards.
//!
//! Target C++ anchors: Player.cpp::Player::StoreLootItem (25643),
//! Loot.cpp::Loot::LootItemInSlot (915), and
//! LootHandler.cpp::WorldSession::HandleLootMasterGiveOpcode (402).
//! These are preflight decisions; the entity authority still revalidates claims.

use crate::{
    CreatureLoot, LOOT_METHOD_MASTER_LIKE_CPP, LootEntry, loot_item_is_looted_for_player_like_cpp,
};
use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectItemRejection {
    NotAllowed,
    Blocked,
    OtherWinner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterItemRejection {
    WrongMethod,
    TargetNotAllowed,
    InvalidSlot,
    ItemTargetNotAllowed,
}

/// Direct requests find the first unconsumed matching list ID, not a vector index.
pub fn find_unlooted_item(
    loot: &CreatureLoot,
    list_id: u8,
    player: ObjectGuid,
) -> Option<&LootEntry> {
    loot.items.iter().find(|entry| {
        entry.loot_list_id == list_id
            && !loot_item_is_looted_for_player_like_cpp(loot, entry, player)
    })
}

/// Preserves the allowed-looter, blocked, then roll-winner rejection order.
pub fn direct_item_rejection(entry: &LootEntry, player: ObjectGuid) -> Option<DirectItemRejection> {
    if !entry.has_allowed_looter_like_cpp(player) {
        return Some(DirectItemRejection::NotAllowed);
    }
    if entry.flags.blocked {
        return Some(DirectItemRejection::Blocked);
    }
    if !entry.roll_winner_allows_like_cpp(player) {
        return Some(DirectItemRejection::OtherWinner);
    }
    None
}

/// Master selection keeps the vector index and the represented empty-list rule.
/// Inventory checks remain in the caller after this selection, as before.
pub fn select_master_item(
    loot: &CreatureLoot,
    slot: u8,
    target: ObjectGuid,
) -> Result<&LootEntry, MasterItemRejection> {
    if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP {
        return Err(MasterItemRejection::WrongMethod);
    }
    if !loot.allowed_looters.contains(&target) {
        return Err(MasterItemRejection::TargetNotAllowed);
    }
    if slot as usize >= loot.items.len() {
        return Err(MasterItemRejection::InvalidSlot);
    }
    let item = &loot.items[slot as usize];
    if !item.allowed_looters.is_empty() && !item.allowed_looters.contains(&target) {
        return Err(MasterItemRejection::ItemTargetNotAllowed);
    }
    Ok(item)
}

/// The remote recipient requires an explicit tap list, unlike master selection.
pub fn master_award_recipient_allowed(entry: &LootEntry, recipient: ObjectGuid) -> bool {
    !entry.allowed_looters.is_empty() && entry.allowed_looters.contains(&recipient)
}

/// Preserves cardinality checks before each entry's tap-list and winner checks.
pub fn roll_award_batch_allowed(
    entries: &[LootEntry],
    is_disenchant: bool,
    recipient: ObjectGuid,
) -> bool {
    !(entries.is_empty()
        || (!is_disenchant && entries.len() != 1)
        || entries.iter().any(|entry| {
            entry.allowed_looters.is_empty()
                || !entry.allowed_looters.contains(&recipient)
                || !entry.roll_winner_allows_like_cpp(recipient)
        }))
}

/// Compares the generated store metadata with the existing item.
/// Entry runtime random fields are not a substitute for that generated metadata.
pub fn loot_store_item_matches(
    loot_entry: &LootEntry,
    random_properties_id: i32,
    property_seed: i32,
    item: &wow_entities::Item,
) -> bool {
    let data = item.data();
    data.random_properties_id == random_properties_id
        && data.property_seed == property_seed
        && u8::try_from(data.context).unwrap_or(0) == loot_entry.item_context
}

#[cfg(test)]
mod tests;
