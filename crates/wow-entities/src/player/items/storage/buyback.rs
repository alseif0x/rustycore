// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Player buyback slot operations.

use super::super::super::*;

impl Player {
    pub fn get_item_from_buyback_slot(&self, slot: u8) -> Option<ObjectGuid> {
        if is_buyback_slot(slot) {
            self.inventory.items[slot as usize]
        } else {
            None
        }
    }

    pub fn remove_item_from_buyback_slot(&mut self, slot: u8) -> Option<ObjectGuid> {
        if !is_buyback_slot(slot) {
            return None;
        }

        let removed = self.inventory.items[slot as usize].take();
        let buyback_index = (slot - BUYBACK_SLOT_START) as usize;
        self.set_inv_slot(slot as usize, ObjectGuid::EMPTY);
        self.set_buyback_price(buyback_index, 0);
        self.set_buyback_timestamp(buyback_index, 0);
        if self.inventory.items[self.inventory.current_buyback_slot as usize].is_some() {
            self.inventory.current_buyback_slot = slot;
        }
        removed
    }

    pub fn remove_item_from_buyback_slot_object(
        &mut self,
        slot: u8,
        item: Option<&mut Item>,
        delete_item: bool,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        if !is_buyback_slot(slot) {
            return Ok(None);
        }

        let stored_guid = self.inventory.items[slot as usize];
        let mut item = item;
        if let (Some(expected), Some(actual_item)) = (stored_guid, item.as_deref()) {
            let actual = actual_item.object().guid();
            if expected != actual {
                return Err(PlayerStorageError::MismatchedItemGuid {
                    slot,
                    expected,
                    actual,
                });
            }
        }

        if stored_guid.is_some() {
            if let Some(item) = item.as_deref_mut() {
                item.object_mut().remove_from_world();
                if delete_item {
                    item.set_state(ItemUpdateState::Removed);
                }
            }
        }

        Ok(self.remove_item_from_buyback_slot(slot))
    }

    pub fn add_item_to_buyback_slot(&mut self, guid: ObjectGuid, price: u32, timestamp: i64) -> u8 {
        let mut slot = self.inventory.current_buyback_slot;
        if self.inventory.items[slot as usize].is_some() {
            let mut oldest_slot = BUYBACK_SLOT_START;
            let mut oldest_time = self.active_data.buyback_timestamp[0];

            for candidate in BUYBACK_SLOT_START + 1..BUYBACK_SLOT_END {
                let candidate_index = (candidate - BUYBACK_SLOT_START) as usize;
                if self.inventory.items[candidate as usize].is_none() {
                    oldest_slot = candidate;
                    break;
                }
                let candidate_time = self.active_data.buyback_timestamp[candidate_index];
                if oldest_time > candidate_time {
                    oldest_time = candidate_time;
                    oldest_slot = candidate;
                }
            }
            slot = oldest_slot;
        }

        self.remove_item_from_buyback_slot(slot);
        self.inventory.items[slot as usize] = Some(guid);
        let buyback_index = (slot - BUYBACK_SLOT_START) as usize;
        self.set_inv_slot(slot as usize, guid);
        self.set_buyback_price(buyback_index, price);
        self.set_buyback_timestamp(buyback_index, timestamp);

        if self.inventory.current_buyback_slot < BUYBACK_SLOT_END - 1 {
            self.inventory.current_buyback_slot += 1;
        }

        slot
    }

    pub fn add_item_to_buyback_slot_object(
        &mut self,
        item: &Item,
        item_template: Option<&ItemStorageTemplate>,
        game_time: i64,
        login_time: i64,
        overwritten_item: Option<&mut Item>,
    ) -> Result<u8, PlayerStorageError> {
        let mut slot = self.inventory.current_buyback_slot;
        if self.inventory.items[slot as usize].is_some() {
            let mut oldest_slot = BUYBACK_SLOT_START;
            let mut oldest_time = self.active_data.buyback_timestamp[0];

            for candidate in BUYBACK_SLOT_START + 1..BUYBACK_SLOT_END {
                let candidate_index = (candidate - BUYBACK_SLOT_START) as usize;
                if self.inventory.items[candidate as usize].is_none() {
                    oldest_slot = candidate;
                    break;
                }
                let candidate_time = self.active_data.buyback_timestamp[candidate_index];
                if oldest_time > candidate_time {
                    oldest_time = candidate_time;
                    oldest_slot = candidate;
                }
            }
            slot = oldest_slot;
        }

        self.remove_item_from_buyback_slot_object(slot, overwritten_item, true)?;

        let buyback_index = (slot - BUYBACK_SLOT_START) as usize;
        let price = item_template
            .map(|proto| proto.sell_price.wrapping_mul(item.count()))
            .unwrap_or(0);
        let timestamp = (game_time - login_time + (30 * 3600)) as u32 as i64;

        self.inventory.items[slot as usize] = Some(item.object().guid());
        self.set_inv_slot(slot as usize, item.object().guid());
        self.set_buyback_price(buyback_index, price);
        self.set_buyback_timestamp(buyback_index, timestamp);

        if self.inventory.current_buyback_slot < BUYBACK_SLOT_END - 1 {
            self.inventory.current_buyback_slot += 1;
        }

        Ok(slot)
    }
}
