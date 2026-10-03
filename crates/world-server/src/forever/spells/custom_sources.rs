//! One-time SQL dependency conversion; main localized strings are enUS only.
//! DB2DatabaseLoader::Load / PrimitiveResultValueConverter at 02245dcd:
//! unsigned SQL enchantment words round-trip through same-width signed cells.
use anyhow::Result;
use wow_data::forever_spells::*;
use wow_persistence::forever::spells::*;

pub(super) fn unit_condition(row: UnitConditionRow) -> Result<UnitConditionRecord> {
    Ok(UnitConditionRecord {
        id: row.id,
        flags: row.flags,
        variable: row.variable,
        op: row.op,
        value: row.value,
    })
}

pub(super) fn talent(row: TalentRow) -> Result<TalentRecord> {
    Ok(TalentRecord {
        id: row.id,
        description: SpellText::from_locale(0, row.description)?,
        tier_id: row.tier_id,
        flags: row.flags,
        column_index: row.column_index,
        tab_id: row.tab_id,
        class_id: row.class_id,
        spec_id: row.spec_id,
        spell_id: row.spell_id,
        overrides_spell_id: row.overrides_spell_id,
        required_spell_id: row.required_spell_id,
        category_mask: row.category_mask,
        spell_rank: row.spell_rank,
        prereq_talent: row.prereq_talent,
        prereq_rank: row.prereq_rank,
    })
}

pub(super) fn spell_item_enchantment(
    row: SpellItemEnchantmentRow,
) -> Result<SpellItemEnchantmentRecord> {
    Ok(SpellItemEnchantmentRecord {
        id: row.id,
        name: SpellText::from_locale(0, row.name)?,
        horde_name: SpellText::from_locale(0, row.horde_name)?,
        duration: row.duration,
        charges: row.charges as i32,
        effect: row.effect.map(|value| value as i32),
        effect_points_min: row.effect_points_min,
        effect_arg: row.effect_arg,
        flags: row.flags,
        effect_scaling_points: row.effect_scaling_points,
        scaling_class: row.scaling_class,
        scaling_class_restricted: row.scaling_class_restricted,
        condition_id: row.condition_id as i32,
        required_skill_id: row.required_skill_id as i32,
        required_skill_rank: row.required_skill_rank as i32,
        min_level: row.min_level as i32,
        max_level: row.max_level as i32,
        icon_file_data_id: row.icon_file_data_id as i32,
        min_item_level: row.min_item_level,
        max_item_level: row.max_item_level,
        transmog_use_condition_id: row.transmog_use_condition_id as i32,
        transmog_cost: row.transmog_cost as i32,
        field_12_1_5_69594_021: row.field_12_1_5_69594_021,
        item_visual: row.item_visual,
        item_level: row.item_level,
    })
}

pub(super) fn spell_visual(row: SpellVisualRow) -> Result<SpellVisualRecord> {
    Ok(SpellVisualRecord {
        id: row.id,
        missile_cast_offset: row.missile_cast_offset,
        missile_impact_offset: row.missile_impact_offset,
        state_kit: row.state_kit,
        anim_event_sound_id: row.anim_event_sound_id,
        flags: row.flags,
        missile_attachment: row.missile_attachment,
        missile_destination_attachment: row.missile_destination_attachment,
        missile_cast_positioner_id: row.missile_cast_positioner_id,
        missile_impact_positioner_id: row.missile_impact_positioner_id,
        missile_targeting_kit: row.missile_targeting_kit,
        hostile_spell_visual_id: row.hostile_spell_visual_id,
        caster_spell_visual_id: row.caster_spell_visual_id,
        spell_visual_missile_set_id: row.spell_visual_missile_set_id,
        damage_number_delay: row.damage_number_delay,
        low_violence_spell_visual_id: row.low_violence_spell_visual_id,
        raid_spell_visual_missile_set_id: row.raid_spell_visual_missile_set_id,
        reduced_unexpected_camera_movement_spell_visual_id: row
            .reduced_unexpected_camera_movement_spell_visual_id,
    })
}

pub(super) fn spell_visual_missile(row: SpellVisualMissileRow) -> Result<SpellVisualMissileRecord> {
    Ok(SpellVisualMissileRecord {
        cast_offset: row.cast_offset,
        impact_offset: row.impact_offset,
        id: row.id,
        spell_visual_effect_name_id: row.spell_visual_effect_name_id,
        sound_entries_id: row.sound_entries_id,
        attachment: row.attachment,
        destination_attachment: row.destination_attachment,
        cast_positioner_id: row.cast_positioner_id,
        impact_positioner_id: row.impact_positioner_id,
        follow_ground_height: row.follow_ground_height,
        follow_ground_drop_speed: row.follow_ground_drop_speed,
        follow_ground_approach: row.follow_ground_approach,
        flags: row.flags,
        spell_missile_motion_id: row.spell_missile_motion_id,
        anim_kit_id: row.anim_kit_id,
        clutter_level: row.clutter_level,
        decay_time_after_impact: row.decay_time_after_impact,
        unused1100: row.unused1100,
        field_12_1_5_69594_018: row.field_12_1_5_69594_018,
        field_12_1_5_69594_019: row.field_12_1_5_69594_019,
        field_12_1_5_69594_020: row.field_12_1_5_69594_020,
        field_12_1_5_69594_021: row.field_12_1_5_69594_021,
        spell_visual_missile_set_id: row.spell_visual_missile_set_id,
    })
}

pub(super) fn spell_visual_effect_name(
    row: SpellVisualEffectNameRow,
) -> Result<SpellVisualEffectNameRecord> {
    Ok(SpellVisualEffectNameRecord {
        id: row.id,
        model_file_data_id: row.model_file_data_id,
        base_missile_speed: row.base_missile_speed,
        scale: row.scale,
        min_allowed_scale: row.min_allowed_scale,
        max_allowed_scale: row.max_allowed_scale,
        alpha: row.alpha,
        flags: row.flags,
        texture_file_data_id: row.texture_file_data_id,
        effect_radius: row.effect_radius,
        r#type: row.r#type,
        generic_id: row.generic_id,
        ribbon_quality_id: row.ribbon_quality_id,
        dissolve_effect_id: row.dissolve_effect_id,
        model_position: row.model_position,
        unknown901: row.unknown901,
        unknown1100: row.unknown1100,
    })
}

pub(super) fn liquid_type(row: LiquidTypeRow) -> Result<LiquidTypeRecord> {
    Ok(LiquidTypeRecord {
        id: row.id,
        name: row.name,
        texture: row.texture,
        flags: row.flags,
        sound_bank: row.sound_bank,
        sound_id: row.sound_id,
        spell_id: row.spell_id,
        max_darken_depth: row.max_darken_depth,
        fog_darken_intensity: row.fog_darken_intensity,
        amb_darken_intensity: row.amb_darken_intensity,
        dir_darken_intensity: row.dir_darken_intensity,
        light_id: row.light_id,
        particle_scale: row.particle_scale,
        particle_movement: row.particle_movement,
        particle_tex_slots: row.particle_tex_slots,
        material_id: row.material_id,
        minimap_static_col: row.minimap_static_col,
        frame_count_texture: row.frame_count_texture,
        color: row.color,
        float_values: row.float_values,
        int_values: row.int_values,
        coefficient: row.coefficient,
    })
}
