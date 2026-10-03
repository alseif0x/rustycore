// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::item::EnchantmentSlot;
use wow_core::ObjectGuid;
use wow_entities::{
    ItemDataUpdate, ItemValuesUpdate, UpdateMask, ITEM_DATA_BITS, ITEM_DATA_DYNAMIC_FLAGS_BIT,
    ITEM_DATA_PARENT_BIT, TYPEID_ITEM,
};
use wow_world_core::{
    entity_update_bridge::item_values_update_to_update_object,
    session::HubRef,
};

impl crate::InventoryState {
    pub fn send_item_contained_in_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
    ) {
        self.send_item_storage_fields_values_update_like_cpp(hub, item_guid, true, false, &[]);
    }

    pub fn send_item_relocation_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
        dynamic_flags2_changed: bool,
        cleared_enchantments: &[EnchantmentSlot],
    ) {
        self.send_item_storage_fields_values_update_like_cpp(
            hub,
            item_guid,
            true,
            dynamic_flags2_changed,
            cleared_enchantments,
        );
    }

    fn send_item_storage_fields_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
        contained_in_changed: bool,
        dynamic_flags2_changed: bool,
        changed_enchantments: &[EnchantmentSlot],
    ) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(hub, item_guid) else {
            return;
        };
        let update = crate::item_storage_fields_values_update_like_cpp(
            &item,
            contained_in_changed,
            dynamic_flags2_changed,
            changed_enchantments,
        );
        if let Some(packet) = item_values_update_to_update_object(
            item_guid,
            hub.core.player_map_id_like_cpp(),
            &update,
        ) {
            hub.core.send_packet(&packet);
        }
    }

    pub fn send_item_dynamic_flags_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(hub, item_guid) else {
            return;
        };
        let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
        item_data_mask.set(ITEM_DATA_PARENT_BIT);
        item_data_mask.set(ITEM_DATA_DYNAMIC_FLAGS_BIT);
        let update = ItemValuesUpdate {
            changed_object_type_mask: 1 << TYPEID_ITEM,
            object_data: None,
            item_data: Some(ItemDataUpdate {
                mask: item_data_mask,
                values: item.data().clone(),
            }),
        };
        if let Some(packet) = item_values_update_to_update_object(
            item_guid,
            hub.core.player_map_id_like_cpp(),
            &update,
        ) {
            hub.core.send_packet(&packet);
        }
    }
}
