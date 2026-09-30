//! ports for the existing persistence owner.

use super::*;

impl WorldSession {
    pub fn set_stored_item_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .persistence_ports_like_cpp
            .player
            .stored_item_money = Some(port);
    }
    pub(crate) fn stored_item_money_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>> {
        self.lifecycle
            .persistence_ports_like_cpp
            .player
            .stored_item_money
            .clone()
    }
    pub fn set_item_template_addon_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .persistence_ports_like_cpp
            .catalogs
            .item_template_addon_catalog = Some(port);
    }
    pub(crate) fn item_template_addon_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>> {
        self.lifecycle
            .persistence_ports_like_cpp
            .catalogs
            .item_template_addon_catalog
            .clone()
    }
}
