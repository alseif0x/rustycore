//! SQL-free numeric projections for the target's creation initialization stores.
//! This is a transient startup batch, not a Player, wire serializer or DB pool.
//! DB2LoadInfo/HotfixDatabase.cpp at pinned target 02245dcd.

pub struct MapRow {
    pub id: u32,
    pub instance_type: i8,
    pub expansion: u8,
    pub parent_map: i16,
    pub flags: [i32; 3],
}

pub struct PowerRow {
    pub id: u32,
    pub power_type: i8,
    pub min_power: i32,
    pub max_base_power: i32,
    pub center_power: i32,
    pub default_power: i32,
    pub display_modifier: i32,
    pub regen_interrupt_ms: i32,
    pub regen_peace: f32,
    pub regen_combat: f32,
    pub flags: i32,
}

pub struct SpecializationRow {
    pub id: u32,
    pub class: u8,
    pub order_index: i8,
    pub pet_talent_type: i8,
    pub role: i8,
    pub flags: i32,
    pub primary_stat_priority: i8,
    pub mastery_spells: [i32; 2],
}

pub struct ClassPowerRow {
    pub id: u32,
    pub power_type: i8,
    pub class: u32,
}

pub struct MovieRow {
    pub id: u32,
    pub volume: u8,
    pub key_id: u8,
    pub audio_file: u32,
    pub subtitle_file: u32,
    pub subtitle_format: u32,
}

pub struct ClassRow {
    pub id: u32,
    pub flags: i32,
    pub starting_level: i32,
    pub cinematic: u16,
    pub default_spec: u16,
    pub strength_bonus: u8,
    pub primary_stat_priority: i8,
    pub display_power: i8,
    pub ranged_attack_per_agility: u8,
    pub attack_per_agility: u8,
    pub attack_per_strength: u8,
    pub spell_class_set: u8,
}

pub struct RaceRow {
    pub id: u32,
    pub flags: i32,
    pub faction: i32,
    pub cinematic: i32,
    pub resurrection_sickness_spell: i32,
    pub starting_level: i32,
    pub base_language: i8,
    pub creature_type: u8,
    pub alliance: i8,
    pub neutral_race: i8,
}

#[derive(Default)]
pub struct InitializationRows {
    pub maps: Vec<MapRow>,
    pub powers: Vec<PowerRow>,
    pub specializations: Vec<SpecializationRow>,
    pub class_powers: Vec<ClassPowerRow>,
    pub movies: Vec<MovieRow>,
    pub classes: Vec<ClassRow>,
    pub races: Vec<RaceRow>,
}

pub struct InitializationOverlays {
    pub official: InitializationRows,
    pub custom: InitializationRows,
}
