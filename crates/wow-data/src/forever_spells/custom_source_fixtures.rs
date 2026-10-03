//! Shared synthetic dependency rows only; never private client fixtures.
use super::*;

pub(crate) fn unit_condition(id: u32) -> UnitConditionRecord {
    UnitConditionRecord {
        id,
        flags: 0x80000020u32 as i32,
        variable: [0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87],
        op: [0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7],
        value: std::array::from_fn(|i| (0x80000030u32 + i as u32) as i32),
    }
}

pub fn talent(id: u32, locale: u8, text: &[u8]) -> TalentRecord {
    let _ = (locale, text);
    TalentRecord {
        id,
        description: SpellText::from_locale(locale, text.to_vec()).unwrap(),
        tier_id: 0x90,
        flags: 0x80000020u32 as i32,
        column_index: 0xb0,
        tab_id: 0x8040,
        class_id: 0xd0u8 as i8,
        spec_id: 0x8060,
        spell_id: 0x80000070,
        overrides_spell_id: 0x80000080,
        required_spell_id: 0x80000090,
        category_mask: [0x800000a0u32 as i32, 0x800000a1u32 as i32],
        spell_rank: [
            0x800000b0, 0x800000b1, 0x800000b2, 0x800000b3, 0x800000b4, 0x800000b5, 0x800000b6,
            0x800000b7, 0x800000b8,
        ],
        prereq_talent: [0x800000c0, 0x800000c1, 0x800000c2],
        prereq_rank: [0xd0, 0xd1, 0xd2],
    }
}

pub fn spell_item_enchantment(id: u32, locale: u8, text: &[u8]) -> SpellItemEnchantmentRecord {
    let _ = (locale, text);
    SpellItemEnchantmentRecord {
        id,
        name: SpellText::from_locale(locale, text.to_vec()).unwrap(),
        horde_name: SpellText::from_locale(locale, text.to_vec()).unwrap(),
        duration: 0x80000020u32 as i32,
        charges: 0x80000030u32 as i32,
        effect: [
            0x80000040u32 as i32,
            0x80000041u32 as i32,
            0x80000042u32 as i32,
        ],
        effect_points_min: [
            0x80000050u32 as i32,
            0x80000051u32 as i32,
            0x80000052u32 as i32,
        ],
        effect_arg: [0x80000060, 0x80000061, 0x80000062],
        flags: 0x80000070u32 as i32,
        effect_scaling_points: [
            f32::from_bits(0x7fc00080),
            f32::from_bits(0x7fc00081),
            f32::from_bits(0x7fc00082),
        ],
        scaling_class: 0x80000090u32 as i32,
        scaling_class_restricted: 0x800000a0u32 as i32,
        condition_id: 0x800000b0u32 as i32,
        required_skill_id: 0x800000c0u32 as i32,
        required_skill_rank: 0x800000d0u32 as i32,
        min_level: 0x800000e0u32 as i32,
        max_level: 0x800000f0u32 as i32,
        icon_file_data_id: 0x80000100u32 as i32,
        min_item_level: 0x80000110u32 as i32,
        max_item_level: 0x80000120u32 as i32,
        transmog_use_condition_id: 0x80000130u32 as i32,
        transmog_cost: 0x80000140u32 as i32,
        field_12_1_5_69594_021: 0x80000150u32 as i32,
        item_visual: 0x8160,
        item_level: 0x8170,
    }
}

pub fn spell_visual(id: u32, locale: u8, text: &[u8]) -> SpellVisualRecord {
    let _ = (locale, text);
    SpellVisualRecord {
        id,
        missile_cast_offset: [
            f32::from_bits(0x7fc00000),
            f32::from_bits(0x7fc00001),
            f32::from_bits(0x7fc00002),
        ],
        missile_impact_offset: [
            f32::from_bits(0x7fc00010),
            f32::from_bits(0x7fc00011),
            f32::from_bits(0x7fc00012),
        ],
        state_kit: 0x80000020u32 as i32,
        anim_event_sound_id: 0x80000030,
        flags: 0x80000040u32 as i32,
        missile_attachment: 0xd0u8 as i8,
        missile_destination_attachment: 0xe0u8 as i8,
        missile_cast_positioner_id: 0x80000070,
        missile_impact_positioner_id: 0x80000080,
        missile_targeting_kit: 0x80000090u32 as i32,
        hostile_spell_visual_id: 0x800000a0,
        caster_spell_visual_id: 0x800000b0,
        spell_visual_missile_set_id: 0x80c0,
        damage_number_delay: 0x80d0,
        low_violence_spell_visual_id: 0x800000e0,
        raid_spell_visual_missile_set_id: 0x800000f0,
        reduced_unexpected_camera_movement_spell_visual_id: 0x80000100u32 as i32,
    }
}

