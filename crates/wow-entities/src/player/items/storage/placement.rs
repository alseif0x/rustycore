// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Placement and stack application for top-level and bag slots.

use crate::player::{
    EQUIPMENT_SLOT_END, PLAYER_SLOT_END, Player, PlayerStorageError, VisibleItemValues,
    is_bag_storage_slot, validate_split_source,
};
use crate::{Bag, Item, MAX_BAG_SIZE};
use wow_constants::ItemUpdateState;
use wow_core::ObjectGuid;

impl Player {
    pub fn register_bag_storage(
        &mut self,
        bag_slot: u8,
        bag_guid: ObjectGuid,
        bag_size: u8,
    ) -> Result<(), PlayerStorageError> {
        self.inventory
            .register_bag_storage(bag_slot, bag_guid, bag_size)
    }

    pub fn store_top_level_item(
        &mut self,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        self.inventory
            .store_top_level_item_native_like_cpp(slot, guid)?;
        self.set_inv_slot(slot as usize, guid);
        Ok(())
    }

    pub fn visualize_item(
        &mut self,
        slot: u8,
        guid: ObjectGuid,
        visible: VisibleItemValues,
    ) -> Result<(), PlayerStorageError> {
        self.store_top_level_item(slot, guid)?;
        if slot < EQUIPMENT_SLOT_END {
            self.set_visible_item_slot(slot, Some(visible));
        }
        Ok(())
    }

    pub fn visualize_item_object(
        &mut self,
        slot: u8,
        item: &mut Item,
        visible: VisibleItemValues,
    ) -> Result<(), PlayerStorageError> {
        let item_guid = item.object().guid();
        self.store_top_level_item(slot, item_guid)?;

        let owner_guid = self.guid();
        item.bind_if_visualized();
        item.set_contained_in(owner_guid);
        item.set_owner_guid(owner_guid);
        item.set_slot(slot);
        item.set_container_guid(ObjectGuid::EMPTY);

        if slot < EQUIPMENT_SLOT_END {
            self.set_visible_item_slot(slot, Some(visible));
        }

        item.set_state(ItemUpdateState::Changed);
        Ok(())
    }

    pub fn store_item_object(
        &mut self,
        slot: u8,
        item: &mut Item,
        count: u32,
    ) -> Result<(), PlayerStorageError> {
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        if self.inventory.items[slot as usize].is_some() {
            return Err(PlayerStorageError::OccupiedPlayerSlot(slot));
        }

        let item_guid = item.object().guid();
        self.store_top_level_item(slot, item_guid)?;

        let owner_guid = self.guid();
        item.set_count(count);
        item.bind_if_stored(is_bag_storage_slot(slot));
        item.set_contained_in(owner_guid);
        item.set_owner_guid(owner_guid);
        item.set_slot(slot);
        item.set_container_guid(ObjectGuid::EMPTY);
        item.set_state(ItemUpdateState::Changed);
        Ok(())
    }

    pub fn store_cloned_item_object(
        &mut self,
        slot: u8,
        source: &Item,
        new_guid: ObjectGuid,
        count: u32,
    ) -> Result<Item, PlayerStorageError> {
        let mut cloned = source.clone_item_for_store(new_guid, Some(self.guid()), count);
        self.store_item_object(slot, &mut cloned, count)?;
        Ok(cloned)
    }

    pub fn split_item_to_empty_top_level_object(
        &mut self,
        slot: u8,
        source: &mut Item,
        new_guid: ObjectGuid,
        count: u32,
    ) -> Result<Item, PlayerStorageError> {
        validate_split_source(source, count)?;

        let cloned = self.store_cloned_item_object(slot, source, new_guid, count)?;
        source.set_count(source.count() - count);
        source.set_state(ItemUpdateState::Changed);
        Ok(cloned)
    }

    pub fn merge_top_level_item_stack_object(
        &mut self,
        slot: u8,
        existing: &mut Item,
        incoming: &mut Item,
        count: u32,
    ) -> Result<(), PlayerStorageError> {
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        let Some(expected_guid) = self.top_level_item_guid(slot) else {
            return Err(PlayerStorageError::EmptyPlayerSlot(slot));
        };

        let actual_guid = existing.object().guid();
        if expected_guid != actual_guid {
            return Err(PlayerStorageError::MismatchedItemGuid {
                slot,
                expected: expected_guid,
                actual: actual_guid,
            });
        }

        existing.bind_if_stored(is_bag_storage_slot(slot));
        existing.set_count(existing.count() + count);
        existing.set_state(ItemUpdateState::Changed);

        let owner_guid = self.guid();
        incoming.set_owner_guid(owner_guid);
        incoming.set_not_refundable();
        incoming.clear_soulbound_tradeable();
        incoming.set_state(ItemUpdateState::Removed);
        Ok(())
    }

