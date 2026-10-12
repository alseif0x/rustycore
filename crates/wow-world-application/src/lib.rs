// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-level operations shared by World adapters.

mod battle_pet_handlers;
pub use battle_pet_handlers::{
    BattlePetHandlerCxLikeCpp, BattlePetHandlerHostLikeCpp, register_battle_pet_handlers_like_cpp,
};
pub use battle_pet_handlers::{
    has_represented_battle_pet_journal_lock_like_cpp, represented_battle_pet_journal_like_cpp,
    represented_battle_pet_like_cpp, send_battle_pet_journal_lock_status_like_cpp,
};
mod character_account_handlers;
pub mod character_creation;
pub mod character_enumeration;
pub use character_account_handlers::{
    CharacterAccountHandlerHostLikeCpp, register_character_account_handlers_like_cpp,
};
mod character_handler_families;
mod character_handlers;
pub mod character_login_support;
mod character_query_handlers;
mod collections_handlers;
mod loot_handlers;
mod loot_release;
mod player_conditions;
mod player_handlers;
mod profession;
mod quest;
mod quest_handlers;
mod quest_query_handlers;
mod registry_sync;
mod spell_acquisition;
mod spell_handlers;
mod talent_handlers;
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
mod trainer_handlers;
mod trainer_purchase;
pub use inventory_scaling::InventoryScalingApplicationCxLikeCpp;
mod inventory_move_planning;
pub use inventory_move_planning::{InventoryMovePlanningCxLikeCpp, InventorySwapTargetLikeCpp};
mod inventory_swap;
pub use inventory_swap::{
    InventoryCommittedRelocationCxLikeCpp, InventoryCommittedSwapCxLikeCpp,
    InventoryEquipCxLikeCpp, InventoryPositionPublicationCxLikeCpp, InventoryStorageMoveCxLikeCpp,
    InventoryStorageMoveHostLikeCpp, InventoryStorageMoveStatsCxLikeCpp,
    InventoryStorageQuestChecksLikeCpp, InventoryStorageTargetLikeCpp,
    InventorySwapEffectsCxLikeCpp, autostore_bank_target_like_cpp,
    bank_store_destination_applies_obtain_spells_like_cpp,
    bank_store_item_added_quest_count_like_cpp, bind_inventory_item_for_destination_like_cpp,
    execute_inventory_storage_move_like_cpp, inventory_storage_move_quest_directions_like_cpp,
    item_dynamic_flags_changed_like_cpp, item_spell_charges_db_string,
    item_storage_mutable_persistence_like_cpp, plan_inventory_storage_move_like_cpp,
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
    plan_item_set_aura_refresh_with_access_like_cpp, player_aura_application_cx_like_cpp,
};

