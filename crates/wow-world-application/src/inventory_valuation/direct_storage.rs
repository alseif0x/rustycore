// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected detached inventory planning shared by reward and CanEquip callers.

use std::collections::HashMap;

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_entities::{
    is_buyback_slot, BagTemplateRef, CanStoreItemArgs, INVENTORY_SLOT_BAG_0, Item, ItemSlotRef,
    ItemStorageRef,
};

use crate::PlayerConditionProjectionCxLikeCpp;

use super::InventoryValuationApplicationCxLikeCpp;

impl InventoryValuationApplicationCxLikeCpp<'_> {
    pub fn plan_store_direct_inventory_item_like_cpp(
        &self,
        realm_id: u16,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
        source_item: Option<&Item>,
        swap: bool,
        overlays: &[wow_world_inventory::DirectInventoryStorageOverlayLikeCpp],
        vacated_positions: &[(u8, u8)],
    ) -> Option<(InventoryResult, Vec<wow_entities::ItemPosCount>, Option<u32>)> {
        let mut player = self.inventory.direct_inventory_player_snapshot_with_access_like_cpp(
            &self.inventory_access,
            self.catalogs.item_store_like_cpp(),
            self.catalogs.item_stats_store_like_cpp(),
        )?;
        // C++ processes every valid deposit (including recursive bag contents)
        // before it calls CanStoreNewItem for withdrawals. Remove those detached
        // positions from this planning snapshot in the same child-before-parent order.
        for &(vacated_bag, vacated_slot) in vacated_positions {
            if vacated_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item_like_cpp(vacated_slot);
            } else {
                let _ = player.remove_bag_item_like_cpp(vacated_bag, vacated_slot);
            }
        }

        let proto = self.catalogs.item_storage_template_like_cpp(entry_id);
        let inventory_items = self
            .inventory
            .resolved_inventory_items_with_access_like_cpp(&self.inventory_access)?;
        let item_objects = self
            .inventory
            .resolved_inventory_item_objects_with_access_like_cpp(&self.inventory_access)?;
        let mut overlay_items = Vec::with_capacity(overlays.len());
        for (index, overlay) in overlays.iter().enumerate() {
            let mut item = self
                .inventory
                .get_inventory_item_by_pos_with_access_like_cpp(
                    &self.inventory_access,
                    self.catalogs.item_store_like_cpp(),
                    self.catalogs.item_stats_store_like_cpp(),
                    overlay.bag,
                    overlay.slot,
                )
                .and_then(|inventory_item| item_objects.get(&inventory_item.guid))
                .cloned()
                .unwrap_or_else(|| {
                    let mut item = Item::default();
                    let placeholder_counter = i64::MAX.saturating_sub(index as i64);
                    item.object_mut().create(ObjectGuid::create_item(
                        realm_id,
                        placeholder_counter,
                    ));
                    item.object_mut().set_entry(overlay.entry_id);
                    item
                });
            // An overlay can reuse a slot vacated by an earlier deposit, so its
            // planned entry is authoritative over the stale runtime item until COMMIT.
            item.object_mut().set_entry(overlay.entry_id);
            item.set_count(overlay.count);
            item.set_slot(overlay.slot);
            overlay_items.push((overlay.bag, overlay.slot, item));
        }

        let mut template_cache = HashMap::new();
        for item in item_objects.values() {
            let item_entry = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(entry) =
                template_cache.entry(item_entry)
                && let Some(template) = self.catalogs.item_storage_template_like_cpp(item_entry)
            {
                entry.insert(template);
            }
        }
        for (_, _, item) in &overlay_items {
            let item_entry = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(entry) =
                template_cache.entry(item_entry)
                && let Some(template) = self.catalogs.item_storage_template_like_cpp(item_entry)
            {
                entry.insert(template);
            }
        }

        let mut represented_bag_slots_by_guid = HashMap::new();
        let mut bag_templates = Vec::new();
        for (&inventory_slot, item) in &inventory_items {
            if is_buyback_slot(inventory_slot) {
                continue;
            }
            if vacated_positions.contains(&(INVENTORY_SLOT_BAG_0, inventory_slot)) {
                continue;
            }
            if is_represented_bag_slot(inventory_slot) && item_objects.contains_key(&item.guid) {
                represented_bag_slots_by_guid.insert(item.guid, inventory_slot);
                if let Some(template) = template_cache.get(&item.entry_id)
                    && template.container_slots > 0
                {
                    bag_templates.push(BagTemplateRef::new(inventory_slot, template));
                }
            }
        }
        for (index, (overlay_bag, overlay_slot, item)) in overlay_items.iter().enumerate() {
            if *overlay_bag != INVENTORY_SLOT_BAG_0 || !is_represented_bag_slot(*overlay_slot) {
                continue;
            }
            let Some(template) = template_cache.get(&item.object().entry()) else {
                continue;
            };
            if template.container_slots == 0 {
                continue;
            }
            if vacated_positions.contains(&(*overlay_bag, *overlay_slot))
                || self
                    .inventory
                    .get_inventory_item_by_pos_with_access_like_cpp(
                        &self.inventory_access,
                        self.catalogs.item_store_like_cpp(),
                        self.catalogs.item_stats_store_like_cpp(),
                        *overlay_bag,
                        *overlay_slot,
                    )
                    .is_none()
            {
                let placeholder_counter = i64::MAX.saturating_sub(index as i64);
                let placeholder_guid = ObjectGuid::create_item(realm_id, placeholder_counter);
                let _ = player.store_top_level_item(*overlay_slot, placeholder_guid);
                let _ = player.register_bag_storage(
                    *overlay_slot,
                    placeholder_guid,
                    template.container_slots,
                );
            }
            if !bag_templates.iter().any(|bag| bag.bag == *overlay_slot) {
                bag_templates.push(BagTemplateRef::new(*overlay_slot, template));
            }
        }

        let mut slot_items = Vec::new();
        let mut stored_items = Vec::new();
        for (&inventory_slot, inventory_item) in &inventory_items {
            if is_buyback_slot(inventory_slot) {
                continue;
            }
            if overlays.iter().any(|overlay| {
                overlay.bag == INVENTORY_SLOT_BAG_0 && overlay.slot == inventory_slot
            }) || vacated_positions.contains(&(INVENTORY_SLOT_BAG_0, inventory_slot))
            {
                continue;
            }
            let Some(item) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            slot_items.push(ItemSlotRef::new(INVENTORY_SLOT_BAG_0, inventory_slot, item));
            stored_items.push(ItemStorageRef::new(
                INVENTORY_SLOT_BAG_0,
                inventory_slot,
                item,
                template_cache.get(&inventory_item.entry_id),
            ));
        }
        for item in item_objects.values() {
            let container_guid = item.container_guid();
            if container_guid.is_empty() {
                continue;
            }
            let Some(&bag_slot) = represented_bag_slots_by_guid.get(&container_guid) else {
                continue;
            };
            if overlays
                .iter()
                .any(|overlay| overlay.bag == bag_slot && overlay.slot == item.slot())
                || vacated_positions.contains(&(bag_slot, item.slot()))
            {
                continue;
            }
            let item_entry = item.object().entry();
            slot_items.push(ItemSlotRef::new(bag_slot, item.slot(), item));
            stored_items.push(ItemStorageRef::new(
                bag_slot,
                item.slot(),
                item,
                template_cache.get(&item_entry),
            ));
        }
        for (overlay_bag, overlay_slot, item) in &overlay_items {
            let item_entry = item.object().entry();
            slot_items.push(ItemSlotRef::new(*overlay_bag, *overlay_slot, item));
            stored_items.push(ItemStorageRef::new(
                *overlay_bag,
                *overlay_slot,
                item,
                template_cache.get(&item_entry),
            ));
        }

        // Preserve the late condition-evaluation point after every selected
        // inventory projection has been assembled.
        let limit_category = proto.as_ref().and_then(|proto| {
            self.item_limit_category_template_like_cpp(proto.item_limit_category)
        });
        let mut dest = Vec::new();
        let outcome = player.can_store_item_like_cpp(
            &mut dest,
            CanStoreItemArgs {
                bag,
                slot,
                entry: entry_id,
                count,
                proto: proto.as_ref(),
                source_item,
                source_is_not_empty_bag: source_item.is_some_and(|item| {
                    self.inventory.direct_item_contains_items_with_access_like_cpp(
                        &self.inventory_access,
                        item.object().guid(),
                    )
                }),
                source_bop_trade_allowed_for_player: false,
                swap,
                limit_category: limit_category.as_ref(),
                slot_items: &slot_items,
                stored_items: &stored_items,
                bag_templates: &bag_templates,
            },
        );

        Some((outcome.result, dest, outcome.no_space_count))
    }
}

impl PlayerConditionProjectionCxLikeCpp<'_> {
    /// Enter the shared App storage plan with selected owners and catalogs.
    pub fn plan_store_direct_inventory_item_like_cpp(
        &self,
        realm_id: u16,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
        source_item: Option<&Item>,
        swap: bool,
        overlays: &[wow_world_inventory::DirectInventoryStorageOverlayLikeCpp],
        vacated_positions: &[(u8, u8)],
    ) -> Option<(InventoryResult, Vec<wow_entities::ItemPosCount>, Option<u32>)> {
        let player = self.player_access_like_cpp();
        let application = InventoryValuationApplicationCxLikeCpp {
            inventory: self.inventory,
            player_conditions: self,
            catalogs: self.valuation_catalogs,
            inventory_access: player.owned_inventory_access_like_cpp(),
            valuation_access: player.inventory_valuation_access_like_cpp(),
            modifier_access: player.owned_item_modifiers_access_like_cpp(),
        };
        application.plan_store_direct_inventory_item_like_cpp(
            realm_id,
            entry_id,
            count,
            bag,
            slot,
            source_item,
            swap,
            overlays,
            vacated_positions,
        )
    }
}
