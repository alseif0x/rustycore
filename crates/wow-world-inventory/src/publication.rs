// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::item::EnchantmentSlot;
use wow_core::ObjectGuid;
use wow_entities::{
    BUYBACK_SLOT_COUNT, BUYBACK_SLOT_END, BUYBACK_SLOT_START, ITEM_DATA_BITS,
    ITEM_DATA_DYNAMIC_FLAGS_BIT, ITEM_DATA_PARENT_BIT, ItemDataUpdate, ItemValuesUpdate,
    PLAYER_SLOT_END, TYPEID_ITEM, UpdateMask, VisibleItemValues,
};
use wow_world_core::{
    entity_update_bridge::player_values_update_to_update_object,
    session::{
        HubRef, InventoryPlayerProjectionLikeCpp, OwnedInventoryAccessLikeCpp,
        PLAYER_FLAGS_RESTING_LIKE_CPP, PacketPublicationAccessLikeCpp,
    },
};

pub fn item_storage_fields_values_update_like_cpp(
    item: &wow_entities::Item,
    contained_in_changed: bool,
    dynamic_flags2_changed: bool,
    changed_enchantments: &[wow_constants::item::EnchantmentSlot],
) -> wow_entities::ItemValuesUpdate {
    let mut item_data_mask = wow_entities::UpdateMask::new(wow_entities::ITEM_DATA_BITS);
    if contained_in_changed || dynamic_flags2_changed {
        item_data_mask.set(wow_entities::ITEM_DATA_PARENT_BIT);
    }
    if contained_in_changed {
        item_data_mask.set(wow_entities::ITEM_DATA_CONTAINED_IN_BIT);
    }
    if dynamic_flags2_changed {
        item_data_mask.set(wow_entities::ITEM_DATA_DYNAMIC_FLAGS2_BIT);
    }
    if !changed_enchantments.is_empty() {
        item_data_mask.set(wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT);
        for slot in changed_enchantments {
            item_data_mask.set(wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT + *slot as usize);
        }
    }
    wow_entities::ItemValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_ITEM,
        object_data: None,
        item_data: Some(wow_entities::ItemDataUpdate {
            mask: item_data_mask,
            values: item.data().clone(),
        }),
    }
}

/// Borrowed owners required to prepare and publish a Player values update.
/// The canonical Player is available only as a detached typed projection.
struct InventoryPublicationCx<'a, 'core, 'packet> {
    inventory: &'a crate::InventoryState,
    access: &'a OwnedInventoryAccessLikeCpp<'core>,
    publication: &'a PacketPublicationAccessLikeCpp<'packet>,
    item_store: Option<&'a std::sync::Arc<wow_data::ItemStore>>,
    item_stats_store: Option<&'a std::sync::Arc<wow_data::ItemStatsStore>>,
}

