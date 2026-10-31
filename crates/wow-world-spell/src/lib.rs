//! World-session spell state and its application-owned operations.

pub mod aura_effects;
mod aura_rows;
mod aura_session;
mod aura_update_session;
mod effect_session;
mod execution_session;
mod login_spell_application;
pub mod login_spell_rules;
pub mod melee_damage;
pub mod melee_rules;
mod module_hooks;
pub mod player_cast;
mod player_cast_session;
mod profession;
mod records;
mod selected_wisp_transform;
mod self_resurrection;
mod session;
mod spell_acquisition;
mod spell_acquisition_application;
pub mod spell_cast_adapter;
mod spell_click_session;
mod spell_history_maintenance;
mod spell_state_session;
mod spellbook_session;
mod state;
mod talent_session;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;
mod toy_session;

pub use aura_rows::{CharacterAuraEffectRowLikeCpp, CharacterAuraRowLikeCpp};
pub use aura_session::selected_aura_max_charges;
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_session::update_represented_flight_flags_for_flight_aura_like_cpp;
pub use aura_session::{
    AuraHostLikeCpp, SelectedAuraLifetimeResult, insert_player_visible_aura_like_cpp,
    load_represented_character_auras_like_cpp, next_player_visible_aura_slot_like_cpp,
    remove_auras_with_interrupt_flags_like_cpp,
    remove_indoor_outdoor_auras_for_current_position_represented_like_cpp,
    remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp,
    remove_represented_auras_due_to_item_spell_like_cpp,
    remove_represented_auras_due_to_spell_like_cpp,
    remove_represented_mounted_auras_by_type_like_cpp,
    represented_mount_capability_selection_for_type_like_cpp,
    represented_spell_area_autocast_source_is_empty_like_cpp,
    represented_trait_config_aura_source_is_empty_like_cpp, send_aura_update_applied,
    set_player_aura_duration, tick_auras_expiry_only, tick_selected_aura_lifetimes,
    update_player_collision_height_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_session::{
    SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP, apply_represented_mounted_aura_like_cpp,
    apply_represented_provide_spell_focus_aura_like_cpp,
    represented_mount_aura_display_candidates_like_cpp,
    represented_player_aura_state_mask_like_cpp, represented_unit_aura_state_mask_like_cpp,
};
pub use aura_session::{SelectedAuraRefreshPolicy, selected_aura_refresh_policy};
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_session::{flush_pending_selected_aura_updates, tick_auras};
pub use aura_update_session::handle_player_aura_update;
#[cfg(any(test, feature = "test-fixtures"))]
pub use effect_session::{
    ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp,
    ApplyEffectSummonObjectSlotSessionStatusLikeCpp, apply_effect_summon_object_slot_like_cpp,
    apply_modify_spell_charges_effect_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use effect_session::{
    ApplyEffectSummonObjectWildSessionOutcomeLikeCpp,
    ApplyEffectSummonObjectWildSessionStatusLikeCpp, DAMAGE_FIRE_LIKE_CPP,
    apply_damage_with_generator_like_cpp, apply_effect_add_farsight_like_cpp,
    apply_effect_summon_object_wild_like_cpp, apply_heal, apply_heal_max_health_like_cpp,
    apply_spell_initial_threat_like_cpp, implicit_target_conditions_like_cpp,
    player_homebind_update_request_like_cpp, represented_target_aura_state_mask_like_cpp,
    represented_target_creature_type_mask_like_cpp, represented_target_mechanic_mask_like_cpp,
};
pub use effect_session::{
    SpellEffectHostLikeCpp, apply_block_effect_like_cpp, apply_parry_effect_like_cpp,
    canonical_threatened_by_me_owner_guids_like_cpp, current_map_dungeon_state_like_cpp,
    current_map_is_dungeon_like_cpp, disallowed_mount_form_from_catalogs,
    hydrate_canonical_threat_relevant_auras_like_cpp,
    notify_other_players_visibility_changed_like_cpp, persist_player_homebind_like_cpp,
    player_current_map_instanceable_like_cpp, record_spell_execute_log_durability_damage_like_cpp,
    represented_disallowed_mount_form_like_cpp, represented_homebind_like_cpp,
    set_represented_homebind_like_cpp, sync_represented_creature_threat_to_canonical_like_cpp,
};
pub use execution_session::take_spell_power_like_cpp;
pub use execution_session::{
    ServerNormalPreGoFailureLikeCpp, ServerNormalPreGoPlanLikeCpp, SpellExecutionHostLikeCpp,
    apply_durability_damage_effect_like_cpp,
    apply_inventory_item_obtain_spells_with_generator_like_cpp, apply_proficiency_effect_like_cpp,
    cast_all_obtain_spells_like_cpp, check_represented_cast_preparation_like_cpp,
    check_represented_caster_death_like_cpp, consume_add_farsight_set_seer_outcome_like_cpp,
    execute_server_triggered_spell_like_cpp,
    execute_spell_with_visual_and_target_data_and_generator_like_cpp,
    execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp,
    normalize_caster_only_stuck_target_data_like_cpp,
    normalize_hearthstone_spell_start_target_data_like_cpp, player_cast_publication_like_cpp,
    publish_player_cast_frame_like_cpp, represented_nearby_entry_destination_like_cpp,
    validate_server_normal_spell_execution_availability_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use execution_session::{
    represented_transport_destination_world_position_like_cpp,
    send_set_viewpoint_target_visibility_like_cpp,
};
pub use login_spell_application::{
    apply_loaded_known_spell_dependencies_like_cpp,
    apply_loaded_spell_dependencies_from_roots_like_cpp,
    apply_loaded_spell_dependency_skills_like_cpp,
    apply_login_known_spell_combat_capabilities_like_cpp,
    apply_represented_first_login_cast_spells_with_catalogs_like_cpp,
    apply_represented_first_login_flag_if_needed_like_cpp,
    apply_represented_start_all_spells_with_catalogs_like_cpp, load_account_mounts_like_cpp,
    promote_loaded_character_mount_spells_like_cpp, set_account_mounts_like_cpp,
    set_known_spells_like_cpp, set_represented_favorite_known_spells_like_cpp,
};
pub use module_hooks::SpellCastHookEventLikeCpp;
pub use player_cast::PlayerCastPublicationPhaseLikeCpp;
pub use player_cast::{
    PhysicalCastItemLikeCpp, ServerNormalCastCostFactsLikeCpp, ServerNormalCastItemUsePlanLikeCpp,
    ServerNormalCastItemUsePlanResultLikeCpp, ServerNormalCooldownFactsLikeCpp,
    ServerNormalCooldownMutationLikeCpp, ServerNormalCooldownPlanLikeCpp,
    ServerNormalCostResultLikeCpp, add_server_normal_global_cooldown_like_cpp,
    apply_server_normal_cooldown_plan_like_cpp, server_normal_cast_item_use_plan_like_cpp,
    server_normal_cooldown_plan_like_cpp, server_normal_cooldown_ready_like_cpp,
    server_normal_cost_plan_like_cpp, server_normal_cost_result_like_cpp,
    server_normal_global_cooldown_state_like_cpp,
    server_normal_take_cast_item_before_effects_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_cast_session::take_ready_player_cast_like_cpp;
pub use player_cast_session::{
    PlayerCastHostLikeCpp, allocate_player_cast_identity_like_cpp,
    handle_cast_spell_with_catalogs_like_cpp, handle_player_cast_publication_like_cpp,
    handle_self_res_with_generator_like_cpp, handle_spell_click_with_generator_like_cpp,
    is_spell_disabled_for_player_like_cpp, next_represented_spell_cast_guid_like_cpp,
    tick_active_spell_cast_with_generator_like_cpp,
};
pub use player_cast_session::{
    cast_server_normal_player_spell_with_generator_like_cpp,
    try_cast_server_normal_player_spell_with_generator_like_cpp,
};
pub use profession::{
    DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP, MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP,
    NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP, PlannedPrimaryProfessionLikeCpp,
    PlayerSkillProfessionSnapshotLikeCpp, PrimaryProfessionCapacityAnalysisLikeCpp,
    PrimaryProfessionCapacityPlanErrorLikeCpp, PrimaryProfessionCapacityPlanLikeCpp,
    PrimaryProfessionEquipmentSlotLikeCpp, PrimaryProfessionSlotNormalizationLikeCpp,
    PrimaryProfessionSlotNormalizationReasonLikeCpp, analyze_primary_professions_like_cpp,
    plan_primary_professions_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use records::canonical_player_spell_runtime_like_cpp;
pub use records::{
    RepresentedCharacterSpellChargeLikeCpp, RepresentedCharacterSpellCooldownLikeCpp,
    RepresentedPlayerSpellLikeCpp, RepresentedPlayerSpellRuntimeLikeCpp,
    RepresentedPlayerSpellStateLikeCpp, RepresentedSpellFocusObjectLikeCpp,
    canonical_player_spell_record_like_cpp, represented_player_spell_record_like_cpp,
    represented_player_spell_runtime_like_cpp,
};
pub use selected_wisp_transform::apply_selected_wisp_transform;
pub use session::{
    AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP, RepresentedShapeshiftMutationLikeCpp,
    SelectedAuraPublicationCatalogs, collect_player_pending_selected_aura_updates,
    player_aura_info_like_cpp, represented_aura_effect_amounts_like_cpp,
    shapeshift_form_of_spell_like_cpp, unit_owned_apply_aura_effect_mask_like_cpp,
};

pub(crate) use aura_session::validate_selected_aura_application;
pub use self_resurrection::{
    SelfResurrectionError, active_spell_cooldown_aura_affects_like_cpp,
    initialize_self_resurrection_spells,
};
pub use spell_acquisition_application::{
    EffectLearningRuntimeLikeCpp, PlayerSpellAcquisitionPersistenceOutcomeLikeCpp,
    PlayerSpellAcquisitionPrepareErrorLikeCpp, PlayerSpellAcquisitionPublicationFaultPointLikeCpp,
    PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp, PlayerSpellAcquisitionRuntimeLikeCpp,
    PreparedPlayerSpellAcquisitionActionsLikeCpp, PreparedPlayerSpellAcquisitionLikeCpp,
    PreparedPlayerSpellAcquisitionOutcomeLikeCpp, apply_base_learning_like_cpp,
    apply_prepared_player_spell_acquisition_actions_like_cpp,
    apply_prepared_player_spell_acquisition_before_save_like_cpp,
    apply_prepared_player_spell_acquisition_like_cpp,
    apply_prepared_player_spell_acquisition_with_before_actions_like_cpp,
    apply_prepared_player_spell_acquisition_with_fault_like_cpp,
    commit_exclusive_player_money_and_spell_acquisition_like_cpp, execute_effect_learning_like_cpp,
    install_prepared_player_spell_acquisition_actions_runtime_like_cpp,
    install_prepared_player_spell_acquisition_runtime_like_cpp,
    may_shallow_fallback_after_profession_plan_error_like_cpp,
    persist_player_spell_acquisition_through_port_like_cpp,
    player_spell_acquisition_persistence_request_like_cpp,
    prepare_player_spell_acquisition_like_cpp, snapshot_has_pending_durable_save_like_cpp,
    validate_prepared_player_spell_acquisition_actions_runtime_like_cpp,
    validate_prepared_player_spell_acquisition_runtime_like_cpp,
};
pub use spell_cast_adapter::present_visual;
pub use spell_click_session::{
    SpellClickHostLikeCpp, represented_handle_spell_click_plan_with_seat_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use spell_click_session::{
    execute_represented_spell_click_plan_with_generator_like_cpp,
    represented_handle_spell_click_plan_like_cpp,
};
pub use spell_history_maintenance::update_player_spell_history;
pub use spell_state_session::{
    SpellStateHostLikeCpp, apply_login_known_spell_proficiencies_like_cpp,
    has_represented_self_res_spell_like_cpp, interrupt_non_melee_spells_for_far_teleport_like_cpp,
    player_persistent_capability_state_snapshot_like_cpp,
    player_resurrection_state_snapshot_like_cpp, player_target_spell_is_hit_inert_like_cpp,
    publish_spell_pull_attack_start_like_cpp, remove_represented_at_login_flag_like_cpp,
    represented_active_glyph_aura_source_is_empty_like_cpp,
    represented_add_player_to_map_aura_source_is_empty_like_cpp,
    represented_cast_speed_multiplier_like_cpp, represented_login_passive_spell_cast_gate_like_cpp,
    represented_player_is_polymorphed_like_cpp,
    represented_update_area_pvp_rule_aura_source_is_empty_like_cpp,
    represented_update_zone_script_aura_source_is_hit_inert_like_cpp,
    resolved_has_represented_aura_effect_like_cpp, resolved_represented_at_login_flags_like_cpp,
    resolved_total_represented_aura_modifier_by_misc_value_like_cpp,
    resolved_total_represented_aura_multiplier_like_cpp,
    set_player_aura_authority_complete_like_cpp, set_represented_at_login_flags_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use spellbook_session::{
    SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP,
    remove_known_spell_with_suppress_messaging_like_cpp,
    replace_loaded_represented_player_spell_rows_like_cpp, resolved_known_spells_like_cpp,
    spell_learn_skill_like_cpp,
};
pub use spellbook_session::{
    SpellbookHostLikeCpp, add_represented_override_spell_like_cpp,
    apply_represented_active_talent_spell_side_effects_like_cpp,
    begin_represented_trait_config_load_like_cpp, clear_represented_fallback_spell_rows_like_cpp,
    clear_represented_trait_and_override_state_like_cpp,
    clear_represented_trait_config_rows_like_cpp, complete_represented_override_spells_like_cpp,
    complete_represented_player_spell_rows_like_cpp,
    complete_represented_spell_trait_definition_ids_like_cpp,
    complete_represented_trait_config_rows_like_cpp, install_loaded_spell_row_like_cpp,
    invalidate_represented_player_spell_rows_like_cpp,
    invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp, known_spells_like_cpp,
    learn_account_mount_spells_like_cpp, learn_dependent_known_spell_like_cpp,
    learn_known_spell_like_cpp, load_represented_talent_row_with_spell_side_effects_like_cpp,
    mark_represented_spell_acquisition_snapshot_complete_like_cpp,
    player_spell_runtime_snapshot_like_cpp, remove_known_spell_like_cpp,
    remove_represented_override_spell_like_cpp, represented_cast_spell_info_like_cpp,
    represented_dependent_known_spells_like_cpp, represented_direct_learn_spell_triggers_like_cpp,
    represented_override_spells_like_cpp, represented_spell_valid_for_learning_like_cpp,
    represented_talent_override_spell_pair_like_cpp, represented_talent_spell_id_like_cpp,
    set_complete_represented_player_spell_rows_like_cpp,
    set_complete_represented_spell_trait_definition_ids_like_cpp,
    take_represented_trait_definition_id_like_cpp, with_player_spell_runtime_like_cpp,
};
pub use state::SessionSpellState;
#[cfg(any(test, feature = "test-fixtures"))]
pub use talent_session::learn_represented_talent_like_cpp;
pub use talent_session::{
    TalentHostLikeCpp, apply_represented_login_talent_reset_if_needed_like_cpp,
    handle_learn_talent, remove_represented_active_talent_side_effects_like_cpp,
    reset_represented_talents_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::PlayerSpellAndTraitTestFixtureLikeCpp;
pub use toy_session::{
    CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP, handle_use_toy_with_generator_like_cpp,
};
