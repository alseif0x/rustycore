use super::*;
use super::super::{append_decoded_row, classify_rows_like_cpp};
use crate::field_like_cpp::NativeFieldLikeCpp as Native;
use wow_persistence::SpellCoreDb2HotfixLoadOutcomeLikeCpp;

fn aura_options_row() -> Vec<Native> {
    vec![
        Native::UInt32(1),
        Native::UInt8(2),
        Native::UInt32(3),
        Native::Int32(-4),
        Native::UInt8(5),
        Native::Int32(-6),
        Native::UInt16(7),
        Native::Int32(i32::MIN),
        Native::Int32(-1),
        Native::UInt32(u32::MAX),
    ]
}

fn numeric_row(columns: usize) -> Vec<Native> {
    vec![Native::Int32(0); columns]
}

#[test]
fn selected_scalar_rows_keep_cpp_widths_signed_values_and_high_bits() {
    let mut aura_fields = aura_options_row();
    aura_fields[2] = Native::Int32(-1);
    aura_fields[5] = Native::Int64(-1);
    let aura = spell_aura_options(&aura_fields).unwrap();
    assert_eq!(aura.cumulative_aura, u32::MAX);
    assert_eq!(aura.proc_category_recovery, -4);
    assert_eq!(aura.proc_chance, 5);
    assert_eq!(aura.proc_charges, -1);
    assert_eq!(aura.spell_procs_per_minute_id, 7);
    assert_eq!(aura.proc_type_mask, [i32::MIN, -1]);
    assert_eq!(aura.spell_id, u32::MAX);

    let class = spell_class_options(&[
        Native::UInt32(11),
        Native::UInt32(u32::MAX),
        Native::UInt32(u32::MAX),
        Native::UInt8(u8::MAX),
        Native::Int32(i32::MIN),
        Native::Int64(-1),
        Native::Int32(17),
        Native::Int32(i32::MAX),
    ])
    .unwrap();
    assert_eq!(class.spell_id, -1);
    assert_eq!(class.modal_next_spell, u32::MAX);
    assert_eq!(class.spell_class_set, u8::MAX);
    assert_eq!(class.spell_class_mask, [i32::MIN, -1, 17, i32::MAX]);

    let mut class_mask_wrong_getter = vec![
        Native::UInt32(1),
        Native::Int32(1),
        Native::UInt32(0),
        Native::UInt8(0),
        Native::UInt64(u64::from(u32::MAX)),
        Native::Int32(0),
        Native::Int32(0),
        Native::Int32(0),
    ];
    assert!(spell_class_options(&class_mask_wrong_getter).is_err());
    class_mask_wrong_getter[4] = Native::Int64(-1);
    assert_eq!(
        spell_class_options(&class_mask_wrong_getter)
            .unwrap()
            .spell_class_mask[0],
        -1
    );

    let label = spell_label(&[
        Native::Int32(-1),
        Native::UInt32(0x8000_0000),
        Native::UInt32(u32::MAX),
    ])
    .unwrap();
    assert_eq!(label.id, u32::MAX);
    assert_eq!(label.label_id, 0x8000_0000);
    assert_eq!(label.spell_id, u32::MAX);

    let levels = spell_levels(&[
        Native::UInt32(19),
        Native::UInt8(u8::MAX),
        Native::Int16(i16::MIN),
        Native::Int16(i16::MAX),
        Native::Int16(-2),
        Native::UInt8(254),
        Native::UInt32(u32::MAX),
    ])
    .unwrap();
    assert_eq!(levels.difficulty_id, u8::MAX);
    assert_eq!(levels.base_level, i16::MIN);
    assert_eq!(levels.max_level, i16::MAX);
    assert_eq!(levels.spell_level, -2);
    assert_eq!(levels.max_passive_aura_level, 254);
    assert_eq!(levels.spell_id, u32::MAX);

    let mut level_overflow = vec![
        Native::UInt32(19),
        Native::UInt8(0),
        Native::Int64(32_768),
        Native::Int16(0),
        Native::Int16(0),
        Native::UInt8(0),
        Native::UInt32(0),
    ];
    assert!(spell_levels(&level_overflow).is_err());
    level_overflow[2] = Native::Int16(32_767);
    assert!(spell_levels(&level_overflow).is_ok());
}

