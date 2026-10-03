//! Four numeric item sources for 70170, before template/equip/instance rules.
//! 02245dcd DB2LoadInfo::{Item,ItemSparse,ItemEffect,ItemXItemEffect}.
mod effective;
mod load;
use super::item_quantities::ItemEffectRelationRecord;
use super::item_sparse::SparseItemRecord;
use anyhow::Result;
pub use effective::{
    ITEM_EFFECT_HASH, ITEM_HASH, ITEM_RELATION_HASH, ITEM_SPARSE_HASH, ItemCatalog,
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
    /// Excluded direct baseline rows, never unresolved effective-ID coverage.
    pub unknown_baseline_records: [usize; 4],
}

impl ItemRecords {
    /// Explicit, exact esES prefix batch. No full-file fallback or implicit
    /// encryption waiver. All four checked reads finish before returning.
    pub fn load_available(directory: &Path) -> Result<Self> {
        load::available(directory)
    }
}
