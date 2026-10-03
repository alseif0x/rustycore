// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::item::EnchantmentSlot;
use wow_core::ObjectGuid;
use wow_entities::{
    ItemDataUpdate, ItemValuesUpdate, Player, UpdateMask, VisibleItemValues, BUYBACK_SLOT_COUNT,
    BUYBACK_SLOT_END, BUYBACK_SLOT_START, ITEM_DATA_BITS, ITEM_DATA_DYNAMIC_FLAGS_BIT,
    ITEM_DATA_PARENT_BIT, PLAYER_SLOT_END, TYPEID_ITEM,
};
use wow_world_core::{
    canonical_player_access::set_player_visible_item_values_like_cpp,
    entity_update_bridge::{
        item_values_update_to_update_object, player_values_update_to_update_object,
    },
    session::{HubRef, PLAYER_FLAGS_RESTING_LIKE_CPP},
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

impl crate::InventoryState {
    pub fn player_values_update_snapshot(&self, hub: HubRef<'_>) -> Option<Player> {
        let mut player = self.direct_inventory_player_snapshot(hub)?;
        player.set_money(self.resolved_player_money_like_cpp(hub)?);
        player.set_bank_bag_slot_count(self.resolved_player_bank_bag_slot_count_like_cpp(hub)?);
        for index in 0..7 {
            let value = self.represented_bank_bag_slot_flag_like_cpp(hub, index)?;
            player.set_bank_bag_slot_flag_value_like_cpp(index, value);
        }
        let inventory_items = self.resolved_inventory_items_like_cpp(hub)?;
        let buyback_items = self.resolved_buyback_items_like_cpp(hub)?;
        let buyback_price = self.resolved_buyback_price_like_cpp(hub)?;
        let buyback_timestamp = self.resolved_buyback_timestamp_like_cpp(hub)?;

        for slot in 0..19u8 {
            let visible = inventory_items.get(&slot).map(|item| VisibleItemValues {
                item_id: item.entry_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            });
            player.set_visible_item_slot(slot, visible);
        }

        for slot in 15..=17u8 {
            let visible = inventory_items.get(&slot).map(|item| VisibleItemValues {
                item_id: item.entry_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            });
            player
                .unit_mut()
                .set_virtual_item((slot - 15) as usize, visible);
        }

        for (&slot, item) in &buyback_items {
            if (slot as usize) < PLAYER_SLOT_END {
                player.set_inv_slot(slot as usize, item.guid);
            }
        }
        for index in 0..BUYBACK_SLOT_COUNT {
            player.set_buyback_price(index, buyback_price[index]);
            player.set_buyback_timestamp(index, buyback_timestamp[index]);
        }

        player.clear_data_changes();
        Some(player)
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
        // Reports whether a packet was enqueued. Callers that record a
        // publication need to know: this returns early when there is no player
        // GUID or snapshot, and the trace must not claim the client saw
        // something that was never sent.
        let Some(guid) = hub.core.player_guid() else {
            return false;
        };
        if !visible_item_changes.is_empty() {
            let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
                for &(slot, item_id, item_appearance_mod_id, item_visual) in visible_item_changes {
                    set_player_visible_item_values_like_cpp(
                        player,
                        slot,
                        (item_id, item_appearance_mod_id, item_visual),
                    );
                }
            });
        }
        let Some(mut player) = self.player_values_update_snapshot(hub) else {
            return false;
        };

        if let Some(coinage) = coinage {
            player.set_money(coinage);
            player.mark_money_changed();
        }

        for &(slot, item_guid) in inv_slot_changes {
            player.set_inv_slot(slot as usize, item_guid);
            player.mark_inv_slot_changed(slot as usize);
        }

        for &(slot, item_id, appearance_mod_id, item_visual) in visible_item_changes {
            set_player_visible_item_values_like_cpp(
                &mut player,
                slot,
                (item_id, appearance_mod_id, item_visual),
            );
            player.mark_visible_item_slot_changed(slot);
        }

        for &(index, item_id, appearance_mod_id, item_visual) in virtual_item_changes {
            let visible = (item_id != 0 || appearance_mod_id != 0 || item_visual != 0).then_some(
                VisibleItemValues {
                    item_id,
                    item_appearance_mod_id: appearance_mod_id,
                    item_visual,
                },
            );
            player.unit_mut().set_virtual_item(index as usize, visible);
            player.unit_mut().mark_virtual_item_changed(index as usize);
        }

        for &(slot, price, timestamp) in buyback_changes {
            if !(BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot) {
                continue;
            }
            let index = (slot - BUYBACK_SLOT_START) as usize;
            player.set_buyback_price(index, price);
            player.mark_buyback_price_changed(index);
            player.set_buyback_timestamp(index, timestamp);
            player.mark_buyback_timestamp_changed(index);
        }

        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, hub.core.player_map_id_like_cpp(), &update)
        {
            // Reaching the send is not the same as the send being accepted:
            // the channel can be closed, and a publication recorded on the
            // strength of getting this far would claim a packet the client
            // never received.
            return hub.core.send_packet(&packet);
        }
        false
    }

    pub fn send_represented_resting_player_flag_update_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
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
            player.replace_all_player_flags(canonical_flags & !PLAYER_FLAGS_RESTING_LIKE_CPP);
            player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
        } else {
            player.replace_all_player_flags(canonical_flags | PLAYER_FLAGS_RESTING_LIKE_CPP);
            player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
        }
        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, hub.core.player_map_id_like_cpp(), &update)
        {
            return hub.core.send_packet(&packet);
        }
        false
    }
}
