//! Represented bag slots and their container capacity.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn represented_bag_contains_active_item_loot_like_cpp(
        &self,
        bag_guid: ObjectGuid,
    ) -> bool {
        if self.active_loot_view_owners.is_empty() {
            return false;
        }

        self.resolved_inventory_item_objects_like_cpp()
            .is_some_and(|items| {
                items.values().any(|item| {
                    item.container_guid() == bag_guid
                        && self.active_loot_view_owners.contains(&item.object().guid())
                        && self.loot_table.contains_key(&item.object().guid())
                })
            })
    }
    pub(crate) fn send_bag_slot_values_update_like_cpp(&self, bag_slot: u8, changed_slot: u8) {
        if changed_slot as usize >= MAX_BAG_SIZE {
            return;
        }
        let Some((bag_guid, bag_size, slot_values)) = self
            .canonical_player_snapshot_like_cpp(|player| {
                let bag = player
                    .inventory()
                    .bags
                    .get(bag_slot as usize)
                    .and_then(Option::as_ref)?;
                let mut slots = [ObjectGuid::EMPTY; MAX_BAG_SIZE];
                for (index, slot) in bag.slots.iter().enumerate() {
                    slots[index] = slot.unwrap_or(ObjectGuid::EMPTY);
                }
                Some((bag.bag_guid, bag.bag_size, slots))
            })
            .flatten()
        else {
            return;
        };

        let mut container_data_mask = UpdateMask::new(CONTAINER_DATA_BITS);
        container_data_mask.set(CONTAINER_DATA_SLOTS_PARENT_BIT);
        container_data_mask.set(CONTAINER_DATA_SLOTS_FIRST_BIT + changed_slot as usize);
        let update = BagValuesUpdate {
            changed_object_type_mask: 1 << TYPEID_CONTAINER,
            object_data: None,
            item_data: None,
            container_data: Some(ContainerDataUpdate {
                mask: container_data_mask,
                values: ContainerDataValues {
                    num_slots: u32::from(bag_size),
                    slots: slot_values,
                },
            }),
        };
        if let Some(packet) =
            bag_values_update_to_update_object(bag_guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
    /// Publish a container-slot change by bag GUID. This is needed for C++'s
    /// bag-content exchange: the previously full bag may no longer occupy a
    /// registered bag slot, but the client still owns that container object
    /// and must see its old child slot cleared.
    pub(crate) fn send_bag_object_slot_values_update_like_cpp(
        &self,
        bag_guid: ObjectGuid,
        changed_slot: u8,
    ) {
        if changed_slot as usize >= MAX_BAG_SIZE {
            return;
        }
        let Some(bag_item) = self.resolved_inventory_item_object_like_cpp(bag_guid) else {
            return;
        };
        let Some(bag_size) = self
            .item_storage_template(bag_item.object().entry())
            .map(|template| template.container_slots)
            .filter(|size| *size > 0)
        else {
            return;
        };
        let mut slots = [ObjectGuid::EMPTY; MAX_BAG_SIZE];
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return;
        };
        for item in item_objects
            .values()
            .filter(|item| item.container_guid() == bag_guid)
        {
            if let Some(slot) = slots.get_mut(item.slot() as usize) {
                *slot = item.object().guid();
            }
        }

        let mut container_data_mask = UpdateMask::new(CONTAINER_DATA_BITS);
        container_data_mask.set(CONTAINER_DATA_SLOTS_PARENT_BIT);
        container_data_mask.set(CONTAINER_DATA_SLOTS_FIRST_BIT + changed_slot as usize);
        let update = BagValuesUpdate {
            changed_object_type_mask: 1 << TYPEID_CONTAINER,
            object_data: None,
            item_data: None,
            container_data: Some(ContainerDataUpdate {
                mask: container_data_mask,
                values: ContainerDataValues {
                    num_slots: u32::from(bag_size),
                    slots,
                },
            }),
        };
        if let Some(packet) =
            bag_values_update_to_update_object(bag_guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
}
