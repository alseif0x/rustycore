//! Persistence plans assembled at the Session boundary.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

pub(crate) fn player_homebind_update_request_like_cpp(
    homebind: RepresentedHomebindLikeCpp,
    guid_counter: u64,
) -> wow_persistence::PlayerHomebindPersistenceRequestLikeCpp {
    wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
        player_guid: guid_counter,
        map_id: homebind.map_id,
        area_id: homebind.area_id,
        x: homebind.position.x,
        y: homebind.position.y,
        z: homebind.position.z,
        orientation: homebind.position.orientation,
    }
}

impl WorldSession {
    pub fn set_character_enumeration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_character_enumeration_persistence_port_like_cpp(port)
    }
    pub fn set_packet_spoof_ban_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_packet_spoof_ban_persistence_port_like_cpp(port)
    }
    pub fn set_void_storage_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_void_storage_persistence_port_like_cpp(port)
    }
    pub fn set_social_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>,
    ) {
        self.lifecycle.set_social_persistence_port_like_cpp(port)
    }
    pub fn set_map_corpse_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_map_corpse_persistence_port_like_cpp(port)
    }
    pub fn set_represented_group_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_represented_group_persistence_port_like_cpp(port)
    }
    pub fn set_support_bug_report_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_support_bug_report_persistence_port_like_cpp(port)
    }
    pub fn set_gossip_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_gossip_catalog_persistence_port_like_cpp(port)
    }
    pub fn set_player_name_query_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_player_name_query_persistence_port_like_cpp(port)
    }
    pub fn set_instance_lock_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_instance_lock_persistence_port_like_cpp(port)
    }
    pub(crate) fn plan_add_currency_vendor_like_cpp(
        &self,
        currencies: &mut HashMap<u32, PlayerCurrency>,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.plan_add_currency_vendor_like_cpp(hub, currencies, currency_id, amount)
    }
    /// Close detached-payout admission, wait for every previously admitted
    /// worker, apply its exact-once durable deltas, then acquire the character
    /// money mutation lock. Callers must derive old/new runtime values only
    /// after this returns and retain the guard through their DB COMMIT.
    pub(crate) async fn begin_exclusive_player_money_persistence_like_cpp(
        &mut self,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut player = self.core.quest_reward_player_access_like_cpp(
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let mut player = self.core.quest_reward_player_access_like_cpp();
        wow_world_application::begin_exclusive_player_money_persistence_like_cpp(
            &mut self.lifecycle,
            &mut self.inventory,
            &mut self.quest_state,
            &mut player,
        )
        .await
    }
    /// Derive one runtime money change only after the shared payout barrier,
    /// persist it while admission and the mutation mutex remain held, then
    /// publish the runtime value. Criteria must be queued/drained by the caller
    /// after this returns so reward callbacks cannot re-enter under the fence.
    ///
    /// World-owned construction seam (#1263 F4): the coordinator itself lives
    /// beside the application money-fence operation
    /// (`wow_world_application::mutate_and_persist_player_gold_exclusive_like_cpp`);
    /// this method only builds the selected Player/Inventory/Lifecycle/Quest
    /// participants the repo already uses.
    pub(crate) async fn mutate_and_persist_player_gold_exclusive_like_cpp<F>(
        &mut self,
        mutation: F,
    ) -> Option<(u64, u64)>
    where
        F: FnOnce(u64) -> u64,
    {
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut player = self.core.quest_reward_player_access_like_cpp(
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let mut player = self.core.quest_reward_player_access_like_cpp();
        wow_world_application::mutate_and_persist_player_gold_exclusive_like_cpp(
            &mut self.lifecycle,
            &mut self.inventory,
            &mut self.quest_state,
            &mut player,
            mutation,
        )
        .await
    }
    pub(in crate::session) fn represented_talent_reset_persistence_plan_like_cpp(
        &self,
        guid_counter: u64,
        old_money: u64,
        new_money: u64,
        cost: u32,
        reset_time_secs: u64,
    ) -> Option<(
        RepresentedTalentResetStatePlanLikeCpp,
        wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp,
    )> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.represented_talent_reset_persistence_plan_like_cpp(
            hub,
            guid_counter,
            old_money,
            new_money,
            cost,
            reset_time_secs,
        )
    }
    pub async fn set_account_data_persisted_like_cpp(
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
    pub(in crate::session) fn player_persistent_capability_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerPersistentCapabilityStateLikeCpp> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.persistent_capability_state_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerPersistentCapabilityStateLikeCpp {
                at_login_flags: self
                    .lifecycle
                    .represented_at_login_flags_for_test_like_cpp(),
                weapon_proficiency: self
                    .fixtures
                    .progression
                    .represented_weapon_proficiency_like_cpp,
                armor_proficiency: self
                    .fixtures
                    .progression
                    .represented_armor_proficiency_like_cpp,
            });
        }
        canonical
    }
    #[cfg(test)]
    pub(in crate::session) fn mutate_player_persistent_capability_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerPersistentCapabilityStateLikeCpp) -> R,
    ) -> Option<R> {
        let mut state = self.player_persistent_capability_state_snapshot_like_cpp()?;
        let result = mutate(&mut state);
        self.lifecycle
            .set_represented_at_login_flags_for_test_like_cpp(state.at_login_flags);
        self.fixtures
            .progression
            .represented_weapon_proficiency_like_cpp = state.weapon_proficiency;
        self.fixtures
            .progression
            .represented_armor_proficiency_like_cpp = state.armor_proficiency;
        Some(result)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/persistence/plans/f3_shims.rs"]
mod f3_shims;
