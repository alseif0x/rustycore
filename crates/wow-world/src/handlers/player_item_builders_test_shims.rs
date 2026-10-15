// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the player-item builders, planning and
//! committed publications moved in #1363 slice 12b1b to
//! `wow-world-application` (`player_items_session`) and `wow-world-inventory`
//! (the area-based scaling, the equipment-set use context and the uncage
//! item-state load): one-call forwards to the moved functions the scenarios
//! call. No scenario call site changes.

#![cfg(test)]

use wow_constants::{EnchantmentSlot, InventoryResult};
use wow_core::ObjectGuid;
use wow_entities::ItemPosCount;
use wow_world_application::InventorySwapTargetLikeCpp;
use wow_world_inventory::DirectInventoryStorageOverlayLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn execute_inventory_equip_to_empty_raw_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) {
        wow_world_application::execute_inventory_equip_to_empty_raw_like_cpp(
            self,
            source_bag,
            source_slot,
            destination,
        )
        .await
    }

    pub(crate) fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]) {
        wow_world_application::publish_inventory_position_changes_like_cpp(self, positions)
    }

    pub(crate) fn apply_inventory_item_remove_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
        cleared_mainhand_enchantments: &[EnchantmentSlot],
    ) -> bool {
        wow_world_application::apply_inventory_item_remove_side_effects_like_cpp(
            self,
            bag,
            slot,
            item_guid,
            cleared_mainhand_enchantments,
        )
    }

    pub(crate) fn apply_inventory_item_store_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        wow_world_application::apply_inventory_item_store_side_effects_like_cpp(
            self, bag, slot, item_guid,
        )
    }

    pub(crate) fn apply_committed_inventory_item_relocation_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        moved_count: u32,
    ) -> bool {
        wow_world_application::apply_committed_inventory_item_relocation_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            moved_count,
        )
    }

    pub(crate) fn apply_committed_inventory_item_swap_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
    ) -> bool {
        wow_world_application::apply_committed_inventory_item_swap_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
        )
    }

    pub(crate) fn validate_inventory_swap_target_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
        require_exact_destination: bool,
    ) -> Option<(InventoryResult, InventorySwapTargetLikeCpp)> {
        wow_world_application::validate_inventory_swap_target_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            swap,
            require_exact_destination,
        )
    }

    pub(crate) fn plan_equip_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        requested_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, u16)> {
        wow_world_application::plan_equip_existing_inventory_item_like_cpp(
            self,
            source_bag,
            source_slot,
            requested_slot,
            swap,
        )
    }

    pub(crate) fn plan_bank_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>)> {
        wow_world_application::plan_bank_existing_inventory_item_at_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            swap,
        )
    }

    pub(crate) fn plan_store_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        wow_world_application::plan_store_existing_inventory_item_at_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            swap,
        )
    }

    pub(crate) fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        wow_world_application::plan_store_new_direct_inventory_item(self, entry_id, count)
    }

    pub(crate) fn plan_store_new_direct_inventory_item_at(
        &self,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        wow_world_application::plan_store_new_direct_inventory_item_at(
            self, entry_id, count, bag, slot,
        )
    }

    pub(crate) fn plan_store_new_direct_inventory_item_with_overlays_like_cpp(
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

    pub(crate) fn update_represented_item_level_area_based_scaling_with_publication_like_cpp(
        &mut self,
        publish: bool,
    ) -> Option<bool> {
        wow_world_inventory::update_represented_item_level_area_based_scaling_with_publication_like_cpp(
            self, publish,
        )
    }

    pub(crate) fn build_equipment_set_use_context_like_cpp(
        &mut self,
    ) -> wow_world_inventory::EquipmentSetUseContextLikeCpp<'_> {
        wow_world_inventory::build_equipment_set_use_context_like_cpp(self)
    }

    pub(crate) async fn uncage_item_state_like_cpp(
        &self,
        player_db_guid: u64,
        item_db_guid: u64,
    ) -> wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp {
        wow_world_inventory::uncage_item_state_like_cpp(self, player_db_guid, item_db_guid).await
    }
}
