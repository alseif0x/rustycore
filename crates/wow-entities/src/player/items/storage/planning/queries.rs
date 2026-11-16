use super::ItemStorageView;
use crate::player::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BANK_SLOT_ITEM_END, BANK_SLOT_ITEM_START,
    CHILD_EQUIPMENT_SLOT_END, CHILD_EQUIPMENT_SLOT_START, EQUIPMENT_SLOT_END, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_END,
    INVENTORY_SLOT_ITEM_START, ItemSearchCallbackResult, ItemSearchLocation, KEYRING_SLOT_END,
    KEYRING_SLOT_START, NULL_BAG, NULL_SLOT, PROFESSION_SLOT_END, PROFESSION_SLOT_START,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START,
};
use wow_core::ObjectGuid;

impl ItemStorageView<'_> {
    pub(in crate::player) fn is_valid_pos(&self, bag: u8, slot: u8, explicit_pos: bool) -> bool {
        if bag == NULL_BAG && !explicit_pos {
            return true;
        }

        if bag == INVENTORY_SLOT_BAG_0 {
            if slot == NULL_SLOT && !explicit_pos {
                return true;
            }
            if slot < EQUIPMENT_SLOT_END {
                return true;
            }
            if (PROFESSION_SLOT_START..PROFESSION_SLOT_END).contains(&slot) {
                return true;
            }
            if (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot) {
                return true;
            }
            if (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot) {
                return true;
            }
            let backpack_end = INVENTORY_SLOT_ITEM_START
                .saturating_add(self.backpack_slots)
                .min(INVENTORY_SLOT_ITEM_END);
            if (INVENTORY_SLOT_ITEM_START..backpack_end).contains(&slot) {
                return true;
            }
            if (BANK_SLOT_ITEM_START..BANK_SLOT_ITEM_END).contains(&slot) {
                return true;
            }
            if (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot) {
                return true;
            }
            if (KEYRING_SLOT_START..KEYRING_SLOT_END).contains(&slot) {
                return true;
            }
            return false;
        }

        let Some(bag_storage) = self
            .inventory
            .bags
            .get(bag as usize)
            .and_then(Option::as_ref)
        else {
            return false;
        };

        if slot == NULL_SLOT && !explicit_pos {
            return true;
        }

        slot < bag_storage.bag_size
    }

    pub(in crate::player) fn is_valid_packed_pos(&self, pos: u16, explicit_pos: bool) -> bool {
        let [bag, slot] = pos.to_be_bytes();
        self.is_valid_pos(bag, slot, explicit_pos)
    }

    pub(in crate::player::items::storage) fn for_each_item_guid(
        &self,
        location: ItemSearchLocation,
        mut callback: impl FnMut(ObjectGuid) -> ItemSearchCallbackResult,
    ) -> bool {
        if location.contains(ItemSearchLocation::EQUIPMENT) {
            for slot in 0..EQUIPMENT_SLOT_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for slot in PROFESSION_SLOT_START..PROFESSION_SLOT_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
        }

        if location.contains(ItemSearchLocation::INVENTORY) {
            let inventory_end = INVENTORY_SLOT_ITEM_START
                .saturating_add(self.backpack_slots)
                .min(INVENTORY_SLOT_ITEM_END);
            for slot in INVENTORY_SLOT_BAG_START..inventory_end {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for slot in KEYRING_SLOT_START..KEYRING_SLOT_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for slot in CHILD_EQUIPMENT_SLOT_START..CHILD_EQUIPMENT_SLOT_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for bag_slot in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                if self.visit_bag_items(bag_slot, &mut callback) {
                    return false;
                }
            }
        }

        if location.contains(ItemSearchLocation::BANK) {
            for slot in BANK_SLOT_ITEM_START..BANK_SLOT_BAG_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
                if self.visit_bag_items(bag_slot, &mut callback) {
                    return false;
                }
            }
        }

        if location.contains(ItemSearchLocation::REAGENT_BANK) {
            for bag_slot in REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END {
                if self.visit_bag_items(bag_slot, &mut callback) {
                    return false;
                }
            }
        }

        true
    }

    fn visit_top_slot(
        &self,
        slot: u8,
        callback: &mut impl FnMut(ObjectGuid) -> ItemSearchCallbackResult,
    ) -> bool {
        self.inventory.items[slot as usize]
            .map(|guid| matches!(callback(guid), ItemSearchCallbackResult::Stop))
            .unwrap_or(false)
    }

    fn visit_bag_items(
        &self,
        bag_slot: u8,
        callback: &mut impl FnMut(ObjectGuid) -> ItemSearchCallbackResult,
    ) -> bool {
        let Some(bag) = self.inventory.bags[bag_slot as usize].as_ref() else {
            return false;
        };

        bag.slots
            .iter()
            .take(bag.bag_size as usize)
            .filter_map(|guid| *guid)
            .any(|guid| matches!(callback(guid), ItemSearchCallbackResult::Stop))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{PlayerBagStorage, PlayerInventoryStorage};

    fn guid(id: i64) -> ObjectGuid {
        ObjectGuid::create_item(0, id)
    }

    #[test]
    fn represented_item_guid_traversal_preserves_order_and_bag_bounds() {
        let mut inventory = PlayerInventoryStorage::default();
        let expected = (1..=11).map(guid).collect::<Vec<_>>();
        inventory.items[1] = Some(expected[0]);
        inventory.items[PROFESSION_SLOT_START as usize] = Some(expected[1]);
        inventory.items[INVENTORY_SLOT_BAG_START as usize] = Some(expected[2]);
        inventory.items[KEYRING_SLOT_START as usize] = Some(expected[3]);
        inventory.items[CHILD_EQUIPMENT_SLOT_START as usize] = Some(expected[4]);

        let mut inventory_bag = PlayerBagStorage::new(guid(20), 2);
        inventory_bag.set_item(0, Some(expected[5]));
        inventory_bag.set_item(1, Some(expected[6]));
        inventory_bag.set_item(2, Some(guid(99)));
        assert_eq!(inventory_bag.bag_size, 2);
        assert_eq!(inventory_bag.slots[2], Some(guid(99)));
        inventory.bags[INVENTORY_SLOT_BAG_START as usize] = Some(Box::new(inventory_bag));

        inventory.items[BANK_SLOT_ITEM_START as usize] = Some(expected[7]);
        inventory.items[BANK_SLOT_BAG_START as usize] = Some(expected[8]);
        let mut bank_bag = PlayerBagStorage::new(guid(21), 1);
        bank_bag.set_item(0, Some(expected[9]));
        inventory.bags[BANK_SLOT_BAG_START as usize] = Some(Box::new(bank_bag));

        let mut reagent_bag = PlayerBagStorage::new(guid(22), 1);
        reagent_bag.set_item(0, Some(expected[10]));
        inventory.bags[REAGENT_BAG_SLOT_START as usize] = Some(Box::new(reagent_bag));

        let view = ItemStorageView::new(guid(23), &inventory, 1);
        let mut visited = Vec::new();
        assert!(
            view.for_each_item_guid(ItemSearchLocation::EVERYWHERE, |item_guid| {
                visited.push(item_guid);
                ItemSearchCallbackResult::Continue
            })
        );
        assert_eq!(visited, expected);
    }

    #[test]
    fn represented_item_guid_traversal_stops_at_the_first_stop_result() {
        let mut inventory = PlayerInventoryStorage::default();
        let first = guid(1);
        let second = guid(2);
        let third = guid(3);
        inventory.items[0] = Some(first);
        inventory.items[PROFESSION_SLOT_START as usize] = Some(second);
        inventory.items[INVENTORY_SLOT_BAG_START as usize] = Some(third);

        let view = ItemStorageView::new(guid(24), &inventory, 1);
        let mut visited = Vec::new();
        assert!(
            !view.for_each_item_guid(ItemSearchLocation::EVERYWHERE, |item_guid| {
                visited.push(item_guid);
                if item_guid == second {
                    ItemSearchCallbackResult::Stop
                } else {
                    ItemSearchCallbackResult::Continue
                }
            })
        );
        assert_eq!(visited, [first, second]);
    }

    #[test]
    fn represented_position_queries_preserve_ranges_and_packed_bounds() {
        let mut inventory = PlayerInventoryStorage::default();
        inventory.bags[5] = Some(PlayerBagStorage::boxed(guid(30), 2));
        let view = ItemStorageView::new(guid(31), &inventory, 2);

        assert!(view.is_valid_pos(NULL_BAG, 200, false));
        assert!(!view.is_valid_pos(NULL_BAG, 200, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, NULL_SLOT, false));
        assert!(!view.is_valid_pos(INVENTORY_SLOT_BAG_0, NULL_SLOT, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_END - 1, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, PROFESSION_SLOT_START, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END - 1, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, REAGENT_BAG_SLOT_START, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, true));
        assert!(!view.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 2, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_BAG_START, true));
        assert!(view.is_valid_pos(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_END - 1, true));
        assert!(!view.is_valid_pos(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_END, true));

        assert!(view.is_valid_pos(5, 1, true));
        assert!(!view.is_valid_pos(5, 2, true));
        assert!(view.is_valid_pos(5, NULL_SLOT, false));
        assert!(!view.is_valid_pos(5, NULL_SLOT, true));
        assert!(!view.is_valid_pos(6, 0, true));

        assert!(view.is_valid_packed_pos(
            u16::from_be_bytes([INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1]),
            true,
        ));
        assert!(!view.is_valid_packed_pos(
            u16::from_be_bytes([INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 2]),
            true,
        ));
        assert!(view.is_valid_packed_pos(u16::from_be_bytes([5, 1]), true));
        assert!(!view.is_valid_packed_pos(u16::from_be_bytes([5, 2]), true));
    }
}
