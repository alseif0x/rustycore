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

use wow_constants::{BagFamilyMask, InventoryType, ItemBondingType, ItemClass, ItemFlags2};
use wow_data::{ItemStatsStore, ItemStore};
use wow_entities::ItemStorageTemplate;

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

/// Resolve the storage-validation subset of C++ `ItemTemplate` from its two
/// selected item stores. This is the single conversion used by session and
/// inventory fixture projections.
pub fn item_storage_template_like_cpp(
    store: Option<&Arc<ItemStore>>,
    stats_store: Option<&Arc<ItemStatsStore>>,
    item_id: u32,
) -> Option<ItemStorageTemplate> {
    let basic = store?.get(item_id)?;
    let sparse = stats_store?.sparse_template(item_id)?;
    let class_id = <ItemClass as num_traits::FromPrimitive>::from_u8(basic.class_id)?;
    let inventory_type =
        <InventoryType as num_traits::FromPrimitive>::from_i8(sparse.inventory_type)?;
    let bonding = <ItemBondingType as num_traits::FromPrimitive>::from_u8(sparse.bonding)?;

    Some(ItemStorageTemplate {
        entry: item_id,
        class_id,
        subclass_id: u32::from(basic.subclass_id),
        inventory_type,
        bonding,
        bag_family: BagFamilyMask::from_bits_retain(sparse.bag_family),
        max_stack_size: sparse.max_stack_size(),
        max_count: sparse.max_count,
        item_limit_category: u32::from(sparse.limit_category),
        container_slots: sparse.container_slots,
        sell_price: sparse.sell_price,
        is_crafting_reagent: (sparse.flags[1] & ItemFlags2::UsedInATradeskill as u32) != 0,
        flags: sparse.item_flags(),
    })
}
