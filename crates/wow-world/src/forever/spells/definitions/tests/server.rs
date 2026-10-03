use super::*;
#[test]
fn server_main_projection_keeps_all_55_scalar_fields_and_full_uint32_dependency_ids() {
    let mut row = server_spell(7, 0);
    row.category_id = u32::MAX;
    row.dispel = u32::MAX;
    row.mechanic = u32::MAX;
    row.attributes = [u32::MAX; 17];
    row.stances = u64::MAX;
    row.stances_not = u64::MAX;
    row.targets = u32::MAX;
    row.target_creature_type = u32::MAX;
    row.requires_spell_focus = u32::MAX;
    row.facing_caster_flags = u32::MAX;
    row.caster_aura_state = u32::MAX;
    row.target_aura_state = u32::MAX;
    row.exclude_caster_aura_state = u32::MAX;
    row.exclude_target_aura_state = u32::MAX;
    row.caster_aura_spell = u32::MAX;
    row.target_aura_spell = u32::MAX;
    row.exclude_caster_aura_spell = u32::MAX;
    row.exclude_target_aura_spell = u32::MAX;
    row.caster_aura_type = i32::MIN;
    row.target_aura_type = i32::MIN;
    row.exclude_caster_aura_type = i32::MIN;
    row.exclude_target_aura_type = i32::MIN;
    row.casting_time_index = u32::MAX;
    row.recovery_time = u32::MAX;
    row.category_recovery_time = u32::MAX;
    row.start_recovery_category = u32::MAX;
    row.start_recovery_time = u32::MAX;
    row.interrupt_flags = u32::MAX;
    row.aura_interrupt_flags = [u32::MAX; 2];
    row.channel_interrupt_flags = [u32::MAX; 2];
    row.proc_flags = [u32::MAX; 2];
    row.proc_chance = u32::MAX;
    row.proc_charges = u32::MAX;
    row.proc_cooldown = u32::MAX;
    row.proc_base_ppm = f32::from_bits(0x7FC0_1234);
    row.max_level = u32::MAX;
    row.base_level = u32::MAX;
    row.spell_level = u32::MAX;
    row.duration_index = u32::MAX;
    row.range_index = u32::MAX;
    row.speed = f32::from_bits(0x7FC0_1234);
    row.launch_delay = f32::from_bits(0x7FC0_1234);
    row.stack_amount = u32::MAX;
    row.equipped_item_class = i32::MIN;
    row.equipped_item_sub_class_mask = i32::MIN;
    row.equipped_item_inventory_type_mask = i32::MIN;
    row.content_tuning_id = u32::MAX;
    row.cone_angle = f32::from_bits(0x7FC0_1234);
    row.cone_width = f32::from_bits(0x7FC0_1234);
    row.max_target_level = u32::MAX;
    row.max_affected_targets = u32::MAX;
    row.spell_family_name = u32::MAX;
    row.spell_family_flags = [u32::MAX; 4];
    row.dmg_class = u32::MAX;
    row.prevention_type = u32::MAX;
    row.area_group_id = i32::MIN;
    row.school_mask = u32::MAX;
    row.charge_category_id = u32::MAX;
    row.spell_name = vec![0xFF, 0, b'x'];
    let raw_rows = SpellRecords {
        spell_cast_times: vec![SpellCastTimesRecord {
            id: u32::MAX,
            base: -1,
            minimum: -2,
        }],
        spell_durations: vec![SpellDurationRecord {
            id: u32::MAX,
            duration: -1,
            max_duration: -2,
            duration_per_resource: -3,
        }],
        spell_ranges: vec![SpellRangeRecord {
            id: u32::MAX,
            display_name: SpellText::default(),
            display_name_short: SpellText::default(),
            flags: -1,
            range_min: [-1.0; 2],
            range_max: [1.0; 2],
        }],
        ..Default::default()
    };
    let raw = catalog(raw_rows);
    let result = SpellLoadPlan::build(raw.clone())
        .unwrap()
        .with_server_spells(ServerSpellRows {
            spells: vec![row],
            effects: vec![],
        })
        .unwrap();
    let definition = result.get_exact(7, 0).unwrap();
    let fields = definition.fields();
    assert_eq!(fields.attributes, [u32::MAX; 17]);
    assert_eq!(fields.speed.to_bits(), 0x7FC0_1234);
    assert_eq!(fields.launch_delay.to_bits(), 0x7FC0_1234);
    assert_eq!(fields.school_mask, u32::MAX);
    assert_eq!(fields.content_tuning_id, u32::MAX);
    assert_eq!(fields.proc_flags, [u32::MAX; 2]);
    assert_eq!(fields.proc_chance, u32::MAX);
    assert_eq!(fields.proc_charges, u32::MAX);
    assert_eq!(fields.proc_cooldown, u32::MAX);
    assert_eq!(fields.stack_amount, u32::MAX);
    assert_eq!(fields.caster_aura_state, u32::MAX);
    assert_eq!(fields.target_aura_state, u32::MAX);
    assert_eq!(fields.exclude_caster_aura_state, u32::MAX);
    assert_eq!(fields.exclude_target_aura_state, u32::MAX);
    assert_eq!(fields.caster_aura_spell, u32::MAX);
    assert_eq!(fields.target_aura_spell, u32::MAX);
    assert_eq!(fields.exclude_caster_aura_spell, u32::MAX);
    assert_eq!(fields.exclude_target_aura_spell, u32::MAX);
    assert_eq!(fields.caster_aura_type, (i32::MIN) as u32);
    assert_eq!(fields.target_aura_type, (i32::MIN) as u32);
    assert_eq!(fields.exclude_caster_aura_type, (i32::MIN) as u32);
    assert_eq!(fields.exclude_target_aura_type, (i32::MIN) as u32);
    assert_eq!(fields.requires_spell_focus, u32::MAX);
    assert_eq!(fields.facing_caster_flags, u32::MAX);
    assert_eq!(fields.required_areas_id, i32::MIN);
    assert_eq!(fields.category_id, u32::MAX);
    assert_eq!(fields.dispel, u32::MAX);
    assert_eq!(fields.mechanic, u32::MAX);
    assert_eq!(fields.start_recovery_category, u32::MAX);
    assert_eq!(fields.damage_class, u32::MAX);
    assert_eq!(fields.prevention_type, u32::MAX);
    assert_eq!(fields.charge_category_id, u32::MAX);
    assert_eq!(fields.spell_family_name, u32::MAX);
    assert_eq!(fields.spell_family_flags, [u32::MAX; 4]);
    assert_eq!(fields.recovery_time, u32::MAX);
    assert_eq!(fields.category_recovery_time, u32::MAX);
    assert_eq!(fields.start_recovery_time, u32::MAX);
    assert_eq!(fields.equipped_item_class, i32::MIN);
    assert_eq!(fields.equipped_item_subclass_mask, i32::MIN);
    assert_eq!(fields.equipped_item_inventory_type_mask, i32::MIN);
    assert_eq!(fields.interrupt_flags, u32::MAX);
    assert_eq!(fields.aura_interrupt_flags, [u32::MAX; 2]);
    assert_eq!(fields.channel_interrupt_flags, [u32::MAX; 2]);
    assert_eq!(fields.max_level, u32::MAX);
    assert_eq!(fields.base_level, u32::MAX);
    assert_eq!(fields.spell_level, u32::MAX);
    assert_eq!(fields.stances, u64::MAX);
    assert_eq!(fields.stances_not, u64::MAX);
    assert_eq!(fields.cone_angle.to_bits(), 0x7FC0_1234);
    assert_eq!(fields.width.to_bits(), 0x7FC0_1234);
    assert_eq!(fields.targets, u32::MAX);
    assert_eq!(fields.target_creature_type, u32::MAX);
    assert_eq!(fields.max_affected_targets, u32::MAX);
    assert_eq!(fields.max_target_level, u32::MAX);
    assert_eq!(fields.proc_base_ppm.to_bits(), 0x7FC0_1234);
    assert_eq!(fields.min_duration, 0.0);
    assert_eq!(fields.min_scaling_level, 0);
    assert_eq!(fields.max_scaling_level, 0);
    assert_eq!(fields.icon_file_data_id, 0);
    assert_eq!(fields.active_icon_file_data_id, 0);
    assert_eq!(fields.show_future_spell_player_condition_id, 0);
    assert_eq!(fields.cooldown_aura_spell_id, 0);
    assert_eq!(fields.reagents, [0; 8]);
    assert_eq!(fields.reagent_counts, [0; 8]);
    assert_eq!(fields.totems, [0; 2]);
    assert_eq!(fields.totem_category, [0; 2]);
    assert!(definition.powers().iter().all(Option::is_none));
    assert_eq!(definition.ppm_modifiers().count(), 0);
    assert!(definition.labels().is_empty());
    assert!(definition.empower_thresholds_ms().is_empty());
    assert!(std::ptr::eq(
        definition.cast_time().unwrap(),
        raw.spell_cast_times(u32::MAX).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.duration().unwrap(),
        raw.spell_duration(u32::MAX).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.range().unwrap(),
        raw.spell_range(u32::MAX).unwrap()
    ));
    for locale in 0..12 {
        assert_eq!(definition.name_bytes(locale), Some(&[0xFF][..]));
    }
    assert_eq!(definition.name_bytes(12), None);
}

