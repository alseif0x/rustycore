use std::sync::Arc;

use super::SessionLifecycleState;
use crate::SessionPersistencePortsLikeCpp;

impl SessionLifecycleState {
    pub fn set_required_persistence_capabilities_like_cpp(
        &mut self,
        capabilities: SessionPersistencePortsLikeCpp,
    ) {
        self.persistence_ports_like_cpp = Box::new(capabilities);
    }

    pub fn set_vendor_trade_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.vendor_trade = Some(port);
    }

    pub fn vendor_trade_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.vendor_trade.clone()
    }

    pub fn set_player_inventory_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_inventory = Some(port);
    }

    pub fn player_inventory_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_inventory
            .clone()
    }

    pub fn set_player_quest_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_quest = Some(port);
    }

    pub fn player_quest_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.player_quest.clone()
    }

    pub fn set_player_quest_reward_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_quest_reward = Some(port);
    }

    pub fn player_quest_reward_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_quest_reward
            .clone()
    }

    pub fn stored_item_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.stored_item.clone()
    }

    pub fn set_stored_item_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.stored_item = Some(port);
    }

    pub fn set_character_administration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .admission
            .character_administration = Some(port);
    }

    pub fn character_administration_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .character_administration
            .clone()
    }

    pub fn loot_template_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .loot_template_catalog
            .clone()
    }

    pub fn set_loot_template_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .catalogs
            .loot_template_catalog = Some(port);
    }

    pub fn vendor_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .vendor_catalog
            .clone()
    }

    pub fn set_vendor_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.catalogs.vendor_catalog = Some(port);
    }

    pub fn visibility_spawn_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .visibility_spawn_catalog
            .clone()
    }

    pub fn set_visibility_spawn_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .catalogs
            .visibility_spawn_catalog = Some(port);
    }

    pub fn set_player_lifecycle_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_lifecycle = Some(port);
    }

    pub fn player_lifecycle_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_lifecycle
            .as_ref()
    }
}