impl<'a, 'core, 'packet> InventoryPublicationCx<'a, 'core, 'packet> {
    fn whole_player_values_update_snapshot(&self) -> Option<InventoryPlayerProjectionLikeCpp> {
        let mut player = self
            .inventory
            .direct_inventory_player_snapshot_with_access_like_cpp(
                self.access,
                self.item_store,
                self.item_stats_store,
            )?;
        player.set_money_like_cpp(
            self.inventory
                .resolved_player_money_with_access_like_cpp(self.access)?,
        );
        player.set_bank_bag_slot_count_like_cpp(
            self.inventory
                .resolved_player_bank_bag_slot_count_with_access_like_cpp(self.access)?,
        );
        for index in 0..7 {
            let value = self
                .inventory
                .represented_bank_bag_slot_flag_with_access_like_cpp(self.access, index)?;
            let _ = player.set_bank_bag_slot_flag_value_like_cpp(index, value);
        }
        let inventory_items = self
            .inventory
            .resolved_inventory_items_with_access_like_cpp(self.access)?;
        let buyback_items = self
            .inventory
            .resolved_buyback_items_with_access_like_cpp(self.access)?;
        let buyback_price = self
            .inventory
            .resolved_buyback_price_with_access_like_cpp(self.access)?;
        let buyback_timestamp = self
            .inventory
            .resolved_buyback_timestamp_with_access_like_cpp(self.access)?;

        for slot in 0..19u8 {
            let visible = inventory_items.get(&slot).map(|item| VisibleItemValues {
                item_id: item.entry_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            });
            player.set_visible_item_slot_like_cpp(slot, visible);
        }

        for slot in 15..=17u8 {
            let visible = inventory_items.get(&slot).map(|item| VisibleItemValues {
                item_id: item.entry_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            });
            player.set_virtual_item_like_cpp((slot - 15) as usize, visible);
        }

        for (&slot, item) in &buyback_items {
            if (slot as usize) < PLAYER_SLOT_END {
                player.set_inv_slot_like_cpp(slot as usize, item.guid);
            }
        }
        for index in 0..BUYBACK_SLOT_COUNT {
            player.set_buyback_price_like_cpp(index, buyback_price[index]);
            player.set_buyback_timestamp_like_cpp(index, buyback_timestamp[index]);
        }

        player.clear_data_changes_like_cpp();
        Some(player)
    }

    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        let Some(guid) = self.access.player_guid_like_cpp() else {
            return false;
        };
        if !visible_item_changes.is_empty() {
            let _ = self
                .access
                .set_player_visible_item_values_by_guid_like_cpp(guid, visible_item_changes);
        }
        let Some(mut player) = self.whole_player_values_update_snapshot() else {
            return false;
        };

        if let Some(coinage) = coinage {
            player.set_money_like_cpp(coinage);
            player.mark_money_changed_like_cpp();
        }

        for &(slot, item_guid) in inv_slot_changes {
            player.set_inv_slot_like_cpp(slot as usize, item_guid);
            player.mark_inv_slot_changed_like_cpp(slot as usize);
        }

        for &(slot, item_id, appearance_mod_id, item_visual) in visible_item_changes {
            player.set_player_visible_item_values_like_cpp(
                slot,
                (item_id, appearance_mod_id, item_visual),
            );
            player.mark_visible_item_slot_changed_like_cpp(slot);
        }

        for &(index, item_id, appearance_mod_id, item_visual) in virtual_item_changes {
            let visible = (item_id != 0 || appearance_mod_id != 0 || item_visual != 0).then_some(
                VisibleItemValues {
                    item_id,
                    item_appearance_mod_id: appearance_mod_id,
                    item_visual,
                },
            );
            player.set_virtual_item_like_cpp(index as usize, visible);
            player.mark_virtual_item_changed_like_cpp(index as usize);
        }

        for &(slot, price, timestamp) in buyback_changes {
            if !(BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot) {
                continue;
            }
            let index = (slot - BUYBACK_SLOT_START) as usize;
            player.set_buyback_price_like_cpp(index, price);
            player.mark_buyback_price_changed_like_cpp(index);
            player.set_buyback_timestamp_like_cpp(index, timestamp);
            player.mark_buyback_timestamp_changed_like_cpp(index);
        }

