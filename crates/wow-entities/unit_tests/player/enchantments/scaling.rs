use super::fixtures::{Effects, Sources, item_facts};
use crate::player::enchantments::{
    SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP, SkillEnchantmentEffectOperation as Op,
    SkillEnchantmentRecord,
};
use crate::{
    PlayerSkills, SkillEnchantmentApplyArgs, SkillEnchantmentError,
    apply_enchantment_native_like_cpp,
};
use wow_constants::ItemEnchantmentType;
use wow_core::ObjectGuid;

const ENCHANTMENT_ID: u32 = 71;
const ITEM_ID: u32 = 710;
const ITEM_GUID_NUMBER: u32 = 711;
const SCALING_CLASS: i32 = 4;

fn scaled_record(
    amount: i32,
    points: f32,
    flags: i32,
    primary_class: i32,
    restricted_class: i32,
    max_level: u32,
) -> SkillEnchantmentRecord {
    SkillEnchantmentRecord::new(
        ENCHANTMENT_ID,
        [ItemEnchantmentType::Resistance as u32, 0, 0],
        [amount, 0, 0],
        [2, 0, 0],
        flags,
        [points, 0.0, 0.0],
        primary_class,
        restricted_class,
        0,
        0,
        0,
        0,
        max_level,
    )
}

fn apply_record(
    record: SkillEnchantmentRecord,
    level: u8,
    limits: (u32, u32),
    sources: &mut Sources,
) -> Result<Vec<Op>, SkillEnchantmentError<&'static str, &'static str>> {
    let guid = ObjectGuid::create_item(1, i64::from(ITEM_GUID_NUMBER));
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[1] = ENCHANTMENT_ID;
    sources.add_enchantment(record);
    let mut effects = Effects {
        level,
        limits,
        ..Effects::default()
    };
    effects.add_item(item_facts(
        ITEM_GUID_NUMBER,
        ITEM_ID,
        crate::EQUIPMENT_SLOT_MAINHAND,
        true,
        false,
        ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    ));
    let mut skills = PlayerSkills::initialize([], || {});
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        1,
        true,
        SkillEnchantmentApplyArgs::new(),
        sources,
        &mut effects,
    )?;
    Ok(effects.operations)
}

fn amount_from(operations: &[Op]) -> f32 {
    match operations[0] {
        Op::HandleStatFlatModifier { amount, .. } => amount,
        ref operation => panic!("expected resistance operation, got {operation:?}"),
    }
}

#[test]
fn restricted_class_and_live_level_caps_follow_native_selector_order() {
    let mut sources = Sources {
        scaling_row_count: 100,
        ..Sources::default()
    };
    sources.scaling_columns.insert((70, 4), 2.0);
    sources.scaling_columns.insert((70, 3), 20.0);
    let record = scaled_record(9, 3.0, 0, 3, 4, 0);
    assert_eq!(
        amount_from(&apply_record(record, 70, (0, 0), &mut sources).unwrap()),
        60.0
    );
    assert_eq!(
        amount_from(&apply_record(record, 70, (1, 0), &mut sources).unwrap()),
        6.0
    );

    // Non-gem minimum wins over a simultaneously exceeded low maximum.
    sources.scaling_columns.insert((60, 3), 2.0);
    sources.scaling_columns.insert((20, 3), 50.0);
    let low_maximum = scaled_record(9, 3.0, 0, 3, 0, 20);
    assert_eq!(
        amount_from(&apply_record(low_maximum, 40, (0, 0), &mut sources).unwrap()),
        6.0
    );

    let gem = scaled_record(9, 3.0, 0x20, 3, 0, 0);
    sources.scaling_columns.insert((10, 3), 4.0);
    assert_eq!(
        amount_from(&apply_record(gem, 10, (0, 0), &mut sources).unwrap()),
        12.0
    );
}

#[test]
fn source_fallback_row_count_and_max_level_narrow_to_u8() {
    let mut sources = Sources {
        scaling_row_count: 61,
        ..Sources::default()
    };
    sources.scaling_columns.insert((60, SCALING_CLASS), 2.0);
    let fallback = scaled_record(5, 4.0, 0x20, SCALING_CLASS, 0, 0);
    assert_eq!(
        amount_from(&apply_record(fallback, 70, (0, 0), &mut sources).unwrap()),
        8.0
    );

    sources.scaling_columns.clear();
    sources.scaling_columns.insert((1, SCALING_CLASS), 1.25);
    let narrowed = scaled_record(5, 4.0, 0x20, SCALING_CLASS, 0, 0x101);
    assert_eq!(
        amount_from(&apply_record(narrowed, 70, (0, 0), &mut sources).unwrap()),
        5.0
    );

    sources.scaling_columns.clear();
    sources.scaling_columns.insert((0, SCALING_CLASS), 2.0);
    let narrowed_zero = scaled_record(5, 4.0, 0x20, SCALING_CLASS, 0, 0x100);
    assert_eq!(
        amount_from(&apply_record(narrowed_zero, 70, (0, 0), &mut sources).unwrap()),
        8.0
    );
}

#[test]
fn absent_row_keeps_signed_amount_but_present_unknown_column_is_zero_before_clamp() {
    let mut sources = Sources {
        scaling_row_count: 100,
        ..Sources::default()
    };
    let positive_missing = scaled_record(13, 8.0, 0x20, SCALING_CLASS, 0, 0);
    assert_eq!(
        amount_from(&apply_record(positive_missing, 70, (0, 0), &mut sources).unwrap()),
        13.0
    );

    let absent = scaled_record(-13, 8.0, 0x20, SCALING_CLASS, 0, 0);
    assert_eq!(
        amount_from(&apply_record(absent, 70, (0, 0), &mut sources).unwrap()),
        1.0
    );

    let mut sources = sources;
    sources.scaling_columns.insert((70, SCALING_CLASS), 0.0);
    let unsupported = scaled_record(20, 8.0, 0x20, SCALING_CLASS, 0, 0);
    assert_eq!(
        amount_from(&apply_record(unsupported, 70, (0, 0), &mut sources).unwrap()),
        1.0
    );

    sources.scaling_columns.insert((70, SCALING_CLASS), -2.0);
    let negative = scaled_record(20, 8.0, 0x20, SCALING_CLASS, 0, 0);
    assert_eq!(
        amount_from(&apply_record(negative, 70, (0, 0), &mut sources).unwrap()),
        1.0
    );
}

#[test]
fn scaling_product_truncates_float32_toward_zero_and_rejects_undefined_cast_ranges() {
    let mut sources = Sources {
        scaling_row_count: 100,
        ..Sources::default()
    };
    sources.scaling_columns.insert((70, SCALING_CLASS), 2.0);
    let fractional = scaled_record(10, 1.75, 0x20, SCALING_CLASS, 0, 0);
    assert_eq!(
        amount_from(&apply_record(fractional, 70, (0, 0), &mut sources).unwrap()),
        3.0
    );

    for points in [f32::NAN, f32::INFINITY, 1_500_000_000.0, -1_500_000_000.0] {
        let invalid = scaled_record(1, points, 0x20, SCALING_CLASS, 0, 0);
        assert!(matches!(
            apply_record(invalid, 70, (0, 0), &mut sources),
            Err(SkillEnchantmentError::InvalidScalingConversion { .. })
        ));
    }

    sources.fail_scaling = true;
    let failed_source = scaled_record(1, 1.0, 0x20, SCALING_CLASS, 0, 1);
    assert!(matches!(
        apply_record(failed_source, 70, (0, 0), &mut sources),
        Err(SkillEnchantmentError::Source("scaling source failure"))
    ));
}
