// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;
use wow_entities::Player;

use crate::canonical_player_access::set_player_visible_item_values_like_cpp;

/// Detached Player projection used by inventory planning and update preparation.
///
/// This value never owns canonical Player authority. It wraps an existing
/// snapshot and deliberately exposes only typed operations through this module.
pub struct InventoryPlayerProjectionLikeCpp {
    player: Player,
}

impl InventoryPlayerProjectionLikeCpp {
    /// Wrap an already-created detached Player snapshot without cloning it.
    pub(crate) fn from_player(player: Player) -> Self {
        Self { player }
    }

    /// Create the fixture-only Player projection with the same identity setup as
    /// the inventory state fixture path.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_like_cpp(guid: ObjectGuid) -> Self {
        let mut player = Player::new(None, false);
        player.unit_mut().world_mut().object_mut().create(guid);
        Self { player }
    }

    pub fn is_valid_pos_like_cpp(&self, bag: u8, slot: u8, explicit_pos: bool) -> bool {
        self.player.is_valid_pos(bag, slot, explicit_pos)
    }

    pub fn for_each_item_guid_like_cpp(
        &self,
        location: wow_entities::ItemSearchLocation,
        callback: impl FnMut(wow_core::ObjectGuid) -> wow_entities::ItemSearchCallbackResult,
    ) -> bool {
        self.player.for_each_item_guid(location, callback)
    }

    pub fn can_store_item_like_cpp(
        &self,
        destination: &mut Vec<wow_entities::ItemPosCount>,
        args: wow_entities::CanStoreItemArgs<'_>,
    ) -> wow_entities::CanStoreItemOutcome {
        self.player.can_store_item(destination, args)
    }

    pub fn can_bank_item_like_cpp(
        &self,
        destination: &mut Vec<wow_entities::ItemPosCount>,
        args: wow_entities::CanBankItemArgs<'_>,
    ) -> wow_constants::InventoryResult {
        self.player.can_bank_item(destination, args)
    }

    pub fn can_use_item_like_cpp(
        &self,
        args: wow_entities::CanUseItemArgs<'_>,
    ) -> wow_constants::InventoryResult {
        self.player.can_use_item(args)
    }

    pub fn can_unequip_item_like_cpp(
        &self,
        args: wow_entities::CanUnequipItemArgs<'_>,
    ) -> wow_constants::InventoryResult {
        self.player.can_unequip_item(args)
    }

    pub fn can_equip_item_like_cpp(
        &self,
        args: wow_entities::CanEquipItemArgs<'_>,
    ) -> wow_entities::CanEquipItemOutcome {
        self.player.can_equip_item(args)
    }

    pub fn can_equip_unique_item_like_cpp(
        &self,
        args: wow_entities::CanEquipUniqueItemArgs<'_>,
    ) -> wow_constants::InventoryResult {
        self.player.can_equip_unique_item(args)
    }

    pub fn is_two_hand_used_template_like_cpp(
        &self,
        main_template: Option<&wow_entities::ItemStorageTemplate>,
    ) -> bool {
        self.player.is_two_hand_used_template(main_template)
    }

    pub fn primary_specialization_id_like_cpp(&self) -> u32 {
        self.player.primary_specialization_id_like_cpp()
    }

    pub fn set_can_dual_wield_like_cpp(&mut self, value: bool) {
        self.player.unit_mut().set_can_dual_wield_like_cpp(value);
    }

    pub fn set_can_titan_grip_like_cpp(&mut self, value: bool, penalty_spell_id: u32) {
        self.player.set_can_titan_grip(value, penalty_spell_id);
    }

    pub fn register_bag_storage_like_cpp(
        &mut self,
        bag_slot: u8,
        bag_guid: wow_core::ObjectGuid,
        bag_size: u8,
    ) -> Result<(), wow_entities::PlayerStorageError> {
        self.player.register_bag_storage(bag_slot, bag_guid, bag_size)
    }

    pub fn store_top_level_item_like_cpp(
        &mut self,
        slot: u8,
        guid: wow_core::ObjectGuid,
    ) -> Result<(), wow_entities::PlayerStorageError> {
        self.player.store_top_level_item(slot, guid)
    }

    pub fn remove_top_level_item_like_cpp(
        &mut self,
        slot: u8,
    ) -> Result<Option<wow_core::ObjectGuid>, wow_entities::PlayerStorageError> {
        self.player.remove_top_level_item(slot)
    }

    pub fn store_bag_item_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        guid: wow_core::ObjectGuid,
    ) -> Result<(), wow_entities::PlayerStorageError> {
        self.player.store_bag_item(bag, slot, guid)
    }

    pub fn remove_bag_item_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
    ) -> Result<Option<wow_core::ObjectGuid>, wow_entities::PlayerStorageError> {
        self.player.remove_bag_item(bag, slot)
    }

    pub fn set_inventory_slot_count_like_cpp(&mut self, count: u8) {
        self.player.set_inventory_slot_count(count);
    }

    pub fn swap_item_preflight_plan_like_cpp(
        &self,
        src: u16,
        dst: u16,
        is_alive: bool,
        src_item: Option<wow_entities::SwapItemPreflightItem>,
        dst_item: Option<wow_entities::SwapItemPreflightItem>,
    ) -> wow_entities::SwapItemPreflightPlan {
        self.player
            .swap_item_preflight_plan(src, dst, is_alive, src_item, dst_item)
    }

    pub fn set_bank_bag_slot_count_like_cpp(&mut self, count: u8) {
        self.player.set_bank_bag_slot_count(count);
    }

    pub fn mark_bank_bag_slot_count_changed_like_cpp(&mut self) {
        self.player.mark_bank_bag_slot_count_changed_like_cpp();
    }

    pub fn set_bank_bag_slot_flag_value_like_cpp(&mut self, index: usize, value: u32) -> bool {
        self.player
            .set_bank_bag_slot_flag_value_like_cpp(index, value)
    }

    pub fn mark_bank_bag_slot_flag_changed_like_cpp(&mut self, index: usize) {
        self.player.mark_bank_bag_slot_flag_changed_like_cpp(index);
    }

    pub fn set_money_like_cpp(&mut self, value: u64) {
        self.player.set_money(value);
    }

    pub fn mark_money_changed_like_cpp(&mut self) {
        self.player.mark_money_changed();
    }

    pub fn set_visible_item_slot_like_cpp(
        &mut self,
        slot: u8,
        item: Option<wow_entities::VisibleItemValues>,
    ) {
        self.player.set_visible_item_slot(slot, item);
    }

    pub fn set_player_visible_item_values_like_cpp(
        &mut self,
        slot: u8,
        values: (i32, u16, u16),
    ) {
        set_player_visible_item_values_like_cpp(&mut self.player, slot, values);
    }

    pub fn mark_visible_item_slot_changed_like_cpp(&mut self, slot: u8) {
        self.player.mark_visible_item_slot_changed(slot);
    }

    pub fn set_virtual_item_like_cpp(
        &mut self,
        index: usize,
        visible: Option<wow_entities::VisibleItemValues>,
    ) {
        self.player.unit_mut().set_virtual_item(index, visible);
    }

    pub fn mark_virtual_item_changed_like_cpp(&mut self, index: usize) {
        self.player.unit_mut().mark_virtual_item_changed(index);
    }

    pub fn set_inv_slot_like_cpp(&mut self, slot: usize, guid: wow_core::ObjectGuid) {
        self.player.set_inv_slot(slot, guid);
    }

    pub fn mark_inv_slot_changed_like_cpp(&mut self, slot: usize) {
        self.player.mark_inv_slot_changed(slot);
    }

    pub fn set_buyback_price_like_cpp(&mut self, slot: usize, price: u32) {
        self.player.set_buyback_price(slot, price);
    }

    pub fn mark_buyback_price_changed_like_cpp(&mut self, slot: usize) {
        self.player.mark_buyback_price_changed(slot);
    }

    pub fn set_buyback_timestamp_like_cpp(&mut self, slot: usize, timestamp: i64) {
        self.player.set_buyback_timestamp(slot, timestamp);
    }

    pub fn mark_buyback_timestamp_changed_like_cpp(&mut self, slot: usize) {
        self.player.mark_buyback_timestamp_changed(slot);
    }

    pub fn clear_data_changes_like_cpp(&mut self) {
        self.player.clear_data_changes();
    }

    pub fn values_update_like_cpp(
        &self,
        include_active_player: bool,
    ) -> wow_entities::PlayerValuesUpdate {
        self.player.values_update(include_active_player)
    }

    pub fn replace_all_player_flags_like_cpp(&mut self, flags: u32) {
        self.player.replace_all_player_flags(flags);
    }

    pub fn set_player_flag_like_cpp(&mut self, flag: u32) {
        self.player.set_player_flag(flag);
    }

    pub fn remove_player_flag_like_cpp(&mut self, flag: u32) {
        self.player.remove_player_flag(flag);
    }
}
