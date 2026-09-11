// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player-owned item slot runtime and its named transitions.
//!
//! Moved out of the Player root so the slot map has one visible owner with its
//! own file. Behaviour is unchanged by the move itself.

use super::{BUYBACK_SLOT_COUNT, BUYBACK_SLOT_START, PlayerInventoryItem};
use crate::Item;
use std::collections::HashMap;
use wow_core::ObjectGuid;

/// Concrete item/object runtime owned by the canonical Player.
///
/// This is deliberately a private Player substate rather than a shared lock:
/// MapManager's generation-checked Player handle remains the only route to a
/// mutable owner, including while the Player is detached for a far teleport.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerInventoryRuntime {
    /// True only after the persisted equipment query completed coherently for
    /// this Player lifetime. An empty runtime inventory is not source proof.
    equipment_inventory_authority_complete_like_cpp: bool,
    inventory_items: HashMap<u8, PlayerInventoryItem>,
    buyback_items: HashMap<u8, PlayerInventoryItem>,
    buyback_price: [u32; BUYBACK_SLOT_COUNT],
    buyback_timestamp: [i64; BUYBACK_SLOT_COUNT],
    current_buyback_slot: u8,
    item_objects: HashMap<ObjectGuid, Item>,
}

impl PlayerInventoryRuntime {
    pub const fn equipment_inventory_authority_complete_like_cpp(&self) -> bool {
        self.equipment_inventory_authority_complete_like_cpp
    }

    pub fn set_equipment_inventory_authority_complete_like_cpp(&mut self, complete: bool) {
        self.equipment_inventory_authority_complete_like_cpp = complete;
    }

    /// C++ installs an item into one of the Player's own slots in
    /// `Player::_StoreItem` (`m_items[slot] = pItem`, Player.cpp:11277) and
    /// `Player::VisualizeItem` (:11535). Returns whatever occupied the slot, as
    /// the C++ callers check before overwriting.
    ///
    /// The C++ installs also set the inv-slot field, container, owner, slot and
    /// `ITEM_CHANGED` state on the item itself. Those participants stay with
    /// their existing owners here; this method owns only the slot map, which is
    /// what the session was reaching into.
    pub fn store_item_in_slot_like_cpp(
        &mut self,
        slot: u8,
        item: PlayerInventoryItem,
    ) -> Option<PlayerInventoryItem> {
        self.inventory_items.insert(slot, item)
    }

    /// C++ `Player::RemoveItem` (Player.h:1412, Player.cpp:11553), which nulls
    /// the slot (:11612, :11754). Its own comment notes that removal takes the
    /// item out of storage without changing the item, and that is exactly what
    /// this does: the returned record is handed back unmodified.
    pub fn remove_item_from_slot_like_cpp(&mut self, slot: u8) -> Option<PlayerInventoryItem> {
        self.inventory_items.remove(&slot)
    }

    /// Empty every slot and drop the item objects with them, as C++ does when
    /// the Player is torn down (`delete m_items[i]`, Player.cpp:360) and when a
    /// fresh Player starts from empty slots (`Player::Create`, :403-404).
    pub fn clear_items_and_objects_like_cpp(&mut self) {
        self.inventory_items.clear();
        self.item_objects.clear();
    }

    /// Refresh the entry and inventory type recorded for one slot, only when
    /// the slot still holds the expected item.
    ///
    /// Departure kept explicit: C++ keeps an `Item*` in `m_items[slot]` and
    /// reads the entry and inventory type off the item itself, so it has no
    /// second copy to refresh. RustyCore's slot record carries them, and this
    /// guarded update is how they stay in step. The guid check is what keeps a
    /// stale caller from relabelling whatever now occupies the slot.
    pub fn update_slot_item_metadata_like_cpp(
        &mut self,
        slot: u8,
        item_guid: ObjectGuid,
        entry_id: u32,
        inventory_type: Option<u8>,
    ) -> bool {
        let Some(item) = self
            .inventory_items
            .get_mut(&slot)
            .filter(|item| item.guid == item_guid)
        else {
            return false;
        };
        item.entry_id = entry_id;
        item.inventory_type = inventory_type;
        true
    }

    pub fn inventory_items(&self) -> &HashMap<u8, PlayerInventoryItem> {
        &self.inventory_items
    }

    pub fn inventory_items_mut(&mut self) -> &mut HashMap<u8, PlayerInventoryItem> {
        &mut self.inventory_items
    }

