//! World-session spell state and its application-owned operations.

pub mod aura_effects;
mod aura_rows;
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
mod state;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;

pub use aura_rows::{CharacterAuraEffectRowLikeCpp, CharacterAuraRowLikeCpp};
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
pub use state::SessionSpellState;
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::PlayerSpellAndTraitTestFixtureLikeCpp;
