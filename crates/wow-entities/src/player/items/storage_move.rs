// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure validation for an already allocated inventory-storage move.
//!
//! The Session adapter still owns source lookup, `CanStoreItem`/
//! `CanBankItem`, persistence, quest checks and publication.  This module owns
//! only the deterministic part after C++ `Player::CanStoreItem` or
//! `Player::CanBankItem` has produced its ordered `ItemPosCount` destinations.
//! The corresponding opcode paths are `ItemHandler.cpp`'s
//! `HandleAutoStoreBagItemOpcode` and `BankHandler.cpp`'s
//! `HandleAutoBankItemOpcode`/`HandleAutoStoreBankItemOpcode`.

use super::super::{ItemPosCount, PlayerInventoryItem};
use wow_constants::InventoryResult;
use wow_core::ObjectGuid;

/// One existing destination stack to update during a storage move.
///
/// This is an owned decision record, not a mutable inventory view.  The
/// handler uses the copied item identity to persist and publish the update
/// after its transaction succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingStorageStackUpdateLikeCpp {
    pub item: PlayerInventoryItem,
    pub bag: u8,
    pub slot: u8,
    pub new_count: u32,
}

/// Deterministic result of validating an ordered storage allocation.
///
/// `moved_destination` is the optional new/remainder stack.  A source may
/// therefore merge into existing stacks and move one remainder stack, or be
/// fully consumed by existing stacks.  Runtime mutation and publication stay
/// with the Session adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryStorageMovePlanLikeCpp {
    pub source_bag: u8,
    pub source_slot: u8,
    pub source: PlayerInventoryItem,
    pub source_count: u32,
    pub existing_updates: Vec<ExistingStorageStackUpdateLikeCpp>,
    pub moved_destination: Option<(u8, u8, u32)>,
}

/// Validate the result of a storage allocator without taking a `Player` or a
/// Session dependency.
///
/// The callbacks deliberately stay separate.  For every occupied destination
/// the read order remains the C++-mirroring handler order: destination item
/// lookup, source GUID/entry rejection, optional destination-object count,
/// then max-stack lookup.  Empty destinations do not trigger object/template
/// reads, and a source self-position is handled before destination lookup.
pub fn plan_inventory_storage_move_like_cpp<DestinationItem, DestinationCount, MaxStack>(
    source_bag: u8,
    source_slot: u8,
    source: PlayerInventoryItem,
    source_count: u32,
    destinations: &[ItemPosCount],
    mut destination_item: DestinationItem,
    mut destination_count: DestinationCount,
    mut max_stack_size: MaxStack,
) -> Result<InventoryStorageMovePlanLikeCpp, InventoryResult>
where
    DestinationItem: FnMut(u8, u8) -> Option<PlayerInventoryItem>,
    DestinationCount: FnMut(ObjectGuid) -> Option<u32>,
    MaxStack: FnMut(u32) -> Option<u32>,
{
    let destination_count_len = destinations.len();
    let mut existing_updates = Vec::new();
    let mut moved_destination = None;
    let mut planned_count = 0u32;

    for destination in destinations {
        let [bag, slot] = destination.pos.to_be_bytes();
        if destination.count == 0 {
            return Err(InventoryResult::InternalBagError);
        }
        planned_count = match planned_count.checked_add(destination.count) {
            Some(count) => count,
            None => return Err(InventoryResult::InternalBagError),
        };

        if bag == source_bag && slot == source_slot {
            if destination_count_len == 1 {
                // C++ HandleAutoStoreBagItemOpcode treats a one-slot
                // autostore result as a no-op and clears the client's grey
                // item state with EQUIP_ERR_INTERNAL_BAG_ERROR.
                return Err(InventoryResult::InternalBagError);
            }
            if moved_destination
                .replace((bag, slot, destination.count))
                .is_some()
            {
                return Err(InventoryResult::InternalBagError);
            }
            continue;
        }

        if let Some(existing) = destination_item(bag, slot) {
            if existing.guid == source.guid || existing.entry_id != source.entry_id {
                return Err(InventoryResult::CantStack);
            }
            let Some(existing_count) = destination_count(existing.guid) else {
                return Err(InventoryResult::ItemNotFound);
            };
            let Some(new_count) = existing_count.checked_add(destination.count) else {
                return Err(InventoryResult::InternalBagError);
            };
            let max_stack = max_stack_size(existing.entry_id).map_or(1, |size| size.max(1));
            if new_count > max_stack {
                return Err(InventoryResult::CantStack);
            }
            existing_updates.push(ExistingStorageStackUpdateLikeCpp {
                item: existing,
                bag,
                slot,
                new_count,
            });
        } else if moved_destination
            .replace((bag, slot, destination.count))
            .is_some()
        {
            // One Item instance can supply merges plus at most one remainder
            // stack.  Preserve the existing handler's error and order.
            return Err(InventoryResult::InternalBagError);
        }
    }

    if planned_count != source_count {
        return Err(InventoryResult::InternalBagError);
    }

    Ok(InventoryStorageMovePlanLikeCpp {
        source_bag,
        source_slot,
        source,
        source_count,
        existing_updates,
        moved_destination,
    })
}

#[cfg(test)]
#[path = "storage_move/tests.rs"]
mod tests;
