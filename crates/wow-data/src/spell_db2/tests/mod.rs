//! DB2 spell entry stores regression scenarios.
//!
//! Separated from the spell_db2.rs root under #646.

use super::*;

fn test_spell_misc_entry(id: u32, spell_id: u32, school_mask: u8) -> SpellMiscEntry {
    SpellMiscEntry {
        id,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 0,
        school_mask,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id,
    }
}

fn test_spell_effect_entry(id: u32, spell_id: u32, effect_mechanic: i32) -> SpellEffectDb2Entry {
    SpellEffectDb2Entry {
        id,
        difficulty_id: 0,
        effect_index: 0,
        effect: 2,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: 0,
        effect_aura_period: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
        effect_item_type: 0,
        effect_mechanic,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id,
    }
}

mod scenarios_1;
