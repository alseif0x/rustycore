//! Checked full-width DB2 dependency readers, not derived custom attributes.
use super::super::*;
use crate::wdc4::creation::CreationDb2;
use anyhow::Result;

pub(super) fn unit_condition(table: &CreationDb2, id: u32) -> Result<UnitConditionRecord> {
    let mut variable = [0; 8];
    let mut op = [0; 8];
    let mut value = [0; 8];
    for index in 0..8 {
        variable[index] = table.bits(id, 1, index)? as u8;
        op[index] = table.bits(id, 2, index)? as u8;
        value[index] = table.bits(id, 3, index)? as i32;
    }
    Ok(UnitConditionRecord {
        id,
        flags: table.bits(id, 0, 0)? as i32,
        variable,
        op,
        value,
    })
}

pub(super) fn talent(table: &CreationDb2, id: u32) -> Result<TalentRecord> {
    Ok(TalentRecord {
        id: id,
        description: SpellText::from_locale(6, table.spell_text(id, 0)?)?,
        tier_id: table.bits(id, 1, 0)? as u8,
        flags: table.bits(id, 2, 0)? as i32,
        column_index: table.bits(id, 3, 0)? as u8,
        tab_id: table.bits(id, 4, 0)? as u16,
        class_id: table.bits(id, 5, 0)? as i8,
        spec_id: table.bits(id, 6, 0)? as u16,
        spell_id: table.bits(id, 7, 0)?,
        overrides_spell_id: table.bits(id, 8, 0)?,
        required_spell_id: table.bits(id, 9, 0)?,
        category_mask: [table.bits(id, 10, 0)? as i32, table.bits(id, 10, 1)? as i32],
        spell_rank: [
            table.bits(id, 11, 0)?,
            table.bits(id, 11, 1)?,
            table.bits(id, 11, 2)?,
            table.bits(id, 11, 3)?,
            table.bits(id, 11, 4)?,
            table.bits(id, 11, 5)?,
            table.bits(id, 11, 6)?,
            table.bits(id, 11, 7)?,
            table.bits(id, 11, 8)?,
        ],
        prereq_talent: [
            table.bits(id, 12, 0)?,
            table.bits(id, 12, 1)?,
            table.bits(id, 12, 2)?,
        ],
        prereq_rank: [
            table.bits(id, 13, 0)? as u8,
            table.bits(id, 13, 1)? as u8,
            table.bits(id, 13, 2)? as u8,
        ],
    })
}

