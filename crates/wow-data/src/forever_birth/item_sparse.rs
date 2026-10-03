//! Numeric ItemSparse baseline for the target's variable-length records.
//! 02245dcd DB2Metadata/DB2LoadInfo/DB2Structure::ItemSparse, and
//! DB2FileLoader.cpp:1019-1050,1291-1317,1455-1584,1938-1968.
//! Five inline byte strings belong to the acquired esES locale (not enUS).
//! No effective overlay, full ItemTemplate, equip/use or item instance claim.
mod decode;
mod prefix;
mod strings;
pub use strings::SparseItemStrings;
#[cfg(test)]
mod tests;

use super::item_quantities::ItemSparseQuantityRecord;
use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;

#[derive(Clone)]
pub struct SparseItemRecord {
    pub id: u32,
    pub strings: SparseItemStrings,
    pub expansion: i32,
    pub damage_variance: f32,
    pub limit_category: i32,
    pub duration: u32,
    pub quality_modifier: f32,
    pub bag_family: u32,
    pub start_quest: i32,
    pub language: i32,
    pub item_range: f32,
    pub socket_percentage: [f32; 10],
    pub stat_percent: [i32; 10],
    pub stat_bonus: [i32; 10],
    pub stackable: i32,
    pub max_count: i32,
    pub min_reputation: i32,
    pub required_ability: u32,
    pub allowable_race: u64,
    pub sell_price: u32,
    pub buy_price: u32,
    pub vendor_stack: u32,
    pub price_variance: f32,
    pub price_random: f32,
    pub flags: [i32; 5],
    pub faction_related: i32,
    pub modified_crafting_reagent: i32,
    pub content_tuning: i32,
    pub player_level_curve: i32,
    pub item_level_offset_curve: i32,
    pub item_level_offset: i32,
    pub squish_era: i32,
    pub name_description: u16,
    pub transmog_holiday: u16,
    pub holiday: u16,
    pub gem_properties: u16,
    pub socket_enchantment: u16,
    pub totem_category: u16,
    pub instance_bound: u16,
    pub zone_bound: [u16; 2],
    pub item_set: u16,
    pub lock: u16,
    pub page: u16,
    pub delay: u16,
    pub min_faction: u16,
    pub required_skill_rank: u16,
    pub required_skill: u16,
    pub item_level: u16,
    pub allowable_class: i16,
    pub artifact: u8,
    pub spell_weight: u8,
    pub spell_weight_category: u8,
    pub socket_type: [u8; 3],
    pub sheathe: u8,
    pub material: u8,
    pub page_material: u8,
    pub bonding: u8,
    pub damage_type: u8,
    pub container_slots: u8,
    pub required_pvp_medal: u8,
    pub required_pvp_rank: i8,
    pub required_level: i8,
    pub inventory_type: i8,
    pub quality: i8,
    pub ammunition: u8,
}

impl SparseItemRecord {
    /// Baseline projection only until the caller completes official/custom/
    /// removal composition. This is NOT full-template existence/equip proof.
    pub fn quantity_projection(&self) -> ItemSparseQuantityRecord {
        ItemSparseQuantityRecord {
            id: self.id,
            vendor_stack: self.vendor_stack,
            stackable: self.stackable,
        }
    }
}

pub struct SparseItemRecords {
    pub records: Vec<SparseItemRecord>,
    /// Direct rows excluded by this exact acquisition; not unknown-ID coverage.
    pub unknown_baseline_records: usize,
}

impl SparseItemRecords {
    /// Explicit available-prefix operation, never a fallback from failed full
    /// loading. Other tables/normal WDC readers retain their strict gates.
    pub fn load_available(directory: &Path) -> Result<Self> {
        let file = std::fs::File::open(directory.join("ItemSparse.available.db2"))
            .context("Cannot read private sparse item baseline")?;
        let mut bytes = Vec::new();
        file.take(6_971_319)
            .read_to_end(&mut bytes)
            .context("Cannot read bounded sparse item baseline")?;
        prefix::load(&bytes)
    }
}