#[test]
fn server_effect_projection_keeps_all_fields_and_zeroes_unselected_scaling_class() {
    let mut row = server_effect(7, 0);
    row.effect_index = 2;
    row.effect = 360_i32;
    row.effect_aura = 664_i16;
    row.effect_amplitude = f32::from_bits(0x7FC0_1234);
    row.effect_attributes = i32::MIN;
    row.effect_aura_period = i32::MIN;
    row.effect_bonus_coefficient = f32::from_bits(0x7FC0_1234);
    row.effect_chain_amplitude = f32::from_bits(0x7FC0_1234);
    row.effect_chain_targets = i32::MIN;
    row.effect_item_type = i32::MIN;
    row.effect_mechanic = i32::MIN;
    row.effect_points_per_resource = f32::from_bits(0x7FC0_1234);
    row.effect_pos_facing = f32::from_bits(0x7FC0_1234);
    row.effect_real_points_per_level = f32::from_bits(0x7FC0_1234);
    row.effect_trigger_spell = i32::MIN;
    row.bonus_coefficient_from_ap = f32::from_bits(0x7FC0_1234);
    row.pvp_multiplier = f32::from_bits(0x7FC0_1234);
    row.coefficient = f32::from_bits(0x7FC0_1234);
    row.variance = f32::from_bits(0x7FC0_1234);
    row.resource_coefficient = f32::from_bits(0x7FC0_1234);
    row.group_size_base_points_coefficient = f32::from_bits(0x7FC0_1234);
    row.effect_base_points = f32::from_bits(0x7FC0_1234);
    row.effect_misc_value = [i32::MIN; 2];
    row.effect_radius_index = [0_u32, 99];
    row.effect_spell_class_mask = [i32::MIN; 4];
    row.implicit_target = [152_i16; 2];
    let raw = catalog(SpellRecords {
        spell_radii: vec![SpellRadiusRecord {
            id: 0,
            radius: 1.0,
            radius_per_level: 2.0,
            radius_min: 3.0,
            radius_max: 4.0,
        }],
        ..Default::default()
    });
    let result = SpellLoadPlan::build(raw.clone())
        .unwrap()
        .with_server_spells(ServerSpellRows {
            spells: vec![server_spell(7, 0)],
            effects: vec![row],
        })
        .unwrap();
    let definition = result.get_exact(7, 0).unwrap();
    let effect = definition.effect(2).unwrap();
    let values = effect.values();
    assert_eq!(definition.effect_count(), 3);
    assert_eq!(values.index, 2);
    assert_eq!(values.effect, (360_i32) as u32);
    assert_eq!(values.aura, (664_i16) as u32);
    assert_eq!(values.aura_period, (i32::MIN) as u32);
    assert_eq!(values.base_points.to_bits(), 0x7FC0_1234);
    assert_eq!(values.real_points_per_level.to_bits(), 0x7FC0_1234);
    assert_eq!(values.points_per_resource.to_bits(), 0x7FC0_1234);
    assert_eq!(values.amplitude.to_bits(), 0x7FC0_1234);
    assert_eq!(values.chain_amplitude.to_bits(), 0x7FC0_1234);
    assert_eq!(values.bonus_coefficient.to_bits(), 0x7FC0_1234);
    assert_eq!(values.misc_values, [i32::MIN; 2]);
    assert_eq!(values.mechanic, (i32::MIN) as u32);
    assert_eq!(values.position_facing.to_bits(), 0x7FC0_1234);
    assert_eq!(
        values.implicit_targets,
        ([152_i16; 2]).map(|target| target as u32)
    );
    assert_eq!(values.chain_targets, (i32::MIN));
    assert_eq!(values.item_type, (i32::MIN) as u32);
    assert_eq!(values.trigger_spell, (i32::MIN) as u32);
    assert_eq!(values.class_mask, ([i32::MIN; 4]).map(|word| word as u32));
    assert_eq!(values.bonus_coefficient_from_ap.to_bits(), 0x7FC0_1234);
    assert_eq!(values.scaling_class, 0);
    assert_eq!(values.scaling_coefficient.to_bits(), 0x7FC0_1234);
    assert_eq!(values.scaling_variance.to_bits(), 0x7FC0_1234);
    assert_eq!(values.scaling_resource_coefficient.to_bits(), 0x7FC0_1234);
    assert_eq!(values.attributes, (i32::MIN) as u32);
    assert!(std::ptr::eq(
        effect.radii()[0].unwrap(),
        raw.spell_radius(0).unwrap()
    ));
    assert!(effect.radii()[1].is_none());
    assert_eq!(result.server_counts().missing_radius_warnings, 1);
    for index in 0..2 {
        let blank = definition.effect(index).unwrap();
        assert_eq!(blank.values().index, index as u32);
        assert_eq!(blank.values().effect, 0);
        assert_eq!(blank.values().class_mask, [0; 4]);
        assert_eq!(blank.values().base_points, 0.0);
        assert_eq!(blank.values().radius_ids, [None; 2]);
    }
}

