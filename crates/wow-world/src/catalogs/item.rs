// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item template and item-data catalogs a session reads.
//!
//! Separated from `WorldSession` under #670: the session holds exactly one
//! field of this type, so no slot is mirrored. This also owns the item
//! template quest-log cache. The owner depends on `wow-data` stores only and
//! performs no async work.

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct ItemCatalogsLikeCpp {
    pub(crate) appearance_store: Option<Arc<wow_data::ItemAppearanceStore>>,
    pub(crate) bonus_db2_store: Option<Arc<wow_data::ItemBonusDb2Store>>,
    pub(crate) child_equipment_store: Option<Arc<wow_data::ItemChildEquipmentStore>>,
    pub(crate) effect_store: Option<Arc<wow_data::ItemEffectStore>>,
    pub(crate) extended_cost_store: Option<Arc<wow_data::ItemExtendedCostStore>>,
    pub(crate) limit_category_condition_store:
        Option<Arc<wow_data::ItemLimitCategoryConditionStore>>,
    pub(crate) limit_category_store: Option<Arc<wow_data::ItemLimitCategoryStore>>,
    pub(crate) modified_appearance_store: Option<Arc<wow_data::ItemModifiedAppearanceStore>>,
    pub(crate) random_enchantment_template_store:
        Option<Arc<wow_data::ItemRandomEnchantmentTemplateStore>>,
    pub(crate) random_properties_store: Option<Arc<wow_data::ItemRandomPropertiesStore>>,
    pub(crate) random_suffix_store: Option<Arc<wow_data::ItemRandomSuffixStore>>,
    pub(crate) search_name_store: Option<Arc<wow_data::ItemSearchNameStore>>,
    pub(crate) set_store: Option<Arc<wow_data::ItemSetStore>>,
    pub(crate) spec_override_store: Option<Arc<wow_data::ItemSpecOverrideStore>>,
    pub(crate) stats_store: Option<Arc<wow_data::ItemStatsStore>>,
    pub(crate) store: Option<Arc<wow_data::ItemStore>>,

    // BankBagSlotPrices.db2 store used by C++ HandleBuyBankSlotOpcode.
    #[cfg(any(test, feature = "test-fixtures"))]
    bank_bag_slot_prices_store: Option<Arc<wow_data::BankBagSlotPricesStore>>,
    // Item class store (ItemClass.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    item_class_store: Option<Arc<wow_data::ItemClassStore>>,
    // Item currency cost store (ItemCurrencyCost.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    item_currency_cost_store: Option<Arc<wow_data::ItemCurrencyCostStore>>,
    // Transmog set item store (TransmogSetItem.db2 data)
    transmog_set_item_store: Option<Arc<wow_data::TransmogSetItemStore>>,
    // Item price base store (ItemPriceBase.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    item_price_base_store: Option<Arc<wow_data::ItemPriceBaseStore>>,
    // PvP item level adjustments used by item valuation.
    pvp_item_store: Option<Arc<wow_data::PvpItemStore>>,
    // C++ `sDurabilityCostsStore` data.
    durability_costs_store: Option<Arc<wow_data::DurabilityCostsStore>>,
    // C++ `sDurabilityQualityStore` data.
    durability_quality_store: Option<Arc<wow_data::DurabilityQualityStore>>,
    // Cached C++ `ItemTemplate::QuestLogItemId` from `item_template_addon`.
    item_template_addon_quest_log_item_ids_like_cpp: HashMap<u32, u32>,
    // RandPropPoints store (RandPropPoints.db2 data)
    rand_prop_points_store: Option<Arc<wow_data::RandPropPointsStore>>,
    // ItemDisenchantLoot store (ItemDisenchantLoot.db2 data)
    #[cfg(any(test, feature = "test-fixtures"))]
    item_disenchant_loot_store: Option<Arc<wow_data::ItemDisenchantLootStore>>,
}

