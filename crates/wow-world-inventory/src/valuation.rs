// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::ItemModifier;
use wow_entities::Item;
use wow_world_core::session::{HubMut, HubRef};

impl crate::InventoryState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn item_buy_price_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<(u32, bool)> {
        hub.catalogs.item_buy_price_with_catalogs_like_cpp(
            &hub.catalogs.item_valuation_catalogs_for_test_like_cpp(),
            item_id,
            quality,
            item_level,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn item_sell_price_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<u32> {
        hub.catalogs.item_sell_price_with_catalogs_like_cpp(
            &hub.catalogs.item_valuation_catalogs_for_test_like_cpp(),
            item_id,
            quality,
            item_level,
        )
    }

    pub fn set_represented_using_pvp_item_levels_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        active: bool,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.activate_pvp_item_levels_like_cpp(active)
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_using_pvp_item_levels_like_cpp = active;
            return true;
        }
        false
    }

    pub fn resolved_using_pvp_item_levels_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<bool> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().using_pvp_item_levels);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(self.represented_using_pvp_item_levels_like_cpp);
        }
        None
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_using_pvp_item_levels_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        self.resolved_using_pvp_item_levels_like_cpp(hub)
            .expect("test Player PvP item-level owner must resolve")
    }

    pub fn represented_player_level_curve_item_level_like_cpp(
        &self,
        hub: HubRef<'_>,
        template: &wow_data::item::stats::ItemSparseTemplateEntry,
        runtime_item: Option<&Item>,
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
            u32::from(hub.player_level_like_cpp())
        };

        if fixed_level == 0
            && let Some(levels) = hub
                .catalogs
                .content_tuning_store
                .as_ref()
                .and_then(|store| {
                    store.content_tuning_data_like_cpp(
                        template.scaling_stat_content_tuning_like_cpp(),
                        true,
                    )
                })
        {
            let clamped = (level as i32).clamp(levels.min_level, levels.max_level);
            level = u32::try_from(clamped).unwrap_or(level);
        }

        let Some((curve_store, curve_point_store)) = hub
            .catalogs
            .curve_store
            .as_ref()
            .zip(hub.catalogs.curve_point_store.as_ref())
        else {
            return Some(0);
        };
        let curve_value =
            curve_store.curve_value_at_like_cpp(curve_point_store, curve_id, level as f32);

        Some(curve_value as i64)
    }
}
