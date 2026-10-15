// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory operations kept in the World (the swap-target validation, the raw
//! equip and the position publication, which run application participants;
//! moved verbatim from the former `items/inventory_moves{,/item_mutations}.rs`),
//! and the World adapter for the item handler bodies moved to
//! `wow_world_inventory::item_session` (#1363 slice 8c). Every adapter method is
//! a pure forward to the same-named inherent `WorldSession` method or
//! application operation.

use super::*;

use std::future::Future;

use wow_constants::EnchantmentSlot;
use wow_entities::{InventoryStorageMovePlanLikeCpp, ItemPosCount, PlayerQuestStatusRecord};
use wow_world_application::InventoryStorageMoveHostLikeCpp;
use wow_world_inventory::ItemSessionHostLikeCpp;

impl WorldSession {
    pub(crate) fn validate_inventory_swap_target_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
        require_exact_destination: bool,
    ) -> Option<(InventoryResult, InventorySwapTargetLikeCpp)> {
        let conditions = self.player_condition_projection_cx_like_cpp();
        wow_world_application::InventoryMovePlanningCxLikeCpp::new(&conditions)
            .validate_inventory_swap_target_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                swap,
                require_exact_destination,
            )
    }

    pub(crate) async fn execute_inventory_equip_to_empty_raw_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) {
        self.inventory_equip_cx_like_cpp()
            .execute_inventory_equip_to_empty_raw_like_cpp(source_bag, source_slot, destination)
            .await;
    }

    pub(crate) fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]) {
        self.inventory_position_publication_cx_like_cpp()
            .publish_inventory_position_changes_like_cpp(positions);
    }
}

impl ItemSessionHostLikeCpp for WorldSession {
    fn storage_move_plan_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        target: InventoryStorageTargetLikeCpp,
    ) -> Option<Result<InventoryStorageMovePlanLikeCpp, InventoryResult>> {
        <WorldSession as InventoryStorageMoveHostLikeCpp>::storage_move_plan_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            target,
        )
    }
    fn execute_inventory_storage_move_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        target: InventoryStorageTargetLikeCpp,
        quest_checks: InventoryStorageQuestChecksLikeCpp,
        represented_move: Option<RepresentedBankItemMoveLikeCpp>,
    ) -> impl Future<Output = ()> + Send {
        wow_world_application::execute_inventory_storage_move_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            target,
            quest_checks,
            represented_move,
        )
    }
    fn validate_inventory_swap_target_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
        require_exact_destination: bool,
    ) -> Option<(InventoryResult, InventorySwapTargetLikeCpp)> {
        WorldSession::validate_inventory_swap_target_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            swap,
            require_exact_destination,
        )
    }
    fn plan_equip_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        requested_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, u16)> {
        WorldSession::plan_equip_existing_inventory_item_like_cpp(
            self,
            source_bag,
            source_slot,
            requested_slot,
            swap,
        )
    }
    fn plan_store_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        WorldSession::plan_store_existing_inventory_item_at_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            swap,
        )
    }
    fn plan_bank_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>)> {
        WorldSession::plan_bank_existing_inventory_item_at_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            swap,
        )
    }
    fn execute_inventory_equip_to_empty_raw_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::execute_inventory_equip_to_empty_raw_like_cpp(
            self,
            source_bag,
            source_slot,
            destination,
        )
    }
    fn apply_committed_inventory_item_relocation_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        moved_count: u32,
    ) -> bool {
        WorldSession::apply_committed_inventory_item_relocation_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            moved_count,
        )
    }
    fn apply_committed_inventory_item_removal_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        WorldSession::apply_committed_inventory_item_removal_like_cpp(
            self,
            source_bag,
            source_slot,
            item_guid,
        )
    }
    fn apply_committed_inventory_item_swap_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
    ) -> bool {
        WorldSession::apply_committed_inventory_item_swap_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
        )
    }
    fn apply_inventory_item_remove_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
        cleared_mainhand_enchantments: &[EnchantmentSlot],
    ) -> bool {
        WorldSession::apply_inventory_item_remove_side_effects_like_cpp(
            self,
            bag,
            slot,
            item_guid,
            cleared_mainhand_enchantments,
        )
    }
    fn apply_inventory_item_store_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        WorldSession::apply_inventory_item_store_side_effects_like_cpp(self, bag, slot, item_guid)
    }
    fn apply_quest_item_removed_like_cpp(&mut self, entry_id: u32) -> Option<Vec<u32>> {
        WorldSession::apply_quest_item_removed_like_cpp(self, entry_id)
    }
    fn plan_item_transfer_quest_persistence_like_cpp(
        &self,
        removed_entries_in_order: &[u32],
        post_removal_non_bank_counts: &[(u32, u32)],
        added_items_in_order: &[(u32, u32, u32)],
    ) -> Vec<PlayerQuestStatusRecord> {
        wow_world_quest::plan_item_transfer_quest_persistence_like_cpp(
            self,
            removed_entries_in_order,
            post_removal_non_bank_counts,
            added_items_in_order,
        )
    }
    fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]) {
        WorldSession::publish_inventory_position_changes_like_cpp(self, positions)
    }
    fn send_represented_item_bonus_player_stat_update_like_cpp(&mut self) -> bool {
        WorldSession::send_represented_item_bonus_player_stat_update_like_cpp(self)
    }
    fn represented_bag_contains_active_item_loot_like_cpp(&self, bag_guid: ObjectGuid) -> bool {
        WorldSession::represented_bag_contains_active_item_loot_like_cpp(self, bag_guid)
    }
    fn do_loot_release_all_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::do_loot_release_all_like_cpp(self, player_guid)
    }
    fn record_represented_titan_grip_penalty_action_like_cpp(&mut self) {
        WorldSession::record_represented_titan_grip_penalty_action_like_cpp(self)
    }
    fn record_represented_avg_equipped_item_level_update_like_cpp(&mut self) {
        WorldSession::record_represented_avg_equipped_item_level_update_like_cpp(self)
    }
}
