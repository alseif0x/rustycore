// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the void-storage bodies that moved to
//! `wow_world_application::void_storage_session` (#1363 slice 4c). Each method
//! forwards to the same-named inherent `WorldSession` operation, or for
//! `split_loot_mut` to the session split, so the bodies call exactly what they
//! called before the move.

use wow_constants::{EnchantmentSlot, InventoryResult};
use wow_core::ObjectGuid;
use wow_entities::{Item, ItemPosCount, PlayerInventoryItem as InventoryItem};
use wow_world_application::VoidStorageSessionHostLikeCpp;
use wow_world_core::session::{HubMut, InventoryPlayerProjectionLikeCpp};
use wow_world_inventory::DirectInventoryStorageOverlayLikeCpp;
use wow_world_loot::LootState;

use crate::session::WorldSession;

impl VoidStorageSessionHostLikeCpp for WorldSession {
    fn split_loot_mut(&mut self) -> (&mut LootState, HubMut<'_>) {
        crate::session::split_loot_mut(self)
    }
    fn void_storage_is_unlocked_like_cpp(&self) -> bool {
        WorldSession::void_storage_is_unlocked_like_cpp(self)
    }
    fn represented_player_flags_value_like_cpp(&self) -> Option<u32> {
        WorldSession::represented_player_flags_value_like_cpp(self)
    }
    fn player_values_update_snapshot(&self) -> Option<InventoryPlayerProjectionLikeCpp> {
        wow_world_inventory::player_values_update_snapshot(self)
    }
    fn get_inventory_item_by_guid_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8, InventoryItem)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.get_inventory_item_by_guid_like_cpp(hub, item_guid)
    }
    fn represented_empty_inventory_positions_like_cpp(&self) -> Option<Vec<(u8, u8)>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_empty_inventory_positions_like_cpp(hub)
    }
    fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        item_guid: ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<EnchantmentSlot>)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.inventory_remove_enchantment_persistence_like_cpp(hub, item_guid, clear_mainhand_only)
    }
    fn plan_store_new_direct_inventory_item_with_overlays_like_cpp(
        &self,
        entry_id: u32,
        count: u32,
        overlays: &[DirectInventoryStorageOverlayLikeCpp],
        vacated_positions: &[(u8, u8)],
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        wow_world_application::plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            self,
            entry_id,
            count,
            overlays,
            vacated_positions,
        )
    }
    fn apply_committed_new_inventory_item_at_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        item_object: Item,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_committed_new_inventory_item_at_like_cpp(
            &mut hub,
            bag,
            slot,
            inventory_item,
            item_object,
        )
    }
    fn refresh_inventory_item_enchantment_duration_refs_like_cpp(&mut self, item_guid: ObjectGuid) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.refresh_inventory_item_enchantment_duration_refs_like_cpp(&mut hub, item_guid)
    }
    fn send_bag_slot_values_update_like_cpp(&self, bag_slot: u8, changed_slot: u8) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_bag_slot_values_update_like_cpp(hub, bag_slot, changed_slot)
    }
}
