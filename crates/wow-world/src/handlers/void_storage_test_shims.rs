// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the void-storage bodies moved to
//! `wow_world_application::void_storage_session` (#1363 slice 4c): the two
//! `cfg(test)` packet entry points that were nested in the moved files, and
//! one-call method forwards with this session as the host for the functions
//! the scenarios call, so no scenario call site changes.

#![cfg(test)]

use wow_core::ObjectGuid;
use wow_entities::Item;
use wow_packet::WorldPacket;
use wow_world_application::VoidWithdrawalItemPublicationLikeCpp;
use wow_world_inventory::EffectiveVoidStorageRandomPropertiesLikeCpp;

use crate::session::WorldSession;

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

    pub(crate) fn void_storage_swap_destination_slot_like_cpp(dst_slot: u32) -> u8 {
        wow_world_application::void_storage_swap_destination_slot_like_cpp(dst_slot)
    }

    pub(crate) fn void_storage_enchantments_db_string_like_cpp(
        enchantment_ids: &[i32; wow_entities::MAX_ENCHANTMENT_SLOT],
    ) -> String {
        wow_world_application::void_storage_enchantments_db_string_like_cpp(enchantment_ids)
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

    pub(crate) fn clear_item_publication_changes_like_cpp(item: &mut Item) {
        wow_world_application::clear_item_publication_changes_like_cpp(item)
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
}
