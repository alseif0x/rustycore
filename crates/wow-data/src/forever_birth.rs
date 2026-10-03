//! Target numeric birth prerequisites, independent of old skill/item layouts.
//! 02245dcd DB2Structure/Metadata/LoadInfo and local build-70170 hashes.
//! This raw batch is not effective hotfix data or a learned/equipped Player.
use anyhow::Result;
use std::path::Path;
mod effective;
pub mod item_quantities;
pub mod item_records;
pub mod item_sparse;
pub mod item_specs;
mod load;
mod race_mask;
pub use effective::{
    ABILITY_HASH, BirthCatalog, LOADOUT_HASH, LOADOUT_ITEM_HASH, RACE_CLASS_HASH, SKILL_LINE_HASH,
};
pub use race_mask::race_in_mask;

#[derive(Clone, Copy)]
pub enum AbilityBaseline {
    Complete,
    /// Exact 70170/esES prefix: 7833 present, five unknown. Never automatic.
    AvailablePrefix,
}

#[derive(Clone, Copy)]
pub struct SkillLineRecord {
    pub id: u32,
    pub category: i8,
    pub spell_icon_file: i32,
    pub can_link: i8,
    pub parent_skill: u32,
    pub parent_tier_index: i32,
    pub flags: i32,
    pub spell_book_spell: i32,
    pub expansion_name_shared_string: i32,
    pub horde_expansion_name_shared_string: i32,
}

#[derive(Clone, Copy)]
pub struct SkillRaceClassRecord {
    pub id: u32,
    /// Source LoadInfo/Structure are unsigned despite signed SHORT DB2Meta.
    pub skill: u16,
    pub class_mask: i32,
    pub flags: i32,
    pub availability: i32,
    pub min_level: i8,
    pub tier: i16,
    /// Two source int32 words, preserved as a 64-bit bit pattern.
    /// This is not a race-ID-minus-one mapping or a race validation claim.
    pub race_mask: u64,
}

#[derive(Clone, Copy)]
pub struct SkillAbilityRecord {
    pub id: u32,
    /// Source LoadInfo/Structure are unsigned despite signed SHORT DB2Meta.
    pub skill_line: u16,
    pub spell: i32,
    pub min_skill_rank: i16,
    pub class_mask: i32,
    pub supercedes_spell: i32,
    pub acquire_method: i32,
    pub trivial_rank_high: i16,
    pub trivial_rank_low: i16,
    pub flags: i32,
    pub num_skill_ups: i8,
    pub unique_bit: i16,
    pub trade_skill_category: i16,
    pub skillup_skill_line: i16,
    pub field_5_5_4_67090_014: [i32; 2],
    pub race_mask: u64,
}

#[derive(Clone, Copy)]
pub struct LoadoutRecord {
    pub id: u32,
    pub class: i8,
    pub purpose: i32,
    pub item_context: u8,
    pub field_1_60_1_69876_003: i32,
    pub race_mask: u64,
}

#[derive(Clone, Copy)]
pub struct LoadoutItemRecord {
    pub id: u32,
    pub loadout: u16,
    pub item: u32,
}

/// All five checked reads finish before returning the transient batch. Strings
/// are intentionally excluded; numeric arrays/types follow the target source.
#[derive(Default)]
pub struct BirthRecords {
    pub skill_lines: Vec<SkillLineRecord>,
    pub race_class: Vec<SkillRaceClassRecord>,
    pub abilities: Vec<SkillAbilityRecord>,
    pub loadouts: Vec<LoadoutRecord>,
    pub loadout_items: Vec<LoadoutItemRecord>,
    pub unknown_ability_records: usize,
}

impl BirthRecords {
    pub fn load(directory: &Path, abilities: AbilityBaseline) -> Result<Self> {
        load::records(directory, abilities)
    }
}
