//! SQL-free target numeric hotfix rows, not an effective catalog or Player.
//! 02245dcd HotfixDatabase/DB2LoadInfo; signed mask words preserve source bits.

pub struct SkillLineRow {
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

pub struct SkillRaceClassRow {
    pub id: u32,
    pub skill: u16,
    pub class_mask: i32,
    pub flags: i32,
    pub availability: i32,
    pub min_level: i8,
    pub tier: i16,
    pub race_mask: [i32; 2],
}

pub struct SkillAbilityRow {
    pub id: u32,
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
    pub race_mask: [i32; 2],
}

pub struct LoadoutRow {
    pub id: u32,
    pub class: i8,
    pub purpose: i32,
    pub item_context: u8,
    pub field_1_60_1_69876_003: i32,
    pub race_mask: [i32; 2],
}

pub struct LoadoutItemRow {
    pub id: u32,
    pub loadout: u16,
    pub item: u32,
}

#[derive(Default)]
pub struct BirthRows {
    pub skill_lines: Vec<SkillLineRow>,
    pub race_class: Vec<SkillRaceClassRow>,
    pub abilities: Vec<SkillAbilityRow>,
    pub loadouts: Vec<LoadoutRow>,
    pub loadout_items: Vec<LoadoutItemRow>,
}

pub struct BirthOverlays {
    pub official: BirthRows,
    pub custom: BirthRows,
}