        let update = player.values_update_like_cpp(true);
        self.publication
            .publish_player_values_update_like_cpp(guid, &update)
    }
}

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
        self.send_item_storage_fields_values_update_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(),
            &hub.core.packet_publication_access_like_cpp(),
            item_guid,
            contained_in_changed,
            dynamic_flags2_changed,
            changed_enchantments,
        );
    }

    pub fn send_item_relocation_values_update_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        publication: &PacketPublicationAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
        dynamic_flags2_changed: bool,
        cleared_enchantments: &[EnchantmentSlot],
    ) {
        self.send_item_storage_fields_values_update_with_access_like_cpp(
            access,
            publication,
            item_guid,
            true,
            dynamic_flags2_changed,
            cleared_enchantments,
        );
    }

    fn send_item_storage_fields_values_update_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        publication: &PacketPublicationAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
        contained_in_changed: bool,
        dynamic_flags2_changed: bool,
        changed_enchantments: &[EnchantmentSlot],
    ) {
        let Some(item) =
            self.resolved_player_inventory_item_object_with_access_like_cpp(access, item_guid)
        else {
            return;
        };
        let update = crate::item_storage_fields_values_update_like_cpp(
            &item,
            contained_in_changed,
            dynamic_flags2_changed,
            changed_enchantments,
        );
        let _ = publication.publish_item_values_update_like_cpp(item_guid, &update);
    }

    pub fn send_item_dynamic_flags_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
    ) {
        self.send_item_dynamic_flags_values_update_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(),
            &hub.core.packet_publication_access_like_cpp(),
            item_guid,
        );
    }

    pub fn send_item_dynamic_flags_values_update_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        publication: &PacketPublicationAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(item) =
            self.resolved_player_inventory_item_object_with_access_like_cpp(access, item_guid)
        else {
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
        let _ = publication.publish_item_values_update_like_cpp(item_guid, &update);
    }
}

impl crate::InventoryState {
    pub fn player_values_update_snapshot(
        &self,
        hub: HubRef<'_>,
    ) -> Option<InventoryPlayerProjectionLikeCpp> {
        let access = hub.core.owned_inventory_access_like_cpp();
        let publication = hub.core.packet_publication_access_like_cpp();
        self.player_values_update_snapshot_with_access_like_cpp(
            &access,
            &publication,
            hub.catalogs.items.store.as_ref(),
            hub.catalogs.items.stats_store.as_ref(),
        )
    }

    pub fn player_values_update_snapshot_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        publication: &PacketPublicationAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
    ) -> Option<InventoryPlayerProjectionLikeCpp> {
        InventoryPublicationCx {
            inventory: self,
            access,
            publication,
            item_store,
            item_stats_store,
        }
        .whole_player_values_update_snapshot()
    }

    pub fn send_player_values_update_from_entity_bridge(
        &self,
        hub: HubRef<'_>,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        let access = hub.core.owned_inventory_access_like_cpp();
        let publication = hub.core.packet_publication_access_like_cpp();
        self.send_player_values_update_from_entity_bridge_with_access_like_cpp(
            &access,
            &publication,
            hub.catalogs.items.store.as_ref(),
            hub.catalogs.items.stats_store.as_ref(),
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
    }

    pub fn send_player_values_update_from_entity_bridge_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        publication: &PacketPublicationAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        // Reports whether a packet was enqueued; a rejected or absent send is
        // not recorded as a delivered Player values update.
        InventoryPublicationCx {
            inventory: self,
            access,
            publication,
            item_store,
            item_stats_store,
        }
        .send_player_values_update_from_entity_bridge(
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
    }

    pub fn send_represented_resting_player_flag_update_like_cpp(&self, hub: HubRef<'_>) -> bool {
        let Some(guid) = hub.core.player_guid() else {
            return false;
        };
        let Some(mut player) = self.player_values_update_snapshot(hub) else {
            return false;
        };

        let canonical_flags = hub
            .core
            .canonical_player_snapshot_like_cpp(|player| player.data().player_flags)
            .unwrap_or_default();
        let Some(is_resting) = hub.resolved_is_resting_like_cpp() else {
            return false;
        };
        if is_resting {
            player.replace_all_player_flags_like_cpp(
                canonical_flags & !PLAYER_FLAGS_RESTING_LIKE_CPP,
            );
            player.set_player_flag_like_cpp(PLAYER_FLAGS_RESTING_LIKE_CPP);
        } else {
            player
                .replace_all_player_flags_like_cpp(canonical_flags | PLAYER_FLAGS_RESTING_LIKE_CPP);
            player.remove_player_flag_like_cpp(PLAYER_FLAGS_RESTING_LIKE_CPP);
        }
        let update = player.values_update_like_cpp(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, hub.core.player_map_id_like_cpp(), &update)
        {
            return hub.core.send_packet(&packet);
        }
        false
    }
}