    /// The buyback-slot half of C++ `Player::AddItemToBuyBackSlot`
    /// (Player.cpp:12644), whose slot install is `m_items[slot] = pItem` at
    /// :12681. Returns whatever occupied the slot.
    ///
    /// Two things stay where they already are rather than moving here. The
    /// oldest-slot search, the sell-price and timestamp fields and the
    /// `m_currentBuybackSlot` advance remain with the vendor callers that
    /// perform them today; and C++ holds buyback items in the same `m_items`
    /// array as the rest of the inventory (slots `BUYBACK_SLOT_START` to
    /// `BUYBACK_SLOT_END`), while RustyCore keeps them in a separate map. That
    /// split is pre-existing, and naming the transition makes it visible
    /// instead of leaving it as a raw map write.
    pub fn store_buyback_item_in_slot_like_cpp(
        &mut self,
        slot: u8,
        item: PlayerInventoryItem,
    ) -> Option<PlayerInventoryItem> {
        self.buyback_items.insert(slot, item)
    }

    /// The slot-clearing half of C++ `Player::RemoveItemFromBuyBackSlot`
    /// (Player.cpp:12709), which nulls the slot at :12729. The item state,
    /// stored-loot cleanup, price/timestamp reset and free-slot bookkeeping
    /// C++ performs around it stay with their current owners.
    pub fn remove_buyback_item_from_slot_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<PlayerInventoryItem> {
        self.buyback_items.remove(&slot)
    }

    /// Return the buyback runtime to its constructed state: every slot empty,
    /// prices and timestamps zero as C++ leaves them after
    /// `RemoveItemFromBuyBackSlot` (Player.cpp:12733-12734), and the current
    /// slot back to `BUYBACK_SLOT_START` as the Player constructor sets it
    /// (Player.cpp:220).
    pub fn clear_buyback_like_cpp(&mut self) {
        self.buyback_items.clear();
        self.buyback_price = [0; BUYBACK_SLOT_COUNT];
        self.buyback_timestamp = [0; BUYBACK_SLOT_COUNT];
        self.current_buyback_slot = BUYBACK_SLOT_START;
    }

    pub fn buyback_items(&self) -> &HashMap<u8, PlayerInventoryItem> {
        &self.buyback_items
    }

    pub fn buyback_items_mut(&mut self) -> &mut HashMap<u8, PlayerInventoryItem> {
        &mut self.buyback_items
    }

    pub const fn buyback_price(&self) -> &[u32; BUYBACK_SLOT_COUNT] {
        &self.buyback_price
    }

    pub fn buyback_price_mut(&mut self) -> &mut [u32; BUYBACK_SLOT_COUNT] {
        &mut self.buyback_price
    }

    pub const fn buyback_timestamp(&self) -> &[i64; BUYBACK_SLOT_COUNT] {
        &self.buyback_timestamp
    }

    pub fn buyback_timestamp_mut(&mut self) -> &mut [i64; BUYBACK_SLOT_COUNT] {
        &mut self.buyback_timestamp
    }

    pub const fn current_buyback_slot(&self) -> u8 {
        self.current_buyback_slot
    }

    pub fn set_current_buyback_slot(&mut self, slot: u8) {
        self.current_buyback_slot = slot;
    }

    /// Take ownership of an item object by guid. In C++ the object itself is
    /// created by `Item::CreateItem` and owned through the pointer the Player
    /// stores (`Player::StoreNewItem`, Player.cpp:11166, and
    /// `Player::MoveItemToInventory`, :11655); RustyCore keeps the objects in
    /// this guid-keyed store beside the slot map, so ownership is this insert.
    /// Returns any object already held under the same guid.
    pub fn store_item_object_like_cpp(&mut self, item: Item) -> Option<Item> {
        let item_guid = item.object().guid();
        self.item_objects.insert(item_guid, item)
    }

    /// Release an item object the Player no longer holds, as C++ does when the
    /// item leaves the Player in `Player::DestroyItem` (Player.cpp:11683) or
    /// `Player::RemoveItem` (:11553). Returns the released object so the caller
    /// can finish whatever C++ does with the pointer it took out.
    pub fn remove_item_object_like_cpp(&mut self, item_guid: ObjectGuid) -> Option<Item> {
        self.item_objects.remove(&item_guid)
    }

    pub fn item_objects(&self) -> &HashMap<ObjectGuid, Item> {
        &self.item_objects
    }

    pub fn item_objects_mut(&mut self) -> &mut HashMap<ObjectGuid, Item> {
        &mut self.item_objects
    }
}

impl Default for PlayerInventoryRuntime {
    fn default() -> Self {
        Self {
            equipment_inventory_authority_complete_like_cpp: false,
            inventory_items: HashMap::new(),
            buyback_items: HashMap::new(),
            buyback_price: [0; BUYBACK_SLOT_COUNT],
            buyback_timestamp: [0; BUYBACK_SLOT_COUNT],
            current_buyback_slot: BUYBACK_SLOT_START,
            item_objects: HashMap::new(),
        }
    }
}
