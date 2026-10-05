// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::PlayerConditionProjectionCxLikeCpp;
use wow_constants::InventoryResult;
use wow_entities::{ItemPosCount, is_bank_pos, is_equipment_pos, is_inventory_pos};

mod bank;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySwapTargetLikeCpp {
    Inventory,
    Bank,
    Equipment { dest: u16 },
    None,
}

/// Inert, read-only participants for complete inventory destination planning.
pub struct InventoryMovePlanningCxLikeCpp<'cx, 'owner> {
    conditions: &'cx PlayerConditionProjectionCxLikeCpp<'owner>,
}

impl<'cx, 'owner> InventoryMovePlanningCxLikeCpp<'cx, 'owner> {
    pub fn new(conditions: &'cx PlayerConditionProjectionCxLikeCpp<'owner>) -> Self {
        Self { conditions }
    }

    fn get_inventory_item_by_pos(
        &self,
        bag: u8,
        slot: u8,
    ) -> Option<wow_entities::PlayerInventoryItem> {
        let access = self
            .conditions
            .player_access_like_cpp()
            .owned_inventory_access_like_cpp();
        self.conditions
            .inventory
            .get_inventory_item_by_pos_with_access_like_cpp(
                &access,
                self.conditions.valuation_catalogs.item_store_like_cpp(),
                self.conditions
                    .valuation_catalogs
                    .item_stats_store_like_cpp(),
                bag,
                slot,
            )
    }

    fn resolved_inventory_item_object_like_cpp(
        &self,
        guid: wow_core::ObjectGuid,
    ) -> Option<wow_entities::Item> {
        let access = self
            .conditions
            .player_access_like_cpp()
            .owned_inventory_access_like_cpp();
        self.conditions
            .inventory
            .resolved_player_inventory_item_object_with_access_like_cpp(&access, guid)
    }

    pub fn validate_inventory_swap_target_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
        require_exact_destination: bool,
    ) -> Option<(InventoryResult, InventorySwapTargetLikeCpp)> {
        let source = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let source_count = self
            .resolved_inventory_item_object_like_cpp(source.guid)?
            .count();
        let destination_pos = wow_entities::make_item_pos(destination_bag, destination_slot);
        if is_inventory_pos(destination_bag, destination_slot) {
            let (mut result, destinations, _) = self
                .plan_store_existing_inventory_item_at_like_cpp(
                    source_bag,
                    source_slot,
                    destination_bag,
                    destination_slot,
                    swap,
                )?;
            if require_exact_destination
                && result == InventoryResult::Ok
                && (destinations.len() != 1
                    || destinations[0].pos != destination_pos
                    || destinations[0].count != source_count)
            {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Inventory));
        }
        if is_bank_pos(destination_bag, destination_slot) {
            let (mut result, destinations) = self.plan_bank_existing_inventory_item_at_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                swap,
            )?;
            if require_exact_destination
                && result == InventoryResult::Ok
                && (destinations.len() != 1
                    || destinations[0].pos != destination_pos
                    || destinations[0].count != source_count)
            {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Bank));
        }
        if is_equipment_pos(destination_bag, destination_slot) {
            let (mut result, dest) = self.plan_equip_existing_inventory_item_like_cpp(
                source_bag,
                source_slot,
                destination_slot,
                swap,
            )?;
            if result == InventoryResult::Ok && dest != destination_pos {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Equipment { dest }));
        }
        Some((InventoryResult::Ok, InventorySwapTargetLikeCpp::None))
    }

    pub fn plan_store_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        let inventory_item = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let source_item = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)?;
        self.conditions.plan_store_direct_inventory_item_like_cpp(
            self.conditions
                .player_access_like_cpp()
                .inventory_valuation_access_like_cpp()
                .realm_id_like_cpp(),
            inventory_item.entry_id,
            source_item.count(),
            destination_bag,
            destination_slot,
            Some(&source_item),
            swap,
            &[],
            &[],
        )
    }

    pub fn plan_equip_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        requested_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, u16)> {
        let inventory_item = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let runtime_item = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)?;
        let valuation = self
            .conditions
            .player_access_like_cpp()
            .inventory_valuation_access_like_cpp();
        let (can_dual_wield, can_titan_grip) = self
            .conditions
            .inventory
            .inventory_equip_capabilities_with_access_like_cpp(&valuation)?;
        let is_in_combat = valuation.resolved_in_combat_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.conditions.in_combat,
        )?;
        let outcome = crate::can_equip_inventory_item_like_cpp(
            self.conditions,
            &inventory_item,
            &runtime_item,
            requested_slot,
            swap,
            true,
            is_in_combat,
            can_dual_wield,
            can_titan_grip,
        );
        Some((outcome.result, outcome.dest))
    }
}
