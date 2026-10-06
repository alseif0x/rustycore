// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-level operations shared by World adapters.

mod battleground_handlers;
pub mod character_creation;
pub mod character_enumeration;
pub mod character_login_support;
mod client_state;
mod collections_handlers;
mod combat_handlers;
mod data_service_handlers;
mod dungeon_finding_handlers;
mod group_handlers;
mod instances;
mod loot_release;
mod player_conditions;
mod player_handlers;
mod player_save;
mod profession;
mod quest;
mod quest_query_handlers;
mod registry_sync;
mod reputation;
mod spell_acquisition;
pub mod spell_click_values;
mod travel_handlers;
pub mod vendor;
pub use loot_release::{
    LootReleaseCxLikeCpp, direct_item_count_after_loot_release_like_cpp,
    durable_loot_item_fanout_viewers_like_cpp,
    queue_chest_gameobject_state_refresh_for_same_map_like_cpp,
    represented_gameobject_can_autostore_loot_item_like_cpp,
};
pub use player_handlers::item_purchase_contents_from_extended_cost;
pub use player_handlers::{
    PlayerHandlerCxLikeCpp, PlayerHandlerHostLikeCpp, register_player_handlers_like_cpp,
};
mod equipment_set_use;
mod inventory_scaling;
mod inventory_valuation;
mod stats;
mod trainer_purchase;
pub use inventory_scaling::InventoryScalingApplicationCxLikeCpp;
mod inventory_move_planning;
pub use inventory_move_planning::{InventoryMovePlanningCxLikeCpp, InventorySwapTargetLikeCpp};
mod inventory_swap;
pub use inventory_swap::{
    InventoryCommittedRelocationCxLikeCpp, InventoryCommittedSwapCxLikeCpp,
    InventoryEquipCxLikeCpp, InventoryPositionPublicationCxLikeCpp, InventorySwapEffectsCxLikeCpp,
    bind_inventory_item_for_destination_like_cpp, item_dynamic_flags_changed_like_cpp,
    item_spell_charges_db_string, item_storage_mutable_persistence_like_cpp,
};
mod bank;
pub use bank::{
    BankHandlerHostLikeCpp, BankSlotFlagApplicationCxLikeCpp,
    can_use_current_bank_with_access_like_cpp, register_bank_handlers_like_cpp,
};
mod aura_removal;
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_removal::AuraApplicationFixtureRefsLikeCpp;
pub use aura_removal::{
    AuraApplicationCatalogsLikeCpp, AuraRemovalCxLikeCpp as PlayerAuraApplicationCxLikeCpp,
    plan_item_set_aura_refresh_with_access_like_cpp,
};

