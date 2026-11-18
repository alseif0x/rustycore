// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory transfers, removal and destruction.

use crate::player::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BANK_SLOT_ITEM_END, BANK_SLOT_ITEM_START,
    CHILD_EQUIPMENT_SLOT_END, CHILD_EQUIPMENT_SLOT_START, DestroyFilteredItemAction,
    DestroyFilteredItemRef, DestroyItemCountItemRef, DestroyItemCountPlan, EQUIPMENT_SLOT_END,
    EQUIPMENT_SLOT_HEAD, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_START, KEYRING_SLOT_END, KEYRING_SLOT_START, Player, PlayerStorageError,
    destroy_filtered_scan_bag_ranges, destroy_filtered_scan_top_level_range,
    destroy_item_count_scan_bag_ranges, destroy_item_count_scan_top_level_range,
    is_bag_storage_slot,
};
use crate::{Bag, Item, NULL_SLOT};
use wow_constants::{ItemFieldFlags2, ItemUpdateState};
use wow_core::ObjectGuid;

impl Player {
    pub fn remove_top_level_item(
        &mut self,
        slot: u8,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let removed = self.inventory.take_top_level_item_native_like_cpp(slot)?;
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

    pub fn destroy_item_count_by_entry_plan(
        &self,
        item_entry: u32,
        count: u32,
        unequip_check: bool,
        inventory_slot_count: u8,
        items: &[DestroyItemCountItemRef<'_>],
    ) -> DestroyItemCountPlan {
        let mut plan = DestroyItemCountPlan {
            removed_count: 0,
            actions: Vec::new(),
        };
        if count == 0 {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            KEYRING_SLOT_START,
            KEYRING_SLOT_END,
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_bag_ranges(
            &mut plan,
            items,
            item_entry,
            count,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
            true,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_ITEM_START,
            BANK_SLOT_ITEM_END,
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_bag_ranges(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_BAG_START,
            BANK_SLOT_BAG_END,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_BAG_START,
            BANK_SLOT_BAG_END,
            true,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            CHILD_EQUIPMENT_SLOT_START,
            CHILD_EQUIPMENT_SLOT_END,
            false,
            unequip_check,
        );

        plan
    }

    pub fn destroy_zone_limited_item_plan(
        &self,
        inventory_slot_count: u8,
        items: &[DestroyFilteredItemRef],
    ) -> Vec<DestroyFilteredItemAction> {
        let mut actions = Vec::new();
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            KEYRING_SLOT_START,
            KEYRING_SLOT_END,
        );
        destroy_filtered_scan_bag_ranges(
            &mut actions,
            items,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
        );
        actions
    }

    pub fn destroy_conjured_items_plan(
        &self,
        inventory_slot_count: u8,
        items: &[DestroyFilteredItemRef],
    ) -> Vec<DestroyFilteredItemAction> {
        let mut actions = Vec::new();
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
        );
        destroy_filtered_scan_bag_ranges(
            &mut actions,
            items,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
        );
        actions
    }

    pub fn remove_bag_item(
        &mut self,
        bag: u8,
        slot: u8,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        self.inventory.remove_bag_item(bag, slot)
    }
}
