// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session-owned inventory state and its bounded represented contracts.

mod appearance;
mod auction_contracts;
mod bank;
mod buyback;
mod catalog;
mod contracts;
mod currency;
mod durability;
mod enchantment;
mod equipment;
mod equipment_set_use;
mod equipment_sets;
mod equipment_slots;
mod guild_inventory_contracts;
mod handlers;
mod inventory_request_contracts;
mod item_modifiers;
mod item_sets;
mod items;
mod limit_category;
mod modifiers;
mod money;
mod offhand;
mod persistence;
mod persistence_load;
mod publication;
mod quest_reward;
mod scaling;
mod state;
mod stats;
mod storage;
mod storage_bags;
mod storage_slots;
mod turnins;
mod valuation;
mod void_storage;
mod void_transfer;
mod void_transfer_contracts;

#[cfg(any(test, feature = "test-fixtures"))]
mod collection_adapter;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
pub mod loaded_item_support;

pub use auction_contracts::{
    RepresentedAuctionPlaceBidLikeCpp, RepresentedAuctionRemoveItemLikeCpp,
    RepresentedAuctionReplicateRequestLikeCpp, RepresentedAuctionSellItemLikeCpp,
};
pub use contracts::{
    AccountItemAppearanceSavePlanLikeCpp, AccountTransmogIllusionSavePlanLikeCpp,
    DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP, MAX_EQUIPMENT_SET_INDEX_LIKE_CPP,
    RepresentedEquipmentSetSavedLikeCpp,
};
pub use enchantment::{
    ItemEnchantmentApplicationCxLikeCpp, ItemEnchantmentCatalogsLikeCpp,
    LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
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
    EquipmentSetsHandlerCxLikeCpp, EquipmentSetsSaveCxLikeCpp, InventoryHandlerHostLikeCpp,
    ItemTextQueryHandlerCxLikeCpp, register_inventory_handlers_like_cpp,
};
pub use inventory_request_contracts::{
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedAutoUnequipOffhandReasonLikeCpp,
};
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
pub use publication::item_storage_fields_values_update_like_cpp;
pub use state::InventoryState;
pub use storage_slots::is_represented_bag_slot;

pub use quest_reward::{
    item_push_result_from_send_new_item_plan, make_inventory_item_object_like_cpp,
};
pub use stats::CR_HIT_MELEE_LIKE_CPP;
pub use storage::{CR_ARMOR_PENETRATION_LIKE_CPP, DirectInventoryStorageOverlayLikeCpp};
pub use turnins::ExtendedCostItemTurninChange;
pub use void_transfer_contracts::{
    EffectiveVoidStorageRandomPropertiesLikeCpp, PlannedVoidDestroyedInventoryItemLikeCpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub use collection_adapter::RepresentedTransmogCriteriaEvent;
#[cfg(any(test, feature = "test-fixtures"))]
pub use fixtures::PlayerItemTestFixtureLikeCpp;
