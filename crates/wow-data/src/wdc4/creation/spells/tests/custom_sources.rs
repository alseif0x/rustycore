use super::{CreationDb2, CreationTable, SpellTable, cell, reader};

pub(super) fn prepare(table: SpellTable, source: &mut crate::wdc4::Wdc4Reader) {
    match table {
        SpellTable::UnitCondition => {
            cell(source, 0, 0, 32, 0x80000020);
            for index in 0..8 {
                cell(source, 1, index, 8, 0x80 + index as u32);
                cell(source, 2, index, 8, 0xf0 + index as u32);
                cell(source, 3, index, 32, 0x80000030 + index as u32);
            }
        }
        SpellTable::Talent => {
            cell(source, 1, 0, 8, 0x90);
            cell(source, 2, 0, 32, 0x80000020);
            cell(source, 3, 0, 8, 0xb0);
            cell(source, 4, 0, 16, 0x8040);
            cell(source, 5, 0, 8, 0xd0);
            cell(source, 6, 0, 16, 0x8060);
            cell(source, 7, 0, 32, 0x80000070);
            cell(source, 8, 0, 32, 0x80000080);
            cell(source, 9, 0, 32, 0x80000090);
            cell(source, 10, 0, 32, 0x800000a0);
            cell(source, 10, 1, 32, 0x800000a1);
            cell(source, 11, 0, 32, 0x800000b0);
            cell(source, 11, 1, 32, 0x800000b1);
            cell(source, 11, 2, 32, 0x800000b2);
            cell(source, 11, 3, 32, 0x800000b3);
            cell(source, 11, 4, 32, 0x800000b4);
            cell(source, 11, 5, 32, 0x800000b5);
            cell(source, 11, 6, 32, 0x800000b6);
            cell(source, 11, 7, 32, 0x800000b7);
            cell(source, 11, 8, 32, 0x800000b8);
            cell(source, 12, 0, 32, 0x800000c0);
            cell(source, 12, 1, 32, 0x800000c1);
            cell(source, 12, 2, 32, 0x800000c2);
            cell(source, 13, 0, 8, 0xd0);
            cell(source, 13, 1, 8, 0xd1);
            cell(source, 13, 2, 8, 0xd2);
        }
        SpellTable::SpellItemEnchantment => {
            cell(source, 2, 0, 32, 0x80000020);
            cell(source, 3, 0, 32, 0x80000030);
            cell(source, 4, 0, 32, 0x80000040);
            cell(source, 4, 1, 32, 0x80000041);
            cell(source, 4, 2, 32, 0x80000042);
            cell(source, 5, 0, 32, 0x80000050);
            cell(source, 5, 1, 32, 0x80000051);
            cell(source, 5, 2, 32, 0x80000052);
            cell(source, 6, 0, 32, 0x80000060);
            cell(source, 6, 1, 32, 0x80000061);
            cell(source, 6, 2, 32, 0x80000062);
            cell(source, 7, 0, 32, 0x80000070);
            cell(source, 8, 0, 32, 0x7fc00080);
            cell(source, 8, 1, 32, 0x7fc00081);
            cell(source, 8, 2, 32, 0x7fc00082);
            cell(source, 9, 0, 32, 0x80000090);
            cell(source, 10, 0, 32, 0x800000a0);
            cell(source, 11, 0, 32, 0x800000b0);
            cell(source, 12, 0, 32, 0x800000c0);
            cell(source, 13, 0, 32, 0x800000d0);
            cell(source, 14, 0, 32, 0x800000e0);
            cell(source, 15, 0, 32, 0x800000f0);
            cell(source, 16, 0, 32, 0x80000100);
            cell(source, 17, 0, 32, 0x80000110);
            cell(source, 18, 0, 32, 0x80000120);
            cell(source, 19, 0, 32, 0x80000130);
            cell(source, 20, 0, 32, 0x80000140);
            cell(source, 21, 0, 32, 0x80000150);
            cell(source, 22, 0, 16, 0x8160);
            cell(source, 23, 0, 16, 0x8170);
        }
        SpellTable::SpellVisual => {
            cell(source, 0, 0, 32, 0x7fc00000);
            cell(source, 0, 1, 32, 0x7fc00001);
            cell(source, 0, 2, 32, 0x7fc00002);
            cell(source, 1, 0, 32, 0x7fc00010);
            cell(source, 1, 1, 32, 0x7fc00011);
            cell(source, 1, 2, 32, 0x7fc00012);
            cell(source, 2, 0, 32, 0x80000020);
            cell(source, 3, 0, 32, 0x80000030);
            cell(source, 4, 0, 32, 0x80000040);
            cell(source, 5, 0, 8, 0xd0);
            cell(source, 6, 0, 8, 0xe0);
            cell(source, 7, 0, 32, 0x80000070);
            cell(source, 8, 0, 32, 0x80000080);
            cell(source, 9, 0, 32, 0x80000090);
            cell(source, 10, 0, 32, 0x800000a0);
            cell(source, 11, 0, 32, 0x800000b0);
            cell(source, 12, 0, 16, 0x80c0);
            cell(source, 13, 0, 16, 0x80d0);
            cell(source, 14, 0, 32, 0x800000e0);
            cell(source, 15, 0, 32, 0x800000f0);
            cell(source, 16, 0, 32, 0x80000100);
        }
        SpellTable::SpellVisualMissile => {
            cell(source, 0, 0, 32, 0x7fc00000);
            cell(source, 0, 1, 32, 0x7fc00001);
            cell(source, 0, 2, 32, 0x7fc00002);
            cell(source, 1, 0, 32, 0x7fc00010);
            cell(source, 1, 1, 32, 0x7fc00011);
            cell(source, 1, 2, 32, 0x7fc00012);
            cell(source, 3, 0, 16, 0x8030);
            cell(source, 4, 0, 32, 0x80000040);
            cell(source, 5, 0, 8, 0xd0);
            cell(source, 6, 0, 8, 0xe0);
            cell(source, 7, 0, 16, 0x8070);
            cell(source, 8, 0, 16, 0x8080);
            cell(source, 9, 0, 32, 0x80000090);
            cell(source, 10, 0, 32, 0x800000a0);
            cell(source, 11, 0, 16, 0x80b0);
            cell(source, 12, 0, 32, 0x800000c0);
            cell(source, 13, 0, 16, 0x80d0);
            cell(source, 14, 0, 32, 0x800000e0);
            cell(source, 15, 0, 32, 0x800000f0);
            cell(source, 16, 0, 32, 0x80000100);
            cell(source, 17, 0, 16, 0x8110);
            cell(source, 18, 0, 32, 0x80000120);
            cell(source, 19, 0, 32, 0x80000130);
            cell(source, 20, 0, 32, 0x80000140);
            cell(source, 21, 0, 32, 0x80000150);
        }
        SpellTable::SpellVisualEffectName => {
            cell(source, 0, 0, 32, 0x80000000);
            cell(source, 1, 0, 32, 0x7fc00010);
            cell(source, 2, 0, 32, 0x7fc00020);
            cell(source, 3, 0, 32, 0x7fc00030);
            cell(source, 4, 0, 32, 0x7fc00040);
            cell(source, 5, 0, 32, 0x7fc00050);
            cell(source, 6, 0, 32, 0x80000060);
            cell(source, 7, 0, 32, 0x80000070);
            cell(source, 8, 0, 32, 0x7fc00080);
            cell(source, 9, 0, 32, 0x80000090);
            cell(source, 10, 0, 32, 0x800000a0);
            cell(source, 11, 0, 32, 0x800000b0);
            cell(source, 12, 0, 32, 0x800000c0);
            cell(source, 13, 0, 32, 0x800000d0);
            cell(source, 14, 0, 8, 0xe0);
            cell(source, 15, 0, 16, 0x80f0);
        }
        SpellTable::LiquidType => {
            cell(source, 2, 0, 32, 0x80000020);
            cell(source, 3, 0, 8, 0xb0);
            cell(source, 4, 0, 32, 0x80000040);
            cell(source, 5, 0, 32, 0x80000050);
            cell(source, 6, 0, 32, 0x7fc00060);
            cell(source, 7, 0, 32, 0x7fc00070);
            cell(source, 8, 0, 32, 0x7fc00080);
            cell(source, 9, 0, 32, 0x7fc00090);
            cell(source, 10, 0, 16, 0x80a0);
            cell(source, 11, 0, 32, 0x7fc000b0);
            cell(source, 12, 0, 8, 0xc0);
            cell(source, 13, 0, 8, 0xd0);
            cell(source, 14, 0, 8, 0xe0);
            cell(source, 15, 0, 32, 0x800000f0);
            cell(source, 16, 0, 8, 0x80);
            cell(source, 16, 1, 8, 0x81);
            cell(source, 16, 2, 8, 0x82);
            cell(source, 16, 3, 8, 0x83);
            cell(source, 16, 4, 8, 0x84);
            cell(source, 16, 5, 8, 0x85);
            cell(source, 17, 0, 32, 0x80000110);
            cell(source, 17, 1, 32, 0x80000111);
            cell(source, 17, 2, 32, 0x80000112);
            cell(source, 18, 0, 32, 0x7fc00120);
            cell(source, 18, 1, 32, 0x7fc00121);
            cell(source, 18, 2, 32, 0x7fc00122);
            cell(source, 18, 3, 32, 0x7fc00123);
            cell(source, 18, 4, 32, 0x7fc00124);
            cell(source, 18, 5, 32, 0x7fc00125);
            cell(source, 18, 6, 32, 0x7fc00126);
            cell(source, 18, 7, 32, 0x7fc00127);
            cell(source, 18, 8, 32, 0x7fc00128);
            cell(source, 18, 9, 32, 0x7fc00129);
            cell(source, 18, 10, 32, 0x7fc0012a);
            cell(source, 18, 11, 32, 0x7fc0012b);
            cell(source, 18, 12, 32, 0x7fc0012c);
            cell(source, 18, 13, 32, 0x7fc0012d);
            cell(source, 18, 14, 32, 0x7fc0012e);
            cell(source, 18, 15, 32, 0x7fc0012f);
            cell(source, 18, 16, 32, 0x7fc00130);
            cell(source, 18, 17, 32, 0x7fc00131);
            cell(source, 18, 18, 32, 0x7fc00132);
            cell(source, 18, 19, 32, 0x7fc00133);
            cell(source, 18, 20, 32, 0x7fc00134);
            cell(source, 18, 21, 32, 0x7fc00135);
            cell(source, 18, 22, 32, 0x7fc00136);
            cell(source, 18, 23, 32, 0x7fc00137);
            cell(source, 18, 24, 32, 0x7fc00138);
            cell(source, 18, 25, 32, 0x7fc00139);
            cell(source, 18, 26, 32, 0x7fc0013a);
            cell(source, 18, 27, 32, 0x7fc0013b);
            cell(source, 18, 28, 32, 0x7fc0013c);
            cell(source, 18, 29, 32, 0x7fc0013d);
            cell(source, 18, 30, 32, 0x7fc0013e);
            cell(source, 18, 31, 32, 0x7fc0013f);
            cell(source, 18, 32, 32, 0x7fc00140);
            cell(source, 18, 33, 32, 0x7fc00141);
            cell(source, 18, 34, 32, 0x7fc00142);
            cell(source, 18, 35, 32, 0x7fc00143);
            cell(source, 18, 36, 32, 0x7fc00144);
            cell(source, 18, 37, 32, 0x7fc00145);
            cell(source, 19, 0, 32, 0x80000130);
            cell(source, 19, 1, 32, 0x80000131);
            cell(source, 19, 2, 32, 0x80000132);
            cell(source, 19, 3, 32, 0x80000133);
            cell(source, 20, 0, 32, 0x7fc00140);
            cell(source, 20, 1, 32, 0x7fc00141);
            cell(source, 20, 2, 32, 0x7fc00142);
            cell(source, 20, 3, 32, 0x7fc00143);
        }
        _ => {}
    }
}

