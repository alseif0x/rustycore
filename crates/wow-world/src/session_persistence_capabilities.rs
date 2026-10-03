//! Typed persistence capabilities installed by the application composition root.

use std::sync::Arc;

use crate::session::{SessionPersistencePortsLikeCpp, WorldSession};

impl WorldSession {
    /// Install the complete production persistence graph atomically.
    pub fn set_required_persistence_capabilities_like_cpp(
        &mut self,
        capabilities: SessionPersistencePortsLikeCpp,
    ) {
        self.lifecycle
            .set_required_persistence_capabilities_like_cpp(capabilities);
    }

    pub fn set_vendor_trade_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_vendor_trade_persistence_port_like_cpp(port)
    }

    pub fn set_player_inventory_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_player_inventory_persistence_port_like_cpp(port)
    }

    pub fn set_player_quest_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_player_quest_persistence_port_like_cpp(port)
    }

    pub fn set_player_quest_reward_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_player_quest_reward_persistence_port_like_cpp(port)
    }

    pub fn set_stored_item_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>,
    ) {
        self.lifecycle.set_stored_item_persistence_port_like_cpp(port);
    }

    pub fn set_character_administration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_character_administration_persistence_port_like_cpp(port)
    }

    pub fn set_loot_template_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_loot_template_catalog_persistence_port_like_cpp(port);
    }

    pub fn set_vendor_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_vendor_catalog_persistence_port_like_cpp(port);
    }

    pub fn set_visibility_spawn_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_visibility_spawn_catalog_persistence_port_like_cpp(port);
    }
}
