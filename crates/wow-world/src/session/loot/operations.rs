//! Represented loot operations owned at the Session boundary.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_money_loot_with_rate_like_cpp(
        &mut self,
        min_amount: u32,
        max_amount: u32,
        rate: f32,
    ) -> u32 {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.represented_money_loot_with_rate_like_cpp(&mut hub, min_amount, max_amount, rate)
    }
    pub(in crate::session) fn represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
        &self,
        loot_ids: impl IntoIterator<Item = u32>,
    ) -> bool {
        let owner = self.core.quest_objective_access_like_cpp();
        let inventory_access = self.core.owned_inventory_access_like_cpp();
        wow_world_application::represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
            &owner,
            &inventory_access,
            &self.catalogs,
            &self.inventory,
            &self.quest_state,
            loot_ids,
            cfg!(test),
        )
    }
    pub(crate) fn read_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.read_legacy_creature_loot_authority_on_map_like_cpp(hub, guid, map_key)
    }
    pub(crate) fn read_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.read_canonical_creature_loot_authority_on_map_like_cpp(hub, guid, map_key)
    }
    pub(crate) fn rebind_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.rebind_canonical_creature_loot_authority_on_map_like_cpp(
            hub,
            guid,
            map_key,
            expected,
            expected_stamp,
            authority,
        )
    }
    pub(crate) fn read_canonical_gameobject_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.read_canonical_gameobject_loot_authority_on_map_like_cpp(hub, guid, map_key)
    }
    pub(crate) fn mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp(
            &mut hub,
            guid,
            authority,
            object_generation,
            lifecycle_revision,
            f,
        )
    }
    pub fn set_group_loot_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>,
    ) {
        crate::session::cx_loot(self).set_group_loot_money_persistence_port_like_cpp(port)
    }
    pub fn set_loot_drop_rates_like_cpp(&mut self, rates: LootDropRatesLikeCpp) {
        self.config.loot_drop_rates = rates;
    }
    pub fn set_enable_ae_loot_like_cpp(&mut self, enabled: bool) {
        self.config.enable_ae_loot_like_cpp = enabled;
    }
    pub(crate) fn enable_ae_loot_like_cpp(&self) -> bool {
        self.config.enable_ae_loot_like_cpp()
    }
    pub fn loot_drop_rates_like_cpp(&self) -> LootDropRatesLikeCpp {
        self.config.loot_drop_rates_like_cpp()
    }
    /// Set the C++ LootTemplates_* foundation stores for this session.
    pub fn set_loot_stores(&mut self, stores: Arc<LootStores>) {
        self.catalogs.loot_stores = Some(stores);
    }
    pub fn loot_stores(&self) -> Option<&Arc<LootStores>> {
        self.catalogs.loot_stores()
    }
    pub(crate) fn resolved_pass_on_group_loot_like_cpp(&self) -> Option<bool> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.resolved_pass_on_group_loot_like_cpp(hub)
    }
    pub(crate) fn set_pass_on_group_loot_like_cpp(&mut self, value: bool) -> bool {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.set_pass_on_group_loot_like_cpp(&mut hub, value)
    }
    pub(in crate::session) async fn reconcile_durable_loot_money_before_save_like_cpp(
        &mut self,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut player = self.core.quest_reward_player_access_like_cpp(
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let mut player = self.core.quest_reward_player_access_like_cpp();
        wow_world_application::reconcile_durable_loot_money_before_save_like_cpp(
            &mut self.lifecycle,
            &mut self.inventory,
            &mut self.quest_state,
            &mut player,
        )
        .await
    }
    pub(crate) fn remove_auras_with_looting_interrupt_flags_like_cpp(&mut self) -> usize {
        self.remove_auras_with_interrupt_flags_like_cpp(
            SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
            0,
        )
    }
    pub(crate) fn set_loot_specialization_id_like_cpp(&mut self, spec_id: u32) -> bool {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.set_loot_specialization_id_like_cpp(&mut hub, spec_id)
    }
    pub(crate) fn loot_reconciliation_map_key_still_valid_like_cpp(
        &self,
        map_key: wow_map::MapKey,
        canonical_player_was_present: bool,
    ) -> bool {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.loot_reconciliation_map_key_still_valid_like_cpp(
            hub,
            map_key,
            canonical_player_was_present,
        )
    }
}

impl crate::session::LootCxRef<'_> {
    pub(crate) fn group_loot_money_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>> {
        self.lifecycle
            .group_loot_money_persistence_port_like_cpp()
            .cloned()
    }

    pub(crate) fn durable_loot_money_persistence_tracker_like_cpp(
        &self,
    ) -> Arc<DurableLootMoneyPersistenceTrackerLikeCpp> {
        Arc::clone(
            self.lifecycle
                .durable_loot_money_persistence_tracker_like_cpp(),
        )
    }
}

impl crate::session::LootCx<'_> {
    pub fn set_group_loot_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_group_loot_money_persistence_port_like_cpp(port);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/loot/operations/f3_shims.rs"]
mod f3_shims;
