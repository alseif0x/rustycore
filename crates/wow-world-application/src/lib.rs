// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-level operations shared by World adapters.

mod profession;
mod player_conditions;
mod quest;
mod player_save;
mod registry_sync;
mod spell_acquisition;
mod instances;
mod stats;
mod equipment_set_use;
mod trainer_purchase;
mod inventory_valuation;
mod inventory_scaling;
pub use inventory_scaling::InventoryScalingApplicationCxLikeCpp;
mod inventory_move_planning;
pub use inventory_move_planning::{InventoryMovePlanningCxLikeCpp, InventorySwapTargetLikeCpp};
mod inventory_swap;
pub use inventory_swap::{
    InventoryCommittedRelocationCxLikeCpp, InventoryCommittedSwapCxLikeCpp, InventoryEquipCxLikeCpp,
    InventoryPositionPublicationCxLikeCpp, InventorySwapEffectsCxLikeCpp,
    bind_inventory_item_for_destination_like_cpp, item_dynamic_flags_changed_like_cpp,
    item_spell_charges_db_string, item_storage_mutable_persistence_like_cpp,
};
mod bank;
pub use bank::{
    can_use_current_bank_with_access_like_cpp, BankSlotFlagApplicationCxLikeCpp,
    BankHandlerHostLikeCpp, register_bank_handlers_like_cpp,
};
mod aura_removal;
pub use aura_removal::{
    AuraApplicationCatalogsLikeCpp, AuraRemovalCxLikeCpp as PlayerAuraApplicationCxLikeCpp,
    plan_item_set_aura_refresh_with_access_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_removal::AuraApplicationFixtureRefsLikeCpp;

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
pub use registry_sync::PlayerRegistrySyncContext;
pub use player_save::{
    PlayerSavePersistenceResultLikeCpp, apply_player_save_acknowledgement_like_cpp,
    capture_player_save_request_like_cpp, save_canonical_player_like_cpp,
    persist_player_save_request_like_cpp,
};
pub use player_conditions::{
    PlayerConditionProjectionCxLikeCpp, PlayerConditionProjectionInputsLikeCpp,
    RepresentedPlayerConditionContextLikeCpp,
};
pub use equipment_set_use::{
    register_equipment_set_use_handler_like_cpp, EquipmentSetUseContextLikeCpp,
    EquipmentSetUseHandlerHostLikeCpp, EquipmentSetUseItemModsStoresLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use equipment_set_use::EquipmentSetUseFixtureRefsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use registry_sync::PlayerRegistryHydrationContext;
pub use quest::QuestRewardDurablePlanLikeCpp;
pub use quest::QuestEligibilityCx;
pub use quest::{QuestDialogClassificationLikeCpp, RepresentedQuestGiverStatusSourceLikeCpp};
pub use quest::add_currency_quest_reward_like_cpp;
pub use quest::represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp;
pub use quest::{
    QuestRewardCommitCx, QuestRewardCx, begin_exclusive_player_money_persistence_like_cpp,
    RepresentedCanSeeSpellClickOutcomeLikeCpp, represented_can_see_spell_click_on_like_cpp,
    represented_viewer_dependent_creature_npc_flags_like_cpp,
    find_quest_slot_like_cpp, plan_quest_status_save_like_cpp,
    get_quest_slot_quest_id_like_cpp, quest_log_create_entries_like_cpp,
    send_represented_quest_log_slot_update_like_cpp,
    save_changed_quest_statuses_like_cpp,
    save_quest_to_db_like_cpp,
    MAX_QUEST_LOG_SIZE_LIKE_CPP,
    reconcile_durable_loot_money_before_save_like_cpp,
    RepresentedPendingQuestSharingLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestCompleteStatusUpdateLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp, SessionQuestState,
};
pub use stats::{
    level_up_stat_deltas_like_cpp, max_health_u32_like_cpp,
    primary_max_power_for_class_like_cpp, CharacterStatsApplicationCxLikeCpp,
};
pub use inventory_valuation::{
    can_equip_inventory_item_like_cpp, can_equip_unique_item_like_cpp,
    can_use_inventory_item_represented_with_loading_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use quest::{
    QuestRewardItemPlanningFixtureRefsLikeCpp,
    QuestRewardReputationFixtureRefsLikeCpp,
    QuestObjectiveRegistryFixtureRefsLikeCpp,
    QuestXpGainFixtureRefsLikeCpp,
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use inventory_swap::InventoryEquipFixtureRefsLikeCpp;
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
    AppTrainerBuyCx, AppTrainerBuyAdmissionCxLikeCpp, AppTrainerCx, AppTrainerListCx,
    TrainerListCatalogsLikeCpp,
    TrainerProjectionCatalogsLikeCpp,
    TrainerListOfferResultLikeCpp,
    PreparedBattlePetTrainerOfferLikeCpp,
    PreparedTrainerOfferLikeCpp,
    TrainerAcquisitionCatalogsLikeCpp, TrainerAdmissionProofLikeCpp,
    TrainerAcquisitionCompletionLikeCpp,
    TrainerAcquisitionPublicationLikeCpp, TrainerAcquisitionResultLikeCpp,
    TrainerBattlePetProofLikeCpp, TrainerHiddenReasonLikeCpp, TrainerKnownReasonLikeCpp,
    TrainerOfferDecisionLikeCpp, TrainerOfferInputLikeCpp, TrainerOfferPreflightLikeCpp,
    TrainerOfferProjectionLikeCpp, TrainerProductLikeCpp,
    TrainerUnavailableReasonLikeCpp, TrainerBuyAdmissionLikeCpp,
    decide_trainer_offer_like_cpp,
    finish_trainer_offer_after_projection_like_cpp, prepare_trainer_offer_like_cpp,
    trainer_condition_admission_proof_like_cpp,
    resolve_creature_trainer_like_cpp,
    trainer_list_required_npc_flags_like_cpp,
    trainer_spell_class_race_fit_like_cpp, trainer_spell_product_like_cpp,
    TRAINER_BUY_NPC_FLAGS_LIKE_CPP, TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP,
    TRAINER_LIST_NPC_FLAGS_LIKE_CPP,
    TrainerAcquisitionRuntimeLikeCpp, execute_trainer_acquisition_like_cpp,
    install_player_spell_acquisition_runtime_snapshot_like_cpp,
    install_represented_spell_acquisition_runtime_like_cpp, publish_spell_acquisition_action_like_cpp,
    trainer_price_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use trainer_purchase::TrainerAcquisitionFixturesLikeCpp;
