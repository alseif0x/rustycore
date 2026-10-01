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
    pub(crate) fn rebind_canonical_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.rebind_canonical_creature_loot_authority_like_cpp(
            hub,
            guid,
            expected,
            expected_stamp,
            authority,
        )
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
    pub(crate) fn rebind_canonical_gameobject_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.rebind_canonical_gameobject_loot_authority_like_cpp(
            hub,
            guid,
            expected,
            expected_stamp,
            authority,
        )
    }
    pub(crate) fn mutate_world_creature_if_fully_looted_observation_like_cpp<F, R>(
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
        state.mutate_world_creature_if_fully_looted_observation_like_cpp(
            &mut hub,
            guid,
            authority,
            object_generation,
            lifecycle_revision,
            f,
        )
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

impl crate::session::state::SessionWorldConfig {
    pub(crate) fn enable_ae_loot_like_cpp(&self) -> bool {
        self.enable_ae_loot_like_cpp
    }

    pub fn loot_drop_rates_like_cpp(&self) -> LootDropRatesLikeCpp {
        self.loot_drop_rates
    }
}

impl crate::session::state::SessionCatalogs {
    pub(in crate::session) fn represented_gameobject_loot_ids_have_quest_loot_like_cpp(
        &self,
        loot_ids: impl IntoIterator<Item = u32>,
    ) -> bool {
        let Some(stores) = self.loot_stores.as_ref() else {
            return false;
        };
        let Some(store) = stores.get(&LootStoreKind::Gameobject) else {
            return false;
        };
        loot_ids
            .into_iter()
            .filter(|id| *id != 0)
            .any(|loot_id| store.have_quest_loot_for_like_cpp(loot_id, stores.as_ref()))
    }

    /// Get the C++ LootTemplates_* foundation stores.
    pub fn loot_stores(&self) -> Option<&Arc<LootStores>> {
        self.loot_stores.as_ref()
    }
}

impl crate::session::state::LootState {
    pub(crate) fn represented_money_loot_with_rate_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        min_amount: u32,
        max_amount: u32,
        rate: f32,
    ) -> u32 {
        wow_loot::generate_money_loot_with_rate_like_cpp(
            min_amount,
            max_amount,
            rate,
            &mut hub.core.driver.represented_runtime_rng_like_cpp,
        )
    }

    pub(crate) fn set_canonical_gameobject_loot_state_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        guid: ObjectGuid,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        Some(managed.map_mut().set_gameobject_loot_state_like_cpp(
            guid,
            state,
            unit_guid,
            game_time_secs,
            chest_restock_time_secs,
            shared_loot_is_changed_like_cpp,
        ))
    }

    /// Applies the global fully-looted transition only if the exact authority
    /// generation and pool topology observed by `DoLootRelease` are still
    /// current. The canonical map lock is acquired before the authority lock,
    /// matching personal-loot upsert order and making check+state mutation one
    /// C++-serialized operation.
    pub(crate) fn set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let object_authority = managed
            .map()
            .get_typed_game_object(guid)?
            .loot_authority_like_cpp()
            .clone();
        if !object_authority.shares_storage_like_cpp(authority) {
            return None;
        }

        authority.with_fully_looted_lifecycle_observation_like_cpp(
            object_generation,
            lifecycle_revision,
            || {
                managed.map_mut().set_gameobject_loot_state_like_cpp(
                    guid,
                    state,
                    unit_guid,
                    game_time_secs,
                    chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        )
    }

    /// Detached durable-claim completion may transition the object only when
    /// no client still has any shared or personal loot pool open. The final
    /// viewer check and map mutation are serialized under the authority lock.
    pub(crate) fn set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let object_authority = managed
            .map()
            .get_typed_game_object(guid)?
            .loot_authority_like_cpp()
            .clone();
        if !object_authority.shares_storage_like_cpp(authority) {
            return None;
        }

        authority.with_unviewed_fully_looted_lifecycle_observation_like_cpp(
            object_generation,
            lifecycle_revision,
            || {
                managed.map_mut().set_gameobject_loot_state_like_cpp(
                    guid,
                    state,
                    unit_guid,
                    game_time_secs,
                    chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        )
    }

    pub(crate) fn read_legacy_creature_loot_authority_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        self.read_legacy_creature_loot_authority_on_map_like_cpp(
            hub,
            guid,
            wow_map::MapKey::new(u32::from(map_id), instance_id),
        )
    }

    pub(crate) fn read_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        let manager = hub.core.map_manager.as_ref()?;
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(map_id, map_key.instance_id, guid)
            .map(|world_creature| world_creature.creature.loot_authority_like_cpp().clone())
    }

    pub(crate) fn read_canonical_creature_loot_authority_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        self.read_canonical_creature_loot_authority_on_map_like_cpp(hub, guid, map_key)
    }

    pub(crate) fn read_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .with_creature_like_cpp(guid, |creature| creature.loot_authority_like_cpp().clone())
    }

    pub(crate) fn rebind_canonical_creature_loot_authority_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        self.rebind_canonical_creature_loot_authority_on_map_like_cpp(
            hub,
            guid,
            map_key,
            expected,
            expected_stamp,
            authority,
        )
    }

    pub(crate) fn rebind_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let mut manager = manager.lock().ok()?;
        manager
            .find_map_mut(map_key.map_id, map_key.instance_id)?
            .map_mut()
            .get_typed_creature_mut(guid)
            .and_then(|creature| {
                creature.rebind_loot_authority_if_current_like_cpp(
                    expected,
                    expected_stamp,
                    authority,
                )
            })
    }

    pub(crate) fn read_canonical_gameobject_loot_authority_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        self.read_canonical_gameobject_loot_authority_on_map_like_cpp(hub, guid, map_key)
    }

    pub(crate) fn read_canonical_gameobject_loot_authority_on_map_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .get_typed_game_object(guid)
            .map(|gameobject| gameobject.loot_authority_like_cpp().clone())
    }

    pub(crate) fn rebind_canonical_gameobject_loot_authority_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let mut manager = manager.lock().ok()?;
        manager
            .find_map_mut(map_key.map_id, map_key.instance_id)?
            .map_mut()
            .get_typed_game_object_mut(guid)
            .and_then(|gameobject| {
                gameobject.rebind_loot_authority_if_current_like_cpp(
                    expected,
                    expected_stamp,
                    authority,
                )
            })
    }

    pub(crate) fn mutate_world_creature_if_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        let manager = hub.core.map_manager.as_ref().cloned()?;
        let guarded_result = {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let creature = manager.find_creature_mut(map_id, instance_id, guid)?;
            if !creature
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(authority)
            {
                return None;
            }
            authority.with_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || {
                    let result = f(creature);
                    (result, creature.creature.clone())
                },
            )
        }?;
        let (result, creature) = guarded_result;
        hub.core.sync_canonical_creature_entity_like_cpp(creature);
        Some(result)
    }

    /// Detached durable-claim completion variant of the guarded creature
    /// mutation. It additionally requires every authoritative loot viewer set
    /// to remain empty through the map mutation.
    pub(crate) fn mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        let manager = hub.core.map_manager.as_ref().cloned()?;
        let guarded_result = {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let creature = manager.find_creature_mut(map_id, instance_id, guid)?;
            if !creature
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(authority)
            {
                return None;
            }
            authority.with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || {
                    let result = f(creature);
                    (result, creature.creature.clone())
                },
            )
        }?;
        let (result, creature) = guarded_result;
        hub.core.sync_canonical_creature_entity_like_cpp(creature);
        Some(result)
    }

    pub(crate) fn set_active_loot_guid(&mut self, guid: ObjectGuid) {
        self.active_loot_guid = ObjectGuid::EMPTY;
        self.active_loot_view_owners.clear();
        self.active_loot_view_generations_like_cpp.clear();
        self.active_loot_view_authorities_like_cpp.clear();
        self.add_active_loot_view_owner_like_cpp(guid);
    }

    pub(crate) fn has_active_loot_views_like_cpp(&self) -> bool {
        !self.active_loot_guid.is_empty() || !self.active_loot_view_owners.is_empty()
    }

    pub(crate) fn add_active_loot_view_owner_like_cpp(&mut self, guid: ObjectGuid) {
        if guid.is_empty() {
            return;
        }

        if self.active_loot_guid.is_empty() {
            self.active_loot_guid = guid;
        }

        self.active_loot_view_owners.insert(guid);
        if let Some(generation) = self
            .represented_loot_cache_generations_like_cpp
            .get(&guid)
            .copied()
        {
            self.active_loot_view_generations_like_cpp
                .insert(guid, generation);
        }
    }

    pub(crate) fn clear_active_loot_guid_if(&mut self, guid: ObjectGuid) {
        self.active_loot_view_owners.remove(&guid);
        self.active_loot_view_generations_like_cpp.remove(&guid);
        self.active_loot_view_authorities_like_cpp.remove(&guid);
        if self.active_loot_guid == guid {
            self.active_loot_guid = ObjectGuid::EMPTY;
        }
    }

    pub(crate) fn is_active_loot_guid(&self, guid: ObjectGuid) -> bool {
        !guid.is_empty() && self.active_loot_guid == guid
    }

    pub(crate) fn resolved_pass_on_group_loot_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<bool> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| player.pass_on_group_loot_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.pass_on_group_loot);
        }
        canonical
    }

    pub(crate) fn set_pass_on_group_loot_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        value: bool,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_pass_on_group_loot_like_cpp(value))
            .is_some();
        #[cfg(test)]
        if hub.core.player_handle_like_cpp.is_none() {
            self.pass_on_group_loot = value;
            return true;
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn pass_on_group_loot_like_cpp(&self, hub: crate::session::HubRef<'_>) -> bool {
        self.resolved_pass_on_group_loot_like_cpp(hub)
            .expect("test Player loot preference owner must resolve")
    }

    pub(crate) fn loot_specialization_id_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<u32> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(Player::loot_specialization_id_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.loot_specialization_id);
        }
        canonical
    }

    pub(crate) fn set_loot_specialization_id_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        spec_id: u32,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_loot_specialization_id_like_cpp(spec_id)
            })
            .is_some();
        #[cfg(test)]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.loot_specialization_id = spec_id;
            return true;
        }
        canonical
    }

    /// Revalidates the exact map ownership captured before a multi-lock loot
    /// authority reconciliation. If a canonical Player existed at capture
    /// time, fallback lookup is forbidden: disappearing or moving during the
    /// attempt must fail closed rather than mutate the old map.
    pub(crate) fn loot_reconciliation_map_key_still_valid_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        map_key: wow_map::MapKey,
        canonical_player_was_present: bool,
    ) -> bool {
        if canonical_player_was_present {
            return hub.core.current_canonical_player_map_key_like_cpp() == Some(map_key);
        }
        if hub.core.canonical_map_manager.is_some() {
            return hub
                .core
                .canonical_object_lookup_map_key_like_cpp(map_key.map_id)
                == Some(map_key);
        }
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        u32::from(map_id) == map_key.map_id && instance_id == map_key.instance_id
    }
}

impl crate::session::state::SessionCore {
    pub(crate) fn rebind_legacy_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        self.rebind_legacy_creature_loot_authority_on_map_like_cpp(
            guid,
            wow_map::MapKey::new(u32::from(map_id), instance_id),
            expected,
            expected_stamp,
            authority,
        )
    }

    pub(crate) fn rebind_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        let manager = self.map_manager.as_ref()?;
        manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature_mut(map_id, map_key.instance_id, guid)
            .and_then(|world_creature| {
                world_creature
                    .creature
                    .rebind_loot_authority_if_current_like_cpp(expected, expected_stamp, authority)
            })
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/loot/operations/f3_shims.rs"]
mod f3_shims;
