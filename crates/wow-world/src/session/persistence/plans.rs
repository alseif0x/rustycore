//! Persistence plans assembled at the Session boundary.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_character_enumeration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .admission
            .character_enumeration = Some(port);
    }
    pub(crate) fn character_enumeration_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .character_enumeration
            .clone()
    }
    pub fn set_packet_spoof_ban_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.admission.packet_spoof_ban = Some(port);
    }
    pub fn set_void_storage_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.void_storage = Some(port);
    }
    pub(crate) fn void_storage_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.void_storage.clone()
    }
    pub fn set_social_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.social = Some(port);
    }
    pub(crate) fn social_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.social.clone()
    }
    pub fn set_map_corpse_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.world.map_corpse = Some(port);
    }
    pub(crate) fn map_corpse_persistence_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.world.map_corpse.as_ref()
    }
    pub fn set_represented_group_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.world.represented_group = Some(port);
    }
    pub(crate) fn represented_group_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .world
            .represented_group
            .clone()
    }
    pub fn set_support_bug_report_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.admission.support_bug_report = Some(port);
    }
    pub(crate) fn support_bug_report_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .support_bug_report
            .clone()
    }
    pub fn set_gossip_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.catalogs.gossip_catalog = Some(port);
    }
    pub(crate) fn gossip_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .gossip_catalog
            .clone()
    }
    pub fn set_player_name_query_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.admission.player_name_query = Some(port);
    }
    pub(crate) fn player_name_query_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .player_name_query
            .clone()
    }
    pub fn set_instance_lock_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.instance_lock = Some(port);
    }
    pub(crate) fn instance_lock_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.instance_lock.clone()
    }
    /// Quarantine this session after an unreconcilable battle-pet purchase
    /// COMMIT, mirroring the #159 money-persistence indeterminate boundary:
    /// normal payout admission stays closed and the client must relog.
    pub(crate) fn quarantine_player_money_persistence_like_cpp(&mut self, reason: &'static str) {
        self.durable_loot_money_persistence_like_cpp
            .mark_indeterminate_like_cpp();
        self.kick(reason);
    }
    /// C++ `Player::AddCurrency(..., CurrencyGainSource::Vendor)` without aura gain bonuses.
    pub(crate) fn plan_add_currency_vendor_like_cpp(
        &self,
        currencies: &mut HashMap<u32, PlayerCurrency>,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        if amount == 0 {
            return Ok(None);
        }

        let Some(entry) = self
            .currency_types_store
            .as_ref()
            .and_then(|store| store.get(currency_id))
            .copied()
        else {
            return Err(());
        };

        let player_team = player_team_for_race_cpp(self.player_race_like_cpp());
        if (entry.is_alliance() && player_team != Team::Alliance)
            || (entry.is_horde() && player_team != Team::Horde)
        {
            return Err(());
        }

        if entry.award_condition_id != 0
            || entry.faction_id != 0
            || currency_id == CurrencyTypes::Azerite as u32
        {
            return Err(());
        }

        let currency = currencies.entry(currency_id).or_insert(PlayerCurrency {
            state: PlayerCurrencyState::New,
            quantity: 0,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        });

        let weekly_cap = entry.max_earnable_per_week;
        let mut applied = amount;
        if weekly_cap != 0 && currency.weekly_quantity.saturating_add(applied) > weekly_cap {
            applied = weekly_cap.saturating_sub(currency.weekly_quantity);
        }

        let max_quantity = currency_max_quantity_cpp(&entry, currency);
        if max_quantity != 0 && currency.quantity.saturating_add(applied) > max_quantity {
            applied = max_quantity.saturating_sub(currency.quantity);
        }

        if applied == 0 {
            return Ok(None);
        }

        if currency.state != PlayerCurrencyState::New {
            currency.state = PlayerCurrencyState::Changed;
        }
        currency.quantity = currency.quantity.saturating_add(applied);
        if weekly_cap != 0 {
            currency.weekly_quantity = currency.weekly_quantity.saturating_add(applied);
        }
        if entry.is_tracking_quantity() {
            currency.tracked_quantity = currency.tracked_quantity.saturating_add(applied);
        }
        if entry.has_total_earned() {
            currency.earned_quantity = currency.earned_quantity.saturating_add(applied);
        }

        let scaler = entry.scaler().max(1) as u32;
        let delta = PlayerCurrencyDelta {
            currency_id,
            quantity: currency.quantity,
            amount: applied,
            weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
                .then_some(currency.weekly_quantity),
            max_quantity: (max_quantity != 0).then_some(max_quantity),
            total_earned: entry.has_total_earned().then_some(currency.earned_quantity),
            suppress_chat_log: entry.is_suppressing_chat_log(false),
        };
        Ok(Some(delta))
    }
    /// Close detached-payout admission, wait for every previously admitted
    /// worker, apply its exact-once durable deltas, then acquire the character
    /// money mutation lock. Callers must derive old/new runtime values only
    /// after this returns and retain the guard through their DB COMMIT.
    pub(crate) async fn begin_exclusive_player_money_persistence_like_cpp(
        &mut self,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
        let tracker = Arc::clone(&self.durable_loot_money_persistence_like_cpp);
        let save_fence = tracker.close_admission_for_save_like_cpp();
        tracker.wait_until_idle_like_cpp().await;
        if !self
            .reconcile_durable_loot_money_before_save_like_cpp()
            .await
        {
            return None;
        }
        let mutation_lock = tracker.lock_money_mutation_like_cpp().await;
        Some(ExclusivePlayerMoneyPersistenceLikeCpp {
            _save_fence: save_fence,
            _mutation_lock: mutation_lock,
        })
    }
    /// Derive one runtime money change only after the shared payout barrier,
    /// persist it while admission and the mutation mutex remain held, then
    /// publish the runtime value. Criteria must be queued/drained by the caller
    /// after this returns so reward callbacks cannot re-enter under the fence.
    pub(crate) async fn mutate_and_persist_player_gold_exclusive_like_cpp<F>(
        &mut self,
        mutation: F,
    ) -> Option<(u64, u64)>
    where
        F: FnOnce(u64) -> u64,
    {
        let money_persistence = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await?;
        let guid = self.player_guid()?.counter() as u64;
        let old_money = self.resolved_player_money_like_cpp()?;
        let new_money = mutation(old_money);

        #[cfg(test)]
        if let Some(success) = self.loot_money_persistence_test_result_like_cpp {
            if !success {
                return None;
            }
            if !self.set_player_gold_like_cpp(new_money) {
                return None;
            }
            drop(money_persistence);
            return Some((old_money, new_money));
        }

        if old_money == new_money {
            drop(money_persistence);
            return Some((old_money, new_money));
        }

        let port = self.player_lifecycle_port_like_cpp().map(Arc::clone)?;
        let request = wow_persistence::PlayerMoneyTransactionRequestLikeCpp {
            player_guid: guid,
            money_after: new_money,
            durability_repairs: Vec::new(),
        };
        let money_persistence = self
            .await_exclusive_player_money_transaction_outcome_like_cpp(
                money_persistence,
                port.persist_money_transaction_like_cpp(request),
                old_money,
                new_money,
                "exclusive player-money mutation",
            )
            .await?;
        if !self.set_player_gold_like_cpp(new_money) {
            self.kick("canonical Player money owner became unavailable after durable COMMIT");
            return None;
        }
        drop(money_persistence);
        Some((old_money, new_money))
    }
    /// Persist an explicit player-money value and surface database failures to
    /// callers that must not expose a loot payout before it is durable.
    ///
    /// Unlike [`Self::save_player_gold`], this helper fails closed when there is
    /// no selected player or lifecycle port. Focused tests must opt into an
    /// explicit persistence result through the test seam below.
    pub(crate) async fn persist_player_gold_checked_like_cpp(
        &self,
        money: u64,
    ) -> Result<(), LootMoneyPersistenceErrorLikeCpp> {
        #[cfg(test)]
        if let Some(success) = self.loot_money_persistence_test_result_like_cpp {
            return success
                .then_some(())
                .ok_or(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase);
        }

        let guid = self
            .player_guid()
            .ok_or(LootMoneyPersistenceErrorLikeCpp::MissingPlayer)?;
        let port = self
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
            .ok_or(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase)?;
        match port
            .persist_money_write_like_cpp(wow_persistence::PlayerMoneyWriteRequestLikeCpp {
                player_guid: guid.counter() as u64,
                money,
            })
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => Ok(()),
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                Err(LootMoneyPersistenceErrorLikeCpp::Persistence(reason))
            }
        }
    }
    fn represented_talent_reset_state_plan_like_cpp(
        &self,
    ) -> Option<RepresentedTalentResetStatePlanLikeCpp> {
        let runtime = self.player_talent_runtime_snapshot_like_cpp()?;
        if !runtime.talents_loaded_like_cpp() {
            return None;
        }

        let active_group = runtime.active_group_like_cpp();
        let active_group_index = usize::from(active_group);
        let active_talents = runtime.talent_group_like_cpp(active_group)?.clone();
        let mut post_talents = runtime.talent_groups_snapshot_like_cpp();
        post_talents[active_group_index].clear();

        Some(RepresentedTalentResetStatePlanLikeCpp {
            active_group,
            active_talents,
            post_talents,
        })
    }
    /// Build the represented durable talent-reset request without mutating the
    /// session. Statement identity, transaction construction and ambiguous
    /// COMMIT reconciliation belong to the lifecycle adapter.
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
        let state_plan = self.represented_talent_reset_state_plan_like_cpp()?;
        let mut retained_talents = Vec::new();

        for (talent_group, talents) in state_plan.post_talents.iter().enumerate() {
            for (talent_id, rank) in talents {
                if self
                    .represented_talent_info_like_cpp(*talent_id, *rank)
                    .is_none()
                {
                    continue;
                }
                retained_talents.push(wow_persistence::PlayerTalentResetSaveRowLikeCpp {
                    talent_id: *talent_id,
                    rank: *rank,
                    talent_group: talent_group as u8,
                });
            }
        }

        debug_assert_eq!(old_money.saturating_sub(new_money), u64::from(cost));
        Some((
            state_plan,
            wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp {
                player_guid: guid_counter,
                money_before: old_money,
                money_after: new_money,
                reset_cost: cost,
                reset_time_secs,
                retained_talents,
            },
        ))
    }
    pub async fn set_account_data_persisted_like_cpp(
        &mut self,
        data_type: u8,
        time: i64,
        data: String,
    ) -> bool {
        if usize::from(data_type) >= NUM_ACCOUNT_DATA_TYPES {
            return false;
        }

        let is_global = (1u32 << data_type) & GLOBAL_CACHE_MASK_LIKE_CPP != 0;
        let player_guid_low = self.recent_player_guid_low_like_cpp;

        if !is_global && player_guid_low == 0 {
            return false;
        }

        let scope = if is_global {
            wow_persistence::SessionAccountDataScopeLikeCpp::Global {
                account_id: self.account_id,
            }
        } else {
            wow_persistence::SessionAccountDataScopeLikeCpp::Character {
                guid_low: player_guid_low,
            }
        };

        let Some(port) = self
            .persistence_ports_like_cpp
            .admission
            .session_account_state
            .clone()
        else {
            warn!(
                account = self.account_id,
                data_type, "SetAccountData persisted fallback: account-state port unavailable"
            );
            return self.set_account_data_like_cpp(data_type, time, data);
        };

        let save = wow_persistence::SessionAccountDataSaveLikeCpp {
            scope,
            data_type,
            time,
            data: data.clone(),
        };
        match port.save_account_data_like_cpp(save).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id,
                    data_type, "SetAccountData persistence failed: {reason}"
                );
                return false;
            }
        }

        self.set_account_data_like_cpp(data_type, time, data)
    }
    pub(in crate::session) fn player_persistent_capability_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerPersistentCapabilityStateLikeCpp> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.gameplay_state().persistent_capabilities);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerPersistentCapabilityStateLikeCpp {
                at_login_flags: self.represented_at_login_flags_like_cpp,
                weapon_proficiency: self.represented_weapon_proficiency_like_cpp,
                armor_proficiency: self.represented_armor_proficiency_like_cpp,
            });
        }
        canonical
    }
    pub(in crate::session) fn mutate_player_persistent_capability_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerPersistentCapabilityStateLikeCpp) -> R,
    ) -> Option<R> {
        let mut state = self.player_persistent_capability_state_snapshot_like_cpp()?;
        let result = mutate(&mut state);
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.install_persistent_capabilities_like_cpp(state);
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_at_login_flags_like_cpp = state.at_login_flags;
            self.represented_weapon_proficiency_like_cpp = state.weapon_proficiency;
            self.represented_armor_proficiency_like_cpp = state.armor_proficiency;
            return Some(result);
        }
        canonical.then_some(result)
    }
}
