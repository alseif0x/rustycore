// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Bank-slot updates and represented bank-state accessors for Session inventory.

use crate::{RepresentedBankItemMoveLikeCpp, RepresentedGuildRepairBankStateLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::{
    RepresentedGuildBankInventoryMoveLikeCpp, RepresentedGuildBankListRequestLikeCpp,
    RepresentedGuildBankMoneyMoveLikeCpp, RepresentedGuildBankTabActionLikeCpp,
    RepresentedGuildRepairBankWithdrawLikeCpp,
};
use wow_entities::{INVENTORY_SLOT_BAG_0, Player};
use wow_world_core::entity_update_bridge::player_values_update_to_update_object;
use wow_world_core::session::{HubMut, HubRef};

impl crate::InventoryState {
    /// C++ `GetItemCount(entry, false)`: count carried/equipped items while
    /// excluding personal-bank slots and bank-bag contents.
    pub fn represented_non_bank_item_count_like_cpp(
        &self,
        hub: HubRef<'_>,
        entry_id: u32,
    ) -> Option<u32> {
        let inventory_items = self.resolved_inventory_items_like_cpp(hub)?;
        let item_objects = self.resolved_inventory_item_objects_like_cpp(hub)?;
        let top_level_count = inventory_items
            .iter()
            .filter(|(slot, _)| !wow_entities::is_bank_pos(INVENTORY_SLOT_BAG_0, **slot))
            .filter_map(|(_, inventory_item)| item_objects.get(&inventory_item.guid))
            .filter(|item| item.object().entry() == entry_id && !item.is_in_trade())
            .fold(0u32, |count, item| count.saturating_add(item.count()));
        Some(
            item_objects
                .values()
                .filter(|item| {
                    item.is_in_bag()
                        && !wow_entities::is_bank_pos(item.bag_slot(), item.slot())
                        && item.object().entry() == entry_id
                        && !item.is_in_trade()
                })
                .fold(top_level_count, |count, item| {
                    count.saturating_add(item.count())
                }),
        )
    }

    pub fn send_player_bank_bag_slots_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        count: u8,
    ) {
        let Some(guid) = hub.core.player_guid() else {
            return;
        };
        let Some(mut player) = self.player_values_update_snapshot(hub) else {
            return;
        };

        player.set_bank_bag_slot_count(count);
        player.mark_bank_bag_slot_count_changed_like_cpp();
        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, hub.core.player_map_id_like_cpp(), &update)
        {
            hub.core.send_packet(&packet);
        }
    }

    pub fn send_player_bank_bag_slot_flag_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        slot: usize,
        value: u32,
    ) {
        let Some(guid) = hub.core.player_guid() else {
            return;
        };
        let Some(mut player) = self.player_values_update_snapshot(hub) else {
            return;
        };

        if !player.set_bank_bag_slot_flag_value_like_cpp(slot, value) {
            return;
        }
        player.mark_bank_bag_slot_flag_changed_like_cpp(slot);
        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, hub.core.player_map_id_like_cpp(), &update)
        {
            hub.core.send_packet(&packet);
        }
    }

    pub fn represented_bank_bag_slot_flag_like_cpp(
        &self,
        hub: HubRef<'_>,
        slot: usize,
    ) -> Option<u32> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| player.bank_bag_slot_flag_value_like_cpp(slot))
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return self
                .represented_bank_bag_slot_flags_like_cpp
                .get(slot)
                .copied();
        }
        canonical
    }

    pub fn represented_guild_repair_bank_state_like_cpp(
        &self,
    ) -> Option<RepresentedGuildRepairBankStateLikeCpp> {
        self.represented_guild_repair_bank_state_like_cpp
    }

    pub fn resolved_player_bank_bag_slot_count_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u8> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(Player::bank_bag_slot_count);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .player_bank_bag_slot_count_like_cpp,
            );
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_bank_bag_slot_count_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> u8 {
        self.resolved_player_bank_bag_slot_count_like_cpp(hub)
            .expect("test Player bank-bag-slot owner must resolve")
    }
}

impl crate::InventoryState {
    pub fn set_player_bank_bag_slot_count_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        count: u8,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_bank_bag_slot_count(count))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_item_test_fixture_like_cpp
                .player_bank_bag_slot_count_like_cpp = count;
        }
        canonical || cfg!(any(test, feature = "test-fixtures")) && hub.core.player_handle_like_cpp.is_none()
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_bank_item_move_like_cpp(
        &mut self,
        move_like_cpp: RepresentedBankItemMoveLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_bank_item_moves_like_cpp
            .push(move_like_cpp);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_bank_item_moves_like_cpp(&self) -> &[RepresentedBankItemMoveLikeCpp] {
        &self.represented_bank_item_moves_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_guild_bank_inventory_moves_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankInventoryMoveLikeCpp] {
        &self.represented_guild_bank_inventory_moves_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_guild_bank_list_requests_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankListRequestLikeCpp] {
        &self.represented_guild_bank_list_requests_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_guild_bank_money_moves_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankMoneyMoveLikeCpp] {
        &self.represented_guild_bank_money_moves_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_guild_bank_tab_actions_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankTabActionLikeCpp] {
        &self.represented_guild_bank_tab_actions_like_cpp
    }

    pub fn set_represented_bank_bag_slot_flag_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: usize,
        value: u32,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_bank_bag_slot_flag_value_like_cpp(slot, value)
            })
            .unwrap_or(false);
        #[cfg(any(test, feature = "test-fixtures"))]
        if (canonical || hub.core.player_handle_like_cpp.is_none())
            && let Some(flag) = self.represented_bank_bag_slot_flags_like_cpp.get_mut(slot)
        {
            *flag = value;
            return true;
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_guild_repair_bank_state_like_cpp(
        &mut self,
        state: Option<RepresentedGuildRepairBankStateLikeCpp>,
    ) {
        self.represented_guild_repair_bank_state_like_cpp = state;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_guild_repair_bank_withdraws_like_cpp(
        &self,
    ) -> &[RepresentedGuildRepairBankWithdrawLikeCpp] {
        &self.represented_guild_repair_bank_withdraws_like_cpp
    }
}
