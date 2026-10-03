//! Source-faithful raw dependencies spell inputs (02245dcd), not gameplay state.
use super::SpellText;

#[derive(Clone)]
pub struct DifficultyRecord {
    pub id: u32,
    pub name: SpellText,
    pub instance_type: u8,
    pub order_index: u8,
    pub old_enum_value: i8,
    pub fallback_difficulty_id: i16,
    pub min_players: u8,
    pub max_players: u8,
    pub flags: i32,
    pub item_context: u8,
    pub toggle_difficulty_id: i16,
    pub group_size_health_curve_id: u32,
    pub group_size_dmg_curve_id: u32,
    pub group_size_spell_points_curve_id: u32,
    pub unknown1105: i32,
}

#[derive(Clone, Copy)]
pub struct SpellCastTimesRecord {
    pub id: u32,
    pub base: i32,
    pub minimum: i32,
}

#[derive(Clone, Copy)]
pub struct SpellDurationRecord {
    pub id: u32,
    pub duration: i32,
    pub max_duration: i32,
    pub duration_per_resource: i32,
}

#[derive(Clone)]
pub struct SpellRangeRecord {
    pub id: u32,
    pub display_name: SpellText,
    pub display_name_short: SpellText,
    pub flags: i32,
    pub range_min: [f32; 2],
    pub range_max: [f32; 2],
}

#[derive(Clone, Copy)]
pub struct SpellRadiusRecord {
    pub id: u32,
    pub radius: f32,
    pub radius_per_level: f32,
    pub radius_min: f32,
    pub radius_max: f32,
}

#[derive(Clone, Copy)]
pub struct SpellProcsPerMinuteRecord {
    pub id: u32,
    pub base_proc_rate: f32,
    pub flags: i32,
}

#[derive(Clone, Copy)]
pub struct SpellProcsPerMinuteModRecord {
    pub id: u32,
    pub r#type: i32,
    pub param: i32,
    pub coeff: f32,
    pub field_12_1_5_69594_003: i32,
    pub spell_procs_per_minute_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellLearnSpellRecord {
    pub id: u32,
    pub spell_id: u32,
    pub learn_spell_id: i32,
    pub overrides_spell_id: i32,
}

#[derive(Clone)]
pub struct SpellShapeshiftFormRecord {
    pub id: u32,
    pub name: SpellText,
    pub creature_display_id: u32,
    pub creature_type: u8,
    pub flags: i32,
    pub attack_icon_file_id: i32,
    pub bonus_action_bar: i8,
    pub combat_round_time: i16,
    pub damage_variance: f32,
    pub mount_type_id: u16,
    pub preset_spell_id: [u32; 8],
}

#[derive(Clone, Copy)]
pub struct SummonPropertiesRecord {
    pub id: u32,
    pub control: i32,
    pub faction: i32,
    pub title: i32,
    pub slot: i32,
    pub flags: [i32; 2],
}

#[derive(Clone)]
pub struct BattlePetSpeciesRecord {
    pub description: SpellText,
    pub source_text: SpellText,
    pub id: u32,
    pub creature_id: i32,
    pub summon_spell_id: i32,
    pub icon_file_data_id: i32,
    pub pet_type_enum: i8,
    pub flags: i32,
    pub source_type_enum: i8,
    pub card_ui_model_scene_id: i32,
    pub loadout_ui_model_scene_id: i32,
    pub covenant_id: i32,
}

#[derive(Clone)]
pub struct SpellCategoryRecord {
    pub id: u32,
    pub name: SpellText,
    pub flags: i32,
    pub uses_per_week: i32,
    pub max_charges: i32,
    pub charge_recovery_time: i32,
    pub type_mask: i32,
}
