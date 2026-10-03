use wow_persistence::forever::spells::{
    BattlePetSpeciesRow, DifficultyRow, SpellCastTimesRow, SpellCategoryRow, SpellDurationRow,
    SpellLearnSpellRow, SpellProcsPerMinuteModRow, SpellProcsPerMinuteRow, SpellRadiusRow,
    SpellRangeRow, SpellShapeshiftFormRow, SummonPropertiesRow,
};

#[test]
fn difficulty_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::difficulty(DifficultyRow {
        id: u32::MAX,
        name: vec![0xFF, 0, b'a'],
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
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.name.at(0), Some(&[0xFF, 0, b'a'][..]));
    assert_eq!(result.name.at(6), None);
    assert_eq!(result.instance_type, u8::MAX);
    assert_eq!(result.order_index, u8::MAX);
    assert_eq!(result.old_enum_value, i8::MIN);
    assert_eq!(result.fallback_difficulty_id, i16::MIN);
    assert_eq!(result.min_players, u8::MAX);
    assert_eq!(result.max_players, u8::MAX);
    assert_eq!(result.flags, i32::MIN);
    assert_eq!(result.item_context, u8::MAX);
    assert_eq!(result.toggle_difficulty_id, i16::MIN);
    assert_eq!(result.group_size_health_curve_id, u32::MAX);
    assert_eq!(result.group_size_dmg_curve_id, u32::MAX);
    assert_eq!(result.group_size_spell_points_curve_id, u32::MAX);
    assert_eq!(result.unknown1105, i32::MIN);
}

#[test]
fn spell_cast_times_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::spell_cast_times(SpellCastTimesRow {
        id: u32::MAX,
        base: i32::MIN,
        minimum: i32::MIN,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.base, i32::MIN);
    assert_eq!(result.minimum, i32::MIN);
}

#[test]
fn spell_duration_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::spell_duration(SpellDurationRow {
        id: u32::MAX,
        duration: i32::MIN,
        max_duration: i32::MIN,
        duration_per_resource: i32::MIN,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.duration, i32::MIN);
    assert_eq!(result.max_duration, i32::MIN);
    assert_eq!(result.duration_per_resource, i32::MIN);
}

#[test]
fn spell_range_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::spell_range(SpellRangeRow {
        id: u32::MAX,
        display_name: vec![0xFF, 0, b'a'],
        display_name_short: vec![0xFF, 0, b'a'],
        flags: i32::MIN,
        range_min: [f32::from_bits(0x7FC0_1234); 2],
        range_max: [f32::from_bits(0x7FC0_1234); 2],
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.display_name.at(0), Some(&[0xFF, 0, b'a'][..]));
    assert_eq!(result.display_name.at(6), None);
    assert_eq!(result.display_name_short.at(0), Some(&[0xFF, 0, b'a'][..]));
    assert_eq!(result.display_name_short.at(6), None);
    assert_eq!(result.flags, i32::MIN);
    assert_eq!(result.range_min, [f32::from_bits(0x7FC0_1234); 2]);
    assert_eq!(result.range_max, [f32::from_bits(0x7FC0_1234); 2]);
}

#[test]
fn spell_radius_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::spell_radius(SpellRadiusRow {
        id: u32::MAX,
        radius: f32::from_bits(0x7FC0_1234),
        radius_per_level: f32::from_bits(0x7FC0_1234),
        radius_min: f32::from_bits(0x7FC0_1234),
        radius_max: f32::from_bits(0x7FC0_1234),
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.radius.to_bits(), 0x7FC0_1234);
    assert_eq!(result.radius_per_level.to_bits(), 0x7FC0_1234);
    assert_eq!(result.radius_min.to_bits(), 0x7FC0_1234);
    assert_eq!(result.radius_max.to_bits(), 0x7FC0_1234);
}

#[test]
fn spell_procs_per_minute_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::spell_procs_per_minute(SpellProcsPerMinuteRow {
        id: u32::MAX,
        base_proc_rate: f32::from_bits(0x7FC0_1234),
        flags: i32::MIN,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.base_proc_rate.to_bits(), 0x7FC0_1234);
    assert_eq!(result.flags, i32::MIN);
}

#[test]
fn spell_procs_per_minute_mod_moves_all_source_columns_without_loss() {
    let result =
        super::super::dependencies::spell_procs_per_minute_mod(SpellProcsPerMinuteModRow {
            id: u32::MAX,
            r#type: i32::MIN,
            param: i32::MIN,
            coeff: f32::from_bits(0x7FC0_1234),
            field_12_1_5_69594_003: i32::MIN,
            spell_procs_per_minute_id: u32::MAX,
        })
        .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.r#type, i32::MIN);
    assert_eq!(result.param, i32::MIN);
    assert_eq!(result.coeff.to_bits(), 0x7FC0_1234);
    assert_eq!(result.field_12_1_5_69594_003, i32::MIN);
    assert_eq!(result.spell_procs_per_minute_id, u32::MAX);
}

