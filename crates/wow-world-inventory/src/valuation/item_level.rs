// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::{InventoryType, ItemFlags3, ItemModifier};
use wow_entities::{
    EQUIPMENT_SLOT_BODY, EQUIPMENT_SLOT_END, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND,
    EQUIPMENT_SLOT_RANGED, EQUIPMENT_SLOT_TABARD, Item,
};
use wow_world_core::session::{
    InventoryValuationAccessLikeCpp, InventoryValuationCatalogViewLikeCpp,
    OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp,
};

impl crate::InventoryState {
    pub fn represented_average_item_level_with_access_like_cpp(
        &self,
        inventory: &OwnedInventoryAccessLikeCpp<'_>,
        valuation: &InventoryValuationAccessLikeCpp<'_>,
        modifiers: &OwnedItemModifiersAccessLikeCpp<'_>,
        catalogs: &InventoryValuationCatalogViewLikeCpp<'_>,
        min_item_level: u32,
        max_item_level: u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_player_level: &u8,
    ) -> Option<f32> {
        let item_objects = self.resolved_inventory_item_objects_with_access_like_cpp(inventory)?;
        let inventory_items = self.resolved_inventory_items_with_access_like_cpp(inventory)?;
        let mut sum = 0.0f32;
        let mut count = 0u32;

        for slot in 0..EQUIPMENT_SLOT_END {
            if matches!(
                slot,
                EQUIPMENT_SLOT_TABARD
                    | EQUIPMENT_SLOT_RANGED
                    | EQUIPMENT_SLOT_OFFHAND
                    | EQUIPMENT_SLOT_BODY
            ) {
                continue;
            }

            if let Some(inventory_item) = inventory_items.get(&slot) {
                let runtime_item = item_objects.get(&inventory_item.guid);
                if let Some(item_level) = self.represented_item_level_with_access_like_cpp(
                    valuation,
                    modifiers,
                    catalogs,
                    inventory_item.entry_id,
                    runtime_item,
                    min_item_level,
                    max_item_level,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixture_player_level,
                ) {
                    sum += item_level as f32;
                }
            }

            count += 1;
        }

        Some(if count == 0 { 0.0 } else { sum / count as f32 })
    }

    pub fn represented_item_level_with_access_like_cpp(
        &self,
        access: &InventoryValuationAccessLikeCpp<'_>,
        modifiers: &OwnedItemModifiersAccessLikeCpp<'_>,
        catalogs: &InventoryValuationCatalogViewLikeCpp<'_>,
        entry_id: u32,
        runtime_item: Option<&Item>,
        min_item_level: u32,
        max_item_level: u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_player_level: &u8,
    ) -> Option<u32> {
        let using_pvp_item_levels =
            self.resolved_using_pvp_item_levels_with_access_like_cpp(access)?;
        let caps = self
            .player_item_modifier_runtime_snapshot_with_access_like_cpp(modifiers)?
            .item_level_caps_like_cpp();
        let item_stats_store = catalogs.item_stats_store_like_cpp()?;
        let random_property_template = item_stats_store.random_property_template(entry_id)?;
        let sparse_template = item_stats_store.sparse_template(entry_id);
        let template_item_level = i64::from(random_property_template.item_level);
        let runtime_item_level = runtime_item
            .map(|item| i64::from(item.data().debug_item_level))
            .filter(|level| *level != 0);
        let item_level = runtime_item_level.unwrap_or_else(|| {
            let mut item_level = sparse_template
                .and_then(|template| {
                    represented_player_level_curve_item_level_like_cpp(
                        access,
                        catalogs,
                        template,
                        runtime_item,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        fixture_player_level,
                    )
                })
                .unwrap_or(template_item_level);
            item_level += catalogs.represented_item_level_bonus_like_cpp(runtime_item);
            let item_level_before_upgrades = item_level;
            if using_pvp_item_levels {
                item_level +=
                    i64::from(catalogs.represented_pvp_item_level_bonus_like_cpp(entry_id));
            }

            let inventory_type = sparse_template
                .map(|template| template.inventory_type)
                .unwrap_or(random_property_template.inventory_type);
            let is_equipable =
                <InventoryType as num_traits::FromPrimitive>::from_i8(inventory_type)
                    .is_some_and(|inventory_type| inventory_type != InventoryType::NonEquip);
            if !is_equipable {
                return item_level;
            }

            if caps.min_item_level != 0
                && (caps.min_item_level_cutoff == 0
                    || item_level_before_upgrades >= i64::from(caps.min_item_level_cutoff))
                && item_level < i64::from(caps.min_item_level)
            {
                item_level = i64::from(caps.min_item_level);
            }

            let flags3 = sparse_template
                .map(|template| template.flags[2])
                .unwrap_or_default();
            let ignore_max_cap = (flags3 & ItemFlags3::IgnoreItemLevelCapInPvp as u32) != 0;
            if caps.max_item_level != 0
                && !ignore_max_cap
                && item_level > i64::from(caps.max_item_level)
            {
                item_level = i64::from(caps.max_item_level);
            }

            item_level
        });
        Some(
            item_level
                .clamp(i64::from(min_item_level), i64::from(max_item_level))
                .try_into()
                .expect("clamped item level fits u32"),
        )
    }

