// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `session_persistence_capabilities` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
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
}
