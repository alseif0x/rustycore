// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-level operations shared by World adapters.

mod character_account_handlers;
pub use character_account_handlers::{
    CharacterAccountHandlerHostLikeCpp, register_character_account_handlers_like_cpp,
};
mod player_handlers;
mod spell_handlers;
mod talent_handlers;
pub use player_handlers::item_purchase_contents_from_extended_cost;
pub use player_handlers::{
    PlayerHandlerCxLikeCpp, PlayerHandlerHostLikeCpp, register_player_handlers_like_cpp,
};
mod equipment_set_use;
mod inventory_scaling;
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
mod aura_removal;
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_removal::AuraApplicationFixtureRefsLikeCpp;
pub use aura_removal::{
    AuraApplicationCatalogsLikeCpp, AuraRemovalCxLikeCpp as PlayerAuraApplicationCxLikeCpp,
    plan_item_set_aura_refresh_with_access_like_cpp, player_aura_application_cx_like_cpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub use equipment_set_use::EquipmentSetUseFixtureRefsLikeCpp;
pub use equipment_set_use::{
    EquipmentSetUseContextLikeCpp, EquipmentSetUseHandlerHostLikeCpp,
    EquipmentSetUseItemModsStoresLikeCpp, register_equipment_set_use_handler_like_cpp,
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
    TrainerProjectionCatalogsLikeCpp, TrainerUnavailableReasonLikeCpp,
    decide_trainer_offer_like_cpp, execute_trainer_acquisition_like_cpp,
    finish_trainer_offer_after_projection_like_cpp,
    handle_trainer_buy_spell_with_generator_like_cpp,
    install_player_spell_acquisition_runtime_snapshot_like_cpp,
    install_represented_spell_acquisition_runtime_like_cpp, prepare_trainer_offer_like_cpp,
    publish_spell_acquisition_action_like_cpp, resolve_creature_trainer_like_cpp,
    trainer_list_required_npc_flags_like_cpp, trainer_price_like_cpp,
    trainer_spell_class_race_fit_like_cpp, trainer_spell_product_like_cpp,
};
