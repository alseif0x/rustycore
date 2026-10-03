//! Full source SQL columns, no reduced custom-attribute projection.
use super::{LoadError, SpellRow};
use crate::SqlResult;
use wow_persistence::forever::spells::*;

pub(super) fn unit_condition(result: &SqlResult) -> Result<UnitConditionRow, LoadError> {
    let mut r = SpellRow::new(result, 26)?;
    let row = UnitConditionRow {
        id: r.read()?,
        flags: r.read()?,
        variable: r.array()?,
        op: r.array()?,
        value: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn talent(result: &SqlResult) -> Result<TalentRow, LoadError> {
    let mut r = SpellRow::new(result, 28)?;
    let row = TalentRow {
        id: r.read()?,
        description: r.text()?,
        tier_id: r.read()?,
        flags: r.read()?,
        column_index: r.read()?,
        tab_id: r.read()?,
        class_id: r.read()?,
        spec_id: r.read()?,
        spell_id: r.read()?,
        overrides_spell_id: r.read()?,
        required_spell_id: r.read()?,
        category_mask: r.array()?,
        spell_rank: r.array()?,
        prereq_talent: r.array()?,
        prereq_rank: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_item_enchantment(
    result: &SqlResult,
) -> Result<SpellItemEnchantmentRow, LoadError> {
    let mut r = SpellRow::new(result, 33)?;
    let row = SpellItemEnchantmentRow {
        id: r.read()?,
        name: r.text()?,
        horde_name: r.text()?,
        duration: r.read()?,
        charges: r.read()?,
        effect: r.array()?,
        effect_points_min: r.array()?,
        effect_arg: r.array()?,
        flags: r.read()?,
        effect_scaling_points: r.array()?,
        scaling_class: r.read()?,
        scaling_class_restricted: r.read()?,
        condition_id: r.read()?,
        required_skill_id: r.read()?,
        required_skill_rank: r.read()?,
        min_level: r.read()?,
        max_level: r.read()?,
        icon_file_data_id: r.read()?,
        min_item_level: r.read()?,
        max_item_level: r.read()?,
        transmog_use_condition_id: r.read()?,
        transmog_cost: r.read()?,
        field_12_1_5_69594_021: r.read()?,
        item_visual: r.read()?,
        item_level: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_visual(result: &SqlResult) -> Result<SpellVisualRow, LoadError> {
    let mut r = SpellRow::new(result, 22)?;
    let row = SpellVisualRow {
        id: r.read()?,
        missile_cast_offset: r.array()?,
        missile_impact_offset: r.array()?,
        state_kit: r.read()?,
        anim_event_sound_id: r.read()?,
        flags: r.read()?,
        missile_attachment: r.read()?,
        missile_destination_attachment: r.read()?,
        missile_cast_positioner_id: r.read()?,
        missile_impact_positioner_id: r.read()?,
        missile_targeting_kit: r.read()?,
        hostile_spell_visual_id: r.read()?,
        caster_spell_visual_id: r.read()?,
        spell_visual_missile_set_id: r.read()?,
        damage_number_delay: r.read()?,
        low_violence_spell_visual_id: r.read()?,
        raid_spell_visual_missile_set_id: r.read()?,
        reduced_unexpected_camera_movement_spell_visual_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_visual_missile(result: &SqlResult) -> Result<SpellVisualMissileRow, LoadError> {
    let mut r = SpellRow::new(result, 27)?;
    let row = SpellVisualMissileRow {
        cast_offset: r.array()?,
        impact_offset: r.array()?,
        id: r.read()?,
        spell_visual_effect_name_id: r.read()?,
        sound_entries_id: r.read()?,
        attachment: r.read()?,
        destination_attachment: r.read()?,
        cast_positioner_id: r.read()?,
        impact_positioner_id: r.read()?,
        follow_ground_height: r.read()?,
        follow_ground_drop_speed: r.read()?,
        follow_ground_approach: r.read()?,
        flags: r.read()?,
        spell_missile_motion_id: r.read()?,
        anim_kit_id: r.read()?,
        clutter_level: r.read()?,
        decay_time_after_impact: r.read()?,
        unused1100: r.read()?,
        field_12_1_5_69594_018: r.read()?,
        field_12_1_5_69594_019: r.read()?,
        field_12_1_5_69594_020: r.read()?,
        field_12_1_5_69594_021: r.read()?,
        spell_visual_missile_set_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_visual_effect_name(
    result: &SqlResult,
) -> Result<SpellVisualEffectNameRow, LoadError> {
    let mut r = SpellRow::new(result, 17)?;
    let row = SpellVisualEffectNameRow {
        id: r.read()?,
        model_file_data_id: r.read()?,
        base_missile_speed: r.read()?,
        scale: r.read()?,
        min_allowed_scale: r.read()?,
        max_allowed_scale: r.read()?,
        alpha: r.read()?,
        flags: r.read()?,
        texture_file_data_id: r.read()?,
        effect_radius: r.read()?,
        r#type: r.read()?,
        generic_id: r.read()?,
        ribbon_quality_id: r.read()?,
        dissolve_effect_id: r.read()?,
        model_position: r.read()?,
        unknown901: r.read()?,
        unknown1100: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn liquid_type(result: &SqlResult) -> Result<LiquidTypeRow, LoadError> {
    let mut r = SpellRow::new(result, 77)?;
    let id = r.read()?;
    let name = r.text()?;
    let mut texture = std::array::from_fn(|_| Vec::new());
    for value in &mut texture {
        *value = r.text()?;
    }
    let row = LiquidTypeRow {
        id: id,
        name: name,
        texture: texture,
        flags: r.read()?,
        sound_bank: r.read()?,
        sound_id: r.read()?,
        spell_id: r.read()?,
        max_darken_depth: r.read()?,
        fog_darken_intensity: r.read()?,
        amb_darken_intensity: r.read()?,
        dir_darken_intensity: r.read()?,
        light_id: r.read()?,
        particle_scale: r.read()?,
        particle_movement: r.read()?,
        particle_tex_slots: r.read()?,
        material_id: r.read()?,
        minimap_static_col: r.read()?,
        frame_count_texture: r.array()?,
        color: r.array()?,
        float_values: r.array()?,
        int_values: r.array()?,
        coefficient: r.array()?,
    };
    r.finish()?;
    Ok(row)
}