#[test]
fn spell_effect_class_masks_use_signed_getters_then_preserve_all_bits() {
    let mut fields = vec![Native::Int32(0); 36];
    fields[1] = Native::Int32(256);
    fields[29] = Native::Int64(-1);
    fields[30] = Native::Int32(i32::MIN);
    fields[31] = Native::Int32(0x1234_5678);
    fields[32] = Native::Int32(0);

    let effect = spell_effect(&fields).unwrap();
    assert_eq!(effect.difficulty_id, 256);
    assert_eq!(
        effect.effect_spell_class_mask,
        [u32::MAX, 0x8000_0000, 0x1234_5678, 0]
    );

    fields[29] = Native::UInt64(u64::from(u32::MAX));
    assert!(spell_effect(&fields).is_err());
}

#[test]
fn existing_spell_rows_decode_all_native_projection_columns() {
    let mut misc_fields = numeric_row(29);
    for (index, field) in misc_fields[1..16].iter_mut().enumerate() {
        *field = Native::Int32(index as i32 + 1);
    }
    misc_fields[1] = Native::Int32(i32::MIN);
    misc_fields[15] = Native::Int64(-1);
    let misc = spell_misc(&misc_fields).unwrap();
    assert_eq!(
        misc.attributes,
        [i32::MIN, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, -1]
    );
    let mut misc_overflow = numeric_row(29);
    misc_overflow[1] = Native::UInt64(i32::MAX as u64 + 1);
    assert!(spell_misc(&misc_overflow).is_err());
    misc_overflow[1] = Native::Int32(0);
    misc_overflow[15] = Native::UInt64(u64::from(u32::MAX));
    assert!(spell_misc(&misc_overflow).is_err());

    assert!(spell_categories(&numeric_row(10)).is_ok());
    assert!(spell_effect(&numeric_row(36)).is_ok());
    assert!(spell_shapeshift(&numeric_row(7)).is_ok());
    assert!(spell_interrupts(&numeric_row(8)).is_ok());
    assert!(spell_cast_times(&numeric_row(4)).is_ok());
    assert!(spell_cooldowns(&numeric_row(6)).is_ok());
    assert!(spell_casting_requirements(&numeric_row(8)).is_ok());
    assert!(spell_power(&numeric_row(14)).is_ok());
    assert!(spell_power_difficulty(&numeric_row(3)).is_ok());
    assert!(spell_aura_restrictions(&numeric_row(11)).is_ok());
    assert!(spell_category(&[
        Native::Int32(0),
        Native::Text("name".to_string()),
        Native::Int32(0),
        Native::Int32(0),
        Native::Int32(0),
        Native::Int32(0),
        Native::Int32(0),
    ])
    .is_ok());
    assert!(spell_duration(&numeric_row(4)).is_ok());
    assert!(spell_radius(&numeric_row(5)).is_ok());

    let mut range_fields = numeric_row(8);
    range_fields[1] = Native::Text("display".to_string());
    range_fields[2] = Native::Text("short".to_string());
    assert!(spell_range(&range_fields).is_ok());

    assert!(spell_equipped_items(&numeric_row(5)).is_ok());
    assert!(spell_target_restrictions(&numeric_row(9)).is_ok());
    assert!(spell_x_spell_visual(&numeric_row(13)).is_ok());
}

#[test]
fn spell_visual_condition_and_priority_columns_keep_exact_widths_and_positions() {
    let mut fields = numeric_row(13);
    fields[5] = Native::Int64(-1);
    fields[6] = Native::Int32(-6);
    fields[7] = Native::Int32(-7);
    fields[8] = Native::UInt16(0x8123);
    fields[9] = Native::UInt32(0xF123_4567);
    fields[10] = Native::UInt16(0x9234);
    fields[11] = Native::UInt32(0xE234_5678);

    let visual = spell_x_spell_visual(&fields).unwrap();
    assert_eq!(visual.priority, -1);
    assert_eq!(visual.spell_icon_file_id, -6);
    assert_eq!(visual.active_icon_file_id, -7);
    assert_eq!(visual.viewer_unit_condition_id, 0x8123);
    assert_eq!(visual.viewer_player_condition_id, 0xF123_4567);
    assert_eq!(visual.caster_unit_condition_id, 0x9234);
    assert_eq!(visual.caster_player_condition_id, 0xE234_5678);

    fields[5] = Native::UInt64(i32::MAX as u64 + 1);
    assert!(spell_x_spell_visual(&fields).is_err());
    fields[5] = Native::UInt64(u64::from(u32::MAX));
    assert!(spell_x_spell_visual(&fields).is_err());
    fields[5] = Native::Int32(0);
    fields[8] = Native::UInt32(u32::from(u16::MAX) + 1);
    assert!(spell_x_spell_visual(&fields).is_err());
}

