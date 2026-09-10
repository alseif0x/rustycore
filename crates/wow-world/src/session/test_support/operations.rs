//! Test-only Session fixtures and accessors.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn initial_player_fixture_like_cpp(&self) -> Option<Player> {
        self.build_initial_player_for_owner_like_cpp(
            wow_map::MapKey::new(u32::from(self.player_map_id_like_cpp()), 0),
            None,
        )
    }
    #[cfg(test)]
    pub(crate) fn adopt_registered_canonical_player_fixture_like_cpp(&mut self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Ok(handle) = manager.adopt_active_player_like_cpp(guid) else {
            return false;
        };
        drop(manager);
        self.player_handle_like_cpp = Some(handle);
        self.with_owned_player_like_cpp(Player::guid) == Some(guid)
    }
    #[cfg(test)]
    pub(crate) fn id_generators_for_test_like_cpp(&self) -> SessionIdGeneratorsLikeCpp {
        let defaults = SessionIdGeneratorsLikeCpp::default();
        SessionIdGeneratorsLikeCpp {
            player: self.guid_generator.clone().unwrap_or(defaults.player),
            item: self
                .item_guid_generator_like_cpp
                .clone()
                .unwrap_or(defaults.item),
            equipment_set: self
                .equipment_set_guid_generator_like_cpp
                .clone()
                .unwrap_or(defaults.equipment_set),
            void_storage_item: self
                .void_storage_item_id_generator_like_cpp
                .clone()
                .unwrap_or(defaults.void_storage_item),
        }
    }
    #[cfg(test)]
    pub(crate) fn emotes_store_for_test_like_cpp(&self) -> Option<&Arc<EmotesStore>> {
        self.emotes_store.as_ref()
    }
    #[cfg(test)]
    pub(crate) fn emotes_text_store_for_test_like_cpp(&self) -> Option<&Arc<EmotesTextStore>> {
        self.emotes_text_store.as_ref()
    }
    #[cfg(test)]
    pub(crate) fn player_bootstrap_catalogs_for_test_like_cpp(
        &self,
    ) -> PlayerBootstrapCatalogsLikeCpp {
        let mut catalogs = PlayerBootstrapCatalogsLikeCpp::default();
        if let Some(store) = &self.player_create_info_store_like_cpp {
            catalogs.create_info = Arc::clone(store);
        }
        if let Some(store) = &self.player_create_cast_spell_store_like_cpp {
            catalogs.cast_spells = Arc::clone(store);
        }
        if let Some(store) = &self.player_create_custom_spell_store_like_cpp {
            catalogs.custom_spells = Arc::clone(store);
        }
        catalogs.start_all_spells = self.start_all_spells_like_cpp;
        catalogs.start_all_explored = self.start_all_explored_like_cpp;
        catalogs.start_all_reputation = self.start_all_reputation_like_cpp;
        catalogs
    }
    #[cfg(test)]
    pub(crate) fn player_rest_rate_policy_for_test_like_cpp(&self) -> PlayerRestRatePolicyLikeCpp {
        PlayerRestRatePolicyLikeCpp {
            offline_wilderness: self.rest_offline_wilderness_rate_like_cpp,
            offline_tavern_or_city: self.rest_offline_tavern_or_city_rate_like_cpp,
            ingame: self.rest_ingame_rate_like_cpp,
        }
    }
    #[cfg(test)]
    pub(crate) fn chat_policy_catalogs_for_test_like_cpp(&self) -> ChatPolicyCatalogsLikeCpp {
        ChatPolicyCatalogsLikeCpp {
            addon_channel: self.addon_channel_like_cpp,
            fake_message_preventing: self.chat_fake_message_preventing_like_cpp,
            strict_link_checking_kick: self.chat_strict_link_checking_kick_like_cpp,
            level_requirements: self.chat_level_requirements_like_cpp,
            listen_ranges: self.chat_listen_ranges_like_cpp,
            flood: self.chat_flood_config_like_cpp,
            party_raid_warnings: self.party_raid_warnings_like_cpp,
        }
    }
    #[cfg(test)]
    pub(crate) fn tact_key_store_for_test_like_cpp(&self) -> Option<&Arc<TactKeyStore>> {
        self.tact_key_store.as_ref()
    }
    #[cfg(test)]
    pub(crate) fn area_trigger_catalogs_for_test_like_cpp(&self) -> AreaTriggerCatalogsLikeCpp {
        AreaTriggerCatalogsLikeCpp {
            db2: self
                .area_trigger_db2_store
                .clone()
                .unwrap_or_else(|| Arc::new(AreaTriggerDb2Store::from_entries([]))),
            destinations: self
                .area_trigger_store
                .clone()
                .unwrap_or_else(|| Arc::new(AreaTriggerStore::default())),
            scripts: self
                .area_trigger_script_store
                .clone()
                .unwrap_or_else(|| Arc::new(AreaTriggerScriptStoreLikeCpp::default())),
            taverns: self
                .tavern_area_trigger_store
                .clone()
                .unwrap_or_else(|| Arc::new(TavernAreaTriggerStoreLikeCpp::default())),
            script_dispatcher: self.area_trigger_script_dispatcher_like_cpp.clone(),
        }
    }
    #[cfg(test)]
    pub(crate) fn battlemaster_list_store_for_test_like_cpp(
        &self,
    ) -> Option<&Arc<BattlemasterListStore>> {
        self.battlemaster_list_store.as_ref()
    }
    #[cfg(test)]
    pub(crate) fn set_represented_cinematic_like_cpp_for_test(
        &mut self,
        cinematic_id: Option<u32>,
    ) {
        let _ =
            self.mutate_player_cinematic_state_like_cpp(|state| state.cinematic_id = cinematic_id);
    }
    #[cfg(test)]
    pub(crate) fn set_represented_movie_like_cpp_for_test(&mut self, movie_id: Option<u32>) {
        let _ = self.mutate_player_cinematic_state_like_cpp(|state| state.movie_id = movie_id);
    }
    #[cfg(test)]
    pub(crate) fn support_feature_policy_for_test_like_cpp(&self) -> SupportFeaturePolicyLikeCpp {
        SupportFeaturePolicyLikeCpp {
            support_enabled: self.represented_support_enabled_like_cpp,
            tickets_enabled: self.represented_support_tickets_enabled_like_cpp,
            bugs_enabled: self.represented_support_bugs_enabled_like_cpp,
            complaints_enabled: self.represented_support_complaints_enabled_like_cpp,
            suggestions_enabled: self.represented_support_suggestions_enabled_like_cpp,
            character_undelete_enabled: self.feature_system_character_undelete_enabled_like_cpp,
            bpay_store_enabled: self.feature_system_bpay_store_enabled_like_cpp,
            max_characters_per_realm: self.characters_per_realm_like_cpp,
            declined_names_used: self.declined_names_used_like_cpp,
        }
    }
    #[cfg(test)]
    pub(crate) fn set_loot_money_persistence_test_result_like_cpp(&mut self, success: bool) {
        self.loot_money_persistence_test_result_like_cpp = Some(success);
    }
    #[cfg(test)]
    pub(crate) fn clear_loot_money_persistence_test_result_like_cpp(&mut self) {
        self.loot_money_persistence_test_result_like_cpp = None;
    }
    pub(crate) fn loot_money_persistence_test_result_for_worker_like_cpp(&self) -> Option<bool> {
        #[cfg(test)]
        {
            self.loot_money_persistence_test_result_like_cpp
        }
        #[cfg(not(test))]
        {
            None
        }
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_set_xp_rest_bonus_like_cpp(&mut self, rest_bonus: f32) -> u8 {
        let Some(old_threshold) = self.resolved_xp_rest_threshold_like_cpp() else {
            return 0;
        };
        let Some(old_state) = self.resolved_xp_rest_state_like_cpp() else {
            return 0;
        };
        let mut rest_bonus = crate::session_rules::sanitize_rest_bonus_like_cpp(rest_bonus);
        let Some(can_gain) = self.can_gain_represented_xp_rest_bonus_like_cpp() else {
            return 0;
        };
        if !can_gain {
            rest_bonus = 0.0;
        }

        let Some(rest_bonus_cap) = self.represented_xp_rest_bonus_cap_like_cpp() else {
            return 0;
        };
        rest_bonus = rest_bonus.clamp(0.0, rest_bonus_cap);
        let is_raf_linked = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        let new_state = if is_raf_linked {
            REST_STATE_RAF_LINKED_LIKE_CPP
        } else if rest_bonus >= 1.0 {
            REST_STATE_RESTED_LIKE_CPP
        } else {
            REST_STATE_NORMAL_LIKE_CPP
        };
        // The older legacy2 snapshot uses a `rest_bonus > 10` deadband here.
        // Current legacy1 uses `rest_bonus >= 1` with multi-type RestInfo and
        // is the selected port target; the divergence is retained as evidence
        // that matching one C++ tree alone is not proof that behavior is sound.
        let Some(()) = self.mutate_player_rest_state_like_cpp(|state| {
            state.rest_bonus = rest_bonus;
            state.rest_state = new_state;
        }) else {
            return 0;
        };
        let Some(new_threshold) = self.resolved_xp_rest_threshold_like_cpp() else {
            return 0;
        };
        let Some(new_state) = self.resolved_xp_rest_state_like_cpp() else {
            return 0;
        };
        // C++ writes both RestInfo fields after this combined early-return,
        // and `ModifyValue` marks both nested bits even if one value stayed
        // equal. Therefore every emitted SetRestBonus delta carries 0x07.
        let nested_mask = if old_threshold != new_threshold || old_state != new_state {
            0x07
        } else {
            0
        };
        nested_mask
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_apply_offline_xp_rest_bonus_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        // Issue #81 input hardening: both C++ references assume a valid past
        // logout timestamp and subtract into `uint32`, so zero/future rows can
        // wrap into an immediately capped bonus. Rust intentionally rejects
        // those corrupt persistence values rather than reproducing that bug.
        if logout_time_secs == 0 {
            return 0.0;
        }
        let Some(time_diff) = now_secs.checked_sub(logout_time_secs) else {
            return 0.0;
        };
        if time_diff == 0 {
            return 0.0;
        }

        let bubble = if was_logout_resting {
            REST_OFFLINE_TAVERN_OR_CITY_BUBBLE_LIKE_CPP * policy.offline_tavern_or_city
        } else {
            REST_OFFLINE_WILDERNESS_BUBBLE_LIKE_CPP * policy.offline_wilderness
        };
        let Some(extra_per_sec) = self.calc_represented_xp_rest_extra_per_sec_like_cpp(bubble)
        else {
            return 0.0;
        };
        let extra = time_diff as f32 * extra_per_sec;
        let _ = self.add_represented_xp_rest_bonus_like_cpp(extra);
        extra
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_update_online_xp_rest_bonus_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        now_secs: u64,
    ) -> (f32, u8) {
        let Some(rest_time) = self
            .player_rest_state_snapshot_like_cpp()
            .map(|state| state.rest_time_secs)
        else {
            return (0.0, 0);
        };
        if rest_time == 0 {
            return (0.0, 0);
        }
        let Some(time_diff) = now_secs.checked_sub(rest_time) else {
            return (0.0, 0);
        };
        if time_diff < 10 {
            return (0.0, 0);
        }

        if self
            .mutate_player_rest_state_like_cpp(|state| state.rest_time_secs = now_secs)
            .is_none()
        {
            return (0.0, 0);
        }
        let bubble = REST_ONLINE_INGAME_BUBBLE_LIKE_CPP * policy.ingame;
        let Some(extra_per_sec) = self.calc_represented_xp_rest_extra_per_sec_like_cpp(bubble)
        else {
            return (0.0, 0);
        };
        let extra = time_diff as f32 * extra_per_sec;
        let nested_mask = self.add_represented_xp_rest_bonus_like_cpp(extra);
        (extra, nested_mask)
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_take_xp_rest_bonus_like_cpp(
        &mut self,
        xp: u32,
        victim: wow_core::ObjectGuid,
    ) -> (u32, u8) {
        if victim.is_empty() {
            return (0, 0);
        }

        let Some(current_rest_bonus) = self.resolved_xp_rest_bonus_like_cpp() else {
            return (0, 0);
        };
        let rested_bonus = (current_rest_bonus as u32).min(xp);
        let Some(rested_consumption_modifier) = self
            .resolved_total_represented_aura_modifier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
            )
        else {
            return (0, 0);
        };
        let rested_loss = crate::session_rules::apply_represented_pct_modifier_to_u32_like_cpp(
            rested_bonus,
            rested_consumption_modifier,
        );
        // Both C++ RestMgr implementations call SetRestBonus unconditionally,
        // including when the float bonus truncates to a zero integer award.
        // That call normalizes a verbatim loaded RestState against the current
        // bonus even though no rested XP is awarded or consumed.
        let nested_mask =
            self.set_represented_xp_rest_bonus_like_cpp(current_rest_bonus - rested_loss as f32);
        (rested_bonus, nested_mask)
    }
    #[cfg(test)]
    pub(crate) fn progression_catalogs_for_test_like_cpp(&self) -> ProgressionCatalogsLikeCpp {
        let mut catalogs = ProgressionCatalogsLikeCpp::default();
        if let Some(table) = &self.player_xp_table {
            catalogs.player_xp = Arc::clone(table);
        }
        if let Some(store) = &self.exploration_base_xp_store {
            catalogs.exploration_base_xp = Arc::clone(store);
        }
        catalogs.exploration_xp_rate = self.exploration_xp_rate_like_cpp;
        catalogs.min_discovered_scaled_xp_ratio = self.min_discovered_scaled_xp_ratio_like_cpp;
        catalogs
    }
    pub(in crate::session) fn player_bootstrap_attached_for_test_like_cpp(&self) -> bool {
        #[cfg(test)]
        {
            return self.player_bootstrap_attached_like_cpp;
        }
        #[cfg(not(test))]
        {
            false
        }
    }
    #[cfg(test)]
    pub(crate) fn install_detached_canonical_player_for_test_like_cpp(&mut self) -> bool {
        let Some(position) = self.player_position else {
            return false;
        };
        self.install_detached_canonical_player_from_session_like_cpp(position)
    }
    #[cfg(test)]
    pub(crate) fn set_represented_trade_item_like_cpp_for_test(
        &mut self,
        slot: u8,
        item_guid: ObjectGuid,
    ) {
        let _ = self.mutate_player_trade_state_like_cpp(|state| {
            if slot < TRADE_SLOT_COUNT_LIKE_CPP
                && let Some(state) = state
            {
                state.items[slot as usize] = Some(item_guid);
            }
        });
    }
}
