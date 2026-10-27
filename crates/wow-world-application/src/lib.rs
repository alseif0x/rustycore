// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-level operations shared by World adapters.

mod character_account_handlers;
pub use character_account_handlers::{
    CharacterAccountHandlerHostLikeCpp, register_character_account_handlers_like_cpp,
};
mod continue_login;
mod player_cast_session;
mod player_handlers;
mod player_spell_costs;
mod session_lifecycle;
pub use continue_login::{
    ContinueLoginHostLikeCpp, PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP,
    PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP, PlayerLoginHookEventLikeCpp,
    apply_loaded_known_spell_previous_rank_passive_auras_like_cpp,
    apply_login_passive_known_spell_auras_like_cpp, apply_represented_group_leader_flag_like_cpp,
    apply_represented_login_spell_reset_if_needed_like_cpp, gender_from_u8,
    handle_continue_player_login_with_catalogs_like_cpp,
    load_represented_character_titles_like_cpp,
    loaded_inventory_slot_count_with_legacy_rust_compat,
    login_known_spells_after_account_collections_like_cpp, set_loaded_player_identity_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_cast_session::pending_spell_cast_snapshot_like_cpp;
pub use player_cast_session::{
    interrupt_non_melee_spell_cast_for_loot_like_cpp, remaining_active_spell_cast_ms_like_cpp,
    remaining_global_cooldown_ms_like_cpp, request_represented_spell_cast_like_cpp,
    tick_pending_spell_cast_request_with_generator_like_cpp,
};
pub use player_spell_costs::{
    apply_server_normal_cooldown_at_cast_start_like_cpp, server_normal_cast_cost_facts_like_cpp,
    server_normal_cooldown_facts_like_cpp, server_normal_pre_go_like_cpp,
    take_server_normal_cast_item_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use session_lifecycle::{
    AT_LOGIN_RESET_TALENTS_LIKE_CPP, CommittedRepresentedTalentResetLikeCpp,
    UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP, commit_represented_talent_reset_at_like_cpp,
    execute_finalization_step,
};
pub use session_lifecycle::{
    PlayerLogoutHookEventLikeCpp, SessionAccountSaveCxLikeCpp, SessionLifecycleHostLikeCpp,
    cleanup_shared_runtime_state, finalize_session_with_generator_like_cpp,
    finalize_session_with_resurrection_catalogs,
    handle_confirm_respec_wipe_with_generator_like_cpp,
    process_pending_periodic_player_save_with_generator_like_cpp,
    resume_deferred_player_save_with_generator_like_cpp, run_logout_timer_with_catalogs,
};
pub use wow_world_entities::ResurrectionOperationCatalogs;
mod spell_handlers;
mod stand_state_session;
pub use stand_state_session::{
    apply_represented_live_intent_like_cpp, begin_logout_wait_movement_like_cpp,
    restore_logout_wait_movement_like_cpp,
};
mod talent_handlers;
pub use player_handlers::item_purchase_contents_from_extended_cost;
pub use player_handlers::{
    PlayerHandlerCxLikeCpp, PlayerHandlerHostLikeCpp, register_player_handlers_like_cpp,
};
mod creature_damage;
mod inventory_move_planning;
mod inebriation;
pub use inebriation::{PlayerDrunkValueInput, set_player_drunk_value_like_cpp};
pub use creature_damage::{CreatureDamageHostLikeCpp, apply_damage_from_caster_like_cpp};
mod player_items_session;
pub use player_items_session::*;
mod trainer_handlers;
mod trainer_purchase;
mod void_storage_session;
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
mod aura_removal;
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_removal::AuraApplicationFixtureRefsLikeCpp;
pub use aura_removal::{
    AuraApplicationCatalogsLikeCpp, AuraRemovalCxLikeCpp as PlayerAuraApplicationCxLikeCpp,
    plan_item_set_aura_refresh_with_access_like_cpp, player_aura_application_cx_like_cpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub use inventory_swap::InventoryEquipFixtureRefsLikeCpp;
// The exact `ApplicationTalent` owner facade the handler-contract checker
// requires (#1263 F5 deferred families).
pub use spell_handlers::{
    SpellHandlerCxLikeCpp, SpellHandlerHostLikeCpp, register_spell_handlers_like_cpp,
};
pub use spell_handlers::{
    cancel_pending_spell_cast_request_like_cpp, interrupt_player_cast_like_cpp,
    mutate_cast_execution_like_cpp, remove_represented_cancelable_auras_by_effect_like_cpp,
};
pub use talent_handlers::{TalentHandlerHostLikeCpp, register_talent_handlers_like_cpp};
pub use trainer_handlers::{TrainerHandlerHostLikeCpp, register_trainer_handlers_like_cpp};
#[cfg(any(test, feature = "test-fixtures"))]
pub use trainer_purchase::TrainerAcquisitionFixturesLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use trainer_purchase::trainer_offer_decision_like_cpp;
pub use trainer_purchase::{
    AppTrainerBuyAdmissionCxLikeCpp, AppTrainerBuyCx, AppTrainerCx, AppTrainerListCx,
    PreparedBattlePetTrainerOfferLikeCpp, PreparedTrainerOfferLikeCpp,
    TRAINER_BUY_NPC_FLAGS_LIKE_CPP, TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP,
    TRAINER_LIST_NPC_FLAGS_LIKE_CPP, TrainerAcquisitionCatalogsLikeCpp,
    TrainerAcquisitionCompletionLikeCpp, TrainerAcquisitionPublicationLikeCpp,
    TrainerAcquisitionResultLikeCpp, TrainerAcquisitionRuntimeLikeCpp,
    TrainerBattlePetProofLikeCpp, TrainerBuyAdmissionLikeCpp, TrainerBuySpellHostLikeCpp,
    TrainerHiddenReasonLikeCpp, TrainerKnownReasonLikeCpp, TrainerListCatalogsLikeCpp,
    TrainerListOfferResultLikeCpp, TrainerOfferDecisionLikeCpp, TrainerOfferInputLikeCpp,
    TrainerOfferPreflightLikeCpp, TrainerOfferProjectionLikeCpp, TrainerProductLikeCpp,
    TrainerProjectionCatalogsLikeCpp, TrainerSessionFieldsLikeCpp, TrainerSessionHostLikeCpp,
    TrainerUnavailableReasonLikeCpp, decide_trainer_offer_like_cpp,
    execute_trainer_acquisition_like_cpp, finish_trainer_offer_after_projection_like_cpp,
    handle_trainer_buy_spell_with_generator_like_cpp, handle_trainer_list,
    handle_trainer_list_for_gossip_option_like_cpp,
    install_player_spell_acquisition_runtime_snapshot_like_cpp,
    install_represented_spell_acquisition_runtime_like_cpp, prepare_trainer_offer_like_cpp,
    publish_spell_acquisition_action_like_cpp, resolve_creature_trainer_like_cpp,
    trainer_list_required_npc_flags_like_cpp, trainer_price_like_cpp,
    trainer_spell_class_race_fit_like_cpp, trainer_spell_product_like_cpp,
};
pub use void_storage_session::{
    VoidStorageSessionHostLikeCpp, handle_void_storage_query, handle_void_storage_swap_item,
    handle_void_storage_transfer_with_generators_like_cpp,
    handle_void_storage_unlock_with_generator_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use void_storage_session::{
    VoidWithdrawalItemPublicationLikeCpp, add_represented_void_storage_item_like_cpp,
    apply_committed_void_storage_destroyed_items_like_cpp,
    apply_committed_void_storage_unlock_like_cpp,
    apply_effective_void_storage_random_properties_like_cpp,
    clear_item_publication_changes_like_cpp, delete_represented_void_storage_item_like_cpp,
    effective_void_storage_random_properties_like_cpp,
    merged_void_withdrawal_item_publications_like_cpp,
    new_void_withdrawal_item_publication_like_cpp, plan_void_storage_destroyed_items_like_cpp,
    publish_void_storage_item_lifecycle_like_cpp, represented_void_storage_free_slots_like_cpp,
    represented_void_storage_item_at_like_cpp, represented_void_storage_item_by_id_like_cpp,
    represented_void_storage_loaded_like_cpp, void_storage_enchantments_db_string_like_cpp,
    void_storage_merged_item_write_like_cpp, void_storage_swap_destination_slot_like_cpp,
    void_storage_withdrawal_container_db_guid_like_cpp,
    void_storage_withdrawal_container_item_guid_like_cpp,
    void_withdrawal_item_create_data_like_cpp,
};
