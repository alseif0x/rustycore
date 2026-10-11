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

    pub(crate) fn set_vendor_trade_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_vendor_trade_persistence_port_like_cpp(port)
    }

    pub(crate) fn set_player_inventory_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_player_inventory_persistence_port_like_cpp(port)
    }

    pub(crate) fn set_player_quest_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_player_quest_persistence_port_like_cpp(port)
    }

    pub(crate) fn set_player_quest_reward_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_player_quest_reward_persistence_port_like_cpp(port)
    }

    pub fn set_character_administration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_character_administration_persistence_port_like_cpp(port)
    }
}