pub(super) fn assert_records(batch: &crate::forever_spells::SpellRecords) {
    let value = &batch.unit_conditions[0];
    assert_eq!(value.id, 7);
    assert_eq!(value.flags, 0x80000020u32 as i32);
    assert_eq!(
        value.variable,
        [0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87]
    );
    assert_eq!(value.op, [0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7]);
    assert_eq!(
        value.value,
        std::array::from_fn(|i| (0x80000030u32 + i as u32) as i32)
    );
    let value = &batch.talents[0];
    assert_eq!(value.id, 7);
    assert_eq!(value.description.at(6), Some([].as_slice()));
    assert_eq!(value.description.at(0), None);
    assert_eq!(value.tier_id, 0x90);
    assert_eq!(value.flags, 0x80000020u32 as i32);
    assert_eq!(value.column_index, 0xb0);
    assert_eq!(value.tab_id, 0x8040);
    assert_eq!(value.class_id, 0xd0u8 as i8);
    assert_eq!(value.spec_id, 0x8060);
    assert_eq!(value.spell_id, 0x80000070);
    assert_eq!(value.overrides_spell_id, 0x80000080);
    assert_eq!(value.required_spell_id, 0x80000090);
    assert_eq!(
        value.category_mask,
        [0x800000a0u32 as i32, 0x800000a1u32 as i32]
    );
    assert_eq!(
        value.spell_rank,
        [
            0x800000b0, 0x800000b1, 0x800000b2, 0x800000b3, 0x800000b4, 0x800000b5, 0x800000b6,
            0x800000b7, 0x800000b8
        ]
    );
    assert_eq!(value.prereq_talent, [0x800000c0, 0x800000c1, 0x800000c2]);
    assert_eq!(value.prereq_rank, [0xd0, 0xd1, 0xd2]);
    let value = &batch.spell_item_enchantments[0];
    assert_eq!(value.id, 7);
    assert_eq!(value.name.at(6), Some([].as_slice()));
    assert_eq!(value.name.at(0), None);
    assert_eq!(value.horde_name.at(6), Some([].as_slice()));
    assert_eq!(value.horde_name.at(0), None);
    assert_eq!(value.duration, 0x80000020u32 as i32);
    assert_eq!(value.charges, 0x80000030u32 as i32);
    assert_eq!(
        value.effect,
        [
            0x80000040u32 as i32,
            0x80000041u32 as i32,
            0x80000042u32 as i32
        ]
    );
    assert_eq!(
        value.effect_points_min,
        [
            0x80000050u32 as i32,
            0x80000051u32 as i32,
            0x80000052u32 as i32
        ]
    );
    assert_eq!(value.effect_arg, [0x80000060, 0x80000061, 0x80000062]);
    assert_eq!(value.flags, 0x80000070u32 as i32);
    assert_eq!(
        value.effect_scaling_points.map(f32::to_bits),
        ([
            f32::from_bits(0x7fc00080),
            f32::from_bits(0x7fc00081),
            f32::from_bits(0x7fc00082)
        ])
        .map(f32::to_bits)
    );
    assert_eq!(value.scaling_class, 0x80000090u32 as i32);
    assert_eq!(value.scaling_class_restricted, 0x800000a0u32 as i32);
    assert_eq!(value.condition_id, 0x800000b0u32 as i32);
    assert_eq!(value.required_skill_id, 0x800000c0u32 as i32);
    assert_eq!(value.required_skill_rank, 0x800000d0u32 as i32);
    assert_eq!(value.min_level, 0x800000e0u32 as i32);
    assert_eq!(value.max_level, 0x800000f0u32 as i32);
    assert_eq!(value.icon_file_data_id, 0x80000100u32 as i32);
    assert_eq!(value.min_item_level, 0x80000110u32 as i32);
    assert_eq!(value.max_item_level, 0x80000120u32 as i32);
    assert_eq!(value.transmog_use_condition_id, 0x80000130u32 as i32);
    assert_eq!(value.transmog_cost, 0x80000140u32 as i32);
    assert_eq!(value.field_12_1_5_69594_021, 0x80000150u32 as i32);
    assert_eq!(value.item_visual, 0x8160);
    assert_eq!(value.item_level, 0x8170);
    let value = &batch.spell_visuals[0];
    assert_eq!(value.id, 7);
    assert_eq!(
        value.missile_cast_offset.map(f32::to_bits),
        ([
            f32::from_bits(0x7fc00000),
            f32::from_bits(0x7fc00001),
            f32::from_bits(0x7fc00002)
        ])
        .map(f32::to_bits)
    );
    assert_eq!(
        value.missile_impact_offset.map(f32::to_bits),
        ([
            f32::from_bits(0x7fc00010),
            f32::from_bits(0x7fc00011),
            f32::from_bits(0x7fc00012)
        ])
        .map(f32::to_bits)
    );
    assert_eq!(value.state_kit, 0x80000020u32 as i32);
    assert_eq!(value.anim_event_sound_id, 0x80000030);
    assert_eq!(value.flags, 0x80000040u32 as i32);
    assert_eq!(value.missile_attachment, 0xd0u8 as i8);
    assert_eq!(value.missile_destination_attachment, 0xe0u8 as i8);
    assert_eq!(value.missile_cast_positioner_id, 0x80000070);
    assert_eq!(value.missile_impact_positioner_id, 0x80000080);
    assert_eq!(value.missile_targeting_kit, 0x80000090u32 as i32);
    assert_eq!(value.hostile_spell_visual_id, 0x800000a0);
    assert_eq!(value.caster_spell_visual_id, 0x800000b0);
    assert_eq!(value.spell_visual_missile_set_id, 0x80c0);
    assert_eq!(value.damage_number_delay, 0x80d0);
    assert_eq!(value.low_violence_spell_visual_id, 0x800000e0);
    assert_eq!(value.raid_spell_visual_missile_set_id, 0x800000f0);
    assert_eq!(
        value.reduced_unexpected_camera_movement_spell_visual_id,
        0x80000100u32 as i32
    );
    let value = &batch.spell_visual_missiles[0];
    assert_eq!(value.id, 7);
    assert_eq!(
        value.cast_offset.map(f32::to_bits),
        ([
            f32::from_bits(0x7fc00000),
            f32::from_bits(0x7fc00001),
            f32::from_bits(0x7fc00002)
        ])
        .map(f32::to_bits)
    );
    assert_eq!(
        value.impact_offset.map(f32::to_bits),
        ([
            f32::from_bits(0x7fc00010),
            f32::from_bits(0x7fc00011),
            f32::from_bits(0x7fc00012)
        ])
        .map(f32::to_bits)
    );
    assert_eq!(value.spell_visual_effect_name_id, 0x8030);
    assert_eq!(value.sound_entries_id, 0x80000040);
    assert_eq!(value.attachment, 0xd0u8 as i8);
    assert_eq!(value.destination_attachment, 0xe0u8 as i8);
    assert_eq!(value.cast_positioner_id, 0x8070);
    assert_eq!(value.impact_positioner_id, 0x8080);
    assert_eq!(value.follow_ground_height, 0x80000090u32 as i32);
    assert_eq!(value.follow_ground_drop_speed, 0x800000a0);
    assert_eq!(value.follow_ground_approach, 0x80b0);
    assert_eq!(value.flags, 0x800000c0u32 as i32);
    assert_eq!(value.spell_missile_motion_id, 0x80d0);
    assert_eq!(value.anim_kit_id, 0x800000e0);
    assert_eq!(value.clutter_level, 0x800000f0u32 as i32);
    assert_eq!(value.decay_time_after_impact, 0x80000100u32 as i32);
    assert_eq!(value.unused1100, 0x8110);
    assert_eq!(value.field_12_1_5_69594_018, 0x80000120u32 as i32);
    assert_eq!(value.field_12_1_5_69594_019, 0x80000130u32 as i32);
    assert_eq!(value.field_12_1_5_69594_020, 0x80000140u32 as i32);
    assert_eq!(value.field_12_1_5_69594_021, 0x80000150u32 as i32);
    assert_eq!(value.spell_visual_missile_set_id, 0);
    let value = &batch.spell_visual_effect_names[0];
    assert_eq!(value.id, 7);
    assert_eq!(value.model_file_data_id, 0x80000000u32 as i32);
    assert_eq!(
        value.base_missile_speed.to_bits(),
        (f32::from_bits(0x7fc00010)).to_bits()
    );
    assert_eq!(
        value.scale.to_bits(),
        (f32::from_bits(0x7fc00020)).to_bits()
    );
    assert_eq!(
        value.min_allowed_scale.to_bits(),
        (f32::from_bits(0x7fc00030)).to_bits()
    );
    assert_eq!(
        value.max_allowed_scale.to_bits(),
        (f32::from_bits(0x7fc00040)).to_bits()
    );
    assert_eq!(
        value.alpha.to_bits(),
        (f32::from_bits(0x7fc00050)).to_bits()
    );
    assert_eq!(value.flags, 0x80000060u32 as i32);
    assert_eq!(value.texture_file_data_id, 0x80000070u32 as i32);
    assert_eq!(
        value.effect_radius.to_bits(),
        (f32::from_bits(0x7fc00080)).to_bits()
    );
    assert_eq!(value.r#type, 0x80000090u32 as i32);
    assert_eq!(value.generic_id, 0x800000a0u32 as i32);
    assert_eq!(value.ribbon_quality_id, 0x800000b0);
    assert_eq!(value.dissolve_effect_id, 0x800000c0u32 as i32);
    assert_eq!(value.model_position, 0x800000d0u32 as i32);
    assert_eq!(value.unknown901, 0xe0u8 as i8);
    assert_eq!(value.unknown1100, 0x80f0);
    let value = &batch.liquid_types[0];
    assert_eq!(value.id, 7);
    assert!(value.name.is_empty());
    assert!(value.texture.iter().all(Vec::is_empty));
    assert_eq!(value.flags, 0x80000020u32 as i32);
    assert_eq!(value.sound_bank, 0xb0);
    assert_eq!(value.sound_id, 0x80000040);
    assert_eq!(value.spell_id, 0x80000050);
    assert_eq!(
        value.max_darken_depth.to_bits(),
        (f32::from_bits(0x7fc00060)).to_bits()
    );
    assert_eq!(
        value.fog_darken_intensity.to_bits(),
        (f32::from_bits(0x7fc00070)).to_bits()
    );
    assert_eq!(
        value.amb_darken_intensity.to_bits(),
        (f32::from_bits(0x7fc00080)).to_bits()
    );
    assert_eq!(
        value.dir_darken_intensity.to_bits(),
        (f32::from_bits(0x7fc00090)).to_bits()
    );
    assert_eq!(value.light_id, 0x80a0);
    assert_eq!(
        value.particle_scale.to_bits(),
        (f32::from_bits(0x7fc000b0)).to_bits()
    );
    assert_eq!(value.particle_movement, 0xc0);
    assert_eq!(value.particle_tex_slots, 0xd0);
    assert_eq!(value.material_id, 0xe0);
    assert_eq!(value.minimap_static_col, 0x800000f0u32 as i32);
    assert_eq!(
        value.frame_count_texture,
        [0x80, 0x81, 0x82, 0x83, 0x84, 0x85]
    );
    assert_eq!(
        value.color,
        [
            0x80000110u32 as i32,
            0x80000111u32 as i32,
            0x80000112u32 as i32
        ]
    );
    assert_eq!(
        value.float_values.map(f32::to_bits),
        ([
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
            f32::from_bits(0x7fc00145)
        ])
        .map(f32::to_bits)
    );
    assert_eq!(
        value.int_values,
        [0x80000130, 0x80000131, 0x80000132, 0x80000133]
    );
    assert_eq!(
        value.coefficient.map(f32::to_bits),
        ([
            f32::from_bits(0x7fc00140),
            f32::from_bits(0x7fc00141),
            f32::from_bits(0x7fc00142),
            f32::from_bits(0x7fc00143)
        ])
        .map(f32::to_bits)
    );
}

#[test]
fn liquid_string_components_use_individual_relative_addresses_without_locale_conversion() {
    let mut source = reader(SpellTable::LiquidType);
    source.header.record_count = 2; // second source record unavailable
    let pool = b"n\xFF\0a\0b\xFE\0c\0d\0e\0f\0";
    source.header.string_table_size = pool.len() as u32 + 4; // unknown pool
    let extent = source.header.record_size * source.header.record_count;
    let fields = [
        (0, 0, 0),
        (1, 0, 3),
        (1, 1, 5),
        (1, 2, 8),
        (1, 3, 10),
        (1, 4, 12),
        (1, 5, 14),
    ];
    for (field, component, offset) in fields {
        let address =
            u32::from(source.field_info[field].field_offset_bits / 8) + component as u32 * 4;
        cell(&mut source, field, component, 32, extent + offset - address);
    }
    source.string_tables = vec![pool.to_vec()];
    let value = CreationDb2::checked(source, CreationTable::Spell(SpellTable::LiquidType)).unwrap();
    assert_eq!(value.spell_text_component(7, 0, 0).unwrap(), b"n\xFF");
    for (component, expected) in [b"a".as_slice(), b"b\xFE", b"c", b"d", b"e", b"f"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            value.spell_text_component(7, 1, component).unwrap(),
            expected
        );
    }
    assert!(value.spell_text_component(7, 1, 6).is_err());
    assert!(value.spell_text_component(7, 0, 1).is_err());
    assert!(value.spell_text_component(7, 2, 0).is_err());
    assert!(value.spell_text_component(99, 1, 0).is_err());
}

