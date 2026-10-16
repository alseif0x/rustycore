// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the item-objective, source-item and reward-inventory
//! quest bodies that moved to `wow_world_quest` (#1363 slice 6b). Each method
//! forwards to the same-named session operation or field
//! (`make_inventory_item_object` to the World inventory context's). Moved out
//! of `handlers/quest` in #1363 slice 12d (one adapter file per destination
//! crate, outside the hotspot trees), where the retired split forwards' bodies
//! now sit.

use wow_constants::ItemContext;
use wow_core::ObjectGuid;
use wow_entities::{Item, ItemObjectUpdateLikeCpp, ItemPosCount, SendNewItemPlan};
use wow_world_quest::QuestItemStorageHostLikeCpp;

use super::quest::InventoryResult;
use crate::session::{InventoryItem, WorldSession};

impl QuestItemStorageHostLikeCpp for WorldSession {
    fn catalogs_mut(&mut self) -> &mut wow_world_core::session::SessionCatalogs {
        &mut self.catalogs
    }
    fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<InventoryItem> {
        self.inventory
            .get_inventory_item_by_pos(crate::session::hub_ref(self), bag, slot)
    }
    fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem> {
        self.inventory
            .resolved_inventory_item_like_cpp(crate::session::hub_ref(self), slot)
    }
    fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item> {
        self.inventory
            .resolved_inventory_item_object_like_cpp(crate::session::hub_ref(self), guid)
    }
    fn represented_non_bank_item_count_like_cpp(&self, entry_id: u32) -> Option<u32> {
        self.inventory
            .represented_non_bank_item_count_like_cpp(crate::session::hub_ref(self), entry_id)
    }
    fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_inventory_item_object_updates_like_cpp(&mut hub, item_guid, updates)
    }
    fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_like_cpp(&mut hub, slot, item)
    }
    fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_object(&mut hub, item)
    }
    fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item {
        crate::session::cx_inventory_ref(self).make_inventory_item_object(
            item_guid, entry_id, owner_guid, count, durability, context, slot,
        )
    }
    fn send_new_item_plan(&self, plan: &SendNewItemPlan) {
        wow_world_inventory::send_new_item_plan(self, plan)
    }
    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        wow_world_inventory::send_player_values_update_from_entity_bridge(
            self,
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
    }
    fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        wow_world_application::plan_store_new_direct_inventory_item(self, entry_id, count)
    }
}
