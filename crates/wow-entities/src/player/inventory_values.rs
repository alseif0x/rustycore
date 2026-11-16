// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed Player value and update-mask operations used by inventory projections.
//! Operation booleans request update admission; they do not report value change or
//! delivery. The bank-flag setter's `Option` separately reports index validity.

use super::quest_log::MAX_QUEST_LOG_SIZE_LIKE_CPP;
use super::{
    ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_FIRST_BIT,
    ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_PARENT_BIT, ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
    ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT, ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT,
    ACTIVE_PLAYER_DATA_COINAGE_BIT, ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT,
    ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT, ACTIVE_PLAYER_DATA_NUM_BACKPACK_SLOTS_BIT,
    ACTIVE_PLAYER_DATA_PARENT_BIT, ActivePlayerDataValues, BUYBACK_SLOT_COUNT, EQUIPMENT_SLOT_END,
    PLAYER_DATA_FLAGS_BIT, PLAYER_DATA_NUM_BANK_SLOTS_BIT, PLAYER_DATA_PARENT_BIT,
    PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT, PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT, PLAYER_SLOT_END,
    Player, PlayerDataValues, VisibleItemValues,
};
use crate::UpdateMask;
use wow_core::ObjectGuid;

pub(super) fn clear_player_data_changes_like_cpp(
    player_data_changes: &mut UpdateMask,
    quest_log_change_masks: &mut [u32; MAX_QUEST_LOG_SIZE_LIKE_CPP],
) {
    player_data_changes.reset_all();
    *quest_log_change_masks = [0; MAX_QUEST_LOG_SIZE_LIKE_CPP];
}

pub(super) fn clear_active_player_data_changes_like_cpp(
    active_player_data_changes: &mut UpdateMask,
    active_player_data: &mut ActivePlayerDataValues,
    rest_info_change_masks: &mut [u8; 2],
) {
    active_player_data_changes.reset_all();
    *rest_info_change_masks = [0; 2];
    let self_res_spells_mask_len = active_player_data.self_res_spells.len().div_ceil(32);
    active_player_data.self_res_spells_update_mask = Some(vec![0; self_res_spells_mask_len]);
}

pub(super) struct InventoryValuesMut<'a> {
    player_data: &'a mut PlayerDataValues,
    active_player_data: &'a mut ActivePlayerDataValues,
    player_data_changes: &'a mut UpdateMask,
    active_player_data_changes: &'a mut UpdateMask,
}

impl<'a> InventoryValuesMut<'a> {
    pub(super) fn new(
        player_data: &'a mut PlayerDataValues,
        active_player_data: &'a mut ActivePlayerDataValues,
        player_data_changes: &'a mut UpdateMask,
        active_player_data_changes: &'a mut UpdateMask,
    ) -> Self {
        Self {
            player_data,
            active_player_data,
            player_data_changes,
            active_player_data_changes,
        }
    }

    pub(super) fn replace_all_player_flags(&mut self, flags: u32) -> bool {
        if self.player_data.player_flags == flags {
            return false;
        }

        self.player_data.player_flags = flags;
        self.player_data_changes.set(PLAYER_DATA_PARENT_BIT);
        self.player_data_changes.set(PLAYER_DATA_FLAGS_BIT);
        true
    }

