// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn item_valuation_catalogs_for_test_like_cpp(&self) -> ItemValuationCatalogsLikeCpp {
        self.catalogs.item_valuation_catalogs_for_test_like_cpp()
    }
    #[cfg(test)]
    pub fn item_buy_price_like_cpp(
        &self,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<(u32, bool)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.item_buy_price_like_cpp(hub, item_id, quality, item_level)
    }
    #[cfg(test)]
    pub fn item_sell_price_like_cpp(
        &self,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<u32> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.item_sell_price_like_cpp(hub, item_id, quality, item_level)
    }
    #[cfg(test)]
    pub(crate) fn represented_using_pvp_item_levels_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_using_pvp_item_levels_like_cpp(hub)
    }
}

impl WorldSession {
    pub(crate) fn set_represented_item_level_caps_like_cpp(
        &mut self,
        caps: RepresentedItemLevelCapsLikeCpp,
    ) -> bool {
        self.set_player_item_level_caps_like_cpp(caps)
    }
    pub(crate) fn set_represented_using_pvp_item_levels_like_cpp(&mut self, active: bool) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_represented_using_pvp_item_levels_like_cpp(&mut hub, active)
    }
    pub(in crate::session) fn resolved_using_pvp_item_levels_like_cpp(&self) -> Option<bool> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_using_pvp_item_levels_like_cpp(hub)
    }
    pub(crate) fn update_represented_item_level_area_based_scaling_like_cpp(&mut self) -> bool {
        wow_world_inventory::update_represented_item_level_area_based_scaling_with_publication_like_cpp(self, true)
            .unwrap_or(false)
    }
    pub(in crate::session) fn represented_item_level_like_cpp(
        &self,
        entry_id: u32,
        runtime_item: Option<&Item>,
    ) -> Option<u32> {
        let valuation_access = self.core.inventory_valuation_access_like_cpp();
        let modifier_access = self.core.owned_item_modifiers_access_like_cpp();
        let catalogs = self.catalogs.inventory_valuation_catalog_view_like_cpp();
        self.inventory.represented_item_level_with_access_like_cpp(
            &valuation_access,
            &modifier_access,
            &catalogs,
            entry_id,
            runtime_item,
            Self::MIN_ITEM_LEVEL_LIKE_CPP,
            Self::MAX_ITEM_LEVEL_LIKE_CPP,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
        )
    }
}
