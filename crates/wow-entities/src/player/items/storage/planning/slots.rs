use super::ItemStorageView;
use crate::player::{
    BUYBACK_SLOT_END, BUYBACK_SLOT_START, INVENTORY_SLOT_BAG_0, ItemPosCount, ItemSlotRef,
    PLAYER_SLOT_END, REAGENT_BAG_SLOT_START, cpp_keyring_family_gate_applies, is_bag_pos,
    is_child_equipment_pos, is_equipment_pos, item_can_go_into_bag, item_ref_by_pos, make_item_pos,
};
use crate::{Item, ItemStorageTemplate};
use wow_constants::{
    BagFamilyMask, InventoryResult, ItemClass, ItemFieldFlags, ItemSubClassContainer,
};

impl ItemStorageView<'_> {
    pub(in crate::player::items::storage) fn can_store_item_in_specific_slot(
        &self,
        bag: u8,
        slot: u8,
        dest: &mut Vec<ItemPosCount>,
        proto: &ItemStorageTemplate,
        count: &mut u32,
        swap: bool,
        existing_item: Option<&Item>,
        source_item: Option<&Item>,
        source_is_not_empty_bag: bool,
        bag_proto: Option<&ItemStorageTemplate>,
    ) -> InventoryResult {
        let existing_item = existing_item.filter(|existing| {
            source_item.is_none_or(|source| existing.object().guid() != source.object().guid())
        });

        if let Some(source) = source_item {
            if source_is_not_empty_bag && !is_bag_pos(make_item_pos(bag, slot)) {
                return InventoryResult::DestroyNonemptyBag;
            }

            let source_is_child = source.has_item_flag(ItemFieldFlags::CHILD);
            if source_is_child && !is_equipment_pos(bag, slot) && !is_child_equipment_pos(bag, slot)
            {
                return InventoryResult::WrongBagType3;
            }
            if !source_is_child && is_child_equipment_pos(bag, slot) {
                return InventoryResult::WrongBagType3;
            }
        }

        let need_space = if existing_item.is_none() || swap {
            if slot == REAGENT_BAG_SLOT_START {
                return InventoryResult::WrongBagType;
            }

            if bag == INVENTORY_SLOT_BAG_0 {
                if cpp_keyring_family_gate_applies(slot)
                    && !proto.bag_family.contains(BagFamilyMask::KEYS)
                {
                    return InventoryResult::WrongBagType;
                }

                if (BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot)
                    || slot as usize >= PLAYER_SLOT_END
                {
                    return InventoryResult::WrongBagType;
                }
            } else {
                if self.get_bag_by_pos(bag).is_none() {
                    return InventoryResult::WrongBagType;
                }

                let Some(bag_proto) = bag_proto else {
                    return InventoryResult::WrongBagType;
                };

                if slot >= bag_proto.container_slots {
                    return InventoryResult::WrongBagType;
                }

                if !item_can_go_into_bag(proto, bag_proto) {
                    return InventoryResult::WrongBagType;
                }
            }

            proto.max_stack_size
        } else {
            let existing_item = existing_item.expect("checked Some above");
            let result = existing_item.can_be_merged_partly_with(proto.entry, proto.max_stack_size);
            if result != InventoryResult::Ok {
                return result;
            }

            proto.max_stack_size - existing_item.count()
        };

        let need_space = need_space.min(*count);
        let new_position = ItemPosCount::new(make_item_pos(bag, slot), need_space);
        if !new_position.is_contained_in(dest) {
            dest.push(new_position);
            *count -= need_space;
        }

        InventoryResult::Ok
    }

    pub(in crate::player::items::storage) fn can_store_item_in_inventory_slots(
        &self,
        slot_begin: u8,
        slot_end: u8,
        dest: &mut Vec<ItemPosCount>,
        proto: &ItemStorageTemplate,
        count: &mut u32,
        merge: bool,
        source_item: Option<&Item>,
        source_is_not_empty_bag: bool,
        skip_bag: u8,
        skip_slot: u8,
        slot_items: &[ItemSlotRef<'_>],
    ) -> InventoryResult {
        if source_item.is_some() && source_is_not_empty_bag {
            return InventoryResult::DestroyNonemptyBag;
        }

        for slot in slot_begin..slot_end {
            if skip_bag == INVENTORY_SLOT_BAG_0 && slot == skip_slot {
                continue;
            }

            if slot == REAGENT_BAG_SLOT_START {
                continue;
            }

            let existing_item =
                item_ref_by_pos(slot_items, INVENTORY_SLOT_BAG_0, slot).filter(|existing| {
                    source_item
                        .is_none_or(|source| existing.object().guid() != source.object().guid())
                });

            if existing_item.is_some() != merge {
                continue;
            }

            let mut need_space = proto.max_stack_size;
            if let Some(existing_item) = existing_item {
                if existing_item.can_be_merged_partly_with(proto.entry, proto.max_stack_size)
                    != InventoryResult::Ok
                {
                    continue;
                }

                need_space -= existing_item.count();
            }

            need_space = need_space.min(*count);
            let new_position =
                ItemPosCount::new(make_item_pos(INVENTORY_SLOT_BAG_0, slot), need_space);
            if !new_position.is_contained_in(dest) {
                dest.push(new_position);
                *count -= need_space;

                if *count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        InventoryResult::Ok
    }

    pub(in crate::player::items::storage) fn can_store_item_in_bag(
        &self,
        bag: u8,
        dest: &mut Vec<ItemPosCount>,
        proto: &ItemStorageTemplate,
        count: &mut u32,
        merge: bool,
        non_specialized: bool,
        source_item: Option<&Item>,
        source_is_not_empty_bag: bool,
        skip_bag: u8,
        skip_slot: u8,
        bag_proto: Option<&ItemStorageTemplate>,
        slot_items: &[ItemSlotRef<'_>],
    ) -> InventoryResult {
        if bag == skip_bag {
            return InventoryResult::WrongBagType;
        }

        let Some(bag_storage) = self
            .inventory
            .bags
            .get(bag as usize)
            .and_then(Option::as_ref)
        else {
            return InventoryResult::WrongBagType;
        };

        if source_item.is_some_and(|source| source.object().guid() == bag_storage.bag_guid) {
            return InventoryResult::WrongBagType;
        }

        if let Some(source) = source_item {
            if source_is_not_empty_bag {
                return InventoryResult::DestroyNonemptyBag;
            }

            if source.has_item_flag(ItemFieldFlags::CHILD) {
                return InventoryResult::WrongBagType3;
            }
        }

        let Some(bag_proto) = bag_proto else {
            return InventoryResult::WrongBagType;
        };

        let bag_is_regular_container = bag_proto.class_id == ItemClass::Container
            && bag_proto.subclass_id == ItemSubClassContainer::Container as u32;
        if non_specialized != bag_is_regular_container {
            return InventoryResult::WrongBagType;
        }

        if !item_can_go_into_bag(proto, bag_proto) {
            return InventoryResult::WrongBagType;
        }

        for slot in 0..bag_storage.bag_size {
            if slot == skip_slot {
                continue;
            }

            let existing_item = item_ref_by_pos(slot_items, bag, slot).filter(|existing| {
                source_item.is_none_or(|source| existing.object().guid() != source.object().guid())
            });

            if existing_item.is_some() != merge {
                continue;
            }

            let mut need_space = proto.max_stack_size;
            if let Some(existing_item) = existing_item {
                if existing_item.can_be_merged_partly_with(proto.entry, proto.max_stack_size)
                    != InventoryResult::Ok
                {
                    continue;
                }

                need_space -= existing_item.count();
            }

            need_space = need_space.min(*count);
            let new_position = ItemPosCount::new(make_item_pos(bag, slot), need_space);
            if !new_position.is_contained_in(dest) {
                dest.push(new_position);
                *count -= need_space;

                if *count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        InventoryResult::Ok
    }
}
