//! Catalog, config and policy accessors used across the Session.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_trainer_store_like_cpp(&mut self, store: Arc<TrainerStoreLikeCpp>) {
        self.catalogs.trainer_store_like_cpp = Some(store);
    }
    pub fn set_trinity_string_store(&mut self, store: Arc<TrinityStringStoreLikeCpp>) {
        self.catalogs.trinity_string_store = Some(store);
    }
    /// Set the heirloom store for this session.
    pub fn set_heirloom_store(&mut self, store: Arc<HeirloomStore>) {
        self.catalogs.heirloom_store = Some(store);
    }
    /// Set the toy store for this session.
    pub fn set_toy_store(&mut self, store: Arc<ToyStore>) {
        self.catalogs.toy_store = Some(store);
    }
    pub fn set_packet_spoof_config_like_cpp(&mut self, config: PacketSpoofConfigLikeCpp) {
        self.core.admission.packet_spoof_config_like_cpp = config;
    }
    /// Set the random property points store for this session.
    pub fn set_rand_prop_points_store(&mut self, store: Arc<RandPropPointsStore>) {
        self.catalogs.rand_prop_points_store = Some(store);
    }
    /// Set the C++ ConditionMgr store loaded from the `conditions` table.
    pub fn set_condition_store(&mut self, store: Arc<ConditionEntriesByTypeStore>) {
        self.catalogs.condition_store = Some(store);
    }
    pub(crate) fn condition_store(&self) -> Option<&Arc<ConditionEntriesByTypeStore>> {
        self.catalogs.condition_store()
    }
    /// Set the C++ PlayerCondition.db2 store for this session.
    pub fn set_player_condition_store(&mut self, store: Arc<PlayerConditionStore>) {
        self.catalogs.player_condition_store = Some(store);
    }
    pub fn set_content_tuning_store(&mut self, store: Arc<ContentTuningStore>) {
        self.catalogs.content_tuning_store = Some(store);
    }
    pub fn set_curve_store(&mut self, store: Arc<CurveStore>) {
        self.catalogs.curve_store = Some(store);
    }
    pub fn set_curve_point_store(&mut self, store: Arc<CurvePointStore>) {
        self.catalogs.curve_point_store = Some(store);
    }
    pub fn set_scaling_stat_distribution_store(
        &mut self,
        store: Arc<ScalingStatDistributionStore>,
    ) {
        self.catalogs.scaling_stat_distribution_store = Some(store);
    }
    pub fn set_scaling_stat_values_store(&mut self, store: Arc<ScalingStatValuesStore>) {
        self.catalogs.scaling_stat_values_store = Some(store);
    }
    pub(crate) fn player_condition_store(&self) -> Option<&Arc<PlayerConditionStore>> {
        self.catalogs.player_condition_store()
    }
    /// Set the lock store for this session.
    pub fn set_lock_store(&mut self, store: Arc<LockStore>) {
        self.catalogs.lock_store = Some(store);
    }
    pub fn set_gem_properties_store(&mut self, store: Arc<GemPropertiesStore>) {
        self.catalogs.gem_properties_store = Some(store);
    }
    /// Set the ChrSpecialization store for this session.
    pub fn set_chr_specialization_store(&mut self, store: Arc<ChrSpecializationStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.chr.specialization_store = Some(store);
    }
    pub fn set_world_safe_loc_store_like_cpp(&mut self, store: Arc<WorldSafeLocStore>) {
        self.catalogs.world_safe_loc_store_like_cpp = Some(store);
    }
    pub fn set_access_requirement_store(&mut self, store: Arc<AccessRequirementStoreLikeCpp>) {
        self.catalogs.access_requirement_store = Some(store);
    }
    pub fn set_lfg_dungeons_store(&mut self, store: Arc<LfgDungeonsStore>) {
        self.catalogs.lfg_dungeons_store = Some(store);
    }
    pub fn set_faction_store(&mut self, store: Arc<FactionStore>) {
        self.catalogs.factions.store = Some(store);
        crate::session::hub_mut(self).initialize_reputation_mgr_like_cpp();
    }
    pub fn set_faction_template_store(&mut self, store: Arc<FactionTemplateStore>) {
        self.catalogs.factions.template_store = Some(store);
    }
    pub fn set_mount_store(&mut self, store: Arc<MountStore>) {
        self.catalogs.mount_store = Some(store);
        self.expand_account_mount_faction_definitions_like_cpp();
        wow_world_spell::learn_account_mount_spells_like_cpp(self);
    }
    pub fn set_mount_definition_store_like_cpp(&mut self, store: Arc<MountDefinitionStoreLikeCpp>) {
        self.catalogs.mount_definition_store_like_cpp = Some(store);
        self.expand_account_mount_faction_definitions_like_cpp();
        wow_world_spell::learn_account_mount_spells_like_cpp(self);
    }
    pub fn set_mount_capability_store(&mut self, store: Arc<MountCapabilityStore>) {
        self.catalogs.mount_capability_store = Some(store);
    }
    pub fn set_mount_type_x_capability_store(&mut self, store: Arc<MountTypeXCapabilityStore>) {
        self.catalogs.mount_type_x_capability_store = Some(store);
    }
    pub fn set_mount_x_display_store(&mut self, store: Arc<MountXDisplayStore>) {
        self.catalogs.mount_x_display_store = Some(store);
    }
    pub fn set_terrain_swap_store(&mut self, store: Arc<wow_data::TerrainSwapStore>) {
        self.catalogs.terrain_swap_store = Some(store);
    }
    pub fn set_trait_definition_store(&mut self, store: Arc<TraitDefinitionStore>) {
        self.catalogs.trait_definition_store = Some(store);
    }
    pub fn set_trait_tree_skill_line_index(
        &mut self,
        index: Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>,
    ) {
        self.catalogs.trait_tree_skill_line_index = Some(index);
    }
    pub fn set_spell_group_store(&mut self, store: Arc<SpellGroupStoreLikeCpp>) {
        self.catalogs.spell_catalogs.spell_group_store = Some(store);
    }
    pub fn set_spell_group_stack_rule_store(
        &mut self,
        store: Arc<SpellGroupStackRuleStoreLikeCpp>,
    ) {
        self.catalogs.spell_catalogs.spell_group_stack_rule_store = Some(store);
    }
    pub fn set_spell_pet_aura_store(&mut self, store: Arc<SpellPetAuraStoreLikeCpp>) {
        self.catalogs.spell_catalogs.spell_pet_aura_store = Some(store);
    }
    pub fn set_movie_store(&mut self, store: Arc<MovieStore>) {
        self.catalogs.movie_store = Some(store);
    }
    pub fn set_chr_classes_store(&mut self, store: Arc<ChrClassesStore>) {
        self.catalogs.chr.classes_store = Some(store);
    }
    pub fn set_chr_races_store(&mut self, store: Arc<ChrRacesStore>) {
        self.catalogs.chr.races_store = Some(store);
    }
    pub fn set_shapeshift_display_links_absent(&mut self, absent: bool) {
        self.catalogs.chr.shapeshift_display_links_absent = Some(absent);
    }
    pub(crate) fn apply_offline_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        #[cfg(test)]
        if self
            .core
            .with_owned_player_for_rest_like_cpp(|_| ())
            .is_none()
        {
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
        let at_max = crate::session::hub_ref(self).player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        wow_world_instances::with_owned_player_mut_for_rest_like_cpp(self, |player| {
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
    pub(crate) fn update_represented_online_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        now_secs: u64,
    ) -> (f32, u8) {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self.fixture_update_online_xp_rest_bonus_like_cpp(policy, now_secs);
        }
        let bubble = REST_ONLINE_INGAME_BUBBLE_LIKE_CPP * policy.ingame;
        let at_max = crate::session::hub_ref(self).player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.core
            .with_owned_player_mut_like_cpp(|player| {
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
        let update_roll_passed = self.core.represented_urand_u32_like_cpp(1, 100) <= 3;
        self.tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
            policy,
            now_secs,
            update_roll_passed,
        );
    }
    pub(crate) fn tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
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
            crate::session::hub_ref(self).send_represented_rest_info_update_like_cpp(nested_mask);
        }
    }
    /// The driver's entry to the tavern revalidation, moved to
    /// `wow_world_instances` (#1363 slice 12b7).
    pub(crate) fn revalidate_represented_tavern_resting_with_catalog_like_cpp(
        &mut self,
        area_trigger_db2_store: &AreaTriggerDb2Store,
    ) {
        wow_world_instances::revalidate_represented_tavern_resting_with_catalog_like_cpp(
            self,
            area_trigger_db2_store,
        )
    }
    pub fn set_max_player_level_config_like_cpp(&mut self, max_player_level_config: u32) {
        self.config.max_player_level_config_like_cpp = max_player_level_config;
    }
    pub fn set_group_visibility_mode(&mut self, mode: i32) {
        self.config.group_visibility_mode = mode;
    }
    #[cfg(test)]
    pub(crate) fn set_rested_xp_config_like_cpp(
        &mut self,
        max_player_level_config: u32,
        rest_offline_wilderness_rate: f32,
        rest_offline_tavern_or_city_rate: f32,
        rest_ingame_rate: f32,
    ) {
        self.config.max_player_level_config_like_cpp = max_player_level_config;
        self.fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .rest_offline_wilderness_rate_like_cpp = rest_offline_wilderness_rate;
        self.fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .rest_offline_tavern_or_city_rate_like_cpp = rest_offline_tavern_or_city_rate;
        self.fixtures
            .progression
            .rest_mgr_test_fixture_like_cpp
            .rest_ingame_rate_like_cpp = rest_ingame_rate;
    }
    /// Send the session init packets (C++ `WorldSession::InitializeSessionCallback`);
    /// the body is `wow_world_character::send_session_init_packets_with_policy_like_cpp`
    /// (#1363 slice 12b7). The `world-server` entry point.
    pub fn send_session_init_packets_with_policy_like_cpp(
        &self,
        policy: &SupportFeaturePolicyLikeCpp,
        hotfixes: &HotfixBlobCache,
    ) {
        wow_world_character::send_session_init_packets_with_policy_like_cpp(self, policy, hotfixes)
    }
}

/// Install the complete immutable condition-row presence catalog at composition.
pub fn install_condition_presence_catalog(
    session: &mut WorldSession,
    catalog: Arc<wow_data::ConditionPresenceCatalog>,
) {
    session.catalogs.condition_presence = Some(catalog);
}

/// Install the effective server-side spell catalog as known session metadata.
pub fn install_serverside_spell_catalog(
    session: &mut WorldSession,
    catalog: Arc<wow_data::ServersideSpellStoreLikeCpp>,
) {
    session.catalogs.spell_catalogs.serverside_spell_store = Some(catalog);
}

#[cfg(test)]
#[path = "../../../unit_tests/session/catalogs/operations/f3_shims.rs"]
mod f3_shims;
