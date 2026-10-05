// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::InventoryMovePlanningCxLikeCpp;
use std::collections::HashMap;
use wow_constants::{BagFamilyMask, InventoryResult};
use wow_entities::{
    BagTemplateRef, CanBankItemArgs, INVENTORY_SLOT_BAG_0, ItemPosCount, ItemSlotRef,
    ItemStorageRef,
};
use wow_world_inventory::is_represented_bag_slot;

impl InventoryMovePlanningCxLikeCpp<'_, '_> {
    pub fn plan_bank_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>)> {
        let inventory_item = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let source_item = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)?;
        let access = self
            .conditions
            .player_access_like_cpp()
            .owned_inventory_access_like_cpp();
        let inventory = self.conditions.inventory;
        let catalogs = self.conditions.valuation_catalogs;
        let player = inventory.direct_inventory_player_snapshot_with_access_like_cpp(
            &access,
            catalogs.item_store_like_cpp(),
            catalogs.item_stats_store_like_cpp(),
        )?;
        let proto = catalogs.item_storage_template_like_cpp(inventory_item.entry_id);
        let inventory_items = inventory.resolved_inventory_items_with_access_like_cpp(&access)?;
        let item_objects =
            inventory.resolved_inventory_item_objects_with_access_like_cpp(&access)?;
        let mut template_cache = HashMap::new();
        for item in item_objects.values() {
            let entry_id = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(entry) = template_cache.entry(entry_id)
                && let Some(template) = catalogs.item_storage_template_like_cpp(entry_id)
            {
                entry.insert(template);
            }
        }
        let mut represented_bag_slots_by_guid = HashMap::new();
        let mut bag_templates = Vec::new();
        for (&slot, item) in &inventory_items {
            if wow_entities::is_buyback_slot(slot) {
                continue;
            }
            if is_represented_bag_slot(slot) && item_objects.contains_key(&item.guid) {
                represented_bag_slots_by_guid.insert(item.guid, slot);
                if let Some(template) = template_cache.get(&item.entry_id)
                    && template.container_slots > 0
                {
                    bag_templates.push(BagTemplateRef::new(slot, template));
                }
            }
        }
        let mut slot_items = Vec::new();
        let mut stored_items = Vec::new();
        for (&slot, stored) in &inventory_items {
            if wow_entities::is_buyback_slot(slot) {
                continue;
            }
            let Some(item) = item_objects.get(&stored.guid) else {
                continue;
            };
            slot_items.push(ItemSlotRef::new(INVENTORY_SLOT_BAG_0, slot, item));
            stored_items.push(ItemStorageRef::new(
                INVENTORY_SLOT_BAG_0,
                slot,
                item,
                template_cache.get(&stored.entry_id),
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
            let entry_id = item.object().entry();
            slot_items.push(ItemSlotRef::new(bag_slot, item.slot(), item));
            stored_items.push(ItemStorageRef::new(
                bag_slot,
                item.slot(),
                item,
                template_cache.get(&entry_id),
            ));
        }
        let limit_category = proto.as_ref().and_then(|proto| {
            self.conditions
                .item_limit_category_template_like_cpp(proto.item_limit_category)
        });
        let can_use_result = crate::can_use_inventory_item_represented_with_loading_like_cpp(
            self.conditions,
            &inventory_item,
            Some(&source_item),
            true,
        );
        let mut dest = Vec::new();
        let result = player.can_bank_item_like_cpp(
            &mut dest,
            CanBankItemArgs {
                bag: destination_bag,
                slot: destination_slot,
                proto: proto.as_ref(),
                source_item: Some(&source_item),
                source_is_not_empty_bag: inventory
                    .direct_item_contains_items_with_access_like_cpp(&access, inventory_item.guid),
                source_is_bag: proto
                    .as_ref()
                    .is_some_and(|proto| proto.container_slots > 0),
                source_is_currency_token: proto
                    .as_ref()
                    .is_some_and(|proto| proto.bag_family.contains(BagFamilyMask::CURRENCY_TOKENS)),
                source_bop_trade_allowed_for_player: false,
                swap,
                can_use_result,
                limit_category: limit_category.as_ref(),
                slot_items: &slot_items,
                stored_items: &stored_items,
                bag_templates: &bag_templates,
            },
        );
        Some((result, dest))
    }
}