pub use battleground_handlers::{
    BattlegroundHandlerCxLikeCpp, BattlegroundHandlerHostLikeCpp,
    register_battleground_handlers_like_cpp,
};
pub use client_state::{
    ClientStateHandlerCxLikeCpp, ClientStateHandlerHostLikeCpp,
    register_client_state_handlers_like_cpp,
};
pub use collections_handlers::{
    CollectionsHandlerCxLikeCpp, CollectionsHandlerHostLikeCpp,
    register_collections_handlers_like_cpp,
};
pub use combat_handlers::{
    CombatHandlerCxLikeCpp, CombatHandlerHostLikeCpp, register_combat_handlers_like_cpp,
};
pub use data_service_handlers::{
    DataServiceHandlerCxLikeCpp, DataServiceHandlerHostLikeCpp,
    register_data_service_handlers_like_cpp,
};
pub use dungeon_finding_handlers::{
    DungeonFindingHandlerCxLikeCpp, DungeonFindingHandlerHostLikeCpp,
    register_dungeon_finding_handlers_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use equipment_set_use::EquipmentSetUseFixtureRefsLikeCpp;
pub use equipment_set_use::{
    EquipmentSetUseContextLikeCpp, EquipmentSetUseHandlerHostLikeCpp,
    EquipmentSetUseItemModsStoresLikeCpp, register_equipment_set_use_handler_like_cpp,
};
pub use group_handlers::GroupPublicationTailLikeCpp;
pub use group_handlers::{
    GroupHandlerCxLikeCpp, GroupHandlerHostLikeCpp, register_group_handlers_like_cpp,
};
pub use instances::{
    InstanceDifficultyHandlerCxLikeCpp, InstanceLockOperationsHandlerCxLikeCpp,
    InstanceLockResponseOutcomeLikeCpp, InstanceRaidInfoHandlerCxLikeCpp,
    InstanceResetMethodLikeCpp, InstancesHandlerHostLikeCpp,
    handle_instance_lock_response_like_cpp, handle_request_raid_info_like_cpp,
    handle_reset_instances_like_cpp, handle_set_difficulty_id_like_cpp,
    handle_set_dungeon_difficulty_like_cpp, handle_set_raid_difficulty_like_cpp,
    handle_set_saved_instance_extend_like_cpp, handle_toggle_difficulty_like_cpp,
    register_instance_handlers_like_cpp, reset_represented_instances_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use inventory_swap::InventoryEquipFixtureRefsLikeCpp;
pub use inventory_valuation::{
    can_equip_inventory_item_like_cpp, can_equip_unique_item_like_cpp,
    can_use_inventory_item_represented_with_loading_like_cpp,
};
pub use player_conditions::{
    PlayerConditionProjectionCxLikeCpp, PlayerConditionProjectionInputsLikeCpp,
    RepresentedPlayerConditionContextLikeCpp,
};
pub use player_save::{
    PlayerSavePersistenceResultLikeCpp, apply_player_save_acknowledgement_like_cpp,
    capture_player_save_request_like_cpp, persist_player_save_request_like_cpp,
    save_canonical_player_like_cpp,
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
pub use quest::QuestEligibilityCx;
pub use quest::QuestRewardDurablePlanLikeCpp;
pub use quest::add_currency_quest_reward_like_cpp;
pub use quest::represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp;
pub use quest::{
    MAX_QUEST_LOG_SIZE_LIKE_CPP, QuestRewardCommitCx, QuestRewardCx,
    RepresentedCanSeeSpellClickOutcomeLikeCpp, RepresentedPendingQuestSharingLikeCpp,
    RepresentedPushQuestToPartyOutcomeLikeCpp, RepresentedPushQuestToPartyOutcomeReasonLikeCpp,
    RepresentedQuestCompleteStatusUpdateLikeCpp, RepresentedQuestConfirmAcceptLikeCpp,
    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp, SessionQuestState,
    begin_exclusive_player_money_persistence_like_cpp, find_quest_slot_like_cpp,
    get_quest_slot_quest_id_like_cpp, plan_quest_status_save_like_cpp,
    quest_log_create_entries_like_cpp, reconcile_durable_loot_money_before_save_like_cpp,
    represented_can_see_spell_click_on_like_cpp,
    represented_viewer_dependent_creature_npc_flags_like_cpp, save_changed_quest_statuses_like_cpp,
    save_quest_to_db_like_cpp, send_represented_quest_log_slot_update_like_cpp,
};
pub use quest::{QuestDialogClassificationLikeCpp, RepresentedQuestGiverStatusSourceLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
pub use quest::{
    QuestObjectiveRegistryFixtureRefsLikeCpp, QuestRewardItemPlanningFixtureRefsLikeCpp,
    QuestRewardReputationFixtureRefsLikeCpp, QuestXpGainFixtureRefsLikeCpp,
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
pub use quest::{
    RepresentedQuestCompleteDialogLikeCpp, represented_quest_complete_dialog_like_cpp,
    represented_quest_has_item_objective_like_cpp, represented_quest_rewards_block_like_cpp,
};
pub use quest::{
    represented_gameobject_activate_to_quest_like_cpp,
    represented_gameobject_dynamic_flags_for_player_like_cpp,
    represented_gameobject_is_for_quests_like_cpp, represented_has_quest_for_gameobject_like_cpp,
    represented_meets_player_condition_id_like_cpp,
};
pub use quest_query_handlers::represented_quest_completion_npc_response_like_cpp;
pub use quest_query_handlers::{
    QuestQueryHandlerCxLikeCpp, QuestQueryHandlerHostLikeCpp,
    register_quest_query_handlers_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use registry_sync::PlayerRegistryHydrationContext;
pub use registry_sync::PlayerRegistrySyncContext;
pub use reputation::{
    ReputationHandlerCxLikeCpp, ReputationHandlerHostLikeCpp, register_reputation_handlers_like_cpp,
};
pub use spell_acquisition::{
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
pub use stats::{
    CharacterStatsApplicationCxLikeCpp, level_up_stat_deltas_like_cpp, max_health_u32_like_cpp,
    primary_max_power_for_class_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use trainer_purchase::TrainerAcquisitionFixturesLikeCpp;
pub use trainer_purchase::{
    AppTrainerBuyAdmissionCxLikeCpp, AppTrainerBuyCx, AppTrainerCx, AppTrainerListCx,
    PreparedBattlePetTrainerOfferLikeCpp, PreparedTrainerOfferLikeCpp,
    TRAINER_BUY_NPC_FLAGS_LIKE_CPP, TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP,
    TRAINER_LIST_NPC_FLAGS_LIKE_CPP, TrainerAcquisitionCatalogsLikeCpp,
    TrainerAcquisitionCompletionLikeCpp, TrainerAcquisitionPublicationLikeCpp,
    TrainerAcquisitionResultLikeCpp, TrainerAcquisitionRuntimeLikeCpp,
    TrainerAdmissionProofLikeCpp, TrainerBattlePetProofLikeCpp, TrainerBuyAdmissionLikeCpp,
    TrainerHiddenReasonLikeCpp, TrainerKnownReasonLikeCpp, TrainerListCatalogsLikeCpp,
    TrainerListOfferResultLikeCpp, TrainerOfferDecisionLikeCpp, TrainerOfferInputLikeCpp,
    TrainerOfferPreflightLikeCpp, TrainerOfferProjectionLikeCpp, TrainerProductLikeCpp,
    TrainerProjectionCatalogsLikeCpp, TrainerUnavailableReasonLikeCpp,
    decide_trainer_offer_like_cpp, execute_trainer_acquisition_like_cpp,
    finish_trainer_offer_after_projection_like_cpp,
    install_player_spell_acquisition_runtime_snapshot_like_cpp,
    install_represented_spell_acquisition_runtime_like_cpp, prepare_trainer_offer_like_cpp,
    publish_spell_acquisition_action_like_cpp, resolve_creature_trainer_like_cpp,
    trainer_condition_admission_proof_like_cpp, trainer_list_required_npc_flags_like_cpp,
    trainer_price_like_cpp, trainer_spell_class_race_fit_like_cpp, trainer_spell_product_like_cpp,
};
pub use travel_handlers::{
    TravelHandlerCxLikeCpp, TravelHandlerHostLikeCpp, register_travel_handlers_like_cpp,
};
