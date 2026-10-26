use super::*;
use super::super::classify_checked;
use crate::field_like_cpp::NativeFieldLikeCpp as Native;
use wow_persistence::SpellAcquisitionStartupLoadOutcomeLikeCpp;

#[test]
fn server_effect_projection_keeps_signed_short_fields_float_and_masks() {
    let mut fields = vec![Native::Int32(0); 34];
    fields[0] = Native::UInt32(u32::MAX);
    fields[1] = Native::Int32(-2);
    fields[2] = Native::UInt32(256);
    fields[3] = Native::Int32(-3);
    fields[4] = Native::Int16(i16::MIN);
    fields[5] = Native::Float(1.234_567_8);
    fields[23] = Native::Float(-12.75);
    fields[28] = Native::Int32(i32::MIN);
    fields[29] = Native::Int64(-1);
    fields[30] = Native::Int32(0x1234_5678);
    fields[31] = Native::Int32(i32::MAX);
    fields[32] = Native::Int16(-321);
    fields[33] = Native::Int16(123);

    let decoded = decode_effect(&fields).unwrap();
    assert_eq!(decoded.spell_id, u32::MAX);
    assert_eq!(decoded.effect_index, -2);
    assert_eq!(decoded.difficulty_id, 256);
    assert_eq!(decoded.effect_aura, i32::from(i16::MIN));
    assert_eq!(decoded.effect_amplitude.to_bits(), 1.234_567_8_f32.to_bits());
    assert_eq!(decoded.effect_base_points.to_bits(), (-12.75_f32).to_bits());
    assert_eq!(
        decoded.effect_spell_class_mask,
        [i32::MIN, -1, 0x1234_5678, i32::MAX]
    );
    assert_eq!(decoded.implicit_target, [-321, 123]);

    fields[4] = Native::Int32(i16::MAX as i32 + 1);
    let error = decode_effect(&fields).unwrap_err().to_string();
    assert!(error.contains("EffectAura"), "{error}");
    assert!(error.contains("GetInt16"), "{error}");
    fields[4] = Native::Int16(0);
    fields[32] = Native::Int32(i16::MAX as i32 + 1);
    assert!(decode_effect(&fields).is_err());
}

#[test]
fn server_spell_projection_keeps_all_source_widths_and_null_name_behavior() {
    let mut fields = vec![Native::Int32(0); 81];
    fields[0] = Native::UInt32(u32::MAX);
    fields[1] = Native::UInt32(999);
    fields[6] = Native::UInt32(0x8000_0000);
    fields[19] = Native::UInt32(u32::MAX);
    fields[20] = Native::UInt64(u64::MAX);
    fields[21] = Native::UInt64(0x8000_0000_0000_0000);
    fields[34] = Native::Int32(-2);
    fields[37] = Native::Int32(i32::MIN);
    fields[51] = Native::UInt32(u32::MAX);
    fields[62] = Native::Int32(i32::MIN);
    fields[63] = Native::Int32(-1);
    fields[64] = Native::Int32(i32::MAX);
    fields[66] = Native::Text("parent".into());
    fields[71] = Native::UInt32(u32::MAX);
    fields[72] = Native::UInt32(0x8000_0000);
    fields[75] = Native::UInt32(u32::MAX);
    fields[78] = Native::Int32(i32::MIN);

    let decoded = decode_spell(&fields).unwrap();
    assert_eq!(decoded.spell_id, u32::MAX);
    assert_eq!(decoded.difficulty_id, 999);
    assert_eq!(decoded.attributes_ex[0], 0x8000_0000);
    assert_eq!(decoded.attributes_ex[13], u32::MAX);
    assert_eq!(decoded.stances, u64::MAX);
    assert_eq!(decoded.stances_not, 0x8000_0000_0000_0000);
    assert_eq!(decoded.caster_aura_type, -2);
    assert_eq!(decoded.exclude_target_aura_type, i32::MIN);
    assert_eq!(decoded.proc_charges, u32::MAX);
    assert_eq!(decoded.equipped_item_class, i32::MIN);
    assert_eq!(decoded.equipped_item_sub_class_mask, -1);
    assert_eq!(decoded.equipped_item_inventory_type_mask, i32::MAX);
    assert_eq!(decoded.spell_name, "parent");
    assert_eq!(decoded.spell_family_name, u32::MAX);
    assert_eq!(decoded.spell_family_flags, [0x8000_0000, 0, 0, u32::MAX]);
    assert_eq!(decoded.area_group_id, i32::MIN);

    fields[66] = Native::Null;
    assert!(decode_spell(&fields).unwrap().spell_name.is_empty());
    fields[5] = Native::UInt64(u64::MAX);
    assert!(decode_spell(&fields).is_err());
    fields[5] = Native::UInt32(0);
    fields.pop();
    assert!(decode_spell(&fields).is_err());
}

#[test]
fn malformed_server_row_fails_the_batch_with_table_row_and_field_chain() {
    let mut valid = vec![Native::UInt32(1); 81];
    valid[66] = Native::Text("valid".into());
    let mut malformed = vec![Native::UInt32(1); 81];
    malformed[66] = Native::UInt32(12);
    let mut rows = Vec::new();
    let mut decode = decode_spell;
    append_checked(&mut rows, "serverside_spell", 0, &valid, &mut decode).unwrap();
    let result = append_checked(&mut rows, "serverside_spell", 1, &malformed, &mut decode);

    let reason = match classify_checked(result.map(|()| rows)) {
        SpellAcquisitionStartupLoadOutcomeLikeCpp::Failed { reason } => reason,
        SpellAcquisitionStartupLoadOutcomeLikeCpp::Loaded(rows) => {
            panic!("malformed server row unexpectedly loaded {} rows", rows.len())
        }
    };
    assert!(reason.contains("serverside_spell row 1"), "{reason}");
    assert!(reason.contains("serverside_spell.SpellName"), "{reason}");
    assert!(reason.contains("Field::GetString"), "{reason}");
}
