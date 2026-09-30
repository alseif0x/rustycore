// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Top-level storage mutations and direct item destruction.

use super::super::super::*;

impl Player {
    pub fn top_level_item_guid(&self, slot: u8) -> Option<ObjectGuid> {
        self.inventory.items.get(slot as usize).copied().flatten()
    }

    pub fn register_bag_storage(
        &mut self,
        bag_slot: u8,
        bag_guid: ObjectGuid,
        bag_size: u8,
    ) -> Result<(), PlayerStorageError> {
        if !is_bag_storage_slot(bag_slot) {
            return Err(PlayerStorageError::InvalidBagSlot(bag_slot));
        }
        if bag_size as usize > MAX_BAG_SIZE {
            return Err(PlayerStorageError::InvalidBagItemSlot(bag_size));
        }

        self.inventory.bags[bag_slot as usize] = Some(PlayerBagStorage::boxed(bag_guid, bag_size));
        Ok(())
    }

    pub fn store_top_level_item(
        &mut self,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        self.inventory.items[slot as usize] = Some(guid);
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

    pub fn remove_top_level_item(
        &mut self,
        slot: u8,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        let removed = self.inventory.items[slot as usize].take();
        self.set_inv_slot(slot as usize, ObjectGuid::EMPTY);
        if slot < EQUIPMENT_SLOT_END {
            self.set_visible_item_slot(slot, None);
        }
        if is_bag_storage_slot(slot) {
            self.inventory.bags[slot as usize] = None;
        }
        Ok(removed)
    }

    pub fn remove_item_object(
        &mut self,
        bag: u8,
        slot: u8,
        item: Option<&mut Item>,
        bag_object: Option<&mut Bag>,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let Some(item) = item else {
            return Ok(None);
        };

        let item_guid = item.object().guid();
        let removed = if bag == INVENTORY_SLOT_BAG_0 {
            let Some(expected_guid) = self.top_level_item_guid(slot) else {
                return Err(PlayerStorageError::EmptyPlayerSlot(slot));
            };
            if expected_guid != item_guid {
                return Err(PlayerStorageError::MismatchedItemGuid {
                    slot,
                    expected: expected_guid,
                    actual: item_guid,
                });
            }

            if slot < INVENTORY_SLOT_BAG_END {
                item.remove_item_flag2(ItemFieldFlags2::EQUIPPED);
            }

            self.remove_top_level_item(slot)?
        } else {
            let Some(bag_object) = bag_object else {
                return Err(PlayerStorageError::UnknownBag(bag));
            };
            let expected_bag_guid = self
                .get_bag_by_pos(bag)
                .ok_or(PlayerStorageError::UnknownBag(bag))?;
            let actual_bag_guid = bag_object.item().object().guid();
            if expected_bag_guid != actual_bag_guid {
                return Err(PlayerStorageError::MismatchedBagGuid {
                    bag,
                    expected: expected_bag_guid,
                    actual: actual_bag_guid,
                });
            }

            let expected_guid = self
                .inventory
                .bags
                .get(bag as usize)
                .and_then(Option::as_ref)
                .and_then(|bag_storage| bag_storage.item_by_pos(slot))
                .ok_or(PlayerStorageError::EmptyBagItemSlot { bag, slot })?;
            if expected_guid != item_guid {
                return Err(PlayerStorageError::MismatchedBagItemGuid {
                    bag,
                    slot,
                    expected: expected_guid,
                    actual: item_guid,
                });
            }

            bag_object.remove_item(slot);
            self.remove_bag_item(bag, slot)?
        };

        item.set_contained_in(ObjectGuid::EMPTY);
        item.set_slot(NULL_SLOT);
        item.set_container_guid(ObjectGuid::EMPTY);
        Ok(removed)
    }

    pub fn move_item_from_inventory_object(
        &mut self,
        bag: u8,
        slot: u8,
        item: Option<&mut Item>,
        bag_object: Option<&mut Bag>,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let Some(item) = item else {
            return Ok(None);
        };

        let removed = self.remove_item_object(bag, slot, Some(&mut *item), bag_object)?;
        if removed.is_some() {
            item.set_not_refundable();
        }
        Ok(removed)
    }

    pub fn finalize_move_item_to_inventory_object(
        &self,
        original_item_guid: ObjectGuid,
        last_item: &mut Item,
        in_character_inventory_db: bool,
    ) -> bool {
        if original_item_guid != last_item.object().guid() {
            return false;
        }

        if last_item.owner_guid() != self.guid() {
            last_item.set_owner_guid(self.guid());
        }

        last_item.set_state(if in_character_inventory_db {
            ItemUpdateState::Changed
        } else {
            ItemUpdateState::New
        });
        true
    }

    pub fn destroy_item_object(
        &mut self,
        bag: u8,
        slot: u8,
        item: Option<&mut Item>,
        bag_object: Option<&mut Bag>,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let Some(item) = item else {
            return Ok(None);
        };

        let item_guid = item.object().guid();
        let removed = if bag == INVENTORY_SLOT_BAG_0 {
            let Some(expected_guid) = self.top_level_item_guid(slot) else {
                return Err(PlayerStorageError::EmptyPlayerSlot(slot));
            };
            if expected_guid != item_guid {
                return Err(PlayerStorageError::MismatchedItemGuid {
                    slot,
                    expected: expected_guid,
                    actual: item_guid,
                });
            }

            self.remove_top_level_item(slot)?
        } else {
            let Some(bag_object) = bag_object else {
                return Err(PlayerStorageError::UnknownBag(bag));
            };
            let expected_bag_guid = self
                .get_bag_by_pos(bag)
                .ok_or(PlayerStorageError::UnknownBag(bag))?;
            let actual_bag_guid = bag_object.item().object().guid();
            if expected_bag_guid != actual_bag_guid {
                return Err(PlayerStorageError::MismatchedBagGuid {
                    bag,
                    expected: expected_bag_guid,
                    actual: actual_bag_guid,
                });
            }

            let expected_guid = self
                .inventory
                .bags
                .get(bag as usize)
                .and_then(Option::as_ref)
                .and_then(|bag_storage| bag_storage.item_by_pos(slot))
                .ok_or(PlayerStorageError::EmptyBagItemSlot { bag, slot })?;
            if expected_guid != item_guid {
                return Err(PlayerStorageError::MismatchedBagItemGuid {
                    bag,
                    slot,
                    expected: expected_guid,
                    actual: item_guid,
                });
            }

            bag_object.remove_item(slot);
            self.remove_bag_item(bag, slot)?
        };

        item.set_not_refundable();
        item.clear_soulbound_tradeable();
        item.set_contained_in(ObjectGuid::EMPTY);
        item.set_slot(NULL_SLOT);
        item.set_container_guid(ObjectGuid::EMPTY);
        item.set_state(ItemUpdateState::Removed);
        Ok(removed)
    }

    pub fn destroy_item_count_for_item_object(
        &mut self,
        item: Option<&mut Item>,
        count: &mut u32,
        bag_object: Option<&mut Bag>,
    ) -> Result<(), PlayerStorageError> {
        let Some(item) = item else {
            return Ok(());
        };

        if item.count() <= *count {
            *count -= item.count();
            let bag = item.bag_slot();
            let slot = item.slot();
            self.destroy_item_object(bag, slot, Some(item), bag_object)?;
        } else {
            item.set_count(item.count() - *count);
            *count = 0;
            item.set_state(ItemUpdateState::Changed);
        }

        Ok(())
    }
}