#[test]
fn liquid_string_components_reject_unknown_unterminated_record_and_out_of_range_targets() {
    for target in [0, 1, 6, 9, 12, u32::MAX] {
        let mut source = reader(SpellTable::LiquidType);
        let extent = source.header.record_size;
        source.header.string_table_size = 12;
        source.string_tables = vec![b"known\0bad".to_vec()];
        let address = u32::from(source.field_info[1].field_offset_bits / 8) + 5 * 4;
        let relative = if target == 0 {
            1
        } else if target == u32::MAX {
            target
        } else {
            extent + target - address
        };
        cell(&mut source, 1, 5, 32, relative);
        let value =
            CreationDb2::checked(source, CreationTable::Spell(SpellTable::LiquidType)).unwrap();
        if target == 1 {
            assert_eq!(value.spell_text_component(7, 1, 5).unwrap(), b"nown");
        } else {
            assert!(value.spell_text_component(7, 1, 5).is_err());
        }
    }
    let source = reader(SpellTable::LiquidType);
    let value = CreationDb2::checked(source, CreationTable::Spell(SpellTable::LiquidType)).unwrap();
    assert!(value.spell_text_component(7, 1, 5).unwrap().is_empty());
}

#[test]
fn missile_inline_copy_id_and_extra_parent_preserve_full_unsigned_source_bits() {
    let mut source = reader(SpellTable::SpellVisualMissile);
    source.relationship_ids[0] = Some(u32::MAX);
    source.copy_table = vec![(8, 7), (9, 8)];
    cell(&mut source, 5, 0, 8, 0x80);
    cell(&mut source, 6, 0, 8, 0xFF);
    let value =
        CreationDb2::checked(source, CreationTable::Spell(SpellTable::SpellVisualMissile)).unwrap();
    assert_eq!(value.bits(9, 2, 0).unwrap(), 9);
    assert_eq!(value.bits(9, 22, 0).unwrap(), u32::MAX);
    assert_eq!(value.bits(9, 5, 0).unwrap() as i8, i8::MIN);
    assert_eq!(value.bits(9, 6, 0).unwrap() as i8, -1);
}
