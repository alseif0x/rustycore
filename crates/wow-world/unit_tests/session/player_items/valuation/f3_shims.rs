// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

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
