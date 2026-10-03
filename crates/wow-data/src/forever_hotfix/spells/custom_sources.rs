//! DB2StorageBase::WriteRecord, 02245dcd: full metadata order/widths.
use super::text;
use crate::forever_spells::*;

pub(super) fn unit_condition(row: &UnitConditionRecord) -> Vec<u8> {
    // External ID is excluded; metadata arrays retain their declared order.
    let mut bytes = row.flags.to_le_bytes().to_vec();
    bytes.extend_from_slice(&row.variable);
    bytes.extend_from_slice(&row.op);
    for value in row.value {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn plain_text(bytes: &mut Vec<u8>, value: &[u8]) {
    let end = value
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(value.len());
    bytes.extend_from_slice(&value[..end]);
    bytes.push(0);
}

pub(super) fn talent(row: &TalentRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    text(&mut bytes, &row.description, locale);
    bytes.extend_from_slice(&row.tier_id.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.column_index.to_le_bytes());
    bytes.extend_from_slice(&row.tab_id.to_le_bytes());
    bytes.extend_from_slice(&row.class_id.to_le_bytes());
    bytes.extend_from_slice(&row.spec_id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.overrides_spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.required_spell_id.to_le_bytes());
    for value in &row.category_mask {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.spell_rank {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.prereq_talent {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.prereq_rank {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let _ = locale;
    bytes
}

pub(super) fn spell_item_enchantment(row: &SpellItemEnchantmentRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    text(&mut bytes, &row.name, locale);
    text(&mut bytes, &row.horde_name, locale);
    bytes.extend_from_slice(&row.duration.to_le_bytes());
    bytes.extend_from_slice(&row.charges.to_le_bytes());
    for value in &row.effect {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.effect_points_min {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.effect_arg {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    for value in &row.effect_scaling_points {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&row.scaling_class.to_le_bytes());
    bytes.extend_from_slice(&row.scaling_class_restricted.to_le_bytes());
    bytes.extend_from_slice(&row.condition_id.to_le_bytes());
    bytes.extend_from_slice(&row.required_skill_id.to_le_bytes());
    bytes.extend_from_slice(&row.required_skill_rank.to_le_bytes());
    bytes.extend_from_slice(&row.min_level.to_le_bytes());
    bytes.extend_from_slice(&row.max_level.to_le_bytes());
    bytes.extend_from_slice(&row.icon_file_data_id.to_le_bytes());
    bytes.extend_from_slice(&row.min_item_level.to_le_bytes());
    bytes.extend_from_slice(&row.max_item_level.to_le_bytes());
    bytes.extend_from_slice(&row.transmog_use_condition_id.to_le_bytes());
    bytes.extend_from_slice(&row.transmog_cost.to_le_bytes());
    bytes.extend_from_slice(&row.field_12_1_5_69594_021.to_le_bytes());
    bytes.extend_from_slice(&row.item_visual.to_le_bytes());
    bytes.extend_from_slice(&row.item_level.to_le_bytes());
    let _ = locale;
    bytes
}

pub(super) fn spell_visual(row: &SpellVisualRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in &row.missile_cast_offset {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.missile_impact_offset {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&row.state_kit.to_le_bytes());
    bytes.extend_from_slice(&row.anim_event_sound_id.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.missile_attachment.to_le_bytes());
    bytes.extend_from_slice(&row.missile_destination_attachment.to_le_bytes());
    bytes.extend_from_slice(&row.missile_cast_positioner_id.to_le_bytes());
    bytes.extend_from_slice(&row.missile_impact_positioner_id.to_le_bytes());
    bytes.extend_from_slice(&row.missile_targeting_kit.to_le_bytes());
    bytes.extend_from_slice(&row.hostile_spell_visual_id.to_le_bytes());
    bytes.extend_from_slice(&row.caster_spell_visual_id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_visual_missile_set_id.to_le_bytes());
    bytes.extend_from_slice(&row.damage_number_delay.to_le_bytes());
    bytes.extend_from_slice(&row.low_violence_spell_visual_id.to_le_bytes());
    bytes.extend_from_slice(&row.raid_spell_visual_missile_set_id.to_le_bytes());
    bytes.extend_from_slice(
        &row.reduced_unexpected_camera_movement_spell_visual_id
            .to_le_bytes(),
    );
    let _ = locale;
    bytes
}

pub(super) fn spell_visual_missile(row: &SpellVisualMissileRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in &row.cast_offset {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.impact_offset {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&row.id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_visual_effect_name_id.to_le_bytes());
    bytes.extend_from_slice(&row.sound_entries_id.to_le_bytes());
    bytes.extend_from_slice(&row.attachment.to_le_bytes());
    bytes.extend_from_slice(&row.destination_attachment.to_le_bytes());
    bytes.extend_from_slice(&row.cast_positioner_id.to_le_bytes());
    bytes.extend_from_slice(&row.impact_positioner_id.to_le_bytes());
    bytes.extend_from_slice(&row.follow_ground_height.to_le_bytes());
    bytes.extend_from_slice(&row.follow_ground_drop_speed.to_le_bytes());
    bytes.extend_from_slice(&row.follow_ground_approach.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.spell_missile_motion_id.to_le_bytes());
    bytes.extend_from_slice(&row.anim_kit_id.to_le_bytes());
    bytes.extend_from_slice(&row.clutter_level.to_le_bytes());
    bytes.extend_from_slice(&row.decay_time_after_impact.to_le_bytes());
    bytes.extend_from_slice(&row.unused1100.to_le_bytes());
    bytes.extend_from_slice(&row.field_12_1_5_69594_018.to_le_bytes());
    bytes.extend_from_slice(&row.field_12_1_5_69594_019.to_le_bytes());
    bytes.extend_from_slice(&row.field_12_1_5_69594_020.to_le_bytes());
    bytes.extend_from_slice(&row.field_12_1_5_69594_021.to_le_bytes());
    bytes.extend_from_slice(&row.spell_visual_missile_set_id.to_le_bytes());
    let _ = locale;
    bytes
}

pub(super) fn spell_visual_effect_name(row: &SpellVisualEffectNameRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.model_file_data_id.to_le_bytes());
    bytes.extend_from_slice(&row.base_missile_speed.to_le_bytes());
    bytes.extend_from_slice(&row.scale.to_le_bytes());
    bytes.extend_from_slice(&row.min_allowed_scale.to_le_bytes());
    bytes.extend_from_slice(&row.max_allowed_scale.to_le_bytes());
    bytes.extend_from_slice(&row.alpha.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.texture_file_data_id.to_le_bytes());
    bytes.extend_from_slice(&row.effect_radius.to_le_bytes());
    bytes.extend_from_slice(&row.r#type.to_le_bytes());
    bytes.extend_from_slice(&row.generic_id.to_le_bytes());
    bytes.extend_from_slice(&row.ribbon_quality_id.to_le_bytes());
    bytes.extend_from_slice(&row.dissolve_effect_id.to_le_bytes());
    bytes.extend_from_slice(&row.model_position.to_le_bytes());
    bytes.extend_from_slice(&row.unknown901.to_le_bytes());
    bytes.extend_from_slice(&row.unknown1100.to_le_bytes());
    let _ = locale;
    bytes
}

pub(super) fn liquid_type(row: &LiquidTypeRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    plain_text(&mut bytes, &row.name);
    for value in &row.texture {
        plain_text(&mut bytes, &value);
    }
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.sound_bank.to_le_bytes());
    bytes.extend_from_slice(&row.sound_id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.max_darken_depth.to_le_bytes());
    bytes.extend_from_slice(&row.fog_darken_intensity.to_le_bytes());
    bytes.extend_from_slice(&row.amb_darken_intensity.to_le_bytes());
    bytes.extend_from_slice(&row.dir_darken_intensity.to_le_bytes());
    bytes.extend_from_slice(&row.light_id.to_le_bytes());
    bytes.extend_from_slice(&row.particle_scale.to_le_bytes());
    bytes.extend_from_slice(&row.particle_movement.to_le_bytes());
    bytes.extend_from_slice(&row.particle_tex_slots.to_le_bytes());
    bytes.extend_from_slice(&row.material_id.to_le_bytes());
    bytes.extend_from_slice(&row.minimap_static_col.to_le_bytes());
    for value in &row.frame_count_texture {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.color {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.float_values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.int_values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.coefficient {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let _ = locale;
    bytes
}
