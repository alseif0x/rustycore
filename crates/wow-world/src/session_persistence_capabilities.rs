//! Typed persistence capabilities installed by the application composition root.

use std::sync::Arc;

use crate::session::WorldSession;

impl WorldSession {
    pub fn set_vendor_trade_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.vendor_trade = Some(port);
    }

    pub(crate) fn vendor_trade_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.vendor_trade.clone()
    }

    pub fn set_player_inventory_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player_inventory = Some(port);
    }

    pub(crate) fn player_inventory_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player_inventory.clone()
    }

    pub fn set_stored_item_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.stored_item = Some(port);
    }

    pub(crate) fn stored_item_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.stored_item.clone()
    }

    pub fn set_character_administration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.character_administration = Some(port);
    }

    pub(crate) fn character_administration_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .character_administration
            .clone()
    }

    pub fn set_loot_template_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.loot_template_catalog = Some(port);
    }

    pub(crate) fn loot_template_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .loot_template_catalog
            .clone()
    }

    pub fn set_vendor_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.vendor_catalog = Some(port);
    }

    pub(crate) fn vendor_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.vendor_catalog.clone()
    }

    pub fn set_visibility_spawn_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.visibility_spawn_catalog = Some(port);
    }

    pub(crate) fn visibility_spawn_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .visibility_spawn_catalog
            .clone()
    }
}
