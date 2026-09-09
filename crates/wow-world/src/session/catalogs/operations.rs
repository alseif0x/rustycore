//! Catalog, config and policy accessors used across the Session.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_trainer_store_like_cpp(&mut self, store: Arc<TrainerStoreLikeCpp>) {
        self.trainer_store_like_cpp = Some(store);
    }
    pub(crate) fn trainer_store_like_cpp(&self) -> Option<&Arc<TrainerStoreLikeCpp>> {
        self.trainer_store_like_cpp.as_ref()
    }
    /// Set the C++ ImportPrice*.db2 stores for this session.
    #[cfg(test)]
    pub fn set_import_price_stores(&mut self, stores: Arc<ImportPriceStores>) {
        self.import_price_stores = Some(stores);
    }
    pub fn set_trinity_string_store(&mut self, store: Arc<TrinityStringStoreLikeCpp>) {
        self.trinity_string_store = Some(store);
    }
    pub fn trinity_string_store(&self) -> Option<&Arc<TrinityStringStoreLikeCpp>> {
        self.trinity_string_store.as_ref()
    }
    /// Set the heirloom store for this session.
    pub fn set_heirloom_store(&mut self, store: Arc<HeirloomStore>) {
        self.heirloom_store = Some(store);
    }
    /// Get the heirloom store reference.
    pub fn heirloom_store(&self) -> Option<&Arc<HeirloomStore>> {
        self.heirloom_store.as_ref()
    }
    /// Set the toy store for this session.
    pub fn set_toy_store(&mut self, store: Arc<ToyStore>) {
        self.toy_store = Some(store);
    }
    /// Get the toy store reference.
    pub fn toy_store(&self) -> Option<&Arc<ToyStore>> {
        self.toy_store.as_ref()
    }
    #[cfg(test)]
    pub fn set_player_create_info_store_like_cpp(
        &mut self,
        store: Arc<PlayerCreateInfoStoreLikeCpp>,
    ) {
        self.player_create_info_store_like_cpp = Some(store);
    }
    pub fn set_packet_spoof_config_like_cpp(&mut self, config: PacketSpoofConfigLikeCpp) {
        self.packet_spoof_config_like_cpp = config;
    }
    #[cfg(test)]
    pub fn set_feature_system_bpay_store_enabled_like_cpp(&mut self, enabled: bool) {
        self.feature_system_bpay_store_enabled_like_cpp = enabled;
    }
    pub(crate) fn update_speak_time_with_policy_like_cpp(
        &mut self,
        index: ChatFloodThrottleIndexLikeCpp,
        config: ChatFloodConfigLikeCpp,
    ) {
        // C++ skips chat spam checks for RBAC_PERM_SKIP_CHECK_CHAT_SPAM. RustyCore
        // has no RBAC store yet; represented GM state is the current session seam.
        if self.player_is_game_master_like_cpp() == Some(true) {
            return;
        }

        let (limit, delay_secs) = match index {
            ChatFloodThrottleIndexLikeCpp::Regular => {
                (config.message_count, config.message_delay_secs)
            }
            ChatFloodThrottleIndexLikeCpp::Addon => {
                (config.addon_message_count, config.addon_message_delay_secs)
            }
        };
        let current = unix_now();
        let data = &mut self.chat_flood_data_like_cpp[index as usize];

        if data.time > current {
            if limit == 0 {
                return;
            }

            data.count = data.count.saturating_add(1);
            if data.count >= limit {
                let new_mute = current.saturating_add(i64::from(config.mute_time_secs));
                if self.mute_time_like_cpp < new_mute {
                    self.mute_time_like_cpp = new_mute;
                }
                data.count = 0;
            }
        } else {
            data.count = 1;
        }

        data.time = current.saturating_add(i64::from(delay_secs));
    }
    pub(crate) fn void_withdrawal_post_store_item_values_update_like_cpp(
        item: &Item,
        create_dynamic_flags: u32,
    ) -> Option<ItemValuesUpdate> {
        let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
        let mut has_parent_field = false;
        if !item.data().creator.is_empty() {
            item_data_mask.set(ITEM_DATA_CREATOR_BIT);
            has_parent_field = true;
        }
        if item.data().dynamic_flags != create_dynamic_flags {
            item_data_mask.set(ITEM_DATA_DYNAMIC_FLAGS_BIT);
            has_parent_field = true;
        }
        if item.data().property_seed != 0 {
            item_data_mask.set(ITEM_DATA_PROPERTY_SEED_BIT);
            has_parent_field = true;
        }
        if item.data().random_properties_id != 0 {
            item_data_mask.set(ITEM_DATA_RANDOM_PROPERTIES_ID_BIT);
            has_parent_field = true;
        }
        if has_parent_field {
            item_data_mask.set(ITEM_DATA_PARENT_BIT);
        }
        for (index, enchantment) in item.data().enchantments.iter().enumerate() {
            if *enchantment != wow_entities::ItemEnchantment::default() {
                item_data_mask.set(ITEM_DATA_ENCHANTMENT_PARENT_BIT);
                item_data_mask.set(ITEM_DATA_ENCHANTMENT_FIRST_BIT + index);
            }
        }
        if !item_data_mask.is_any_set() {
            return None;
        }
        Some(ItemValuesUpdate {
            changed_object_type_mask: 1 << TYPEID_ITEM,
            object_data: None,
            item_data: Some(ItemDataUpdate {
                mask: item_data_mask,
                values: item.data().clone(),
            }),
        })
    }
    pub(crate) fn send_void_withdrawal_post_store_item_values_update_like_cpp(
        &self,
        item_guid: ObjectGuid,
        create_dynamic_flags: u32,
    ) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        let Some(update) = Self::void_withdrawal_post_store_item_values_update_like_cpp(
            &item,
            create_dynamic_flags,
        ) else {
            return;
        };
        if let Some(packet) =
            item_values_update_to_update_object(item_guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
    /// Set the random property points store for this session.
    pub fn set_rand_prop_points_store(&mut self, store: Arc<RandPropPointsStore>) {
        self.rand_prop_points_store = Some(store);
    }
    /// Get the random property points store reference.
    pub fn rand_prop_points_store(&self) -> Option<&Arc<RandPropPointsStore>> {
        self.rand_prop_points_store.as_ref()
    }
    /// Set the C++ ConditionMgr store loaded from the `conditions` table.
    pub fn set_condition_store(&mut self, store: Arc<ConditionEntriesByTypeStore>) {
        self.condition_store = Some(store);
    }
    /// Get the loaded ConditionMgr store reference.
    pub fn condition_store(&self) -> Option<&Arc<ConditionEntriesByTypeStore>> {
        self.condition_store.as_ref()
    }
    /// Set the C++ PlayerCondition.db2 store for this session.
    pub fn set_player_condition_store(&mut self, store: Arc<PlayerConditionStore>) {
        self.player_condition_store = Some(store);
    }
    pub fn set_content_tuning_store(&mut self, store: Arc<ContentTuningStore>) {
        self.content_tuning_store = Some(store);
    }
    pub fn set_curve_store(&mut self, store: Arc<CurveStore>) {
        self.curve_store = Some(store);
    }
    pub fn set_curve_point_store(&mut self, store: Arc<CurvePointStore>) {
        self.curve_point_store = Some(store);
    }
    pub fn set_scaling_stat_distribution_store(
        &mut self,
        store: Arc<ScalingStatDistributionStore>,
    ) {
        self.scaling_stat_distribution_store = Some(store);
    }
    pub fn set_scaling_stat_values_store(&mut self, store: Arc<ScalingStatValuesStore>) {
        self.scaling_stat_values_store = Some(store);
    }
    /// Get the loaded PlayerCondition.db2 store reference.
    pub fn player_condition_store(&self) -> Option<&Arc<PlayerConditionStore>> {
        self.player_condition_store.as_ref()
    }
    /// Set the lock store for this session.
    pub fn set_lock_store(&mut self, store: Arc<LockStore>) {
        self.lock_store = Some(store);
    }
    pub(crate) fn lock_store(&self) -> Option<&Arc<LockStore>> {
        self.lock_store.as_ref()
    }
    pub fn set_gem_properties_store(&mut self, store: Arc<GemPropertiesStore>) {
        self.gem_properties_store = Some(store);
    }
    /// Set the TactKey.db2 store for typed SMSG_DB_REPLY serialization.
    #[cfg(test)]
    pub fn set_tact_key_store(&mut self, store: Arc<TactKeyStore>) {
        self.tact_key_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_graveyard_store(&mut self, store: Arc<GraveyardStore>) {
        self.graveyard_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn graveyard_store(&self) -> Option<&Arc<GraveyardStore>> {
        self.graveyard_store.as_ref()
    }
    /// Set the ChrSpecialization store for this session.
    pub fn set_chr_specialization_store(&mut self, store: Arc<ChrSpecializationStore>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.chr.specialization_store = Some(store);
    }
    /// Get the ChrSpecialization store reference.
    pub fn chr_specialization_store(&self) -> Option<&Arc<ChrSpecializationStore>> {
        self.chr.specialization_store.as_ref()
    }
    pub fn set_world_safe_loc_store_like_cpp(&mut self, store: Arc<WorldSafeLocStore>) {
        self.world_safe_loc_store_like_cpp = Some(store);
    }
    pub(crate) fn world_safe_loc_store_like_cpp(&self) -> Option<&Arc<WorldSafeLocStore>> {
        self.world_safe_loc_store_like_cpp.as_ref()
    }
    pub fn set_access_requirement_store(&mut self, store: Arc<AccessRequirementStoreLikeCpp>) {
        self.access_requirement_store = Some(store);
    }
    pub(crate) fn access_requirement_store(&self) -> Option<&Arc<AccessRequirementStoreLikeCpp>> {
        self.access_requirement_store.as_ref()
    }
    pub fn set_lfg_dungeons_store(&mut self, store: Arc<LfgDungeonsStore>) {
        self.lfg_dungeons_store = Some(store);
    }
    pub(crate) fn lfg_dungeons_store(&self) -> Option<&Arc<LfgDungeonsStore>> {
        self.lfg_dungeons_store.as_ref()
    }
    #[cfg(test)]
    pub fn set_lfg_dungeon_store_like_cpp(&mut self, store: Arc<LfgDungeonStoreLikeCpp>) {
        self.lfg_dungeon_store_like_cpp = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn lfg_dungeon_store_like_cpp(&self) -> Option<&Arc<LfgDungeonStoreLikeCpp>> {
        self.lfg_dungeon_store_like_cpp.as_ref()
    }
    #[cfg(test)]
    pub fn set_battlemaster_list_store(&mut self, store: Arc<BattlemasterListStore>) {
        self.battlemaster_list_store = Some(store);
    }
    pub fn set_faction_store(&mut self, store: Arc<FactionStore>) {
        self.factions.store = Some(store);
        self.initialize_reputation_mgr_like_cpp();
    }
    pub(crate) fn faction_store(&self) -> Option<&Arc<FactionStore>> {
        self.factions.store.as_ref()
    }
    pub fn set_faction_template_store(&mut self, store: Arc<FactionTemplateStore>) {
        self.factions.template_store = Some(store);
    }
    pub fn set_mount_store(&mut self, store: Arc<MountStore>) {
        self.mount_store = Some(store);
        self.expand_account_mount_faction_definitions_like_cpp();
        self.learn_account_mount_spells_like_cpp();
    }
    pub(crate) fn mount_store(&self) -> Option<&Arc<MountStore>> {
        self.mount_store.as_ref()
    }
    pub fn set_mount_definition_store_like_cpp(&mut self, store: Arc<MountDefinitionStoreLikeCpp>) {
        self.mount_definition_store_like_cpp = Some(store);
        self.expand_account_mount_faction_definitions_like_cpp();
        self.learn_account_mount_spells_like_cpp();
    }
    pub fn set_mount_capability_store(&mut self, store: Arc<MountCapabilityStore>) {
        self.mount_capability_store = Some(store);
    }
    pub fn set_mount_type_x_capability_store(&mut self, store: Arc<MountTypeXCapabilityStore>) {
        self.mount_type_x_capability_store = Some(store);
    }
    pub fn set_mount_x_display_store(&mut self, store: Arc<MountXDisplayStore>) {
        self.mount_x_display_store = Some(store);
    }
    #[allow(dead_code)]
    pub(crate) fn mount_capability_store(&self) -> Option<&Arc<MountCapabilityStore>> {
        self.mount_capability_store.as_ref()
    }
    #[allow(dead_code)]
    pub(crate) fn mount_type_x_capability_store(&self) -> Option<&Arc<MountTypeXCapabilityStore>> {
        self.mount_type_x_capability_store.as_ref()
    }
    #[allow(dead_code)]
    pub(crate) fn mount_x_display_store(&self) -> Option<&Arc<MountXDisplayStore>> {
        self.mount_x_display_store.as_ref()
    }
    pub fn set_terrain_swap_store(&mut self, store: Arc<wow_data::TerrainSwapStore>) {
        self.terrain_swap_store = Some(store);
    }
    pub fn set_trait_definition_store(&mut self, store: Arc<TraitDefinitionStore>) {
        self.trait_definition_store = Some(store);
    }
    pub(crate) fn trait_definition_store(&self) -> Option<&Arc<TraitDefinitionStore>> {
        self.trait_definition_store.as_ref()
    }
    pub fn set_spell_group_store(&mut self, store: Arc<SpellGroupStoreLikeCpp>) {
        self.spell_catalogs.spell_group_store = Some(store);
    }
    pub fn set_spell_group_stack_rule_store(
        &mut self,
        store: Arc<SpellGroupStackRuleStoreLikeCpp>,
    ) {
        self.spell_catalogs.spell_group_stack_rule_store = Some(store);
    }
    pub fn set_spell_pet_aura_store(&mut self, store: Arc<SpellPetAuraStoreLikeCpp>) {
        self.spell_catalogs.spell_pet_aura_store = Some(store);
    }
    pub(crate) fn spell_pet_aura_store_like_cpp(&self) -> Option<&SpellPetAuraStoreLikeCpp> {
        self.spell_catalogs.spell_pet_aura_store.as_deref()
    }
    #[cfg(test)]
    pub fn set_pet_levelup_spell_store(&mut self, store: Arc<PetLevelupSpellStoreLikeCpp>) {
        self.spell_catalogs.pet_levelup_spell_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_pet_default_spell_store(&mut self, store: Arc<PetDefaultSpellStoreLikeCpp>) {
        self.spell_catalogs.pet_default_spell_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_pet_family_spell_store(&mut self, store: Arc<PetFamilySpellStoreLikeCpp>) {
        self.spell_catalogs.pet_family_spell_store = Some(store);
    }
    pub fn set_movie_store(&mut self, store: Arc<MovieStore>) {
        self.movie_store = Some(store);
    }
    pub fn set_chr_classes_store(&mut self, store: Arc<ChrClassesStore>) {
        self.chr.classes_store = Some(store);
    }
    pub fn set_chr_races_store(&mut self, store: Arc<ChrRacesStore>) {
        self.chr.races_store = Some(store);
    }
    pub fn set_cinematic_sequences_store(&mut self, store: Arc<CinematicSequencesStore>) {
        self.cinematic_sequences_store = Some(store);
    }
    pub(crate) fn feature_system_status_with_policy_like_cpp(
        &self,
        policy: &SupportFeaturePolicyLikeCpp,
    ) -> FeatureSystemStatus {
        FeatureSystemStatus::from_config_like_cpp(
            policy.feature_system_config_like_cpp(),
            !self.can_speak_like_cpp(),
        )
    }
    pub(crate) fn feature_system_status_glue_screen_with_policy_like_cpp(
        &self,
        policy: &SupportFeaturePolicyLikeCpp,
    ) -> FeatureSystemStatusGlueScreen {
        FeatureSystemStatusGlueScreen::from_config_like_cpp(
            policy.feature_system_config_like_cpp(),
            policy.max_characters_per_realm as i32,
            i32::from(self.server_expansion_like_cpp),
        )
    }
    #[cfg(test)]
    pub fn set_object_mgr_catalogs_like_cpp(&mut self, catalogs: Arc<ObjectMgrCatalogsLikeCpp>) {
        self.object_mgr_catalogs_like_cpp = Some(catalogs);
    }
    #[cfg(test)]
    pub(crate) fn world_query_catalogs_like_cpp(&self) -> Option<&ObjectMgrCatalogsLikeCpp> {
        self.object_mgr_catalogs_like_cpp.as_deref()
    }
    pub(in crate::session) fn player_is_at_configured_max_level_like_cpp(&self) -> bool {
        let max_level = self.max_player_level_config_like_cpp;
        max_level != 0 && u32::from(self.player_level_like_cpp()) >= max_level
    }
    pub(crate) fn apply_offline_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_apply_offline_xp_rest_bonus_like_cpp(
                policy,
                logout_time_secs,
                now_secs,
                was_logout_resting,
            );
        }
        let bubble = if was_logout_resting {
            REST_OFFLINE_TAVERN_OR_CITY_BUBBLE_LIKE_CPP * policy.offline_tavern_or_city
        } else {
            REST_OFFLINE_WILDERNESS_BUBBLE_LIKE_CPP * policy.offline_wilderness
        };
        let at_max = self.player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.apply_offline_xp_rest_bonus_like_cpp(
                logout_time_secs,
                now_secs,
                bubble,
                at_max,
                raf,
            )
        })
        .unwrap_or(0.0)
    }
    pub(in crate::session) fn update_represented_online_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        now_secs: u64,
    ) -> (f32, u8) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_update_online_xp_rest_bonus_like_cpp(policy, now_secs);
        }
        let bubble = REST_ONLINE_INGAME_BUBBLE_LIKE_CPP * policy.ingame;
        let at_max = self.player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.update_online_xp_rest_bonus_like_cpp(now_secs, bubble, at_max, raf)
        })
        .unwrap_or((0.0, 0))
    }
    pub(in crate::session) fn tick_represented_online_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        now_secs: u64,
    ) {
        // C++ `RestMgr::Update` freezes the elapsed-time update behind
        // `roll_chance_i(3)`. Use the session's runtime RNG so the gate is
        // probabilistic in production and seedable in focused tests.
        let update_roll_passed = self.represented_urand_u32_like_cpp(1, 100) <= 3;
        self.tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
            policy,
            now_secs,
            update_roll_passed,
        );
    }
    pub(in crate::session) fn tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        now_secs: u64,
        update_roll_passed: bool,
    ) {
        if !update_roll_passed {
            return;
        }
        let (_, nested_mask) =
            self.update_represented_online_xp_rest_bonus_with_policy_like_cpp(policy, now_secs);
        if nested_mask != 0 {
            self.send_represented_rest_info_update_like_cpp(nested_mask);
        }
    }
    pub(in crate::session) fn revalidate_represented_tavern_resting_with_catalog_like_cpp(
        &mut self,
        area_trigger_db2_store: &AreaTriggerDb2Store,
    ) {
        let Some(rest) = self.player_rest_state_snapshot_like_cpp() else {
            return;
        };
        if (rest.rest_flag_mask & REST_FLAG_IN_TAVERN_LIKE_CPP) == 0 {
            return;
        }

        let Some(at_entry) = area_trigger_db2_store
            .get(rest.inn_area_trigger_id)
            .cloned()
        else {
            if self.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP) {
                self.send_represented_resting_player_flag_update_like_cpp();
            }
            return;
        };

        if !self.player_is_in_area_trigger_radius_like_cpp(&at_entry)
            && self.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP)
        {
            self.send_represented_resting_player_flag_update_like_cpp();
        }
    }
    #[cfg(test)]
    pub fn set_exploration_base_xp_store_like_cpp(
        &mut self,
        store: Arc<ExplorationBaseXpStoreLikeCpp>,
    ) {
        self.exploration_base_xp_store = Some(store);
    }
    pub fn set_max_player_level_config_like_cpp(&mut self, max_player_level_config: u32) {
        self.max_player_level_config_like_cpp = max_player_level_config;
    }
    #[cfg(test)]
    pub fn set_rested_xp_config_like_cpp(
        &mut self,
        max_player_level_config: u32,
        rest_offline_wilderness_rate: f32,
        rest_offline_tavern_or_city_rate: f32,
        rest_ingame_rate: f32,
    ) {
        self.max_player_level_config_like_cpp = max_player_level_config;
        self.rest_offline_wilderness_rate_like_cpp = rest_offline_wilderness_rate;
        self.rest_offline_tavern_or_city_rate_like_cpp = rest_offline_tavern_or_city_rate;
        self.rest_ingame_rate_like_cpp = rest_ingame_rate;
    }
    /// Update player_next_level_xp from the table based on current level.
    pub(crate) fn refresh_next_level_xp_with_catalogs_like_cpp(
        &mut self,
        catalogs: &ProgressionCatalogsLikeCpp,
    ) {
        let lvl = self.player_level_like_cpp() as usize;
        let next_level_xp = catalogs.player_xp.get(lvl).copied().unwrap_or(u32::MAX);
        let table = Arc::clone(&catalogs.player_xp);
        let installed = self
            .with_owned_player_mut_like_cpp(|player| {
                player.install_player_xp_table_like_cpp(table);
            })
            .is_some();
        if installed {
            self.set_player_next_level_xp_like_cpp(next_level_xp);
        }
        #[cfg(test)]
        if !installed && self.player_handle_like_cpp.is_none() {
            self.set_player_next_level_xp_like_cpp(next_level_xp);
        }
        #[cfg(not(test))]
        let _ = installed;
    }
    /// Send session initialization packets (first encrypted packets after
    /// EnterEncryptedModeAck). Matches C++ `WorldSession::InitializeSessionCallback`.
    ///
    /// These packets are sent immediately when the session starts, before any
    /// client packets are processed. They tell the client that auth succeeded
    /// and provide the initial glue screen data (character select).
    ///
    /// Exact C++ order:
    /// 1. AuthResponse
    /// 2. SetTimeZoneInformation
    /// 3. FeatureSystemStatusGlueScreen (NOT the in-game FeatureSystemStatus!)
    /// 4. ClientCacheVersion
    /// 5. AvailableHotfixes
    /// 6. AccountDataTimes (global)
    /// 7. TutorialFlags
    /// 8. ConnectionStatus (State=1)
    pub fn send_session_init_packets_with_policy_like_cpp(
        &self,
        policy: &SupportFeaturePolicyLikeCpp,
        hotfixes: &HotfixBlobCache,
    ) {
        use wow_packet::packets::auth::*;
        use wow_packet::packets::misc::*;

        let vra = self.virtual_realm_address();
        let (realm_name_actual, realm_name_normalized) = self
            .realm_names_for_address_like_cpp(vra)
            .unwrap_or(("RustyCore", "RustyCore"));

        // 1. AuthResponse (OK) — tells the client authentication succeeded
        let auth_response = AuthResponse {
            result: 0, // OK
            success_info: Some(AuthSuccessInfo {
                virtual_realm_address: vra,
                virtual_realms: vec![VirtualRealmInfo {
                    realm_address: vra,
                    is_local: true,
                    is_internal_realm: false,
                    realm_name_actual: realm_name_actual.to_string(),
                    realm_name_normalized: realm_name_normalized.to_string(),
                }],
                time_rested: 0,
                active_expansion_level: self.expansion,
                account_expansion_level: self.account_expansion,
                time_seconds_until_pc_kick: 0,
                available_classes: default_available_classes(),
                templates: vec![],
                currency_id: 0,
                time: unix_now(),
                game_time_info: GameTimeInfo {
                    billing_plan: 0,
                    time_remain: 0,
                    unknown735: 0,
                    in_game_room: false,
                },
                is_expansion_trial: false,
                force_character_template: false,
                num_players_horde: None,
                num_players_alliance: None,
                expansion_trial_expiration: None,
            }),
            wait_info: None,
        };
        self.send_packet(&auth_response);

        // 2. SetTimeZoneInformation
        self.send_packet(&SetTimeZoneInformation::utc());

        // 3. FeatureSystemStatusGlueScreen (character select version, NOT in-game)
        self.send_packet(&self.feature_system_status_glue_screen_with_policy_like_cpp(policy));

        // 4. ClientCacheVersion (from world DB version.cache_id = 24081)
        self.send_packet(&ClientCacheVersion {
            cache_version: 24081,
        });

        let hotfixes = hotfixes
            .available_hotfix_ids(&self.locale)
            .into_iter()
            .map(|id| HotfixId {
                push_id: id.push_id,
                unique_id: id.unique_id,
            })
            .collect();

        // 5. AvailableHotfixes
        self.send_packet(&AvailableHotfixes {
            virtual_realm_address: vra,
            hotfixes,
        });

        // 6. AccountDataTimes (global)
        self.send_packet(
            &self.account_data_times_like_cpp(ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP),
        );

        // 7. TutorialFlags
        self.send_packet(&self.tutorial_flags_packet_like_cpp());

        // 8. ConnectionStatus (State=1, SuppressNotification=true)
        // This compatibility packet has no ConnectionType override,
        // so it's sent on the realm socket. State uses 2 bits, SuppressNotification
        // defaults to true.
        self.send_packet(&ConnectionStatus {
            state: 1,
            suppress_notification: true,
        });

        info!(
            "Session init packets sent for account {} (8 packets: AuthResponse → ConnectionStatus)",
            self.account_id
        );
    }
}
