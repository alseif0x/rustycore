// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World adapter for the item handler bodies moved to
//! `wow_world_inventory::item_session` (#1363 slice 8c). Every adapter method is
//! a pure forward to the same-named inherent `WorldSession` method or
//! application operation; the swap-target validation, the raw equip and the
//! position publication moved to `wow_world_application::player_items_session`
//! (#1363 slice 12b1b). Also the session's field split for the inventory
//! player-item builders (`PlayerItemsSessionHostLikeCpp`, slice 12b1b).

use super::*;

use std::future::Future;

use wow_constants::EnchantmentSlot;
use wow_entities::{InventoryStorageMovePlanLikeCpp, ItemPosCount, PlayerQuestStatusRecord};
use wow_world_application::InventoryStorageMoveHostLikeCpp;
use wow_world_inventory::ItemSessionHostLikeCpp;

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
    fn plan_equip_existing_inventory_item_like_cpp(
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
    fn plan_store_existing_inventory_item_at_like_cpp(
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
    fn plan_bank_existing_inventory_item_at_like_cpp(
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
    fn execute_inventory_equip_to_empty_raw_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) -> impl Future<Output = ()> + Send {
        wow_world_application::execute_inventory_equip_to_empty_raw_like_cpp(
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
        wow_world_application::apply_committed_inventory_item_relocation_like_cpp(
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
        wow_world_application::apply_committed_inventory_item_removal_like_cpp(
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
        wow_world_application::apply_committed_inventory_item_swap_like_cpp(
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
        wow_world_application::apply_inventory_item_remove_side_effects_like_cpp(
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
        wow_world_application::apply_inventory_item_store_side_effects_like_cpp(
            self, bag, slot, item_guid,
        )
    }
    fn apply_quest_item_removed_like_cpp(&mut self, entry_id: u32) -> Option<Vec<u32>> {
        wow_world_quest::apply_quest_item_removed_like_cpp(self, entry_id)
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
        wow_world_application::publish_inventory_position_changes_like_cpp(self, positions)
    }
    fn represented_bag_contains_active_item_loot_like_cpp(&self, bag_guid: ObjectGuid) -> bool {
        wow_world_loot_ops::represented_bag_contains_active_item_loot_like_cpp(self, bag_guid)
    }
    fn do_loot_release_all_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::do_loot_release_all_like_cpp(self, player_guid)
    }
    #[cfg(test)]
    fn represented_avg_equipped_item_level_like_cpp(&self) -> Option<f32> {
        WorldSession::represented_avg_equipped_item_level_like_cpp(self)
    }
}

impl wow_world_inventory::PlayerItemsSessionHostLikeCpp for WorldSession {
    fn split_player_items_mut(
        &mut self,
    ) -> wow_world_inventory::PlayerItemsSessionFieldsLikeCpp<'_> {
        wow_world_inventory::PlayerItemsSessionFieldsLikeCpp {
            hub: wow_world_core::session::HubMut {
                core: &mut self.core,
                catalogs: &self.catalogs,
                config: &self.config,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures: &mut self.fixtures,
            },
            inventory: &mut self.inventory,
            loot: &self.loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            spell_state: &self.spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state: &self.quest_state,
        }
    }
}
