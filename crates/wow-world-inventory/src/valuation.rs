// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::Item;
use wow_world_core::session::{
    HubMut, HubRef, InventoryValuationAccessLikeCpp, OwnedItemModifiersAccessLikeCpp,
};

mod item_level;

impl crate::InventoryState {
    pub fn set_player_item_level_caps_with_access_like_cpp(
        &mut self,
        access: &OwnedItemModifiersAccessLikeCpp<'_>,
        caps: wow_entities::PlayerItemLevelCapsLikeCpp,
        consumer_test: bool,
    ) -> bool {
        let canonical = access.set_item_level_caps_like_cpp(caps);
        if canonical.is_some() {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test && access.owner_handle_absent_like_cpp() {
            self.set_player_item_level_caps_for_test_like_cpp(caps);
            return true;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;
        false
    }

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
        let access = hub.core.inventory_valuation_access_like_cpp();
        self.set_represented_using_pvp_item_levels_with_access_like_cpp(&access, active)
    }

    pub fn set_represented_using_pvp_item_levels_with_access_like_cpp(
        &mut self,
        access: &InventoryValuationAccessLikeCpp<'_>,
        active: bool,
    ) -> bool {
        let canonical = access.activate_pvp_item_levels_like_cpp(active);
        if canonical {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            self.represented_using_pvp_item_levels_like_cpp = active;
            return true;
        }
        false
    }

    pub fn resolved_using_pvp_item_levels_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<bool> {
        let access = hub.core.inventory_valuation_access_like_cpp();
        self.resolved_using_pvp_item_levels_with_access_like_cpp(&access)
    }

    pub fn resolved_using_pvp_item_levels_with_access_like_cpp(
        &self,
        access: &InventoryValuationAccessLikeCpp<'_>,
    ) -> Option<bool> {
        let canonical = access.using_pvp_item_levels_like_cpp();
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
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
        let access = hub.core.inventory_valuation_access_like_cpp();
        let catalogs = hub.catalogs.inventory_valuation_catalog_view_like_cpp();
        item_level::represented_player_level_curve_item_level_like_cpp(
            &access,
            &catalogs,
            template,
            runtime_item,
            #[cfg(any(test, feature = "test-fixtures"))]
            &hub.fixtures.identity.player_level,
        )
    }
}
