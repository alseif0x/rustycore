//! Actor-free inventory values and update-mask snapshot.

use crate::UpdateMask;
use crate::player::effective_stats::derived_stats_values_update_like_cpp;
use crate::player::inventory_values::{
    InventoryValuesMut, clear_active_player_data_changes_like_cpp,
    clear_player_data_changes_like_cpp,
};
use crate::player::quest_log::MAX_QUEST_LOG_SIZE_LIKE_CPP;
use crate::player::quest_log::quest_log_rows_for_update_like_cpp;
use crate::player::{
    ActivePlayerDataUpdate, ActivePlayerDataValues, Player, PlayerDataUpdate, PlayerDataValues,
    PlayerEffectiveCombatStatsLikeCpp, PlayerInventoryStorage, PlayerValuesUpdate,
    VisibleItemValues,
};
use crate::unit::UnitValuesSnapshot;
use crate::update_fields::{TYPEID_ACTIVE_PLAYER, TYPEID_PLAYER};
use wow_constants::PowerType;
use wow_core::ObjectGuid;

/// Owned Player inventory/update-field facts for later Core-side projection.
///
/// Capture copies current values and masks only. It does not evaluate update
/// payloads or retain a live entity, Object, Session, or runtime handle.
pub struct InventoryPlayerSnapshotLikeCpp {
    owner_guid: ObjectGuid,
    inventory: Box<PlayerInventoryStorage>,
    player_data: PlayerDataValues,
    active_player_data: ActivePlayerDataValues,
    player_data_changes: UpdateMask,
    quest_log_change_masks: [u32; MAX_QUEST_LOG_SIZE_LIKE_CPP],
    active_player_data_changes: UpdateMask,
    rest_info_change_masks: [u8; 2],
    active_local_flags: u32,
    effective_combat_stats: PlayerEffectiveCombatStatsLikeCpp,
    mana_power_index: Option<usize>,
    can_titan_grip: bool,
    unit: UnitValuesSnapshot,
}

mod queries;

impl InventoryPlayerSnapshotLikeCpp {
    pub fn capture(player: &Player) -> Self {
        Self {
            owner_guid: player.guid(),
            inventory: player.inventory.clone(),
            player_data: player.data,
            active_player_data: player.active_data.clone(),
            player_data_changes: player.player_data_changes.clone(),
            quest_log_change_masks: player.quest_log_change_masks,
            active_player_data_changes: player.active_player_data_changes.clone(),
            rest_info_change_masks: player.rest_info_change_masks,
            active_local_flags: player.gameplay_state.active_local_flags,
            effective_combat_stats: player.effective_combat_stats,
            mana_power_index: player
                .unit
                .get_power_index(PowerType::Mana)
                .filter(|index| *index < crate::unit::MAX_POWERS_PER_CLASS),
            can_titan_grip: player.can_titan_grip,
            unit: UnitValuesSnapshot::capture(&player.unit),
        }
    }

