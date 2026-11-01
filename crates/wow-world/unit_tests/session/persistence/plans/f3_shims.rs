// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) async fn persist_player_gold_checked_like_cpp(
        &self,
        money: u64,
    ) -> Result<(), LootMoneyPersistenceErrorLikeCpp> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.persist_player_gold_checked_like_cpp(hub, money).await
    }
}

impl WorldSession {
    pub(crate) fn set_character_enumeration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_character_enumeration_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_packet_spoof_ban_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_packet_spoof_ban_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_void_storage_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_void_storage_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_social_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>,
    ) {
        self.lifecycle.set_social_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_map_corpse_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_map_corpse_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_represented_group_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_represented_group_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_support_bug_report_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_support_bug_report_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_gossip_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_gossip_catalog_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_player_name_query_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_player_name_query_persistence_port_like_cpp(port)
    }
    pub(crate) fn set_instance_lock_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_instance_lock_persistence_port_like_cpp(port)
    }
    pub(crate) async fn set_account_data_persisted_like_cpp(
        &mut self,
        data_type: u8,
        time: i64,
        data: String,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .set_account_data_persisted_like_cpp(&mut hub, data_type, time, data)
            .await
    }
}
