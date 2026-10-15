// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the void-storage bodies moved to
//! `wow_world_application::void_storage_session` (#1363 slice 4c): the two
//! `cfg(test)` packet entry points that were nested in the moved files, and
//! one-call method forwards with this session as the host for the functions
//! the scenarios call, so no scenario call site changes.

#![cfg(test)]

use std::collections::HashMap;

use wow_core::ObjectGuid;
use wow_entities::{Item, PlayerInventoryItem as InventoryItem};
use wow_packet::WorldPacket;
use wow_world_application::VoidWithdrawalItemPublicationLikeCpp;
use wow_world_inventory::{
    EffectiveVoidStorageRandomPropertiesLikeCpp, PlannedVoidDestroyedInventoryItemLikeCpp,
};

use crate::session::{RepresentedVoidStorageItemLikeCpp, WorldSession};

impl WorldSession {
    pub(crate) async fn handle_void_storage_unlock(&mut self, pkt: WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_application::handle_void_storage_unlock_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    pub(crate) async fn handle_void_storage_transfer(&mut self, pkt: WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_application::handle_void_storage_transfer_with_generators_like_cpp(
            self,
            &generators,
            pkt,
        )
        .await;
    }

    pub(crate) async fn handle_void_storage_swap_item(&mut self, pkt: WorldPacket) {
        wow_world_application::handle_void_storage_swap_item(self, pkt).await
    }

    pub(crate) fn apply_committed_void_storage_unlock_like_cpp(&mut self) {
        wow_world_application::apply_committed_void_storage_unlock_like_cpp(self)
    }

    pub(crate) fn void_storage_swap_destination_slot_like_cpp(dst_slot: u32) -> u8 {
        wow_world_application::void_storage_swap_destination_slot_like_cpp(dst_slot)
    }

    pub(crate) fn effective_void_storage_random_properties_like_cpp(
        &self,
        random_properties_id: i32,
        random_properties_seed: i32,
    ) -> EffectiveVoidStorageRandomPropertiesLikeCpp {
        wow_world_application::effective_void_storage_random_properties_like_cpp(
            self,
            random_properties_id,
            random_properties_seed,
        )
    }

    pub(crate) fn void_storage_enchantments_db_string_like_cpp(
        enchantment_ids: &[i32; wow_entities::MAX_ENCHANTMENT_SLOT],
    ) -> String {
        wow_world_application::void_storage_enchantments_db_string_like_cpp(enchantment_ids)
    }

    pub(crate) fn void_storage_merged_item_write_like_cpp(
        &self,
        inventory_item: &InventoryItem,
        item: &Item,
        enchantments: &str,
    ) -> wow_persistence::VoidStorageMergedInventoryItemWriteLikeCpp {
        wow_world_application::void_storage_merged_item_write_like_cpp(
            self,
            inventory_item,
            item,
            enchantments,
        )
    }

    pub(crate) fn apply_effective_void_storage_random_properties_like_cpp(
        item: &mut Item,
        random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
    ) {
        wow_world_application::apply_effective_void_storage_random_properties_like_cpp(
            item,
            random_properties,
        )
    }

    pub(crate) fn plan_void_storage_destroyed_items_like_cpp(
        &self,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        cleared_mainhand_enchantments: Vec<wow_constants::EnchantmentSlot>,
    ) -> Option<Vec<PlannedVoidDestroyedInventoryItemLikeCpp>> {
        wow_world_application::plan_void_storage_destroyed_items_like_cpp(
            self,
            bag,
            slot,
            inventory_item,
            cleared_mainhand_enchantments,
        )
    }

    pub(crate) fn clear_item_publication_changes_like_cpp(item: &mut Item) {
        wow_world_application::clear_item_publication_changes_like_cpp(item)
    }

    pub(crate) fn apply_committed_void_storage_destroyed_items_like_cpp(
        &mut self,
        destroyed_items: &[PlannedVoidDestroyedInventoryItemLikeCpp],
    ) -> Option<(Vec<ObjectGuid>, Vec<u32>)> {
        wow_world_application::apply_committed_void_storage_destroyed_items_like_cpp(
            self,
            destroyed_items,
        )
    }

    pub(crate) fn void_storage_withdrawal_container_db_guid_like_cpp(
        &self,
        bag: u8,
        planned_container_db_guids: &HashMap<u8, u64>,
    ) -> Option<u64> {
        wow_world_application::void_storage_withdrawal_container_db_guid_like_cpp(
            self,
            bag,
            planned_container_db_guids,
        )
    }

    pub(crate) fn void_storage_withdrawal_container_item_guid_like_cpp(
        &self,
        bag: u8,
        planned_container_item_guids: &HashMap<u8, ObjectGuid>,
    ) -> Option<ObjectGuid> {
        wow_world_application::void_storage_withdrawal_container_item_guid_like_cpp(
            self,
            bag,
            planned_container_item_guids,
        )
    }

    pub(crate) fn new_void_withdrawal_item_publication_like_cpp(
        create_item: &Item,
        post_store_item: &Item,
        create_dynamic_flags: u32,
        container_slots: u32,
        map_id: u16,
    ) -> VoidWithdrawalItemPublicationLikeCpp {
        wow_world_application::new_void_withdrawal_item_publication_like_cpp(
            create_item,
            post_store_item,
            create_dynamic_flags,
            container_slots,
            map_id,
        )
    }

    pub(crate) fn merged_void_withdrawal_item_publications_like_cpp(
        item_guid: ObjectGuid,
        store_stack_count: u32,
        store_dynamic_flags: Option<u32>,
        post_store_item: &Item,
        map_id: u16,
    ) -> Vec<VoidWithdrawalItemPublicationLikeCpp> {
        wow_world_application::merged_void_withdrawal_item_publications_like_cpp(
            item_guid,
            store_stack_count,
            store_dynamic_flags,
            post_store_item,
            map_id,
        )
    }

    pub(crate) fn publish_void_storage_item_lifecycle_like_cpp(
        &self,
        map_id: u16,
        destroyed_deposit_items: Vec<((u8, u8), Vec<ObjectGuid>)>,
        withdrawal_item_publications: Vec<VoidWithdrawalItemPublicationLikeCpp>,
    ) {
        wow_world_application::publish_void_storage_item_lifecycle_like_cpp(
            self,
            map_id,
            destroyed_deposit_items,
            withdrawal_item_publications,
        )
    }

    pub(crate) fn represented_void_storage_free_slots_like_cpp(&self) -> Option<usize> {
        wow_world_application::represented_void_storage_free_slots_like_cpp(self)
    }

    pub(crate) fn represented_void_storage_item_by_id_like_cpp(
        &self,
        item_id: u64,
    ) -> Option<(u8, RepresentedVoidStorageItemLikeCpp)> {
        wow_world_application::represented_void_storage_item_by_id_like_cpp(self, item_id)
    }

    pub(crate) fn represented_void_storage_item_at_like_cpp(
        &self,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        wow_world_application::represented_void_storage_item_at_like_cpp(self, slot)
    }

    pub(crate) fn add_represented_void_storage_item_like_cpp(
        &mut self,
        item: RepresentedVoidStorageItemLikeCpp,
    ) -> Option<u8> {
        wow_world_application::add_represented_void_storage_item_like_cpp(self, item)
    }

    pub(crate) fn delete_represented_void_storage_item_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        wow_world_application::delete_represented_void_storage_item_like_cpp(self, slot)
    }
}
