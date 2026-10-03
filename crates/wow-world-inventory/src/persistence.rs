// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_entities::{
    is_bag_pos, make_item_pos, Item, ItemFieldFlags, PlayerInventoryItem as InventoryItem,
    PlayerItemTimeUpdate, SwapItemPreflightItem, SwapItemPreflightPlan, INVENTORY_SLOT_BAG_0,
};
use wow_packet::packets::item::ItemTimeUpdate;
use wow_world_core::session::{HubMut, HubRef};

use crate::is_represented_bag_slot;

impl crate::InventoryState {
    /// Publish one new item whose CharacterDB rows already committed.
    pub fn apply_committed_new_inventory_item_at_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        mut item_object: Item,
    ) -> bool {
        if self
            .get_inventory_item_by_pos(hub.shared(), bag, slot)
            .is_some()
        {
            return false;
        }

        if bag == INVENTORY_SLOT_BAG_0 {
            item_object.set_container_guid(ObjectGuid::EMPTY);
            item_object.set_slot(slot);
            self.insert_inventory_item_like_cpp(hub, slot, inventory_item.clone());
        } else {
            let Some(bag_item) = self.resolved_inventory_item_like_cpp(hub.shared(), bag) else {
                return false;
            };
            item_object.set_contained_in(bag_item.guid);
            item_object.set_slot(slot);
            item_object.set_container_guid_and_slot(bag_item.guid, bag);
        }

        let item_guid = inventory_item.guid;
        let new_bag_size = (bag == INVENTORY_SLOT_BAG_0 && is_represented_bag_slot(slot))
            .then(|| hub.catalogs.item_storage_template(inventory_item.entry_id))
            .flatten()
            .map(|template| template.container_slots)
            .filter(|size| *size > 0);
        self.insert_inventory_item_object(hub, item_object);
        let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
            if bag == INVENTORY_SLOT_BAG_0 {
                let stored = player.store_top_level_item(slot, item_guid);
                debug_assert!(stored.is_ok());
                if let Some(bag_size) = new_bag_size {
                    // C++ installs the new `Bag` object in m_items before a
                    // later sequential StoreNewItem can address its children.
                    // The canonical Rust Player keeps bag contents in a
                    // separate registry, so establish it at the same point.
                    let registered = player.register_bag_storage(slot, item_guid, bag_size);
                    debug_assert!(registered.is_ok());
                }
            } else {
                let stored = player.store_bag_item(bag, slot, item_guid);
                debug_assert!(stored.is_ok());
            }
        });
        true
    }

    /// C++ `Player::SwapItem` preflight (child redirects, life state,
    /// `CanUnequipItem`, and bag-in-bag guards) for live session positions.
    pub fn plan_inventory_swap_preflight_like_cpp(
        &self,
        hub: HubRef<'_>,
        src: u16,
        dst: u16,
    ) -> Option<SwapItemPreflightPlan> {
        let [src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, dst_slot] = dst.to_be_bytes();
        let source = self.get_inventory_item_by_pos(hub, src_bag, src_slot);
        let destination = self.get_inventory_item_by_pos(hub, dst_bag, dst_slot);

        let preflight_item = |item: &InventoryItem, bag: u8, slot: u8, swap: bool| {
            let runtime_item = self.resolved_inventory_item_object_like_cpp(hub, item.guid);
            let proto = hub.catalogs.item_storage_template(item.entry_id);
            let is_bag = proto
                .as_ref()
                .is_some_and(|template| template.container_slots > 0);
            let parent_pos = runtime_item
                .as_ref()
                .filter(|item| item.has_item_flag(ItemFieldFlags::CHILD))
                .and_then(|item| self.get_inventory_item_by_guid_like_cpp(hub, item.data().creator))
                .map(|(bag, slot, _)| make_item_pos(bag, slot));
            SwapItemPreflightItem {
                is_bag,
                is_empty_bag: is_bag && !self.direct_item_contains_items(hub, item.guid),
                is_child: runtime_item
                    .as_ref()
                    .is_some_and(|item| item.has_item_flag(ItemFieldFlags::CHILD)),
                parent_pos,
                can_unequip_result: self.can_unequip_inventory_item_at_like_cpp(
                    hub,
                    bag,
                    slot,
                    swap,
                    runtime_item.as_ref(),
                    proto.as_ref(),
                    self.direct_item_contains_items(hub, item.guid),
                ),
            }
        };

        let source_is_bag_pos = is_bag_pos(src);
        let destination_is_bag_pos = is_bag_pos(dst);
        let source_is_bag = source.as_ref().is_some_and(|item| {
            hub.catalogs
                .item_storage_template(item.entry_id)
                .is_some_and(|template| template.container_slots > 0)
        });
        let destination_is_empty_bag = destination.as_ref().is_some_and(|item| {
            hub.catalogs
                .item_storage_template(item.entry_id)
                .is_some_and(|template| template.container_slots > 0)
                && !self.direct_item_contains_items(hub, item.guid)
        });
        let source_is_empty_bag = source_is_bag
            && source
                .as_ref()
                .is_some_and(|item| !self.direct_item_contains_items(hub, item.guid));
        let source_swap = !source_is_bag_pos || destination_is_bag_pos || destination_is_empty_bag;
        let destination_swap = !destination_is_bag_pos || source_is_bag_pos || source_is_empty_bag;
        let source_preflight = source
            .as_ref()
            .map(|item| preflight_item(item, src_bag, src_slot, source_swap));
        let destination_preflight = destination
            .as_ref()
            .map(|item| preflight_item(item, dst_bag, dst_slot, destination_swap));

        let player = self.direct_inventory_player_snapshot(hub)?;
        let player_is_alive = hub.resolved_player_is_alive_like_cpp()?;
        Some(player.swap_item_preflight_plan_like_cpp(
            src,
            dst,
            player_is_alive,
            source_preflight,
            destination_preflight,
        ))
    }
}

impl crate::InventoryState {
    pub fn send_item_time_update_plan(
        &self,
        hub: HubRef<'_>,
        update: &PlayerItemTimeUpdate,
    ) {
        hub.core.send_packet(&ItemTimeUpdate {
            item_guid: update.item_guid,
            duration_left: update.expiration,
        });
    }

    pub fn send_item_time_update_plans(
        &self,
        hub: HubRef<'_>,
        updates: &[PlayerItemTimeUpdate],
    ) {
        for update in updates {
            self.send_item_time_update_plan(hub, update);
        }
    }
}
