// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use wow_data::{
    ItemStore, ScalingStatDistributionEntry, ScalingStatDistributionStore,
    ScalingStatValuesStore,
};
use wow_world_core::session::{HubRef, RepresentedScalingStatContextLikeCpp};

impl crate::InventoryState {
    pub fn find_free_backpack_slot_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u8> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.find_free_backpack_slot_for_equipment_set_like_cpp(&access)
    }
}

impl crate::InventoryState {
    pub fn represented_scaling_stat_context_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_entry: u32,
    ) -> Option<RepresentedScalingStatContextLikeCpp> {
        represented_scaling_stat_context_from_selected_inputs_like_cpp(
            hub.catalogs.items.store.as_ref(),
            hub.catalogs.scaling_stat_distribution_store.as_ref(),
            hub.catalogs.scaling_stat_values_store.as_ref(),
            item_entry,
            |distribution| {
                self.represented_scaling_stat_character_level_like_cpp(hub, distribution)
            },
        )
    }

    pub(crate) fn represented_scaling_stat_character_level_like_cpp(
        &self,
        hub: HubRef<'_>,
        distribution: &ScalingStatDistributionEntry,
    ) -> u32 {
        scaling_stat_character_level_like_cpp(distribution, || hub.player_level_like_cpp())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_total_stat_multipliers_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> [f32; 5] {
        hub.resolved_represented_total_stat_multipliers_like_cpp()
            .expect("test Player aura owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_total_stat_buff_multipliers_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> [f32; 5] {
        hub.resolved_represented_total_stat_buff_multipliers_like_cpp()
            .expect("test Player aura owner must resolve")
    }
}

pub(crate) fn represented_scaling_stat_context_from_selected_inputs_like_cpp(
    item_store: Option<&Arc<ItemStore>>,
    distribution_store: Option<&Arc<ScalingStatDistributionStore>>,
    values_store: Option<&Arc<ScalingStatValuesStore>>,
    item_entry: u32,
    character_level: impl FnOnce(&ScalingStatDistributionEntry) -> u32,
) -> Option<RepresentedScalingStatContextLikeCpp> {
    let item_store = item_store?;
    let scaling_stat_distribution_id = item_store.scaling_stat_distribution_id(item_entry);
    let scaling_stat_value = item_store.scaling_stat_value(item_entry);
    if scaling_stat_distribution_id == 0 || scaling_stat_value == 0 {
        return None;
    }
    let distribution_store = distribution_store?;
    let values_store = values_store?;
    let distribution = distribution_store.get(u32::from(scaling_stat_distribution_id))?;
    let character_level = character_level(distribution);
    let values = values_store.get_for_character_level_like_cpp(character_level)?;
    let mask = scaling_stat_value as u32;
    Some(RepresentedScalingStatContextLikeCpp {
        stat_id: distribution.stat_id,
        bonus: distribution.bonus,
        ssd_multiplier: values.ssd_multiplier_like_cpp(mask),
        spell_bonus: values.spell_bonus_like_cpp(mask),
        armor_mod: values.armor_mod_like_cpp(mask),
        dps_mod: values.dps_mod_like_cpp(mask),
        is_two_hand: values.is_two_hand_like_cpp(mask),
    })
}

pub(crate) fn scaling_stat_character_level_like_cpp(
    distribution: &ScalingStatDistributionEntry,
    player_level: impl FnOnce() -> u8,
) -> u32 {
    let min_level = u32::try_from(distribution.min_level).unwrap_or(0);
    let max_level = u32::try_from(distribution.max_level).unwrap_or(min_level);
    let (min_level, max_level) = if min_level <= max_level {
        (min_level, max_level)
    } else {
        (max_level, min_level)
    };
    u32::from(player_level()).clamp(min_level, max_level)
}
