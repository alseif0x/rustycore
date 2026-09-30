// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Inventory destruction plans for counted, zone-limited and conjured items.

use super::super::super::*;

impl Player {
    pub fn destroy_item_count_by_entry_plan(
        &self,
        item_entry: u32,
        count: u32,
        unequip_check: bool,
        inventory_slot_count: u8,
        items: &[DestroyItemCountItemRef<'_>],
    ) -> DestroyItemCountPlan {
        let mut plan = DestroyItemCountPlan {
            removed_count: 0,
            actions: Vec::new(),
        };
        if count == 0 {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            KEYRING_SLOT_START,
            KEYRING_SLOT_END,
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_bag_ranges(
            &mut plan,
            items,
            item_entry,
            count,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
            true,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_ITEM_START,
            BANK_SLOT_ITEM_END,
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_bag_ranges(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_BAG_START,
            BANK_SLOT_BAG_END,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_BAG_START,
            BANK_SLOT_BAG_END,
            true,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            CHILD_EQUIPMENT_SLOT_START,
            CHILD_EQUIPMENT_SLOT_END,
            false,
            unequip_check,
        );

        plan
    }

    pub fn destroy_zone_limited_item_plan(
        &self,
        inventory_slot_count: u8,
        items: &[DestroyFilteredItemRef],
    ) -> Vec<DestroyFilteredItemAction> {
        let mut actions = Vec::new();
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            KEYRING_SLOT_START,
            KEYRING_SLOT_END,
        );
        destroy_filtered_scan_bag_ranges(
            &mut actions,
            items,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
        );
        actions
    }

    pub fn destroy_conjured_items_plan(
        &self,
        inventory_slot_count: u8,
        items: &[DestroyFilteredItemRef],
    ) -> Vec<DestroyFilteredItemAction> {
        let mut actions = Vec::new();
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
        );
        destroy_filtered_scan_bag_ranges(
            &mut actions,
            items,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
        );
        actions
    }
}