#[test]
fn spell_learn_spell_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::spell_learn_spell(SpellLearnSpellRow {
        id: u32::MAX,
        spell_id: u32::MAX,
        learn_spell_id: i32::MIN,
        overrides_spell_id: i32::MIN,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, u32::MAX);
    assert_eq!(result.learn_spell_id, i32::MIN);
    assert_eq!(result.overrides_spell_id, i32::MIN);
}

#[test]
fn spell_shapeshift_form_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::spell_shapeshift_form(SpellShapeshiftFormRow {
        id: u32::MAX,
        name: vec![0xFF, 0, b'a'],
        creature_display_id: u32::MAX,
        creature_type: u8::MAX,
        flags: i32::MIN,
        attack_icon_file_id: i32::MIN,
        bonus_action_bar: i8::MIN,
        combat_round_time: i16::MIN,
        damage_variance: f32::from_bits(0x7FC0_1234),
        mount_type_id: u16::MAX,
        preset_spell_id: [u32::MAX; 8],
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.name.at(0), Some(&[0xFF, 0, b'a'][..]));
    assert_eq!(result.name.at(6), None);
    assert_eq!(result.creature_display_id, u32::MAX);
    assert_eq!(result.creature_type, u8::MAX);
    assert_eq!(result.flags, i32::MIN);
    assert_eq!(result.attack_icon_file_id, i32::MIN);
    assert_eq!(result.bonus_action_bar, i8::MIN);
    assert_eq!(result.combat_round_time, i16::MIN);
    assert_eq!(result.damage_variance.to_bits(), 0x7FC0_1234);
    assert_eq!(result.mount_type_id, u16::MAX);
    assert_eq!(result.preset_spell_id, [u32::MAX; 8]);
}

#[test]
fn summon_properties_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::summon_properties(SummonPropertiesRow {
        id: u32::MAX,
        control: i32::MIN,
        faction: i32::MIN,
        title: i32::MIN,
        slot: i32::MIN,
        flags: [i32::MIN; 2],
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.control, i32::MIN);
    assert_eq!(result.faction, i32::MIN);
    assert_eq!(result.title, i32::MIN);
    assert_eq!(result.slot, i32::MIN);
    assert_eq!(result.flags, [i32::MIN; 2]);
}

#[test]
fn battle_pet_species_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::battle_pet_species(BattlePetSpeciesRow {
        description: vec![0xFF, 0, b'a'],
        source_text: vec![0xFF, 0, b'a'],
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
    })
    .unwrap();
    assert_eq!(result.description.at(0), Some(&[0xFF, 0, b'a'][..]));
    assert_eq!(result.description.at(6), None);
    assert_eq!(result.source_text.at(0), Some(&[0xFF, 0, b'a'][..]));
    assert_eq!(result.source_text.at(6), None);
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.creature_id, i32::MIN);
    assert_eq!(result.summon_spell_id, i32::MIN);
    assert_eq!(result.icon_file_data_id, i32::MIN);
    assert_eq!(result.pet_type_enum, i8::MIN);
    assert_eq!(result.flags, i32::MIN);
    assert_eq!(result.source_type_enum, i8::MIN);
    assert_eq!(result.card_ui_model_scene_id, i32::MIN);
    assert_eq!(result.loadout_ui_model_scene_id, i32::MIN);
    assert_eq!(result.covenant_id, i32::MIN);
}

#[test]
fn spell_category_moves_all_source_columns_without_loss() {
    let result = super::super::dependencies::spell_category(SpellCategoryRow {
        id: u32::MAX,
        name: vec![0xFF, 0, b'a'],
        flags: i32::MIN,
        uses_per_week: i32::MIN,
        max_charges: i32::MIN,
        charge_recovery_time: i32::MIN,
        type_mask: i32::MIN,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.name.at(0), Some(&[0xFF, 0, b'a'][..]));
    assert_eq!(result.name.at(6), None);
    assert_eq!(result.flags, i32::MIN);
    assert_eq!(result.uses_per_week, i32::MIN);
    assert_eq!(result.max_charges, i32::MIN);
    assert_eq!(result.charge_recovery_time, i32::MIN);
    assert_eq!(result.type_mask, i32::MIN);
}