    fn inventory_values_mut(&mut self) -> InventoryValuesMut<'_> {
        InventoryValuesMut::new(
            &mut self.player_data,
            &mut self.active_player_data,
            &mut self.player_data_changes,
            &mut self.active_player_data_changes,
        )
    }

    pub fn replace_all_player_flags(&mut self, flags: u32) {
        let _ = self.inventory_values_mut().replace_all_player_flags(flags);
    }

    pub fn set_player_flag(&mut self, flag: u32) {
        self.replace_all_player_flags(self.player_data.player_flags | flag);
    }

    pub fn remove_player_flag(&mut self, flag: u32) {
        self.replace_all_player_flags(self.player_data.player_flags & !flag);
    }

    pub fn set_money(&mut self, value: u64) {
        let _ = self.inventory_values_mut().set_money(value);
    }

    pub fn mark_money_changed(&mut self) {
        let _ = self.inventory_values_mut().mark_money_changed();
    }

    pub fn set_inventory_slot_count(&mut self, count: u8) {
        let _ = self.inventory_values_mut().set_inventory_slot_count(count);
    }

    pub fn set_bank_bag_slot_count(&mut self, count: u8) {
        let _ = self.inventory_values_mut().set_bank_bag_slot_count(count);
    }

    pub fn mark_bank_bag_slot_count_changed_like_cpp(&mut self) {
        let _ = self
            .inventory_values_mut()
            .mark_bank_bag_slot_count_changed_like_cpp();
    }

    /// Returns whether the bank-bag slot index is valid, matching Player's API.
    pub fn set_bank_bag_slot_flag_value_like_cpp(&mut self, index: usize, value: u32) -> bool {
        self.inventory_values_mut()
            .set_bank_bag_slot_flag_value_like_cpp(index, value)
            .is_some()
    }

    pub fn mark_bank_bag_slot_flag_changed_like_cpp(&mut self, index: usize) {
        let _ = self
            .inventory_values_mut()
            .mark_bank_bag_slot_flag_changed_like_cpp(index);
    }

    pub fn set_visible_item_slot(&mut self, slot: u8, item: Option<VisibleItemValues>) {
        let _ = self
            .inventory_values_mut()
            .set_visible_item_slot(slot, item);
    }

    pub fn mark_visible_item_slot_changed(&mut self, slot: u8) {
        let _ = self
            .inventory_values_mut()
            .mark_visible_item_slot_changed(slot);
    }

    pub fn set_inv_slot(&mut self, slot: usize, guid: ObjectGuid) {
        let _ = self.inventory_values_mut().set_inv_slot(slot, guid);
    }

    pub fn mark_inv_slot_changed(&mut self, slot: usize) {
        let _ = self.inventory_values_mut().mark_inv_slot_changed(slot);
    }

    pub fn set_buyback_price(&mut self, slot: usize, price: u32) {
        let _ = self.inventory_values_mut().set_buyback_price(slot, price);
    }

    pub fn mark_buyback_price_changed(&mut self, slot: usize) {
        let _ = self.inventory_values_mut().mark_buyback_price_changed(slot);
    }

    pub fn set_buyback_timestamp(&mut self, slot: usize, timestamp: i64) {
        let _ = self
            .inventory_values_mut()
            .set_buyback_timestamp(slot, timestamp);
    }

    pub fn mark_buyback_timestamp_changed(&mut self, slot: usize) {
        let _ = self
            .inventory_values_mut()
            .mark_buyback_timestamp_changed(slot);
    }

    pub fn set_virtual_item(&mut self, index: usize, visible: Option<VisibleItemValues>) {
        self.unit.set_virtual_item(index, visible);
    }

    pub fn mark_virtual_item_changed(&mut self, index: usize) {
        self.unit.mark_virtual_item_changed(index);
    }

    pub fn clear_data_changes(&mut self) {
        clear_player_data_changes_like_cpp(
            &mut self.player_data_changes,
            &mut self.quest_log_change_masks,
        );
        clear_active_player_data_changes_like_cpp(
            &mut self.active_player_data_changes,
            &mut self.active_player_data,
            &mut self.rest_info_change_masks,
        );
        self.unit.clear_unit_data_changes();
        self.unit.clear_object_data_changes();
    }

    pub fn values_update(&self, include_active_player: bool) -> PlayerValuesUpdate {
        let unit_update = self.unit.values_update();
        PlayerValuesUpdate {
            changed_object_type_mask: unit_update.changed_object_type_mask
                | if self.player_data_changes.is_any_set() {
                    1 << TYPEID_PLAYER
                } else {
                    0
                }
                | if include_active_player && self.active_player_data_changes.is_any_set() {
                    1 << TYPEID_ACTIVE_PLAYER
                } else {
                    0
                },
            object_data: unit_update.object_data,
            unit_data: unit_update.unit_data,
            player_data: self
                .player_data_changes
                .is_any_set()
                .then(|| PlayerDataUpdate {
                    mask: self.player_data_changes.clone(),
                    values: self.player_data,
                    quest_log: quest_log_rows_for_update_like_cpp(
                        &self.player_data,
                        &self.quest_log_change_masks,
                    ),
                }),
            active_player_data: (include_active_player
                && self.active_player_data_changes.is_any_set())
            .then(|| ActivePlayerDataUpdate {
                mask: self.active_player_data_changes.clone(),
                values: self.active_player_data.clone(),
                local_flags: self.active_local_flags,
                rest_info_change_masks: self.rest_info_change_masks,
            }),
            derived_stats: derived_stats_values_update_like_cpp(
                include_active_player,
                self.mana_power_index,
                self.unit.unit_data_changes_mask(),
                &self.active_player_data_changes,
                self.effective_combat_stats,
            ),
        }
    }
}

#[cfg(test)]
#[path = "projection/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "projection/query_tests.rs"]
mod query_tests;