// The exact `ApplicationCharacter` owner facade the handler-contract checker
// requires: the three canonical exports as one use item. The remaining
// application-level re-exports of the same owner module follow separately so
// the facade item stays exact.
pub use character_handlers::{
    CharDeleteStepLikeCpp, CharRenameStepLikeCpp, ConnectToFailedStepLikeCpp,
    CreateCharacterStepLikeCpp, PlayerLoginStepLikeCpp, initial_character_rest_state_like_cpp,
    parse_equipment_cache, send_represented_cinematic_start_like_cpp,
};
pub use character_handlers::{
    CharacterHandlerCxLikeCpp, CharacterHandlerHostLikeCpp, register_character_handlers_like_cpp,
};
pub use character_query_handlers::realm_query_response_like_cpp;
pub use character_query_handlers::{
    CharacterQueryHandlerCxLikeCpp, CharacterQueryHandlerHostLikeCpp,
    register_character_query_handlers_like_cpp,
};
pub use collections_handlers::{
    CollectionsHandlerCxLikeCpp, CollectionsHandlerHostLikeCpp,
    register_collections_handlers_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use equipment_set_use::EquipmentSetUseFixtureRefsLikeCpp;
pub use equipment_set_use::{
    EquipmentSetUseContextLikeCpp, EquipmentSetUseHandlerHostLikeCpp,
    EquipmentSetUseItemModsStoresLikeCpp, register_equipment_set_use_handler_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use inventory_swap::InventoryEquipFixtureRefsLikeCpp;
pub use inventory_valuation::{
    can_equip_inventory_item_like_cpp, can_equip_unique_item_like_cpp,
    can_use_inventory_item_represented_with_loading_like_cpp,
};
pub use loot_handlers::{
    LootHandlerCxLikeCpp, LootHandlerHostLikeCpp, register_loot_handlers_like_cpp,
};
pub use player_conditions::{
    PlayerConditionProjectionCxLikeCpp, PlayerConditionProjectionInputsLikeCpp,
    RepresentedPlayerConditionContextLikeCpp, meets_player_condition_id_like_cpp,
    player_condition_projection_cx_like_cpp,
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
    MAX_QUEST_LOG_SIZE_LIKE_CPP, QuestObjectiveProgressCx, QuestRewardCommitCx, QuestRewardCx,
    RepresentedPendingQuestSharingLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestCompleteStatusUpdateLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp, SessionQuestState,
    begin_exclusive_player_money_persistence_like_cpp, find_quest_slot_like_cpp,
    get_quest_slot_quest_id_like_cpp, invalidate_player_quest_status_authority_like_cpp,
    mutate_and_persist_player_gold_exclusive_like_cpp, plan_quest_status_save_like_cpp,
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
    VisibilityRefreshCxLikeCpp, VisibilityRefreshHostLikeCpp,
    represented_gameobject_activate_to_quest_like_cpp,
    represented_gameobject_dynamic_flags_for_player_like_cpp,
    represented_gameobject_dynamic_flags_update_like_cpp,
    represented_gameobject_go_state_for_viewer_like_cpp,
    represented_gameobject_is_for_quests_like_cpp, represented_has_quest_for_gameobject_like_cpp,
    represented_meets_player_condition_id_like_cpp, update_visible_gameobjects_like_cpp,
    update_visible_gameobjects_or_spell_clicks_like_cpp, update_visible_spell_clicks_like_cpp,
};
pub use quest::{
    clear_represented_pending_quest_sharing_like_cpp, mutate_player_quest_gameplay_like_cpp,
    player_quest_gameplay_snapshot_like_cpp, represented_pending_quest_sharing_like_cpp,
};
pub use quest_query_handlers::QuestPushResultTailLikeCpp;
pub use quest_query_handlers::represented_quest_completion_npc_response_like_cpp;
pub use quest_query_handlers::{
    QuestQueryHandlerCxLikeCpp, QuestQueryHandlerHostLikeCpp,
    register_quest_query_handlers_like_cpp,
};
// The exact `ApplicationQuest` owner facade the handler-contract checker
// requires: the two canonical exports as one use item (#1263 F5 remaining
// families).
pub use quest_handlers::{QuestHandlerHostLikeCpp, register_quest_handlers_like_cpp};
// The exact `ApplicationTalent` owner facade the handler-contract checker
// requires (#1263 F5 deferred families).
pub use talent_handlers::{TalentHandlerHostLikeCpp, register_talent_handlers_like_cpp};
// The moved quest-completion body and the two quest helpers it owns are exported
// as a separate item so the owner's exact three-name facade above stays unchanged.
pub use quest_query_handlers::{
    QuestGiverCompleteQuestHostLikeCpp, handle_quest_giver_complete_quest_like_cpp,
    quest_giver_creature_id_from_source_like_cpp, quest_is_disabled_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use registry_sync::PlayerRegistryHydrationContext;
pub use registry_sync::PlayerRegistrySyncContext;
pub use registry_sync::sync_player_registry_state_like_cpp;
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
pub use spell_handlers::{
    SpellHandlerCxLikeCpp, SpellHandlerHostLikeCpp, register_spell_handlers_like_cpp,
};
pub use spell_handlers::{
    cancel_pending_spell_cast_request_like_cpp, interrupt_player_cast_like_cpp,
    mutate_cast_execution_like_cpp, remove_represented_cancelable_auras_by_effect_like_cpp,
};
pub use trainer_handlers::{TrainerHandlerHostLikeCpp, register_trainer_handlers_like_cpp};
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
    TrainerBuySpellHostLikeCpp, TrainerHiddenReasonLikeCpp, TrainerKnownReasonLikeCpp,
    TrainerListCatalogsLikeCpp, TrainerListOfferResultLikeCpp, TrainerOfferDecisionLikeCpp,
    TrainerOfferInputLikeCpp, TrainerOfferPreflightLikeCpp, TrainerOfferProjectionLikeCpp,
    TrainerProductLikeCpp, TrainerProjectionCatalogsLikeCpp, TrainerUnavailableReasonLikeCpp,
    decide_trainer_offer_like_cpp, execute_trainer_acquisition_like_cpp,
    finish_trainer_offer_after_projection_like_cpp,
    handle_trainer_buy_spell_with_generator_like_cpp,
    install_player_spell_acquisition_runtime_snapshot_like_cpp,
    install_represented_spell_acquisition_runtime_like_cpp, prepare_trainer_offer_like_cpp,
    publish_spell_acquisition_action_like_cpp, resolve_creature_trainer_like_cpp,
    trainer_condition_admission_proof_like_cpp, trainer_list_required_npc_flags_like_cpp,
    trainer_price_like_cpp, trainer_spell_class_race_fit_like_cpp, trainer_spell_product_like_cpp,
};