pub fn spell_visual_missile(id: u32, locale: u8, text: &[u8]) -> SpellVisualMissileRecord {
    let _ = (locale, text);
    SpellVisualMissileRecord {
        cast_offset: [
            f32::from_bits(0x7fc00000),
            f32::from_bits(0x7fc00001),
            f32::from_bits(0x7fc00002),
        ],
        impact_offset: [
            f32::from_bits(0x7fc00010),
            f32::from_bits(0x7fc00011),
            f32::from_bits(0x7fc00012),
        ],
        id: id,
        spell_visual_effect_name_id: 0x8030,
        sound_entries_id: 0x80000040,
        attachment: 0xd0u8 as i8,
        destination_attachment: 0xe0u8 as i8,
        cast_positioner_id: 0x8070,
        impact_positioner_id: 0x8080,
        follow_ground_height: 0x80000090u32 as i32,
        follow_ground_drop_speed: 0x800000a0,
        follow_ground_approach: 0x80b0,
        flags: 0x800000c0u32 as i32,
        spell_missile_motion_id: 0x80d0,
        anim_kit_id: 0x800000e0,
        clutter_level: 0x800000f0u32 as i32,
        decay_time_after_impact: 0x80000100u32 as i32,
        unused1100: 0x8110,
        field_12_1_5_69594_018: 0x80000120u32 as i32,
        field_12_1_5_69594_019: 0x80000130u32 as i32,
        field_12_1_5_69594_020: 0x80000140u32 as i32,
        field_12_1_5_69594_021: 0x80000150u32 as i32,
        spell_visual_missile_set_id: 0x80000160,
    }
}

pub fn spell_visual_effect_name(id: u32, locale: u8, text: &[u8]) -> SpellVisualEffectNameRecord {
    let _ = (locale, text);
    SpellVisualEffectNameRecord {
        id,
        model_file_data_id: 0x80000000u32 as i32,
        base_missile_speed: f32::from_bits(0x7fc00010),
        scale: f32::from_bits(0x7fc00020),
        min_allowed_scale: f32::from_bits(0x7fc00030),
        max_allowed_scale: f32::from_bits(0x7fc00040),
        alpha: f32::from_bits(0x7fc00050),
        flags: 0x80000060u32 as i32,
        texture_file_data_id: 0x80000070u32 as i32,
        effect_radius: f32::from_bits(0x7fc00080),
        r#type: 0x80000090u32 as i32,
        generic_id: 0x800000a0u32 as i32,
        ribbon_quality_id: 0x800000b0,
        dissolve_effect_id: 0x800000c0u32 as i32,
        model_position: 0x800000d0u32 as i32,
        unknown901: 0xe0u8 as i8,
        unknown1100: 0x80f0,
    }
}

pub fn liquid_type(id: u32, locale: u8, text: &[u8]) -> LiquidTypeRecord {
    let _ = (locale, text);
    LiquidTypeRecord {
        id,
        name: text.to_vec(),
        texture: [
            text.to_vec(),
            text.to_vec(),
            text.to_vec(),
            text.to_vec(),
            text.to_vec(),
            text.to_vec(),
        ],
        flags: 0x80000020u32 as i32,
        sound_bank: 0xb0,
        sound_id: 0x80000040,
        spell_id: 0x80000050,
        max_darken_depth: f32::from_bits(0x7fc00060),
        fog_darken_intensity: f32::from_bits(0x7fc00070),
        amb_darken_intensity: f32::from_bits(0x7fc00080),
        dir_darken_intensity: f32::from_bits(0x7fc00090),
        light_id: 0x80a0,
        particle_scale: f32::from_bits(0x7fc000b0),
        particle_movement: 0xc0,
        particle_tex_slots: 0xd0,
        material_id: 0xe0,
        minimap_static_col: 0x800000f0u32 as i32,
        frame_count_texture: [0x80, 0x81, 0x82, 0x83, 0x84, 0x85],
        color: [
            0x80000110u32 as i32,
            0x80000111u32 as i32,
            0x80000112u32 as i32,
        ],
        float_values: [
            f32::from_bits(0x7fc00120),
            f32::from_bits(0x7fc00121),
            f32::from_bits(0x7fc00122),
            f32::from_bits(0x7fc00123),
            f32::from_bits(0x7fc00124),
            f32::from_bits(0x7fc00125),
            f32::from_bits(0x7fc00126),
            f32::from_bits(0x7fc00127),
            f32::from_bits(0x7fc00128),
            f32::from_bits(0x7fc00129),
            f32::from_bits(0x7fc0012a),
            f32::from_bits(0x7fc0012b),
            f32::from_bits(0x7fc0012c),
            f32::from_bits(0x7fc0012d),
            f32::from_bits(0x7fc0012e),
            f32::from_bits(0x7fc0012f),
            f32::from_bits(0x7fc00130),
            f32::from_bits(0x7fc00131),
            f32::from_bits(0x7fc00132),
            f32::from_bits(0x7fc00133),
            f32::from_bits(0x7fc00134),
            f32::from_bits(0x7fc00135),
            f32::from_bits(0x7fc00136),
            f32::from_bits(0x7fc00137),
            f32::from_bits(0x7fc00138),
            f32::from_bits(0x7fc00139),
            f32::from_bits(0x7fc0013a),
            f32::from_bits(0x7fc0013b),
            f32::from_bits(0x7fc0013c),
            f32::from_bits(0x7fc0013d),
            f32::from_bits(0x7fc0013e),
            f32::from_bits(0x7fc0013f),
            f32::from_bits(0x7fc00140),
            f32::from_bits(0x7fc00141),
            f32::from_bits(0x7fc00142),
            f32::from_bits(0x7fc00143),
            f32::from_bits(0x7fc00144),
            f32::from_bits(0x7fc00145),
        ],
        int_values: [0x80000130, 0x80000131, 0x80000132, 0x80000133],
        coefficient: [
            f32::from_bits(0x7fc00140),
            f32::from_bits(0x7fc00141),
            f32::from_bits(0x7fc00142),
            f32::from_bits(0x7fc00143),
        ],
    }
}