    pub(super) fn set_money(&mut self, value: u64) -> bool {
        if self.active_player_data.coinage == value {
            return false;
        }

        self.active_player_data.coinage = value;
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_COINAGE_BIT);
        true
    }

    pub(super) fn mark_money_changed(&mut self) -> bool {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_COINAGE_BIT);
        true
    }

    pub(super) fn set_inventory_slot_count(&mut self, count: u8) -> bool {
        if self.active_player_data.num_backpack_slots == count {
            return false;
        }

        self.active_player_data.num_backpack_slots = count;
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_NUM_BACKPACK_SLOTS_BIT);
        true
    }

    pub(super) fn set_bank_bag_slot_count(&mut self, count: u8) -> bool {
        if self.player_data.num_bank_slots == count {
            return false;
        }

        self.player_data.num_bank_slots = count;
        self.player_data_changes.set(PLAYER_DATA_PARENT_BIT);
        self.player_data_changes.set(PLAYER_DATA_NUM_BANK_SLOTS_BIT);
        true
    }

    pub(super) fn mark_bank_bag_slot_count_changed_like_cpp(&mut self) -> bool {
        self.player_data_changes.set(PLAYER_DATA_PARENT_BIT);
        self.player_data_changes.set(PLAYER_DATA_NUM_BANK_SLOTS_BIT);
        true
    }

    pub(super) fn set_bank_bag_slot_flag_value_like_cpp(
        &mut self,
        index: usize,
        value: u32,
    ) -> Option<bool> {
        if index >= self.active_player_data.bank_bag_slot_flags.len() {
            return None;
        }

        if self.active_player_data.bank_bag_slot_flags[index] == value {
            return Some(false);
        }

        self.active_player_data.bank_bag_slot_flags[index] = value;
        Some(self.mark_bank_bag_slot_flag_changed_like_cpp(index))
    }

    pub(super) fn mark_bank_bag_slot_flag_changed_like_cpp(&mut self, index: usize) -> bool {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_PARENT_BIT);
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_FIRST_BIT + index);
        true
    }

    pub(super) fn set_visible_item_slot(
        &mut self,
        slot: u8,
        item: Option<VisibleItemValues>,
    ) -> bool {
        if slot >= EQUIPMENT_SLOT_END {
            return false;
        }

        let value = item.unwrap_or_default();
        let index = slot as usize;
        let target = &mut self.player_data.visible_items[index];
        if *target != value {
            *target = value;
            self.mark_player_data_array(
                PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT,
                PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT,
                index,
            )
        } else {
            false
        }
    }

    pub(super) fn mark_visible_item_slot_changed(&mut self, slot: u8) -> bool {
        if slot >= EQUIPMENT_SLOT_END {
            return false;
        }

        self.mark_player_data_array(
            PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT,
            PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT,
            slot as usize,
        )
    }

    pub(super) fn set_inv_slot(&mut self, slot: usize, guid: ObjectGuid) -> bool {
        if slot >= PLAYER_SLOT_END || self.active_player_data.inv_slots[slot] == guid {
            return false;
        }

        self.active_player_data.inv_slots[slot] = guid;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT,
            ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT,
            slot,
        )
    }

    pub(super) fn mark_inv_slot_changed(&mut self, slot: usize) -> bool {
        if slot >= PLAYER_SLOT_END {
            return false;
        }

        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT,
            ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT,
            slot,
        )
    }

    pub(super) fn set_buyback_price(&mut self, slot: usize, price: u32) -> bool {
        if slot >= BUYBACK_SLOT_COUNT || self.active_player_data.buyback_price[slot] == price {
            return false;
        }

        self.active_player_data.buyback_price[slot] = price;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT,
            slot,
        )
    }

    pub(super) fn mark_buyback_price_changed(&mut self, slot: usize) -> bool {
        if slot >= BUYBACK_SLOT_COUNT {
            return false;
        }

        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT,
            slot,
        )
    }

    pub(super) fn set_buyback_timestamp(&mut self, slot: usize, timestamp: i64) -> bool {
        if slot >= BUYBACK_SLOT_COUNT
            || self.active_player_data.buyback_timestamp[slot] == timestamp
        {
            return false;
        }

        self.active_player_data.buyback_timestamp[slot] = timestamp;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT,
            slot,
        )
    }

    pub(super) fn mark_buyback_timestamp_changed(&mut self, slot: usize) -> bool {
        if slot >= BUYBACK_SLOT_COUNT {
            return false;
        }

        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT,
            slot,
        )
    }

    fn mark_player_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) -> bool {
        self.player_data_changes.set(parent_bit);
        self.player_data_changes.set(first_element_bit + index);
        true
    }

    fn mark_active_player_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) -> bool {
        self.active_player_data_changes.set(parent_bit);
        self.active_player_data_changes
            .set(first_element_bit + index);
        true
    }
}

impl Player {
    pub(super) fn inventory_values_mut(&mut self) -> InventoryValuesMut<'_> {
        InventoryValuesMut::new(
            &mut self.data,
            &mut self.active_data,
            &mut self.player_data_changes,
            &mut self.active_player_data_changes,
        )
    }

    pub(super) fn apply_inventory_value_update_request(&mut self, update_requested: bool) {
        if update_requested {
            self.unit
                .world_mut()
                .object_mut()
                .add_to_object_update_if_needed();
        }
    }
}
