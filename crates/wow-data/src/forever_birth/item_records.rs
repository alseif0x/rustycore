//! Nine numeric item sources for 70170, before template/equip/instance rules.
//! 02245dcd DB2LoadInfo::{Item,ItemSparse,ItemEffect,ItemXItemEffect,ItemBonus,
//! AzeriteEmpoweredItem,ItemScalingConfig,ItemOffsetCurve,ItemSquishEra}.
mod bonus_records;
mod effective;
mod load;
use super::item_quantities::ItemEffectRelationRecord;
use super::item_sparse::SparseItemRecord;
use anyhow::Result;
pub use bonus_records::{
    AzeriteEmpoweredItemRecord, ItemBonusRecord, ItemOffsetCurveRecord, ItemScalingConfigRecord,
    ItemSquishEraRecord,
};
pub use effective::{
    AZERITE_EMPOWERED_ITEM_HASH, ITEM_BONUS_HASH, ITEM_EFFECT_HASH, ITEM_HASH,
    ITEM_OFFSET_CURVE_HASH, ITEM_RELATION_HASH, ITEM_SCALING_CONFIG_HASH, ITEM_SPARSE_HASH,
    ITEM_SQUISH_ERA_HASH, ItemCatalog,
};
use std::path::Path;

#[derive(Clone, Copy)]
pub struct ItemRecord {
    pub id: u32,
    pub class: i32,
    pub subclass: u8,
    pub material: u8,
    pub inventory_type: i8,
    pub sheathe: u8,
    pub pet_food: i32,
    pub sound_override: i8,
    pub icon_file: i32,
    pub group_sounds: u32,
    pub content_tuning: i32,
    pub modified_crafting_reagent: i32,
    pub unknown_1200: u8,
    pub crafting_quality: i32,
    pub squish_era: i32,
    pub recraft_reagent_percentage: f32,
    pub order_source: u8,
}

#[derive(Clone, Copy)]
pub struct ItemEffectRecord {
    pub id: u32,
    pub legacy_slot: u8,
    pub trigger: u8,
    pub charges: i16,
    pub cooldown: i32,
    pub category_cooldown: i32,
    pub spell_category: u16,
    pub spell: i32,
    pub specialization: u16,
    pub player_condition: i32,
}

#[derive(Default)]
pub struct ItemRecords {
    pub items: Vec<ItemRecord>,
    pub sparse: Vec<SparseItemRecord>,
    pub effects: Vec<ItemEffectRecord>,
    pub relations: Vec<ItemEffectRelationRecord>,
    pub item_bonuses: Vec<ItemBonusRecord>,
    pub azerite_empowered_items: Vec<AzeriteEmpoweredItemRecord>,
    pub item_scaling_configs: Vec<ItemScalingConfigRecord>,
    pub item_offset_curves: Vec<ItemOffsetCurveRecord>,
    pub item_squish_eras: Vec<ItemSquishEraRecord>,
    /// Source GetMaxId before copies/removals; None only for synthetic input.
    pub item_squish_era_file_last_index: Option<u32>,
    /// SQL MAX(ID)+1 observed separately after each nonempty overlay batch.
    pub item_squish_era_sql_index_size: Option<u32>,
    /// Excluded direct baseline rows, never unresolved effective-ID coverage.
    pub unknown_baseline_records: [usize; 9],
}

impl ItemRecords {
    /// Explicit acquired item input plus strict numeric bonus tables. No
    /// full-file fallback or implicit encryption waiver.
    pub fn load_available(directory: &Path) -> Result<Self> {
        load::available(directory)
    }
}
