// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session-owned inventory state and its bounded represented contracts.

mod account_collection_loading;
mod appearance;
mod auction_contracts;
mod bank;
mod bank_handlers;
mod battle_pet_handlers;
mod battle_pet_purchase;
mod battle_pet_session;
mod buyback;
mod catalog;
mod collections_handlers;
mod contracts;
mod currency;
mod durability;
mod enchantment;
mod equipment;
mod equipment_set_use;
mod equipment_set_use_handlers;
mod equipment_sets;
mod equipment_slots;
mod guild_inventory_contracts;
mod handlers;
mod inventory_move_contracts;
mod inventory_request_contracts;
mod inventory_scaling;
mod item_modifiers;
mod item_session;
mod item_sets;
mod items;
mod limit_category;
mod modifiers;
mod money;
mod new_item_push;
mod offhand;
mod persistence;
mod persistence_load;
mod player_items_session;
mod publication;
pub use new_item_push::send_new_item_plan;
mod currency_session;
mod quest_reward;
mod registry_sync;
mod scaling;
mod state;
mod stats;
mod stats_application;
mod storage;
mod storage_bags;
mod storage_slots;
mod toy_session;
mod turnins;
mod valuation;
pub use currency_session::{
    add_currency_item_refund, has_currency, player_currency_quantity,
    represented_set_currency_flags_like_cpp, setup_currencies_packet_like_cpp,
    stage_player_money_change_like_cpp,
};
mod values_update_session;
pub use values_update_session::{
    player_values_update_snapshot, send_player_values_update_from_entity_bridge,
};
mod vendor_session;
mod void_storage;
mod void_storage_handlers;
mod void_transfer;
mod void_transfer_contracts;

#[cfg(any(test, feature = "test-fixtures"))]
mod collection_adapter;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
pub mod loaded_item_support;
pub mod vendor;