pub(super) fn spell_item_enchantment(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellItemEnchantmentRecord> {
    Ok(SpellItemEnchantmentRecord {
        id: id,
        name: SpellText::from_locale(6, table.spell_text(id, 0)?)?,
        horde_name: SpellText::from_locale(6, table.spell_text(id, 1)?)?,
        duration: table.bits(id, 2, 0)? as i32,
        charges: table.bits(id, 3, 0)? as i32,
        effect: [
            table.bits(id, 4, 0)? as i32,
            table.bits(id, 4, 1)? as i32,
            table.bits(id, 4, 2)? as i32,
        ],
        effect_points_min: [
            table.bits(id, 5, 0)? as i32,
            table.bits(id, 5, 1)? as i32,
            table.bits(id, 5, 2)? as i32,
        ],
        effect_arg: [
            table.bits(id, 6, 0)?,
            table.bits(id, 6, 1)?,
            table.bits(id, 6, 2)?,
        ],
        flags: table.bits(id, 7, 0)? as i32,
        effect_scaling_points: [
            f32::from_bits(table.bits(id, 8, 0)?),
            f32::from_bits(table.bits(id, 8, 1)?),
            f32::from_bits(table.bits(id, 8, 2)?),
        ],
        scaling_class: table.bits(id, 9, 0)? as i32,
        scaling_class_restricted: table.bits(id, 10, 0)? as i32,
        condition_id: table.bits(id, 11, 0)? as i32,
        required_skill_id: table.bits(id, 12, 0)? as i32,
        required_skill_rank: table.bits(id, 13, 0)? as i32,
        min_level: table.bits(id, 14, 0)? as i32,
        max_level: table.bits(id, 15, 0)? as i32,
        icon_file_data_id: table.bits(id, 16, 0)? as i32,
        min_item_level: table.bits(id, 17, 0)? as i32,
        max_item_level: table.bits(id, 18, 0)? as i32,
        transmog_use_condition_id: table.bits(id, 19, 0)? as i32,
        transmog_cost: table.bits(id, 20, 0)? as i32,
        field_12_1_5_69594_021: table.bits(id, 21, 0)? as i32,
        item_visual: table.bits(id, 22, 0)? as u16,
        item_level: table.bits(id, 23, 0)? as u16,
    })
}

pub(super) fn spell_visual(table: &CreationDb2, id: u32) -> Result<SpellVisualRecord> {
    Ok(SpellVisualRecord {
        id: id,
        missile_cast_offset: [
            f32::from_bits(table.bits(id, 0, 0)?),
            f32::from_bits(table.bits(id, 0, 1)?),
            f32::from_bits(table.bits(id, 0, 2)?),
        ],
        missile_impact_offset: [
            f32::from_bits(table.bits(id, 1, 0)?),
            f32::from_bits(table.bits(id, 1, 1)?),
            f32::from_bits(table.bits(id, 1, 2)?),
        ],
        state_kit: table.bits(id, 2, 0)? as i32,
        anim_event_sound_id: table.bits(id, 3, 0)?,
        flags: table.bits(id, 4, 0)? as i32,
        missile_attachment: table.bits(id, 5, 0)? as i8,
        missile_destination_attachment: table.bits(id, 6, 0)? as i8,
        missile_cast_positioner_id: table.bits(id, 7, 0)?,
        missile_impact_positioner_id: table.bits(id, 8, 0)?,
        missile_targeting_kit: table.bits(id, 9, 0)? as i32,
        hostile_spell_visual_id: table.bits(id, 10, 0)?,
        caster_spell_visual_id: table.bits(id, 11, 0)?,
        spell_visual_missile_set_id: table.bits(id, 12, 0)? as u16,
        damage_number_delay: table.bits(id, 13, 0)? as u16,
        low_violence_spell_visual_id: table.bits(id, 14, 0)?,
        raid_spell_visual_missile_set_id: table.bits(id, 15, 0)?,
        reduced_unexpected_camera_movement_spell_visual_id: table.bits(id, 16, 0)? as i32,
    })
}

pub(super) fn spell_visual_missile(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellVisualMissileRecord> {
    Ok(SpellVisualMissileRecord {
        cast_offset: [
            f32::from_bits(table.bits(id, 0, 0)?),
            f32::from_bits(table.bits(id, 0, 1)?),
            f32::from_bits(table.bits(id, 0, 2)?),
        ],
        impact_offset: [
            f32::from_bits(table.bits(id, 1, 0)?),
            f32::from_bits(table.bits(id, 1, 1)?),
            f32::from_bits(table.bits(id, 1, 2)?),
        ],
        id: id,
        spell_visual_effect_name_id: table.bits(id, 3, 0)? as u16,
        sound_entries_id: table.bits(id, 4, 0)?,
        attachment: table.bits(id, 5, 0)? as i8,
        destination_attachment: table.bits(id, 6, 0)? as i8,
        cast_positioner_id: table.bits(id, 7, 0)? as u16,
        impact_positioner_id: table.bits(id, 8, 0)? as u16,
        follow_ground_height: table.bits(id, 9, 0)? as i32,
        follow_ground_drop_speed: table.bits(id, 10, 0)?,
        follow_ground_approach: table.bits(id, 11, 0)? as u16,
        flags: table.bits(id, 12, 0)? as i32,
        spell_missile_motion_id: table.bits(id, 13, 0)? as u16,
        anim_kit_id: table.bits(id, 14, 0)?,
        clutter_level: table.bits(id, 15, 0)? as i32,
        decay_time_after_impact: table.bits(id, 16, 0)? as i32,
        unused1100: table.bits(id, 17, 0)? as u16,
        field_12_1_5_69594_018: table.bits(id, 18, 0)? as i32,
        field_12_1_5_69594_019: table.bits(id, 19, 0)? as i32,
        field_12_1_5_69594_020: table.bits(id, 20, 0)? as i32,
        field_12_1_5_69594_021: table.bits(id, 21, 0)? as i32,
        spell_visual_missile_set_id: table.bits(id, 22, 0)?,
    })
}

pub(super) fn spell_visual_effect_name(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellVisualEffectNameRecord> {
    Ok(SpellVisualEffectNameRecord {
        id: id,
        model_file_data_id: table.bits(id, 0, 0)? as i32,
        base_missile_speed: f32::from_bits(table.bits(id, 1, 0)?),
        scale: f32::from_bits(table.bits(id, 2, 0)?),
        min_allowed_scale: f32::from_bits(table.bits(id, 3, 0)?),
        max_allowed_scale: f32::from_bits(table.bits(id, 4, 0)?),
        alpha: f32::from_bits(table.bits(id, 5, 0)?),
        flags: table.bits(id, 6, 0)? as i32,
        texture_file_data_id: table.bits(id, 7, 0)? as i32,
        effect_radius: f32::from_bits(table.bits(id, 8, 0)?),
        r#type: table.bits(id, 9, 0)? as i32,
        generic_id: table.bits(id, 10, 0)? as i32,
        ribbon_quality_id: table.bits(id, 11, 0)?,
        dissolve_effect_id: table.bits(id, 12, 0)? as i32,
        model_position: table.bits(id, 13, 0)? as i32,
        unknown901: table.bits(id, 14, 0)? as i8,
        unknown1100: table.bits(id, 15, 0)? as u16,
    })
}

pub(super) fn liquid_type(table: &CreationDb2, id: u32) -> Result<LiquidTypeRecord> {
    Ok(LiquidTypeRecord {
        id: id,
        name: table.spell_text_component(id, 0, 0)?,
        texture: [
            table.spell_text_component(id, 1, 0)?,
            table.spell_text_component(id, 1, 1)?,
            table.spell_text_component(id, 1, 2)?,
            table.spell_text_component(id, 1, 3)?,
            table.spell_text_component(id, 1, 4)?,
            table.spell_text_component(id, 1, 5)?,
        ],
        flags: table.bits(id, 2, 0)? as i32,
        sound_bank: table.bits(id, 3, 0)? as u8,
        sound_id: table.bits(id, 4, 0)?,
        spell_id: table.bits(id, 5, 0)?,
        max_darken_depth: f32::from_bits(table.bits(id, 6, 0)?),
        fog_darken_intensity: f32::from_bits(table.bits(id, 7, 0)?),
        amb_darken_intensity: f32::from_bits(table.bits(id, 8, 0)?),
        dir_darken_intensity: f32::from_bits(table.bits(id, 9, 0)?),
        light_id: table.bits(id, 10, 0)? as u16,
        particle_scale: f32::from_bits(table.bits(id, 11, 0)?),
        particle_movement: table.bits(id, 12, 0)? as u8,
        particle_tex_slots: table.bits(id, 13, 0)? as u8,
        material_id: table.bits(id, 14, 0)? as u8,
        minimap_static_col: table.bits(id, 15, 0)? as i32,
        frame_count_texture: [
            table.bits(id, 16, 0)? as u8,
            table.bits(id, 16, 1)? as u8,
            table.bits(id, 16, 2)? as u8,
            table.bits(id, 16, 3)? as u8,
            table.bits(id, 16, 4)? as u8,
            table.bits(id, 16, 5)? as u8,
        ],
        color: [
            table.bits(id, 17, 0)? as i32,
            table.bits(id, 17, 1)? as i32,
            table.bits(id, 17, 2)? as i32,
        ],
        float_values: [
            f32::from_bits(table.bits(id, 18, 0)?),
            f32::from_bits(table.bits(id, 18, 1)?),
            f32::from_bits(table.bits(id, 18, 2)?),
            f32::from_bits(table.bits(id, 18, 3)?),
            f32::from_bits(table.bits(id, 18, 4)?),
            f32::from_bits(table.bits(id, 18, 5)?),
            f32::from_bits(table.bits(id, 18, 6)?),
            f32::from_bits(table.bits(id, 18, 7)?),
            f32::from_bits(table.bits(id, 18, 8)?),
            f32::from_bits(table.bits(id, 18, 9)?),
            f32::from_bits(table.bits(id, 18, 10)?),
            f32::from_bits(table.bits(id, 18, 11)?),
            f32::from_bits(table.bits(id, 18, 12)?),
            f32::from_bits(table.bits(id, 18, 13)?),
            f32::from_bits(table.bits(id, 18, 14)?),
            f32::from_bits(table.bits(id, 18, 15)?),
            f32::from_bits(table.bits(id, 18, 16)?),
            f32::from_bits(table.bits(id, 18, 17)?),
            f32::from_bits(table.bits(id, 18, 18)?),
            f32::from_bits(table.bits(id, 18, 19)?),
            f32::from_bits(table.bits(id, 18, 20)?),
            f32::from_bits(table.bits(id, 18, 21)?),
            f32::from_bits(table.bits(id, 18, 22)?),
            f32::from_bits(table.bits(id, 18, 23)?),
            f32::from_bits(table.bits(id, 18, 24)?),
            f32::from_bits(table.bits(id, 18, 25)?),
            f32::from_bits(table.bits(id, 18, 26)?),
            f32::from_bits(table.bits(id, 18, 27)?),
            f32::from_bits(table.bits(id, 18, 28)?),
            f32::from_bits(table.bits(id, 18, 29)?),
            f32::from_bits(table.bits(id, 18, 30)?),
            f32::from_bits(table.bits(id, 18, 31)?),
            f32::from_bits(table.bits(id, 18, 32)?),
            f32::from_bits(table.bits(id, 18, 33)?),
            f32::from_bits(table.bits(id, 18, 34)?),
            f32::from_bits(table.bits(id, 18, 35)?),
            f32::from_bits(table.bits(id, 18, 36)?),
            f32::from_bits(table.bits(id, 18, 37)?),
        ],
        int_values: [
            table.bits(id, 19, 0)?,
            table.bits(id, 19, 1)?,
            table.bits(id, 19, 2)?,
            table.bits(id, 19, 3)?,
        ],
        coefficient: [
            f32::from_bits(table.bits(id, 20, 0)?),
            f32::from_bits(table.bits(id, 20, 1)?),
            f32::from_bits(table.bits(id, 20, 2)?),
            f32::from_bits(table.bits(id, 20, 3)?),
        ],
    })
}
