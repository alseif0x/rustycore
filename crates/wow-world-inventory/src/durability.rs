// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashSet;

use wow_core::ObjectGuid;
use wow_entities::{
    is_equipment_packed_pos, make_item_pos, EQUIPMENT_SLOT_END, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_END,
    INVENTORY_SLOT_ITEM_START, INVENTORY_DEFAULT_SIZE, MAX_BAG_SIZE, PLAYER_SLOT_END,
};
use wow_world_core::session::HubRef;

impl crate::InventoryState {
    /// The `DurabilityLossAll`/`DurabilityPointsLossAll` target set.
    pub fn represented_durability_targets_like_cpp(
        &self,
        hub: HubRef<'_>,
        inventory: bool,
    ) -> Vec<(ObjectGuid, u8, bool)> {
        let Some(items) = self.resolved_inventory_item_objects_like_cpp(hub) else {
            return Vec::new();
        };
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(
                self.resolved_player_inventory_slot_count_like_cpp(hub)
                    .unwrap_or(0),
            )
            .min(INVENTORY_SLOT_ITEM_END);
        items
            .iter()
            .filter_map(|(guid, item)| {
                let slot = item.slot();
                if item.container_guid().is_empty() {
                    if is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)) {
                        return Some((*guid, slot, true));
                    }
                    if inventory && (INVENTORY_SLOT_ITEM_START..inventory_end).contains(&slot) {
                        return Some((*guid, slot, false));
                    }
                    return None;
                }
                inventory.then_some((*guid, slot, false))
            })
            .collect()
    }

    pub fn repairable_inventory_item_costs_like_cpp(
        &self,
        hub: HubRef<'_>,
        discount: f32,
        repair_cost_rate: f32,
    ) -> Option<Vec<(ObjectGuid, u64)>> {
        let mut repair_items = Vec::new();
        let item_objects = self.resolved_inventory_item_objects_like_cpp(hub)?;
        let inventory_items = self.resolved_inventory_items_like_cpp(hub)?;
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(PLAYER_SLOT_END as u8);

        for (&slot, inventory_item) in &inventory_items {
            if !((slot < EQUIPMENT_SLOT_END)
                || (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
                || (INVENTORY_SLOT_ITEM_START..inventory_end).contains(&slot))
            {
                continue;
            }
            let Some(item_object) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            let cost = hub.catalogs.item_durability_repair_cost_like_cpp(
                inventory_item.entry_id,
                item_object.data().durability,
                item_object.data().max_durability,
                discount,
                repair_cost_rate,
            );
            if cost != 0 {
                repair_items.push((inventory_item.guid, cost));
            }
        }

        let represented_bag_guids: HashSet<_> = inventory_items
            .iter()
            .filter_map(|(&slot, item)| {
                ((INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot))
                    .then_some(item.guid)
            })
            .collect();
        for item_object in item_objects.values() {
            if !represented_bag_guids.contains(&item_object.container_guid())
                || item_object.slot() as usize >= MAX_BAG_SIZE
            {
                continue;
            }
            let cost = hub.catalogs.item_durability_repair_cost_like_cpp(
                item_object.object().entry(),
                item_object.data().durability,
                item_object.data().max_durability,
                discount,
                repair_cost_rate,
            );
            if cost != 0 {
                repair_items.push((item_object.object().guid(), cost));
            }
        }

        Some(repair_items)
    }
}

impl crate::InventoryState {
    /// C++ `GetTotalAuraMultiplier(SPELL_AURA_MOD_DURABILITY_LOSS)`.
    pub fn represented_durability_loss_aura_multiplier_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> f32 {
        hub.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_DURABILITY_LOSS,
        )
        .unwrap_or_default()
        .into_iter()
        .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }

    /// C++ `HasAuraType(SPELL_AURA_PREVENT_DURABILITY_LOSS)`.
    pub fn represented_prevent_durability_loss_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        hub.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_PREVENT_DURABILITY_LOSS,
        )
        .is_some_and(|effects| !effects.is_empty())
    }

    /// C++ `Player::InBattleground` (`Player.h:2335`) read through the canonical
    /// Player's represented battleground state.
    #[must_use]
    pub fn represented_player_in_battleground_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        hub.core
            .with_owned_player_like_cpp(|player| {
                player
                    .battleground_state_like_cpp()
                    .in_battleground_like_cpp()
            })
            .unwrap_or(false)
    }
}