pub use account_collection_loading::{
    load_account_item_appearances_like_cpp, load_account_transmog_illusions_like_cpp,
};
pub use auction_contracts::{
    RepresentedAuctionPlaceBidLikeCpp, RepresentedAuctionRemoveItemLikeCpp,
    RepresentedAuctionReplicateRequestLikeCpp, RepresentedAuctionSellItemLikeCpp,
};
// The exact bank owner facade the handler-contract checker requires.
pub use bank_handlers::{
    BankHandlerHostLikeCpp, BankSlotFlagApplicationCxLikeCpp,
    can_use_current_bank_with_access_like_cpp, register_bank_handlers_like_cpp,
};
pub use battle_pet_handlers::{
    BattlePetHandlerCxLikeCpp, BattlePetHandlerHostLikeCpp, register_battle_pet_handlers_like_cpp,
};
pub use battle_pet_handlers::{
    has_represented_battle_pet_journal_lock_like_cpp, represented_battle_pet_journal_like_cpp,
    represented_battle_pet_like_cpp, send_battle_pet_journal_lock_status_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use battle_pet_purchase::BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP;
pub use battle_pet_purchase::{
    BattlePetPurchaseAdmissionFailureLikeCpp, BattlePetPurchaseExecutionLikeCpp,
    BattlePetPurchaseHostLikeCpp, BattlePetPurchaseRecoveryLikeCpp,
    PreparedBattlePetTrainerOfferLikeCpp,
    execute_battle_pet_trainer_purchase_with_generator_like_cpp,
    recover_battle_pet_trainer_purchases_with_generator_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use battle_pet_session::battle_pet_count_like_cpp;
pub use battle_pet_session::{
    BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP, BattlePetSessionHostLikeCpp,
    MAX_BATTLE_PET_LEVEL_LIKE_CPP, RepresentedBattlePetGrantExperienceOutcomeLikeCpp,
    RepresentedBattlePetGrantLevelOutcomeLikeCpp, RepresentedBattlePetQualityOutcomeLikeCpp,
    RepresentedBattlePetXpSourceLikeCpp, apply_battle_pet_calculated_stats_like_cpp,
    apply_grant_battle_pet_experience_effect_like_cpp,
    apply_grant_battle_pet_level_effect_like_cpp, apply_uncage_battle_pet_effect_like_cpp,
    battle_pet_account_owner_lease_like_cpp, battle_pet_add_request_committed_like_cpp,
    battle_pet_calculate_stats_like_cpp, battle_pet_grant_experience_durable_like_cpp,
    battle_pet_grant_level_durable_like_cpp, battle_pet_has_max_pet_count_like_cpp,
    battle_pet_max_pet_level_like_cpp, battle_pet_purchase_store_like_cpp,
    battle_pet_species_entry_like_cpp, battle_pet_species_has_flag_like_cpp,
    battle_pet_try_acquire_journal_lease_like_cpp, battle_pet_try_add_pet_durable_like_cpp,
    battle_pet_xp_per_level_like_cpp, destroy_uncaged_battle_pet_item_durable_like_cpp,
    record_battle_pet_trainer_purchase_criteria_like_cpp, uncage_item_state_like_cpp,
};
pub use collections_handlers::{
    CollectionsHandlerCxLikeCpp, CollectionsHandlerHostLikeCpp,
    register_collections_handlers_like_cpp,
};
pub use contracts::{
    AccountItemAppearanceSavePlanLikeCpp, AccountTransmogIllusionSavePlanLikeCpp,
    DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP, MAX_EQUIPMENT_SET_INDEX_LIKE_CPP,
    RepresentedEquipmentSetSavedLikeCpp,
};
pub use enchantment::{
    InitialLoadedItemModsOutcomeLikeCpp, ItemEnchantmentApplicationCxLikeCpp,
    ItemEnchantmentCatalogsLikeCpp, LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
};
// The exact equipment-set use owner facade the handler-contract checker
// requires (#1363 slice Q2).
#[cfg(any(test, feature = "test-fixtures"))]
pub use equipment_set_use_handlers::EquipmentSetUseFixtureRefsLikeCpp;
pub use equipment_set_use_handlers::{
    EquipmentSetUseContextLikeCpp, EquipmentSetUseHandlerHostLikeCpp,
    EquipmentSetUseItemModsStoresLikeCpp, register_equipment_set_use_handler_like_cpp,
};
pub use equipment_sets::represented_equipment_set_from_packet_like_cpp;
pub use guild_inventory_contracts::{
    RepresentedBankItemMoveLikeCpp, RepresentedGuildBankTabActionKindLikeCpp,
    RepresentedGuildRepairBankStateLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use guild_inventory_contracts::{
    RepresentedGuildBankInventoryMoveLikeCpp, RepresentedGuildBankListRequestLikeCpp,
    RepresentedGuildBankMoneyMoveLikeCpp, RepresentedGuildBankTabActionLikeCpp,
    RepresentedGuildRepairBankWithdrawLikeCpp,
};
pub use handlers::{
    AuctionHandlerCxLikeCpp, EquipmentSetsHandlerCxLikeCpp, EquipmentSetsSaveCxLikeCpp,
    InventoryHandlerHostLikeCpp, ItemTextQueryHandlerCxLikeCpp,
    register_inventory_handlers_like_cpp,
};
pub use inventory_move_contracts::{
    InventoryStorageQuestChecksLikeCpp, InventoryStorageTargetLikeCpp, InventorySwapTargetLikeCpp,
    autostore_bank_target_like_cpp, bind_inventory_item_for_destination_like_cpp,
    item_dynamic_flags_changed_like_cpp, item_storage_mutable_persistence_like_cpp,
};
pub use inventory_request_contracts::{
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedAutoUnequipOffhandReasonLikeCpp,
};
pub use inventory_scaling::InventoryScalingApplicationCxLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use item_modifiers::{
    RepresentedCombatStatRecalculationLikeCpp, RepresentedItemModsReapplyEventLikeCpp,
};
pub use item_modifiers::{
    RepresentedItemBonusActionLikeCpp, RepresentedItemSetAuraRefreshEventLikeCpp,
    RepresentedItemSetSpellEventLikeCpp,
};
pub use item_sets::ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP;
pub use modifiers::{ItemModsCatalogsViewLikeCpp, represented_player_stat_changes_like_cpp};
pub use persistence::item_spell_charges_db_string;
pub use publication::item_storage_fields_values_update_like_cpp;
pub use state::InventoryState;
pub use storage_slots::is_represented_bag_slot;

pub use quest_reward::{
    item_push_result_from_send_new_item_plan, make_inventory_item_object_like_cpp,
};
pub use stats::CR_HIT_MELEE_LIKE_CPP;
pub use stats_application::{
    CharacterStatsApplicationCxLikeCpp, level_up_stat_deltas_like_cpp, max_health_u32_like_cpp,
    primary_max_power_for_class_like_cpp, stats_application_cx_from_hub_like_cpp,
};
pub use storage::{CR_ARMOR_PENETRATION_LIKE_CPP, DirectInventoryStorageOverlayLikeCpp};
pub use turnins::ExtendedCostItemTurninChange;
// The exact void-storage owner facade the handler-contract checker requires
// (#1263 F5 deferred families).
pub use void_storage_handlers::{
    VoidStorageHandlerHostLikeCpp, register_void_storage_handlers_like_cpp,
};
// The vendor and bank character handlers (#1363 slice 8b).
#[cfg(any(test, feature = "test-fixtures"))]
pub use vendor_session::update_vendor_item_current_count;
pub use vendor_session::{
    PlayerConditionContextSourceLikeCpp, VendorSessionHostLikeCpp, handle_banker_activate,
    handle_buy_back_item_with_generator_like_cpp,
    handle_buy_bank_slot_with_prices_and_generator_like_cpp,
    handle_buy_item_with_generator_like_cpp, handle_item_purchase_refund_with_generator_like_cpp,
    handle_list_inventory, handle_repair_item_with_generator_like_cpp,
    handle_sell_item_with_generator_like_cpp,
};
// The item character handlers (#1363 slice 8c).
#[cfg(any(test, feature = "test-fixtures"))]
pub use item_session::{
    DestroyQuestItemLikeCpp, InventoryEquipChildPlanLikeCpp, autostore_bank_quest_checks_like_cpp,
    plan_destroyed_inventory_quest_persistence_like_cpp, plan_inventory_child_redirect_like_cpp,
    plan_inventory_equip_child_like_cpp, plan_inventory_real_swap_children_like_cpp,
};
pub use item_session::{
    ItemSessionHostLikeCpp, destroy_inventory_full_stack_by_pos_like_cpp,
    destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp,
    handle_auto_equip_item_slot_with_generator_like_cpp,
    handle_auto_equip_item_with_generator_like_cpp,
    handle_auto_store_bag_item_with_generator_like_cpp,
    handle_autobank_item_with_generator_like_cpp,
    handle_autostore_bank_item_with_generator_like_cpp, handle_destroy_item,
    handle_swap_inv_item_with_generator_like_cpp, handle_swap_item_with_generator_like_cpp,
};
pub use toy_session::{AddToyHostLikeCpp, handle_add_toy};
pub use void_transfer_contracts::{
    EffectiveVoidStorageRandomPropertiesLikeCpp, PlannedVoidDestroyedInventoryItemLikeCpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub use collection_adapter::RepresentedTransmogCriteriaEvent;
#[cfg(any(test, feature = "test-fixtures"))]
pub use fixtures::PlayerItemTestFixtureLikeCpp;
// The World-session registry publication sequence (#1363 slice Q: moved from
// wow-world-quest once its fixture hydration became quest-agnostic).
#[cfg(any(test, feature = "test-fixtures"))]
pub use registry_sync::PlayerRegistryHydrationContext;
pub use registry_sync::PlayerRegistrySyncContext;
pub use registry_sync::sync_player_registry_state_like_cpp;
// The represented player-item operations moved from the World session
// (#1363 slice 12b1).
pub use player_items_session::*;
