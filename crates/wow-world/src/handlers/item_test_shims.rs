// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the item handler bodies moved to
//! `wow_world_inventory::item_session` (#1363 slice 8c). Each forwards to the
//! moved body with this session as its host, so the scenarios keep their method
//! syntax and the character hotspot's test lines do not grow. The seven packet
//! entry points moved here verbatim from the `cfg(test)` methods of
//! `handlers/character/{items/handlers,bank}.rs`; the five plan/destroy entry
//! points name the moved free functions.
//!
//! The six one-statement wrappers after them came verbatim from
//! `session/player_items/{items,persistence,storage_slots,publication,bank}.rs`:
//! their only production callers were the moved bodies, which now make the same
//! split, `InventoryState` or bank-use call through the host.

#![cfg(test)]

use wow_constants::{EnchantmentSlot, InventoryResult};
use wow_core::ObjectGuid;
use wow_entities::{Item, SwapItemPreflightPlan};
use wow_packet::packets::item::{AutoEquipItem, AutoEquipItemSlot, SwapInvItem};
use wow_packet::packets::misc::{AutoBankItem, AutoStoreBankItem};
use wow_world_inventory::{DestroyQuestItemLikeCpp, InventoryEquipChildPlanLikeCpp};

use crate::handlers::quest::PlayerQuestStatus;
use crate::session::{InventoryItem, WorldSession};

impl WorldSession {
    pub(crate) async fn handle_swap_inv_item(&mut self, swap: SwapInvItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_inventory::handle_swap_inv_item_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &catalogs,
            swap,
        )
        .await;
    }

    pub(crate) async fn handle_auto_equip_item(&mut self, equip: AutoEquipItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_inventory::handle_auto_equip_item_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &catalogs,
            equip,
        )
        .await;
    }

    pub(crate) async fn handle_auto_equip_item_slot(&mut self, equip: AutoEquipItemSlot) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_inventory::handle_auto_equip_item_slot_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &catalogs,
            equip,
        )
        .await;
    }

    pub(crate) async fn handle_swap_item(&mut self, swap: wow_packet::packets::item::SwapItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_inventory::handle_swap_item_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &catalogs,
            swap,
        )
        .await;
    }

    pub(crate) async fn handle_auto_store_bag_item(
        &mut self,
        store: wow_packet::packets::item::AutoStoreBagItem,
    ) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_inventory::handle_auto_store_bag_item_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &catalogs,
            store,
        )
        .await;
    }

    pub(crate) async fn handle_autobank_item(&mut self, packet: AutoBankItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_inventory::handle_autobank_item_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &catalogs,
            packet,
        )
        .await;
    }

    pub(crate) async fn handle_autostore_bank_item(&mut self, packet: AutoStoreBankItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_inventory::handle_autostore_bank_item_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &catalogs,
            packet,
        )
        .await;
    }

    pub(crate) fn plan_destroyed_inventory_quest_persistence_like_cpp(
        &self,
        destroyed_items: &[DestroyQuestItemLikeCpp],
    ) -> Option<Vec<PlayerQuestStatus>> {
        wow_world_inventory::plan_destroyed_inventory_quest_persistence_like_cpp(
            self,
            destroyed_items,
        )
    }

    pub(crate) async fn destroy_inventory_full_stack_by_pos_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item: InventoryItem,
        runtime_item: Option<Item>,
        context: &str,
    ) -> bool {
        wow_world_inventory::destroy_inventory_full_stack_by_pos_like_cpp(
            self,
            bag,
            slot,
            item,
            runtime_item,
            context,
        )
        .await
    }

    pub(crate) fn plan_inventory_child_redirect_like_cpp(
        &mut self,
        child_bag: u8,
        child_slot: u8,
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    ) -> Result<u8, InventoryResult> {
        wow_world_inventory::plan_inventory_child_redirect_like_cpp(
            self, child_bag, child_slot, first_src, first_dst, second_src, second_dst,
        )
    }

    pub(crate) fn plan_inventory_equip_child_like_cpp(
        &self,
        parent_bag: u8,
        parent_slot: u8,
        parent_guid: ObjectGuid,
    ) -> Result<Option<InventoryEquipChildPlanLikeCpp>, InventoryResult> {
        wow_world_inventory::plan_inventory_equip_child_like_cpp(
            self,
            parent_bag,
            parent_slot,
            parent_guid,
        )
    }

    pub(crate) fn plan_inventory_real_swap_children_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        source_guid: ObjectGuid,
        destination_bag: u8,
        destination_slot: u8,
        destination_guid: ObjectGuid,
    ) -> Result<Vec<InventoryEquipChildPlanLikeCpp>, InventoryResult> {
        wow_world_inventory::plan_inventory_real_swap_children_like_cpp(
            self,
            source_bag,
            source_slot,
            source_guid,
            destination_bag,
            destination_slot,
            destination_guid,
        )
    }

    pub(crate) fn direct_item_contains_items(&self, item_guid: ObjectGuid) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.direct_item_contains_items(hub, item_guid)
    }

    pub(crate) fn plan_inventory_swap_preflight_like_cpp(
        &self,
        src: u16,
        dst: u16,
    ) -> Option<SwapItemPreflightPlan> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.plan_inventory_swap_preflight_like_cpp(hub, src, dst)
    }

    pub(crate) fn represented_inventory_descendants_postorder_like_cpp(
        &self,
        container_guid: ObjectGuid,
    ) -> Option<Vec<(u8, u8, InventoryItem)>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_inventory_descendants_postorder_like_cpp(hub, container_guid)
    }

    pub(crate) fn send_item_relocation_values_update_like_cpp(
        &self,
        item_guid: ObjectGuid,
        dynamic_flags2_changed: bool,
        cleared_enchantments: &[EnchantmentSlot],
    ) {
        self.inventory
            .send_item_relocation_values_update_with_access_like_cpp(
                &self.core.owned_inventory_access_like_cpp(),
                &self.core.packet_publication_access_like_cpp(),
                item_guid,
                dynamic_flags2_changed,
                cleared_enchantments,
            )
    }

    pub(crate) fn send_item_dynamic_flags_values_update_like_cpp(&self, item_guid: ObjectGuid) {
        self.inventory
            .send_item_dynamic_flags_values_update_with_access_like_cpp(
                &self.core.owned_inventory_access_like_cpp(),
                &self.core.packet_publication_access_like_cpp(),
                item_guid,
            )
    }

    pub(crate) fn represented_can_use_current_bank_like_cpp(&self) -> bool {
        wow_world_inventory::can_use_current_bank_with_access_like_cpp(
            &self.interaction,
            &crate::session::hub_ref(self).trainer_npc_interaction_access_like_cpp(),
            &self.core.owned_inventory_access_like_cpp(),
        )
    }
}
