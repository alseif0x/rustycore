// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_entities::{
    BagValuesUpdate, CONTAINER_DATA_BITS, CONTAINER_DATA_SLOTS_FIRST_BIT,
    CONTAINER_DATA_SLOTS_PARENT_BIT, ContainerDataUpdate, ContainerDataValues, MAX_BAG_SIZE,
    TYPEID_CONTAINER, UpdateMask,
};
use wow_world_core::session::{
    HubRef, OwnedInventoryAccessLikeCpp, PacketPublicationAccessLikeCpp,
};

impl crate::InventoryState {
    /// Publish a container-slot change by bag GUID. This is needed for C++'s
    /// bag-content exchange: the previously full bag may no longer occupy a
    /// registered bag slot, but the client still owns that container object
    /// and must see its old child slot cleared.
    pub fn send_bag_object_slot_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        bag_guid: ObjectGuid,
        changed_slot: u8,
    ) {
        self.send_bag_object_slot_values_update_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(),
            &hub.core.packet_publication_access_like_cpp(),
            hub.catalogs.items.store.as_ref(),
            hub.catalogs.items.stats_store.as_ref(),
            bag_guid,
            changed_slot,
        );
    }

    pub fn send_bag_object_slot_values_update_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        publication: &PacketPublicationAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
        bag_guid: ObjectGuid,
        changed_slot: u8,
    ) {
        if changed_slot as usize >= MAX_BAG_SIZE {
            return;
        }
        let Some(bag_item) =
            self.resolved_player_inventory_item_object_with_access_like_cpp(access, bag_guid)
        else {
            return;
        };
        let Some(bag_size) = wow_world_core::catalogs::item::item_storage_template_like_cpp(
            item_store,
            item_stats_store,
            bag_item.object().entry(),
        )
        .map(|template| template.container_slots)
        .filter(|size| *size > 0) else {
            return;
        };
        let mut slots = [ObjectGuid::EMPTY; MAX_BAG_SIZE];
        let Some(item_objects) = self.resolved_inventory_item_objects_with_access_like_cpp(access)
        else {
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
        let _ = publication.publish_bag_values_update_like_cpp(bag_guid, &update);
    }
}

impl crate::InventoryState {
    pub fn send_bag_slot_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        bag_slot: u8,
        changed_slot: u8,
    ) {
        self.send_bag_slot_values_update_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(),
            &hub.core.packet_publication_access_like_cpp(),
            bag_slot,
            changed_slot,
        );
    }

    pub fn send_bag_slot_values_update_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        publication: &PacketPublicationAccessLikeCpp<'_>,
        bag_slot: u8,
        changed_slot: u8,
    ) {
        if changed_slot as usize >= MAX_BAG_SIZE {
            return;
        }
        let Some((bag_guid, bag_size, slot_values)) =
            access.canonical_inventory_bag_slots_snapshot_like_cpp(bag_slot)
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
        let _ = publication.publish_bag_values_update_like_cpp(bag_guid, &update);
    }
}
