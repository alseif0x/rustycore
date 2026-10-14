//! World-session spell state and its application-owned operations.

pub mod aura_effects;
mod aura_rows;
mod aura_session;
mod effect_session;
mod execution_session;
pub mod login_spell_rules;
pub mod melee_damage;
pub mod melee_rules;
pub mod player_cast;
mod profession;
mod records;
mod session;
mod spell_acquisition;
mod spell_acquisition_application;
pub mod spell_cast_adapter;
mod spell_click_session;
mod spellbook_session;
mod state;
mod talent_session;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;

pub use aura_rows::{CharacterAuraEffectRowLikeCpp, CharacterAuraRowLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_session::update_represented_flight_flags_for_flight_aura_like_cpp;
pub use aura_session::{
    AuraHostLikeCpp, insert_player_visible_aura_like_cpp,
    load_represented_character_auras_like_cpp, next_player_visible_aura_slot_like_cpp,
    remove_auras_with_interrupt_flags_like_cpp,
    remove_indoor_outdoor_auras_for_current_position_represented_like_cpp,
    remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp,
    remove_represented_auras_due_to_spell_like_cpp,
    remove_represented_mounted_auras_by_type_like_cpp,
    represented_spell_area_autocast_source_is_empty_like_cpp,
    represented_trait_config_aura_source_is_empty_like_cpp, send_aura_update_applied, tick_auras,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_session::{
    SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP, apply_represented_mounted_aura_like_cpp,
    apply_represented_provide_spell_focus_aura_like_cpp,
    represented_mount_aura_display_candidates_like_cpp,
    represented_player_aura_state_mask_like_cpp, represented_unit_aura_state_mask_like_cpp,
};
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
    implicit_target_conditions_like_cpp, represented_target_aura_state_mask_like_cpp,
    represented_target_creature_type_mask_like_cpp, represented_target_mechanic_mask_like_cpp,
};
pub use effect_session::{
    SpellEffectHostLikeCpp, apply_block_effect_like_cpp, apply_parry_effect_like_cpp,
    canonical_threatened_by_me_owner_guids_like_cpp, current_map_dungeon_state_like_cpp,
    current_map_is_dungeon_like_cpp, notify_other_players_visibility_changed_like_cpp,
    player_current_map_instanceable_like_cpp, record_spell_execute_log_durability_damage_like_cpp,
    represented_disallowed_mount_form_like_cpp, represented_homebind_like_cpp,
    sync_represented_creature_threat_to_canonical_like_cpp,
};
pub use execution_session::{
    SpellExecutionHostLikeCpp, check_represented_cast_preparation_like_cpp,
    execute_server_triggered_spell_like_cpp,
    execute_spell_with_visual_and_target_data_and_generator_like_cpp,
    execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp,
    player_cast_publication_like_cpp, publish_player_cast_frame_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use execution_session::{
    represented_transport_destination_world_position_like_cpp, take_spell_power_like_cpp,
};
pub use player_cast::PlayerCastPublicationPhaseLikeCpp;
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
pub use session::{
    AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP, RepresentedShapeshiftMutationLikeCpp,
    player_aura_info_like_cpp, represented_aura_effect_amounts_like_cpp,
    shapeshift_form_of_spell_like_cpp, unit_owned_apply_aura_effect_mask_like_cpp,
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
    SpellClickHostLikeCpp, execute_represented_spell_click_clickee_caster_like_cpp,
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
    mark_represented_spell_acquisition_snapshot_complete_like_cpp, remove_known_spell_like_cpp,
    remove_represented_override_spell_like_cpp, represented_cast_spell_info_like_cpp,
    represented_dependent_known_spells_like_cpp, represented_direct_learn_spell_triggers_like_cpp,
    represented_override_spells_like_cpp, represented_spell_valid_for_learning_like_cpp,
    represented_talent_override_spell_pair_like_cpp, represented_talent_spell_id_like_cpp,
    set_complete_represented_player_spell_rows_like_cpp,
    set_complete_represented_spell_trait_definition_ids_like_cpp,
    take_represented_trait_definition_id_like_cpp,
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