#[test]
fn duplicate_server_rows_keep_first_name_effects_and_reapply_last_scalars_links() {
    let mut first = server_spell(7, 0);
    first.spell_name = b"first".to_vec();
    first.recovery_time = 1;
    first.casting_time_index = 1;
    let mut last = server_spell(7, 0);
    last.spell_name = b"last".to_vec();
    last.recovery_time = 2;
    last.casting_time_index = 99;
    let mut first_effect = server_effect(7, 0);
    first_effect.effect_index = 2;
    first_effect.effect_trigger_spell = 10;
    let mut last_effect = server_effect(7, 0);
    last_effect.effect_index = 2;
    last_effect.effect_trigger_spell = 20;
    let raw = catalog(SpellRecords {
        spell_cast_times: vec![SpellCastTimesRecord {
            id: 1,
            base: 100,
            minimum: 0,
        }],
        ..Default::default()
    });
    let result = SpellLoadPlan::build(raw)
        .unwrap()
        .with_server_spells(ServerSpellRows {
            spells: vec![first, last],
            effects: vec![first_effect, last_effect],
        })
        .unwrap();
    let definition = result.get_exact(7, 0).unwrap();
    assert_eq!(definition.name_bytes(6), Some(&b"first"[..]));
    assert_eq!(definition.fields().recovery_time, 2);
    assert!(definition.cast_time().is_none());
    assert_eq!(definition.effect(2).unwrap().values().trigger_spell, 20);
    assert_eq!(result.server_counts().definitions_added, 1);
    assert_eq!(result.server_counts().duplicate_spell_rows, 1);
}