    pub fn represented_avg_equipped_item_level_with_access_like_cpp(
        &self,
        inventory: &OwnedInventoryAccessLikeCpp<'_>,
        valuation: &InventoryValuationAccessLikeCpp<'_>,
        modifiers: &OwnedItemModifiersAccessLikeCpp<'_>,
        catalogs: &InventoryValuationCatalogViewLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_player_level: &u8,
        min_item_level: u32,
        max_item_level: u32,
    ) -> Option<f32> {
        let (_, can_titan_grip) =
            self.inventory_equip_capabilities_with_access_like_cpp(valuation)?;
        let inventory_items = self.resolved_inventory_items_with_access_like_cpp(inventory)?;
        let mut total_item_level = 0u32;
        for slot in 0..EQUIPMENT_SLOT_END {
            let Some(inventory_item) = inventory_items.get(&slot) else {
                continue;
            };
            let runtime_item = self.resolved_player_inventory_item_object_with_access_like_cpp(
                inventory,
                inventory_item.guid,
            );
            let Some(item_level) = self.represented_item_level_with_access_like_cpp(
                valuation,
                modifiers,
                catalogs,
                inventory_item.entry_id,
                runtime_item.as_ref(),
                min_item_level,
                max_item_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_player_level,
            ) else {
                continue;
            };
            total_item_level = total_item_level.saturating_add(item_level);
            let is_mainhand_two_hand = slot == EQUIPMENT_SLOT_MAINHAND
                && !can_titan_grip
                && catalogs
                    .item_storage_template_like_cpp(inventory_item.entry_id)
                    .is_some_and(|item_template| {
                        item_template.inventory_type == InventoryType::Weapon2Hand
                    });
            if is_mainhand_two_hand {
                total_item_level = total_item_level.saturating_add(item_level);
            }
        }

        Some(total_item_level as f32 / 16.0)
    }
}

pub(super) fn represented_player_level_curve_item_level_like_cpp(
    access: &InventoryValuationAccessLikeCpp<'_>,
    catalogs: &InventoryValuationCatalogViewLikeCpp<'_>,
    template: &wow_data::item::stats::ItemSparseTemplateEntry,
    runtime_item: Option<&Item>,
    #[cfg(any(test, feature = "test-fixtures"))] fixture_player_level: &u8,
) -> Option<i64> {
    let curve_id = template.player_level_to_item_level_curve_id_like_cpp();
    if curve_id == 0 {
        return None;
    }

    let fixed_level = runtime_item
        .map(|item| item.get_modifier(ItemModifier::TimewalkerLevel))
        .unwrap_or(0);
    let mut level = if fixed_level != 0 {
        fixed_level
    } else {
        u32::from(access.player_level_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_player_level,
        ))
    };

    if fixed_level == 0
        && let Some(levels) = catalogs.content_tuning_store_like_cpp().and_then(|store| {
            store
                .content_tuning_data_like_cpp(template.scaling_stat_content_tuning_like_cpp(), true)
        })
    {
        let clamped = (level as i32).clamp(levels.min_level, levels.max_level);
        level = u32::try_from(clamped).unwrap_or(level);
    }

    let (Some(curve_store), Some(curve_point_store)) = (
        catalogs.curve_store_like_cpp(),
        catalogs.curve_point_store_like_cpp(),
    ) else {
        return Some(0);
    };
    let curve_value =
        curve_store.curve_value_at_like_cpp(curve_point_store, curve_id, level as f32);

    Some(curve_value as i64)
}
