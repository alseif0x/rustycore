// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item character handlers: the swap, equip, bag-store and destroy handlers,
//! the inventory move planning and execution they run (swap queue, child
//! redirects and equips, stack merge, real swap, offhand auto-unequip) and the
//! autobank/autostore-bank moves (#1363 slice 8c; moved from the World
//! `handlers/character/{items/*,bank,mod}.rs`).
//!
//! C++ anchors stay on each body: `WorldSession::HandleSwapInvItemOpcode`,
//! `HandleAutoEquipItemOpcode`, `HandleAutoEquipItemSlotOpcode`,
//! `HandleSwapItem`, `HandleAutoStoreBagItemOpcode`, `HandleDestroyItemOpcode`,
//! `HandleAutoBankItemOpcode`, `HandleAutoStoreBankItemOpcode` and the
//! `Player::SwapItem`/`DestroyItem`/`EquipChildItem` paths they reach.
//!
//! The bodies run on the Player's session through [`ItemSessionHostLikeCpp`].
//! Each host method is the same-named `WorldSession` operation, called at the
//! exact point the body called it before the move. The application storage move
//! (`wow_world_application::execute_inventory_storage_move_like_cpp`) and its
//! plan (`InventoryStorageMoveHostLikeCpp::storage_move_plan_like_cpp`) stay
//! behind the host, and their futures are returned unchanged. A World wrapper
//! that was one `split_inventory_{ref,mut}` plus one `InventoryState` call is
//! that same split and call here; so are the wrappers that were one
//! `InventoryState` call on the session's inventory field with core access
//! arguments (the item relocation and dynamic-flag publications, the offhand
//! request record), whose arguments are the same reads through the split's hub,
//! and the bank-use check, which is the same `can_use_current_bank_with_access`
//! call with the interaction, NPC and inventory accesses through the host.

use std::future::Future;

use wow_constants::{EnchantmentSlot, InventoryResult};
use wow_core::ObjectGuid;
use wow_entities::{InventoryStorageMovePlanLikeCpp, ItemPosCount, PlayerQuestStatusRecord};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use crate::{
    InventoryStorageQuestChecksLikeCpp, InventoryStorageTargetLikeCpp, InventorySwapTargetLikeCpp,
    RepresentedBankItemMoveLikeCpp, VendorSessionHostLikeCpp,
};

mod bank;
mod destruction;
mod handlers;
mod item_mutations;
mod moves;
mod real_swap;

pub use bank::{
    handle_autobank_item_with_generator_like_cpp,
    handle_autostore_bank_item_with_generator_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use destruction::plan_destroyed_inventory_quest_persistence_like_cpp;
pub use destruction::{
    destroy_inventory_full_stack_by_pos_like_cpp,
    destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp, handle_destroy_item,
};
pub use handlers::{
    handle_auto_equip_item_slot_with_generator_like_cpp,
    handle_auto_equip_item_with_generator_like_cpp,
    handle_auto_store_bag_item_with_generator_like_cpp,
    handle_swap_inv_item_with_generator_like_cpp, handle_swap_item_with_generator_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use moves::{plan_inventory_child_redirect_like_cpp, plan_inventory_equip_child_like_cpp};
#[cfg(any(test, feature = "test-fixtures"))]
pub use real_swap::plan_inventory_real_swap_children_like_cpp;

pub trait ItemSessionHostLikeCpp: VendorSessionHostLikeCpp {
    fn storage_move_plan_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        target: InventoryStorageTargetLikeCpp,
    ) -> Option<Result<InventoryStorageMovePlanLikeCpp, InventoryResult>>;
    #[allow(clippy::too_many_arguments)]
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
    ) -> impl Future<Output = ()> + Send;
    fn validate_inventory_swap_target_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
        require_exact_destination: bool,
    ) -> Option<(InventoryResult, InventorySwapTargetLikeCpp)>;
    fn plan_equip_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        requested_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, u16)>;
    fn plan_store_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)>;
    fn plan_bank_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>)>;
    fn execute_inventory_equip_to_empty_raw_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) -> impl Future<Output = ()> + Send;
    fn apply_committed_inventory_item_relocation_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        moved_count: u32,
    ) -> bool;
    fn apply_committed_inventory_item_removal_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        item_guid: ObjectGuid,
    ) -> bool;
    fn apply_committed_inventory_item_swap_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
    ) -> bool;
    fn apply_inventory_item_remove_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
        cleared_mainhand_enchantments: &[EnchantmentSlot],
    ) -> bool;
    fn apply_inventory_item_store_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool;
    fn apply_quest_item_removed_like_cpp(&mut self, entry_id: u32) -> Option<Vec<u32>>;
    fn plan_item_transfer_quest_persistence_like_cpp(
        &self,
        removed_entries_in_order: &[u32],
        post_removal_non_bank_counts: &[(u32, u32)],
        added_items_in_order: &[(u32, u32, u32)],
    ) -> Vec<PlayerQuestStatusRecord>;
    fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]);
    fn represented_bag_contains_active_item_loot_like_cpp(&self, bag_guid: ObjectGuid) -> bool;
    fn do_loot_release_all_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send;
    fn record_represented_titan_grip_penalty_action_like_cpp(&mut self);
    /// The World test build's average equipped item level
    /// (`WorldSession::represented_avg_equipped_item_level_like_cpp`); the record
    /// below calls it only when `test_fixtures_ref_like_cpp` is `Some`, so this
    /// default is never reached (#1363 slice 12c).
    #[cfg(any(test, feature = "test-fixtures"))]
    fn represented_avg_equipped_item_level_like_cpp(&self) -> Option<f32> {
        None
    }
}

// The equipped-item-level record of the represented equip paths (#1363 slice
// 12c; moved from the World `session/player_items/equipment.rs`).
#[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
pub fn record_represented_avg_equipped_item_level_update_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
) {
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() {
        let Some(avg_equipped_item_level) = host.represented_avg_equipped_item_level_like_cpp()
        else {
            return;
        };
        host.split_inventory_mut()
            .0
            .record_represented_avg_equipped_item_level_update_for_test_like_cpp(
                avg_equipped_item_level,
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryEquipChildPlanLikeCpp {
    pub child_guid: ObjectGuid,
    pub destination_slot: u8,
    pub displaced_storage: Option<(u8, u8, InventoryStorageTargetLikeCpp)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventorySwapStepLikeCpp {
    Done,
    ChildRedirect {
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestroyItemCountAction {
    FullStack,
    PartialStack { new_count: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyQuestItemLikeCpp {
    pub bag: u8,
    pub slot: u8,
    pub entry_id: u32,
    pub count: u32,
}

fn destroy_item_count_action(current_count: u32, requested_count: u32) -> DestroyItemCountAction {
    if requested_count != 0 && current_count > requested_count {
        return DestroyItemCountAction::PartialStack {
            new_count: current_count - requested_count,
        };
    }

    DestroyItemCountAction::FullStack
}

pub fn autostore_bank_quest_checks_like_cpp(
    target: InventoryStorageTargetLikeCpp,
) -> InventoryStorageQuestChecksLikeCpp {
    if target == InventoryStorageTargetLikeCpp::Inventory {
        InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded
    } else {
        // C++ HandleAutoStoreBankItemOpcode intentionally does not call
        // ItemRemovedQuestCheck in its inventory-to-bank branch.
        InventoryStorageQuestChecksLikeCpp::None
    }
}

#[cfg(test)]
#[path = "../../unit_tests/item_session_helpers.rs"]
mod tests;
