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
        let Some(stores) = self.catalogs.loot_stores.as_ref() else {
            return false;
        };
        let Some(store) = stores.get(&LootStoreKind::Gameobject) else {
            return false;
        };
        loot_ids.into_iter().filter(|id| *id != 0).any(|loot_id| {
            store.have_quest_loot_for_player_like_cpp(loot_id, stores.as_ref(), |item_id| {
                self.represented_player_has_quest_for_loot_item_like_cpp(item_id)
            })
        })
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
        let tracker = Arc::clone(&self.lifecycle.durable_loot_money_persistence_like_cpp);
        tracker.wait_until_idle_like_cpp().await;
        let completions = tracker.pending_completions_like_cpp();
        for completion in completions {
            if completion
                .applied
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            let Some(old_money) = self.resolved_player_money_like_cpp() else {
                self.kick(
                    "canonical Player money owner is unavailable during durable reconciliation",
                );
                return false;
            };
            let new_money = old_money
                .checked_add(completion.durable_applied_amount)
                .filter(|money| *money <= MAX_MONEY_AMOUNT)
                .unwrap_or(old_money);
            if !self.set_player_gold_like_cpp(new_money) {
                self.kick(
                    "canonical Player money owner became unavailable during durable reconciliation",
                );
                return false;
            }
            if old_money != new_money {
                self.quest_state
                    .enqueue_represented_quest_objective_progress_like_cpp(
                        RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                            old_money,
                            new_money,
                        },
                    );
            }
        }

        // Do not drain money criteria while the save fence is held. That path
        // can reward a quest and re-enter `save_player_gold`, which would wait
        // on this same fence. Queue the exact transition here; normal command
        // publication or the save caller drains it only after releasing the
        // fence. This keeps a save-first completion from losing MoneyChanged.

        if tracker.is_indeterminate_like_cpp() {
            self.kick("loot-money COMMIT outcome is unknown; skipping absolute money save");
            return false;
        }
        true
    }
    pub(in crate::session) fn represented_creature_has_loot_recipient_like_cpp(
        &self,
        creature_guid: wow_core::ObjectGuid,
    ) -> Option<bool> {
        if let Some(manager) = self.core.map_manager.as_ref() {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) =
                manager.find_creature(self.core.player_map_id_like_cpp(), 0, creature_guid)
            {
                return Some(creature.creature.has_loot_recipient());
            }
        }

        let key = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .unwrap_or(wow_map::MapKey::new(
                u32::from(self.core.player_map_id_like_cpp()),
                0,
            ));
        let manager = self.core.canonical_map_manager.as_ref()?.lock().ok()?;
        manager
            .find_map(key.map_id, key.instance_id)?
            .map()
            .with_creature_like_cpp(creature_guid, |creature| creature.has_loot_recipient())
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
            .persistence_ports_like_cpp
            .world
            .group_loot_money
            .clone()
    }

    pub(crate) fn durable_loot_money_persistence_tracker_like_cpp(
        &self,
    ) -> Arc<DurableLootMoneyPersistenceTrackerLikeCpp> {
        Arc::clone(&self.lifecycle.durable_loot_money_persistence_like_cpp)
    }
}

impl crate::session::LootCx<'_> {
    pub(crate) fn canonical_gameobject_is_fully_looted_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<bool> {
        self.world_entities
            .mutate_canonical_gameobject_by_guid_like_cpp(&mut self.hub, guid, |gameobject| {
                gameobject.is_fully_looted_like_cpp()
            })
    }

    pub fn set_group_loot_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .persistence_ports_like_cpp
            .world
            .group_loot_money = Some(port);
    }
}


#[cfg(test)]
#[path = "../../../unit_tests/session/loot/operations/f3_shims.rs"]
mod f3_shims;
