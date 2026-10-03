// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-level operations shared by World adapters.

mod profession;
mod quest;
mod registry_sync;
mod spell_acquisition;
mod instances;
mod stats;
mod trainer_purchase;

pub use instances::{
    handle_instance_lock_response_like_cpp, handle_request_raid_info_like_cpp,
    handle_reset_instances_like_cpp, handle_set_difficulty_id_like_cpp,
    handle_set_dungeon_difficulty_like_cpp, handle_set_raid_difficulty_like_cpp,
    handle_set_saved_instance_extend_like_cpp, handle_toggle_difficulty_like_cpp,
    register_instance_handlers_like_cpp, reset_represented_instances_like_cpp,
    InstanceDifficultyHandlerCxLikeCpp, InstanceLockOperationsHandlerCxLikeCpp,
    InstanceLockResponseOutcomeLikeCpp, InstanceRaidInfoHandlerCxLikeCpp,
    InstanceResetMethodLikeCpp, InstancesHandlerHostLikeCpp,
};
pub use stats::{
    level_up_stat_deltas_like_cpp, max_health_u32_like_cpp,
    primary_max_power_for_class_like_cpp, CharacterStatsApplicationCxLikeCpp,
};
pub use registry_sync::PlayerRegistrySyncContext;
#[cfg(any(test, feature = "test-fixtures"))]
pub use registry_sync::PlayerRegistryHydrationContext;
pub use quest::QuestRewardDurablePlanLikeCpp;
pub use quest::{
    QuestRewardCommitCx, begin_exclusive_player_money_persistence_like_cpp,
    reconcile_durable_loot_money_before_save_like_cpp,
    RepresentedPendingQuestSharingLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestCompleteStatusUpdateLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp, SessionQuestState,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use quest::{
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
pub use profession::{
    DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP, MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP,
    NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP, PrimaryProfessionCapacityAnalysisLikeCpp,
    PrimaryProfessionCapacityPlanErrorLikeCpp, PrimaryProfessionCapacityPlanLikeCpp,
    PrimaryProfessionEquipmentSlotLikeCpp, PrimaryProfessionSlotNormalizationLikeCpp,
    PrimaryProfessionSlotNormalizationReasonLikeCpp, PlannedPrimaryProfessionLikeCpp,
    PlayerSkillProfessionSnapshotLikeCpp, analyze_primary_professions_like_cpp,
    plan_primary_professions_like_cpp,
};
pub use spell_acquisition::{
    PlayerSpellAcquisitionPersistenceOutcomeLikeCpp,
    PlayerSpellAcquisitionPublicationFaultPointLikeCpp,
    PlayerSpellAcquisitionPrepareErrorLikeCpp, PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp,
    PlayerSpellAcquisitionRuntimeLikeCpp, PreparedPlayerSpellAcquisitionActionsLikeCpp,
    PreparedPlayerSpellAcquisitionLikeCpp, PreparedPlayerSpellAcquisitionOutcomeLikeCpp,
    apply_prepared_player_spell_acquisition_actions_like_cpp,
    apply_prepared_player_spell_acquisition_before_save_like_cpp,
    apply_prepared_player_spell_acquisition_like_cpp,
    apply_prepared_player_spell_acquisition_with_before_actions_like_cpp,
    apply_prepared_player_spell_acquisition_with_fault_like_cpp,
    commit_exclusive_player_money_and_spell_acquisition_like_cpp,
    install_prepared_player_spell_acquisition_actions_runtime_like_cpp,
    install_prepared_player_spell_acquisition_runtime_like_cpp,
    persist_player_spell_acquisition_through_port_like_cpp,
    player_spell_acquisition_persistence_request_like_cpp,
    prepare_player_spell_acquisition_like_cpp,
    snapshot_has_pending_durable_save_like_cpp,
    validate_prepared_player_spell_acquisition_actions_runtime_like_cpp,
    validate_prepared_player_spell_acquisition_runtime_like_cpp,
};
pub use trainer_purchase::{
    AppTrainerCx, PreparedTrainerOfferLikeCpp, TrainerAcquisitionCatalogsLikeCpp,
    TrainerAcquisitionCompletionLikeCpp,
    TrainerAcquisitionPublicationLikeCpp, TrainerAcquisitionResultLikeCpp,
    TrainerAcquisitionRuntimeLikeCpp, execute_trainer_acquisition_like_cpp,
    install_player_spell_acquisition_runtime_snapshot_like_cpp,
    install_represented_spell_acquisition_runtime_like_cpp,
    publish_spell_acquisition_action_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use trainer_purchase::TrainerAcquisitionFixturesLikeCpp;
