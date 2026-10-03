// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session-owned inventory state and its bounded represented contracts.

mod auction_contracts;
mod bank;
mod buyback;
mod appearance;
mod guild_inventory_contracts;
mod inventory_request_contracts;
mod item_modifiers;
mod contracts;
mod currency;
mod enchantment;
mod equipment;
mod equipment_slots;
mod equipment_sets;
mod handlers;
mod durability;
mod modifiers;
mod offhand;
mod money;
mod persistence;
mod publication;
mod items;
mod item_sets;
mod catalog;
mod storage;
mod storage_bags;
mod storage_slots;
mod scaling;
mod valuation;
mod void_storage;
mod void_transfer;
mod void_transfer_contracts;
mod turnins;
mod stats;
mod persistence_load;
mod state;

#[cfg(any(test, feature = "test-fixtures"))]
mod collection_adapter;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;

pub use auction_contracts::{
    RepresentedAuctionPlaceBidLikeCpp, RepresentedAuctionRemoveItemLikeCpp,
    RepresentedAuctionReplicateRequestLikeCpp, RepresentedAuctionSellItemLikeCpp,
};
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
pub use inventory_request_contracts::{
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedAutoUnequipOffhandReasonLikeCpp,
};
pub use item_modifiers::{
    RepresentedItemBonusActionLikeCpp, RepresentedItemSetAuraRefreshEventLikeCpp,
    RepresentedItemSetSpellEventLikeCpp,
};
pub use item_sets::ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP;
pub use contracts::{
    AccountItemAppearanceSavePlanLikeCpp, AccountTransmogIllusionSavePlanLikeCpp,
    DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP, MAX_EQUIPMENT_SET_INDEX_LIKE_CPP,
    RepresentedEquipmentSetSavedLikeCpp,
};
pub use enchantment::LoadedEquippedItemEnchantmentsOutcomeLikeCpp;
pub use equipment_sets::represented_equipment_set_from_packet_like_cpp;
pub use handlers::{
    EquipmentSetsHandlerCxLikeCpp, InventoryHandlerHostLikeCpp,
    EquipmentSetsSaveCxLikeCpp,
    register_inventory_handlers_like_cpp,
};
pub use modifiers::{ItemModsCatalogsViewLikeCpp, represented_player_stat_changes_like_cpp};
pub use publication::item_storage_fields_values_update_like_cpp;
pub use storage_slots::is_represented_bag_slot;
#[cfg(any(test, feature = "test-fixtures"))]
pub use item_modifiers::{
    RepresentedCombatStatRecalculationLikeCpp, RepresentedItemModsReapplyEventLikeCpp,
};
pub use state::InventoryState;

pub use storage::CR_ARMOR_PENETRATION_LIKE_CPP;
pub use stats::CR_HIT_MELEE_LIKE_CPP;
pub use turnins::ExtendedCostItemTurninChange;
pub use void_transfer_contracts::{
    EffectiveVoidStorageRandomPropertiesLikeCpp, PlannedVoidDestroyedInventoryItemLikeCpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub use collection_adapter::RepresentedTransmogCriteriaEvent;
#[cfg(any(test, feature = "test-fixtures"))]
pub use fixtures::PlayerItemTestFixtureLikeCpp;
