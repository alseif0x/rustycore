// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item template and item-data catalogs a session reads.
//!
//! Separated from `WorldSession` under #670: the session holds exactly one
//! field of this type, so no slot is mirrored. The owner depends on `wow-data`
//! stores only and performs no async work.

use std::sync::Arc;

#[derive(Default)]
pub struct ItemCatalogsLikeCpp {
    pub appearance_store: Option<Arc<wow_data::ItemAppearanceStore>>,
    pub bonus_db2_store: Option<Arc<wow_data::ItemBonusDb2Store>>,
    pub child_equipment_store: Option<Arc<wow_data::ItemChildEquipmentStore>>,
    pub effect_store: Option<Arc<wow_data::ItemEffectStore>>,
    pub extended_cost_store: Option<Arc<wow_data::ItemExtendedCostStore>>,
    pub limit_category_condition_store: Option<Arc<wow_data::ItemLimitCategoryConditionStore>>,
    pub limit_category_store: Option<Arc<wow_data::ItemLimitCategoryStore>>,
    pub modified_appearance_store: Option<Arc<wow_data::ItemModifiedAppearanceStore>>,
    pub random_enchantment_template_store:
        Option<Arc<wow_data::ItemRandomEnchantmentTemplateStore>>,
    pub random_properties_store: Option<Arc<wow_data::ItemRandomPropertiesStore>>,
    pub random_suffix_store: Option<Arc<wow_data::ItemRandomSuffixStore>>,
    pub search_name_store: Option<Arc<wow_data::ItemSearchNameStore>>,
    pub set_store: Option<Arc<wow_data::ItemSetStore>>,
    pub spec_override_store: Option<Arc<wow_data::ItemSpecOverrideStore>>,
    pub stats_store: Option<Arc<wow_data::ItemStatsStore>>,
    pub store: Option<Arc<wow_data::ItemStore>>,
}
