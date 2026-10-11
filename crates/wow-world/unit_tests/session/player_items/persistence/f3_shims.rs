// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn begin_durable_item_loot_persistence_like_cpp(
        &self,
    ) -> DurableItemLootPersistenceGuardLikeCpp {
        crate::session::cx_inventory_ref(self).begin_durable_item_loot_persistence_like_cpp()
    }
}

impl WorldSession {
    pub(crate) fn set_stored_item_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>,
    ) {
        crate::session::cx_inventory(self).set_stored_item_money_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_item_template_addon_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>,
    ) {
        crate::session::cx_inventory(self)
            .set_item_template_addon_catalog_persistence_port_like_cpp(port)
    }
    pub(crate) fn send_item_time_update_plan(&self, update: &PlayerItemTimeUpdate) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_item_time_update_plan(hub, update)
    }
}

impl crate::session::InventoryCx<'_> {
    pub(crate) fn set_stored_item_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_stored_item_money_persistence_port_like_cpp(port);
    }
    pub(crate) fn set_item_template_addon_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_item_template_addon_catalog_persistence_port_like_cpp(port);
    }
}