impl ItemCatalogsLikeCpp {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn install_bank_bag_slot_prices_store(
        &mut self,
        store: Arc<wow_data::BankBagSlotPricesStore>,
    ) {
        self.bank_bag_slot_prices_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn bank_bag_slot_prices_store(
        &self,
    ) -> Option<&Arc<wow_data::BankBagSlotPricesStore>> {
        self.bank_bag_slot_prices_store.as_ref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn install_item_class_store(&mut self, store: Arc<wow_data::ItemClassStore>) {
        self.item_class_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn item_class_store(&self) -> Option<&Arc<wow_data::ItemClassStore>> {
        self.item_class_store.as_ref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn install_item_currency_cost_store(
        &mut self,
        store: Arc<wow_data::ItemCurrencyCostStore>,
    ) {
        self.item_currency_cost_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn item_currency_cost_store(
        &self,
    ) -> Option<&Arc<wow_data::ItemCurrencyCostStore>> {
        self.item_currency_cost_store.as_ref()
    }

    pub(crate) fn install_transmog_set_item_store(
        &mut self,
        store: Arc<wow_data::TransmogSetItemStore>,
    ) {
        self.transmog_set_item_store = Some(store);
    }

    pub(crate) fn transmog_set_item_store(
        &self,
    ) -> Option<&Arc<wow_data::TransmogSetItemStore>> {
        self.transmog_set_item_store.as_ref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn install_item_price_base_store(
        &mut self,
        store: Arc<wow_data::ItemPriceBaseStore>,
    ) {
        self.item_price_base_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn item_price_base_store(&self) -> Option<&Arc<wow_data::ItemPriceBaseStore>> {
        self.item_price_base_store.as_ref()
    }

    pub(crate) fn install_pvp_item_store(&mut self, store: Arc<wow_data::PvpItemStore>) {
        self.pvp_item_store = Some(store);
    }

    pub(crate) fn pvp_item_store(&self) -> Option<&Arc<wow_data::PvpItemStore>> {
        self.pvp_item_store.as_ref()
    }

    pub(crate) fn install_durability_costs_store(
        &mut self,
        store: Arc<wow_data::DurabilityCostsStore>,
    ) {
        self.durability_costs_store = Some(store);
    }

    pub(crate) fn durability_costs_store(&self) -> Option<&Arc<wow_data::DurabilityCostsStore>> {
        self.durability_costs_store.as_ref()
    }

    pub(crate) fn install_durability_quality_store(
        &mut self,
        store: Arc<wow_data::DurabilityQualityStore>,
    ) {
        self.durability_quality_store = Some(store);
    }

    pub(crate) fn durability_quality_store(
        &self,
    ) -> Option<&Arc<wow_data::DurabilityQualityStore>> {
        self.durability_quality_store.as_ref()
    }

    pub(crate) fn item_template_addon_quest_log_item_id(&self, item_id: u32) -> Option<u32> {
        self.item_template_addon_quest_log_item_ids_like_cpp
            .get(&item_id)
            .copied()
    }

    pub(crate) fn cache_item_template_addon_quest_log_item_id(
        &mut self,
        item_id: u32,
        quest_log_item_id: u32,
    ) {
        self.item_template_addon_quest_log_item_ids_like_cpp
            .insert(item_id, quest_log_item_id);
    }

    pub(crate) fn install_rand_prop_points_store(
        &mut self,
        store: Arc<wow_data::RandPropPointsStore>,
    ) {
        self.rand_prop_points_store = Some(store);
    }

    pub(crate) fn rand_prop_points_store(
        &self,
    ) -> Option<&Arc<wow_data::RandPropPointsStore>> {
        self.rand_prop_points_store.as_ref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn install_item_disenchant_loot_store(
        &mut self,
        store: Arc<wow_data::ItemDisenchantLootStore>,
    ) {
        self.item_disenchant_loot_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn item_disenchant_loot_store(
        &self,
    ) -> Option<&Arc<wow_data::ItemDisenchantLootStore>> {
        self.item_disenchant_loot_store.as_ref()
    }
}