#[test]
fn main_rows_allow_missing_difficulty_and_orphan_effects_never_manufacture_definitions() {
    let result = build(ServerSpellRows {
        spells: vec![server_spell(7, 5)],
        effects: vec![server_effect(7, 5), server_effect(8, 0)],
    });
    assert!(result.get_exact(7, 5).is_some());
    assert_eq!(result.get_exact(7, 5).unwrap().effect_count(), 0);
    assert!(result.get_exact(8, 0).is_none());
    assert_eq!(result.server_counts().skipped_missing_difficulty_effects, 1);
    assert_eq!(result.server_counts().orphan_effect_groups, 1);
}

#[test]
fn replay_server_requests_keep_admitted_duplicates_and_reject_client_name_collisions() {
    let plan = SpellLoadPlan::build(catalog(SpellRecords {
        spell_names: vec![named(10)],
        ..Default::default()
    }))
    .unwrap();
    let result = plan
        .with_server_spells(ServerSpellRows {
            spells: vec![
                server_spell(9, 0),
                server_spell(2, 0),
                server_spell(9, 0),
                server_spell(10, 0),
            ],
            effects: Vec::new(),
        })
        .unwrap();
    let inputs = result.traversal_inputs().unwrap();
    assert!(inputs.helper_insertions().is_empty());
    assert!(inputs.client_keys().next().is_none());
    assert_eq!(inputs.server_requests(), &[(9, 0), (2, 0), (9, 0)]);
    assert_eq!(result.len(), 2);
    assert_eq!(result.server_counts().duplicate_spell_rows, 1);
    assert_eq!(result.server_counts().rejected_client_names, 1);
}
