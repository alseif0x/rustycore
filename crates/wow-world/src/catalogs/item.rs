// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item template and item-data catalogs a session reads.
//!
//! Separated from `WorldSession` under #670: the session holds exactly one
//! field of this type, so no slot is mirrored. The owner depends on `wow-data`
//! stores only and performs no async work.

use std::collections::BTreeSet;
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
}
