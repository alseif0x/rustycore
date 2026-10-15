// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the player-item bodies moved to
//! `wow-world-inventory` in #1363 slice 12b1 (`player_items_session`): the
//! three repair test entry points (verbatim from the nested `#[cfg(test)]`
//! methods of `session/player_items/{durability,bank}.rs`, now calling the
//! moved functions), one-call forwards to the moved functions the scenarios
//! call, and the five one-statement wrappers (the appearance adds, the
//! modified-appearance catalog reads and the account heirloom add) moved
//! verbatim because only the scenarios still call them once the moved bodies
//! make the same split and state call through the host, and the recorded
//! offhand-request accessor (verbatim from the retired
//! `unit_tests/session/player_items/offhand/f3_shims.rs`, without its inner
//! `#[cfg(test)]`, which this file's `#![cfg(test)]` covers). No scenario call
//! site changes.

#![cfg(test)]

use wow_core::ObjectGuid;
use wow_world_inventory::RepresentedAutoUnequipOffhandLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn repair_inventory_item_durability_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        take_cost: bool,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_inventory::repair_inventory_item_durability_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            item_guid,
            take_cost,
            discount,
            repair_cost_rate,
        )
        .await
    }

    pub(crate) async fn repair_all_inventory_item_durability_with_player_money_like_cpp(
        &mut self,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_inventory::repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp(
            self,
            generators.item.as_ref(),
            discount,
            repair_cost_rate,
        )
        .await
    }

    pub(crate) async fn repair_all_inventory_item_durability_with_guild_bank_like_cpp(
        &mut self,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_inventory::repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp(
            self,
            generators.item.as_ref(),
            discount,
            repair_cost_rate,
        )
        .await
    }

    pub(crate) fn apply_represented_durability_loss_all_like_cpp(
        &mut self,
        percent: f64,
        inventory: bool,
    ) -> usize {
        wow_world_inventory::apply_represented_durability_loss_all_like_cpp(
            self, percent, inventory,
        )
    }

    pub(crate) fn record_represented_item_mods_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        apply: bool,
    ) -> usize {
        wow_world_inventory::record_represented_item_mods_like_cpp(self, item_guid, slot, apply)
    }

    pub(crate) fn send_represented_item_bonus_player_stat_update_like_cpp(&mut self) -> bool {
        wow_world_inventory::send_represented_item_bonus_player_stat_update_like_cpp(self)
    }

    pub(crate) fn record_represented_items_set_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        apply: bool,
    ) -> bool {
        wow_world_inventory::record_represented_items_set_item_like_cpp(self, item_guid, apply)
    }

    pub(crate) fn record_direct_inventory_item_set_remove_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        wow_world_inventory::record_direct_inventory_item_set_remove_like_cpp(
            self, bag, slot, item_guid,
        )
    }

    pub(crate) fn represented_auto_unequip_offhand_if_need_like_cpp(
        &mut self,
        force: bool,
    ) -> bool {
        wow_world_inventory::represented_auto_unequip_offhand_if_need_like_cpp(self, force)
    }

    pub(crate) fn represented_auto_unequip_offhand_requests_like_cpp(
        &self,
    ) -> &[RepresentedAutoUnequipOffhandLikeCpp] {
        self.inventory
            .represented_auto_unequip_offhand_requests_like_cpp()
    }

    pub(crate) fn on_item_added_to_collection_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) -> Vec<wow_entities::PlayerValuesUpdate> {
        wow_world_inventory::on_item_added_to_collection_like_cpp(self, item)
    }

    pub(crate) fn add_item_appearance_for_item_like_cpp(
        &mut self,
        item_id: u32,
        appearance_mod_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        wow_world_inventory::add_item_appearance_for_item_like_cpp(self, item_id, appearance_mod_id)
    }

    pub(crate) fn can_add_item_appearance_represented_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> bool {
        wow_world_inventory::can_add_item_appearance_represented_like_cpp(
            self,
            item_modified_appearance_id,
        )
    }

    pub(crate) fn add_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_item_appearance_like_cpp(&mut hub, item_modified_appearance_id)
    }

    pub(crate) fn add_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_temporary_item_appearance_like_cpp(
            &mut hub,
            item_modified_appearance_id,
            item_guid,
        )
    }

    pub(crate) fn item_modified_appearance_ref(&self, id: u32) -> Option<(u32, u16)> {
        self.catalogs.item_modified_appearance_ref(id)
    }

    pub(crate) fn item_modified_appearance_for_item(
        &self,
        item_id: u32,
        appearance_mod_id: u32,
    ) -> Option<u32> {
        self.catalogs
            .item_modified_appearance_for_item(item_id, appearance_mod_id)
    }

    pub(crate) fn add_account_heirloom_like_cpp(&mut self, item_id: u32, flags: u32) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.add_account_heirloom_like_cpp(&mut hub, item_id, flags)
    }
}