#[test]
fn nullable_fields_default_to_cpp_zero_or_empty_but_invalid_fields_fail() {
    let mut aura_fields = aura_options_row();
    aura_fields[1] = Native::Null;
    aura_fields[2] = Native::Null;
    aura_fields[4] = Native::Null;
    let aura = spell_aura_options(&aura_fields).unwrap();
    assert_eq!(aura.difficulty_id, 0);
    assert_eq!(aura.cumulative_aura, 0);
    assert_eq!(aura.proc_chance, 0);

    let name = spell_name(&[Native::Null, Native::Null]).unwrap();
    assert_eq!(name.id, 0);
    assert!(name.name.is_empty());

    let mut malformed = aura_options_row();
    malformed[4] = Native::Text("not numeric".to_string());
    assert!(spell_aura_options(&malformed)
        .unwrap_err()
        .to_string()
        .contains("ProcChance"));

    let mut unsupported = aura_options_row();
    unsupported[4] = Native::Unsupported("DECIMAL".to_string());
    assert!(spell_aura_options(&unsupported).is_err());

    let mut missing = aura_options_row();
    missing.pop();
    assert!(spell_aura_options(&missing).is_err());

    let mut difficulty_overflow = aura_options_row();
    difficulty_overflow[1] = Native::UInt16(256);
    assert!(spell_aura_options(&difficulty_overflow).is_err());

    let mut proc_charges_wrong_getter = aura_options_row();
    proc_charges_wrong_getter[5] = Native::UInt64(u64::from(u32::MAX));
    assert!(spell_aura_options(&proc_charges_wrong_getter).is_err());
}

#[test]
fn malformed_row_in_a_batch_classifies_the_whole_load_as_failed_with_context() {
    let mut valid = aura_options_row();
    valid[4] = Native::UInt8(50);
    let mut malformed = aura_options_row();
    malformed[4] = Native::Text("invalid numeric column".to_string());
    let batch = [valid, malformed];
    let mut decoded = Vec::new();
    let mut mapper = spell_aura_options;
    let result = batch.iter().enumerate().try_for_each(|(row_index, fields)| {
        append_decoded_row(
            &mut decoded,
            "spell_aura_options",
            row_index,
            fields,
            &mut mapper,
        )
    });

    let outcome = classify_rows_like_cpp(result.map(|()| decoded));
    match outcome {
        SpellCoreDb2HotfixLoadOutcomeLikeCpp::Failed { reason } => {
            assert!(reason.contains("spell_aura_options row 1"));
            assert!(reason.contains("spell_aura_options.ProcChance"));
            assert!(reason.contains("Field::GetUInt8"));
        }
        SpellCoreDb2HotfixLoadOutcomeLikeCpp::Loaded(rows) => {
            panic!("malformed row unexpectedly yielded {} rows", rows.len());
        }
    }
}

#[test]
fn ppm_native_projection_preserves_float_and_flags_widths() {
    let rate = f32::from_bits(0x3F9E_0652);
    let decoded = spell_procs_per_minute(&[
        Native::UInt32(u32::MAX),
        Native::Float(rate),
        Native::UInt8(u8::MAX),
    ])
    .unwrap();
    assert_eq!(decoded.id, u32::MAX);
    assert_eq!(decoded.base_proc_rate.to_bits(), rate.to_bits());
    assert_eq!(decoded.flags, u8::MAX);

    let null = spell_procs_per_minute(&[Native::Null, Native::Null, Native::Null]).unwrap();
    assert_eq!(null.id, 0);
    assert_eq!(null.base_proc_rate, 0.0);
    assert_eq!(null.flags, 0);

    assert!(spell_procs_per_minute(&[
        Native::UInt32(1),
        Native::Double(1.1),
        Native::UInt8(0),
    ])
    .is_err());
    assert!(spell_procs_per_minute(&[
        Native::UInt32(1),
        Native::Float(1.5),
        Native::UInt16(256),
    ])
    .is_err());
}
