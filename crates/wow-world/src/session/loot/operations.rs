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
        wow_loot::generate_money_loot_with_rate_like_cpp(
            min_amount,
            max_amount,
            rate,
            &mut self.driver.represented_runtime_rng_like_cpp,
        )
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
    pub(crate) fn group_loot_money_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>> {
        self.lifecycle
            .persistence_ports_like_cpp
            .world
            .group_loot_money
            .clone()
    }
    pub fn set_loot_drop_rates_like_cpp(&mut self, rates: LootDropRatesLikeCpp) {
        self.loot_drop_rates = rates;
    }
    pub fn set_enable_ae_loot_like_cpp(&mut self, enabled: bool) {
        self.enable_ae_loot_like_cpp = enabled;
    }
    pub(crate) fn enable_ae_loot_like_cpp(&self) -> bool {
        self.enable_ae_loot_like_cpp
    }
    pub fn loot_drop_rates_like_cpp(&self) -> LootDropRatesLikeCpp {
        self.loot_drop_rates
    }
    pub(crate) fn set_active_loot_guid(&mut self, guid: ObjectGuid) {
        self.loot_views.set_primary(guid);
        if guid.is_empty() {
            return;
        }
        if let Some(generation) = self
            .represented_loot_cache_generations_like_cpp
            .get(&guid)
            .copied()
        {
            self.loot_views.record_generation(guid, generation);
        }
    }
    pub(crate) fn has_active_loot_views_like_cpp(&self) -> bool {
        self.loot_views.has_views()
    }
    pub(crate) fn add_active_loot_view_owner_like_cpp(&mut self, guid: ObjectGuid) {
        if guid.is_empty() {
            return;
        }

        self.loot_views.add_owner(guid);
        if let Some(generation) = self
            .represented_loot_cache_generations_like_cpp
            .get(&guid)
            .copied()
        {
            self.loot_views.record_generation(guid, generation);
        }
    }
    pub(crate) fn clear_active_loot_guid_if(&mut self, guid: ObjectGuid) {
        self.loot_views.remove_owner(guid);
    }
    pub(crate) fn is_active_loot_guid(&self, guid: ObjectGuid) -> bool {
        self.loot_views.is_primary(guid)
    }
    /// Set the C++ LootTemplates_* foundation stores for this session.
    pub fn set_loot_stores(&mut self, stores: Arc<LootStores>) {
        self.loot_stores = Some(stores);
    }
    /// Get the C++ LootTemplates_* foundation stores.
    pub fn loot_stores(&self) -> Option<&Arc<LootStores>> {
        self.loot_stores.as_ref()
    }
    pub(crate) fn resolved_pass_on_group_loot_like_cpp(&self) -> Option<bool> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.pass_on_group_loot_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.pass_on_group_loot);
        }
        canonical
    }
    pub(crate) fn set_pass_on_group_loot_like_cpp(&mut self, value: bool) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_pass_on_group_loot_like_cpp(value))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            self.pass_on_group_loot = value;
            return true;
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn pass_on_group_loot_like_cpp(&self) -> bool {
        self.resolved_pass_on_group_loot_like_cpp()
            .expect("test Player loot preference owner must resolve")
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
                self.enqueue_represented_quest_objective_progress_like_cpp(
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
    pub(crate) fn durable_loot_money_persistence_tracker_like_cpp(
        &self,
    ) -> Arc<DurableLootMoneyPersistenceTrackerLikeCpp> {
        Arc::clone(&self.lifecycle.durable_loot_money_persistence_like_cpp)
    }
    pub(in crate::session) fn represented_creature_has_loot_recipient_like_cpp(
        &self,
        creature_guid: wow_core::ObjectGuid,
    ) -> Option<bool> {
        match self.capture_creature_loot_owner(creature_guid) {
            wow_map::manager::CreatureLootAccess::Rejected(_) => return None,
            wow_map::manager::CreatureLootAccess::Ready(handle) => {
                let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
                return match manager.idle_creature_has_loot_recipient(&handle) {
                    wow_map::manager::CreatureLootAccess::Ready(value) => Some(value), _ => None,
                };
            }
            wow_map::manager::CreatureLootAccess::NoActor => {}
        }
        if let Some(manager) = self.map_manager.as_ref() {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) =
                manager.find_creature(self.player_map_id_like_cpp(), 0, creature_guid)
            {
                return Some(creature.creature.has_loot_recipient());
            }
        }

        let key = self
            .current_canonical_player_map_key_like_cpp()
            .unwrap_or(wow_map::MapKey::new(
                u32::from(self.player_map_id_like_cpp()),
                0,
            ));
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
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
    pub(crate) fn loot_specialization_id_like_cpp(&self) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(Player::loot_specialization_id_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.loot_specialization_id);
        }
        canonical
    }
    pub(crate) fn set_loot_specialization_id_like_cpp(&mut self, spec_id: u32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_loot_specialization_id_like_cpp(spec_id)
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
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
        map_key: wow_map::MapKey,
        canonical_player_was_present: bool,
    ) -> bool {
        if canonical_player_was_present {
            return self.current_canonical_player_map_key_like_cpp() == Some(map_key);
        }
        if self.canonical_map_manager.is_some() {
            return self.canonical_object_lookup_map_key_like_cpp(map_key.map_id) == Some(map_key);
        }
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        u32::from(map_id) == map_key.map_id && instance_id == map_key.instance_id
    }
}
