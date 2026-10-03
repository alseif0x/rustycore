//! SQL-free source-width costs spell inputs (02245dcd), not gameplay state.

#[derive(Clone, Copy)]
pub struct SpellEmpowerRow {
    pub id: u32,
    pub spell_id: i32,
    pub unused1000: i32,
}

#[derive(Clone, Copy)]
pub struct SpellEmpowerStageRow {
    pub id: u32,
    pub stage: i32,
    pub duration_ms: i32,
    pub spell_empower_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellPowerRow {
    pub id: u32,
    pub order_index: u8,
    pub mana_cost: i32,
    pub mana_cost_per_level: i32,
    pub mana_per_second: i32,
    pub power_display_id: u32,
    pub alt_power_bar_id: i32,
    pub power_cost_pct: f32,
    pub power_cost_max_pct: f32,
    pub optional_cost_pct: f32,
    pub power_pct_per_second: f32,
    pub power_type: i8,
    pub required_aura_spell_id: i32,
    pub optional_cost: u32,
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellPowerDifficultyRow {
    pub id: u32,
    pub difficulty_id: i16,
    pub order_index: u8,
}

#[derive(Clone, Copy)]
pub struct SpellReagentsRow {
    pub id: u32,
    pub spell_id: i32,
    pub reagent: [i32; 8],
    pub reagent_count: [i16; 8],
    pub reagent_recraft_count: [i16; 8],
    pub reagent_source: [u8; 8],
}

#[derive(Clone, Copy)]
pub struct SpellReagentsCurrencyRow {
    pub id: u32,
    pub spell_id: u32,
    pub currency_types_id: i32,
    pub currency_count: i32,
    pub override_recraft_currency_count: i32,
    pub order_source: u8,
}

#[derive(Clone, Copy)]
pub struct SpellScalingRow {
    pub id: u32,
    pub spell_id: i32,
    pub min_scaling_level: u32,
    pub max_scaling_level: u32,
}

#[derive(Clone, Copy)]
pub struct SpellShapeshiftRow {
    pub id: u32,
    pub spell_id: i32,
    pub stance_bar_order: i8,
    pub shapeshift_exclude: [i32; 2],
    pub shapeshift_mask: [i32; 2],
}

#[derive(Clone, Copy)]
pub struct SpellTargetRestrictionsRow {
    pub id: u32,
    pub difficulty_id: i16,
    pub cone_degrees: f32,
    pub max_targets: u8,
    pub max_target_level: u32,
    pub target_creature_type: i16,
    pub targets: i32,
    pub width: f32,
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellTotemsRow {
    pub id: u32,
    pub spell_id: i32,
    pub required_totem_category_id: [u16; 2],
    pub totem: [i32; 2],
}

#[derive(Clone, Copy)]
pub struct SpellXSpellVisualRow {
    pub id: u32,
    pub difficulty_id: i16,
    pub spell_visual_id: u32,
    pub probability: f32,
    pub flags: i32,
    pub priority: i32,
    pub spell_icon_file_id: i32,
    pub active_icon_file_id: i32,
    pub viewer_unit_condition_id: u16,
    pub viewer_player_condition_id: u32,
    pub caster_unit_condition_id: u16,
    pub caster_player_condition_id: u32,
    pub spell_id: u32,
}
