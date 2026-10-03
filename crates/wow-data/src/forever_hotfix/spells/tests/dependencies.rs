//! Synthetic Source-width wire goldens; no client values or keys.
use crate::forever_spells::*;

#[test]
fn difficulty_writes_complete_metadata_order_and_widths() {
    let row = DifficultyRecord {
        id: u32::MAX,
        name: SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap(),
        instance_type: u8::MAX,
        order_index: u8::MAX,
        old_enum_value: i8::MIN,
        fallback_difficulty_id: i16::MIN,
        min_players: u8::MAX,
        max_players: u8::MAX,
        flags: i32::MIN,
        item_context: u8::MAX,
        toggle_difficulty_id: i16::MIN,
        group_size_health_curve_id: u32::MAX,
        group_size_dmg_curve_id: u32::MAX,
        group_size_spell_points_curve_id: u32::MAX,
        unknown1105: i32::MIN,
    };
    assert_eq!(
        super::super::dependencies::difficulty(&row, 6),
        [
            255, 0, 255, 255, 128, 0, 128, 255, 255, 0, 0, 0, 128, 255, 0, 128, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 128
        ]
    );
}

#[test]
fn spell_cast_times_writes_complete_metadata_order_and_widths() {
    let row = SpellCastTimesRecord {
        id: u32::MAX,
        base: i32::MIN,
        minimum: i32::MIN,
    };
    assert_eq!(
        super::super::dependencies::spell_cast_times(&row, 6),
        [0, 0, 0, 128, 0, 0, 0, 128]
    );
}

#[test]
fn spell_duration_writes_complete_metadata_order_and_widths() {
    let row = SpellDurationRecord {
        id: u32::MAX,
        duration: i32::MIN,
        max_duration: i32::MIN,
        duration_per_resource: i32::MIN,
    };
    assert_eq!(
        super::super::dependencies::spell_duration(&row, 6),
        [0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128]
    );
}

#[test]
fn spell_range_writes_complete_metadata_order_and_widths() {
    let row = SpellRangeRecord {
        id: u32::MAX,
        display_name: SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap(),
        display_name_short: SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap(),
        flags: i32::MIN,
        range_min: [f32::from_bits(0x7FC0_1234); 2],
        range_max: [f32::from_bits(0x7FC0_1234); 2],
    };
    assert_eq!(
        super::super::dependencies::spell_range(&row, 6),
        [
            255, 0, 255, 0, 0, 0, 0, 128, 52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127, 52,
            18, 192, 127
        ]
    );
}

#[test]
fn spell_radius_writes_complete_metadata_order_and_widths() {
    let row = SpellRadiusRecord {
        id: u32::MAX,
        radius: f32::from_bits(0x7FC0_1234),
        radius_per_level: f32::from_bits(0x7FC0_1234),
        radius_min: f32::from_bits(0x7FC0_1234),
        radius_max: f32::from_bits(0x7FC0_1234),
    };
    assert_eq!(
        super::super::dependencies::spell_radius(&row, 6),
        [
            52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127
        ]
    );
}

#[test]
fn spell_procs_per_minute_writes_complete_metadata_order_and_widths() {
    let row = SpellProcsPerMinuteRecord {
        id: u32::MAX,
        base_proc_rate: f32::from_bits(0x7FC0_1234),
        flags: i32::MIN,
    };
    assert_eq!(
        super::super::dependencies::spell_procs_per_minute(&row, 6),
        [52, 18, 192, 127, 0, 0, 0, 128]
    );
}

#[test]
fn spell_procs_per_minute_mod_writes_complete_metadata_order_and_widths() {
    let row = SpellProcsPerMinuteModRecord {
        id: u32::MAX,
        r#type: i32::MIN,
        param: i32::MIN,
        coeff: f32::from_bits(0x7FC0_1234),
        field_12_1_5_69594_003: i32::MIN,
        spell_procs_per_minute_id: u32::MAX,
    };
    assert_eq!(
        super::super::dependencies::spell_procs_per_minute_mod(&row, 6),
        [
            0, 0, 0, 128, 0, 0, 0, 128, 52, 18, 192, 127, 0, 0, 0, 128, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_learn_spell_writes_complete_metadata_order_and_widths() {
    let row = SpellLearnSpellRecord {
        id: u32::MAX,
        spell_id: u32::MAX,
        learn_spell_id: i32::MIN,
        overrides_spell_id: i32::MIN,
    };
    assert_eq!(
        super::super::dependencies::spell_learn_spell(&row, 6),
        [255, 255, 255, 255, 0, 0, 0, 128, 0, 0, 0, 128]
    );
}

#[test]
fn spell_shapeshift_form_writes_complete_metadata_order_and_widths() {
    let row = SpellShapeshiftFormRecord {
        id: u32::MAX,
        name: SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap(),
        creature_display_id: u32::MAX,
        creature_type: u8::MAX,
        flags: i32::MIN,
        attack_icon_file_id: i32::MIN,
        bonus_action_bar: i8::MIN,
        combat_round_time: i16::MIN,
        damage_variance: f32::from_bits(0x7FC0_1234),
        mount_type_id: u16::MAX,
        preset_spell_id: [u32::MAX; 8],
    };
    assert_eq!(
        super::super::dependencies::spell_shapeshift_form(&row, 6),
        [
            255, 0, 255, 255, 255, 255, 255, 0, 0, 0, 128, 0, 0, 0, 128, 128, 0, 128, 52, 18, 192,
            127, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255
        ]
    );
}

#[test]
fn summon_properties_writes_complete_metadata_order_and_widths() {
    let row = SummonPropertiesRecord {
        id: u32::MAX,
        control: i32::MIN,
        faction: i32::MIN,
        title: i32::MIN,
        slot: i32::MIN,
        flags: [i32::MIN; 2],
    };
    assert_eq!(
        super::super::dependencies::summon_properties(&row, 6),
        [
            0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128
        ]
    );
}

#[test]
fn battle_pet_species_writes_complete_metadata_order_and_widths() {
    let row = BattlePetSpeciesRecord {
        description: SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap(),
        source_text: SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap(),
        id: u32::MAX,
        creature_id: i32::MIN,
        summon_spell_id: i32::MIN,
        icon_file_data_id: i32::MIN,
        pet_type_enum: i8::MIN,
        flags: i32::MIN,
        source_type_enum: i8::MIN,
        card_ui_model_scene_id: i32::MIN,
        loadout_ui_model_scene_id: i32::MIN,
        covenant_id: i32::MIN,
    };
    assert_eq!(
        super::super::dependencies::battle_pet_species(&row, 6),
        [
            255, 0, 255, 0, 255, 255, 255, 255, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 128, 0,
            0, 0, 128, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128
        ]
    );
}

#[test]
fn spell_category_writes_complete_metadata_order_and_widths() {
    let row = SpellCategoryRecord {
        id: u32::MAX,
        name: SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap(),
        flags: i32::MIN,
        uses_per_week: i32::MIN,
        max_charges: i32::MIN,
        charge_recovery_time: i32::MIN,
        type_mask: i32::MIN,
    };
    assert_eq!(
        super::super::dependencies::spell_category(&row, 6),
        [
            255, 0, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128
        ]
    );
}
