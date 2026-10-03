//! Source-faithful raw core spell inputs (02245dcd), not gameplay state.
use super::SpellText;

#[derive(Clone)]
pub struct SpellNameRecord {
    pub id: u32,
    pub name: SpellText,
}

#[derive(Clone, Copy)]
pub struct SpellEffectRecord {
    pub id: u32,
    pub effect_aura: i16,
    pub difficulty_id: i16,
    pub effect_index: i32,
    pub effect: u32,
    pub effect_amplitude: f32,
    pub effect_attributes: i32,
    pub effect_aura_period: i32,
    pub effect_bonus_coefficient: f32,
    pub effect_chain_amplitude: f32,
    pub effect_chain_targets: i32,
    pub effect_item_type: i32,
    pub effect_mechanic: i32,
    pub effect_points_per_resource: f32,
    pub effect_pos_facing: f32,
    pub effect_real_points_per_level: f32,
    pub effect_trigger_spell: i32,
    pub bonus_coefficient_from_ap: f32,
    pub pvp_multiplier: f32,
    pub coefficient: f32,
    pub variance: f32,
    pub resource_coefficient: f32,
    pub group_size_base_points_coefficient: f32,
    pub effect_base_points: f32,
    pub scaling_class: i32,
    pub target_node_graph: i32,
    pub effect_misc_value: [i32; 2],
    pub effect_radius_index: [u32; 2],
    pub effect_spell_class_mask: [u32; 4],
    pub implicit_target: [i16; 2],
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellMiscRecord {
    pub id: u32,
    pub attributes: [i32; 17],
    pub difficulty_id: i16,
    pub casting_time_index: u16,
    pub duration_index: u16,
    pub pv_p_duration_index: u16,
    pub range_index: u16,
    pub school_mask: u8,
    pub speed: f32,
    pub launch_delay: f32,
    pub min_duration: f32,
    pub spell_icon_file_data_id: i32,
    pub active_icon_file_data_id: i32,
    pub content_tuning_id: i32,
    pub show_future_spell_player_condition_id: i32,
    pub spell_visual_script: i32,
    pub active_spell_visual_script: i32,
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellAuraOptionsRecord {
    pub id: u32,
    pub difficulty_id: i16,
    pub cumulative_aura: u16,
    pub proc_category_recovery: i32,
    pub proc_chance: u8,
    pub proc_charges: i32,
    pub spell_procs_per_minute_id: u16,
    pub proc_type_mask: [i32; 2],
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellAuraRestrictionsRecord {
    pub id: u32,
    pub difficulty_id: i16,
    pub caster_aura_state: i32,
    pub target_aura_state: i32,
    pub exclude_caster_aura_state: i32,
    pub exclude_target_aura_state: i32,
    pub caster_aura_spell: i32,
    pub target_aura_spell: i32,
    pub exclude_caster_aura_spell: i32,
    pub exclude_target_aura_spell: i32,
    pub caster_aura_type: i16,
    pub target_aura_type: i16,
    pub exclude_caster_aura_type: i16,
    pub exclude_target_aura_type: i16,
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellCastingRequirementsRecord {
    pub id: u32,
    pub spell_id: i32,
    pub facing_caster_flags: i32,
    pub min_faction_id: u16,
    pub min_reputation: i32,
    pub required_areas_id: u16,
    pub required_aura_vision: u8,
    pub requires_spell_focus: u16,
}

#[derive(Clone, Copy)]
pub struct SpellCategoriesRecord {
    pub id: u32,
    pub difficulty_id: i16,
    pub category: i16,
    pub defense_type: i8,
    pub diminish_type: i32,
    pub dispel_type: i8,
    pub mechanic: i8,
    pub prevention_type: i32,
    pub start_recovery_category: i16,
    pub charge_category: i16,
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellClassOptionsRecord {
    pub id: u32,
    pub spell_id: i32,
    pub modal_next_spell: u32,
    pub spell_class_set: i32,
    pub spell_class_mask: [u32; 4],
}

#[derive(Clone, Copy)]
pub struct SpellCooldownsRecord {
    pub id: u32,
    pub difficulty_id: i16,
    pub category_recovery_time: i32,
    pub recovery_time: i32,
    pub start_recovery_time: i32,
    pub aura_spell_id: i32,
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellEquippedItemsRecord {
    pub id: u32,
    pub spell_id: i32,
    pub equipped_item_class: i32,
    pub equipped_item_inv_types: i32,
    pub equipped_item_subclass: i32,
}

#[derive(Clone, Copy)]
pub struct SpellInterruptsRecord {
    pub id: u32,
    pub difficulty_id: i16,
    pub interrupt_flags: i32,
    pub aura_interrupt_flags: [i32; 2],
    pub channel_interrupt_flags: [i32; 2],
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellLabelRecord {
    pub id: u32,
    pub label_id: u32,
    pub spell_id: u32,
}

#[derive(Clone, Copy)]
pub struct SpellLevelsRecord {
    pub id: u32,
    pub difficulty_id: i16,
    pub max_level: i16,
    pub max_passive_aura_level: u8,
    pub base_level: i32,
    pub spell_level: i32,
    pub spell_id: u32,
}