    pub fn store_bag_item(
        &mut self,
        bag: u8,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        self.inventory.store_bag_item(bag, slot, guid)
    }

    pub fn store_bag_item_object(
        &mut self,
        bag_slot: u8,
        bag: &mut Bag,
        item_slot: u8,
        item: &mut Item,
        count: u32,
    ) -> Result<(), PlayerStorageError> {
        let bag_guid = bag.item().object().guid();
        let bag_storage = self
            .inventory
            .bags
            .get(bag_slot as usize)
            .and_then(Option::as_ref)
            .ok_or(PlayerStorageError::UnknownBag(bag_slot))?;

        if bag_storage.bag_guid != bag_guid {
            return Err(PlayerStorageError::MismatchedBagGuid {
                bag: bag_slot,
                expected: bag_storage.bag_guid,
                actual: bag_guid,
            });
        }

        if item_slot as usize >= MAX_BAG_SIZE || item_slot >= bag_storage.bag_size {
            return Err(PlayerStorageError::InvalidBagItemSlot(item_slot));
        }

        if bag_storage.item_by_pos(item_slot).is_some() {
            return Err(PlayerStorageError::OccupiedBagItemSlot {
                bag: bag_slot,
                slot: item_slot,
            });
        }

        item.set_count(count);
        item.bind_if_stored(false);
        bag.store_item(item_slot, item);
        self.store_bag_item(bag_slot, item_slot, item.object().guid())?;
        item.set_state(ItemUpdateState::Changed);
        bag.item_mut().set_state(ItemUpdateState::Changed);
        Ok(())
    }

    pub fn store_cloned_bag_item_object(
        &mut self,
        bag_slot: u8,
        bag: &mut Bag,
        item_slot: u8,
        source: &Item,
        new_guid: ObjectGuid,
        count: u32,
    ) -> Result<Item, PlayerStorageError> {
        let mut cloned = source.clone_item_for_store(new_guid, Some(self.guid()), count);
        self.store_bag_item_object(bag_slot, bag, item_slot, &mut cloned, count)?;
        Ok(cloned)
    }

    pub fn split_item_to_empty_bag_item_object(
        &mut self,
        bag_slot: u8,
        bag: &mut Bag,
        item_slot: u8,
        source: &mut Item,
        new_guid: ObjectGuid,
        count: u32,
    ) -> Result<Item, PlayerStorageError> {
        validate_split_source(source, count)?;

        let cloned =
            self.store_cloned_bag_item_object(bag_slot, bag, item_slot, source, new_guid, count)?;
        source.set_count(source.count() - count);
        source.set_state(ItemUpdateState::Changed);
        Ok(cloned)
    }

    pub fn merge_bag_item_stack_object(
        &mut self,
        bag_slot: u8,
        bag: &Bag,
        item_slot: u8,
        existing: &mut Item,
        incoming: &mut Item,
        count: u32,
    ) -> Result<(), PlayerStorageError> {
        let bag_guid = bag.item().object().guid();
        let bag_storage = self
            .inventory
            .bags
            .get(bag_slot as usize)
            .and_then(Option::as_ref)
            .ok_or(PlayerStorageError::UnknownBag(bag_slot))?;

        if bag_storage.bag_guid != bag_guid {
            return Err(PlayerStorageError::MismatchedBagGuid {
                bag: bag_slot,
                expected: bag_storage.bag_guid,
                actual: bag_guid,
            });
        }

        if item_slot as usize >= MAX_BAG_SIZE || item_slot >= bag_storage.bag_size {
            return Err(PlayerStorageError::InvalidBagItemSlot(item_slot));
        }

        let Some(expected_guid) = bag_storage.item_by_pos(item_slot) else {
            return Err(PlayerStorageError::EmptyBagItemSlot {
                bag: bag_slot,
                slot: item_slot,
            });
        };

        let bag_slot_guid = bag.item_by_pos(item_slot).unwrap_or(ObjectGuid::EMPTY);
        if bag_slot_guid != expected_guid {
            return Err(PlayerStorageError::MismatchedBagItemGuid {
                bag: bag_slot,
                slot: item_slot,
                expected: expected_guid,
                actual: bag_slot_guid,
            });
        }

        let actual_guid = existing.object().guid();
        if expected_guid != actual_guid {
            return Err(PlayerStorageError::MismatchedBagItemGuid {
                bag: bag_slot,
                slot: item_slot,
                expected: expected_guid,
                actual: actual_guid,
            });
        }

        existing.bind_if_stored(false);
        existing.set_count(existing.count() + count);
        existing.set_state(ItemUpdateState::Changed);

        let owner_guid = self.guid();
        incoming.set_owner_guid(owner_guid);
        incoming.set_not_refundable();
        incoming.clear_soulbound_tradeable();
        incoming.set_state(ItemUpdateState::Removed);
        Ok(())
    }
}
