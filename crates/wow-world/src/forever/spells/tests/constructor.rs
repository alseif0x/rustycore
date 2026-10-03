use super::*;

#[test]
fn constructor_copies_every_target_scalar_with_signed_bits_and_source_defaults() {
    let mut rows = named(&[1]);
    let mut row = spell_aura_options(1, 1);
    row.cumulative_aura = u16::MAX;
    row.proc_category_recovery = i32::MIN;
    row.proc_chance = u8::MAX;
    row.proc_charges = i32::MIN;
    row.spell_procs_per_minute_id = u16::MAX;
    row.proc_type_mask = [i32::MIN; 2];
    rows.spell_aura_options = vec![row];
    let mut row = spell_aura_restrictions(1, 1);
    row.caster_aura_state = i32::MIN;
    row.target_aura_state = i32::MIN;
    row.exclude_caster_aura_state = i32::MIN;
    row.exclude_target_aura_state = i32::MIN;
    row.caster_aura_spell = i32::MIN;
    row.target_aura_spell = i32::MIN;
    row.exclude_caster_aura_spell = i32::MIN;
    row.exclude_target_aura_spell = i32::MIN;
    row.caster_aura_type = i16::MIN;
    row.target_aura_type = i16::MIN;
    row.exclude_caster_aura_type = i16::MIN;
    row.exclude_target_aura_type = i16::MIN;
    rows.spell_aura_restrictions = vec![row];
    let mut row = spell_casting_requirements(1, 1);
    row.facing_caster_flags = i32::MIN;
    row.min_faction_id = u16::MAX;
    row.min_reputation = i32::MIN;
    row.required_areas_id = u16::MAX;
    row.required_aura_vision = u8::MAX;
    row.requires_spell_focus = u16::MAX;
    rows.spell_casting_requirements = vec![row];
    let mut row = spell_categories(1, 1);
    row.category = i16::MIN;
    row.defense_type = i8::MIN;
    row.diminish_type = i32::MIN;
    row.dispel_type = i8::MIN;
    row.mechanic = i8::MIN;
    row.prevention_type = i32::MIN;
    row.start_recovery_category = i16::MIN;
    row.charge_category = i16::MIN;
    rows.spell_categories = vec![row];
    let mut row = spell_class_options(1, 1);
    row.modal_next_spell = u32::MAX;
    row.spell_class_set = i32::MIN;
    row.spell_class_mask = [u32::MAX; 4];
    rows.spell_class_options = vec![row];
    let mut row = spell_cooldowns(1, 1);
    row.category_recovery_time = i32::MIN;
    row.recovery_time = i32::MIN;
    row.start_recovery_time = i32::MIN;
    row.aura_spell_id = i32::MIN;
    rows.spell_cooldowns = vec![row];
    let mut row = spell_equipped_items(1, 1);
    row.equipped_item_class = i32::MIN;
    row.equipped_item_inv_types = i32::MIN;
    row.equipped_item_subclass = i32::MIN;
    rows.spell_equipped_items = vec![row];
    let mut row = spell_interrupts(1, 1);
    row.interrupt_flags = i32::MIN;
    row.aura_interrupt_flags = [i32::MIN; 2];
    row.channel_interrupt_flags = [i32::MIN; 2];
    rows.spell_interrupts = vec![row];
    let mut row = spell_levels(1, 1);
    row.max_level = i16::MIN;
    row.max_passive_aura_level = u8::MAX;
    row.base_level = i32::MIN;
    row.spell_level = i32::MIN;
    rows.spell_levels = vec![row];
    let mut row = spell_misc(1, 1);
    row.attributes = [i32::MIN; 17];
    row.casting_time_index = u16::MAX;
    row.duration_index = u16::MAX;
    row.pv_p_duration_index = u16::MAX;
    row.range_index = u16::MAX;
    row.school_mask = u8::MAX;
    row.speed = f32::from_bits(0x7FC0_1234);
    row.launch_delay = f32::from_bits(0x7FC0_1234);
    row.min_duration = f32::from_bits(0x7FC0_1234);
    row.spell_icon_file_data_id = i32::MIN;
    row.active_icon_file_data_id = i32::MIN;
    row.content_tuning_id = i32::MIN;
    row.show_future_spell_player_condition_id = i32::MIN;
    row.spell_visual_script = i32::MIN;
    row.active_spell_visual_script = i32::MIN;
    rows.spell_misc = vec![row];
    let mut row = spell_reagents(1, 1);
    row.reagent = [i32::MIN; 8];
    row.reagent_count = [i16::MIN; 8];
    row.reagent_recraft_count = [i16::MIN; 8];
    row.reagent_source = [u8::MAX; 8];
    rows.spell_reagents = vec![row];
    let mut row = spell_scaling(1, 1);
    row.min_scaling_level = u32::MAX;
    row.max_scaling_level = u32::MAX;
    rows.spell_scaling = vec![row];
    let mut row = spell_shapeshift(1, 1);
    row.stance_bar_order = i8::MIN;
    row.shapeshift_exclude = [i32::MIN; 2];
    row.shapeshift_mask = [i32::MIN; 2];
    rows.spell_shapeshifts = vec![row];
    let mut row = spell_target_restrictions(1, 1);
    row.cone_degrees = f32::from_bits(0x7FC0_1234);
    row.max_targets = u8::MAX;
    row.max_target_level = u32::MAX;
    row.target_creature_type = i16::MIN;
    row.targets = i32::MIN;
    row.width = f32::from_bits(0x7FC0_1234);
    rows.spell_target_restrictions = vec![row];
    let mut row = spell_totems(1, 1);
    row.required_totem_category_id = [u16::MAX; 2];
    row.totem = [i32::MIN; 2];
    rows.spell_totems = vec![row];
    rows.spell_cast_times = vec![SpellCastTimesRecord {
        id: 65535,
        base: -1,
        minimum: -2,
    }];
    rows.spell_durations = vec![SpellDurationRecord {
        id: 65535,
        duration: -1,
        max_duration: -2,
        duration_per_resource: -3,
    }];
    rows.spell_ranges = vec![SpellRangeRecord {
        id: 65535,
        display_name: SpellText::default(),
        display_name_short: SpellText::default(),
        flags: -1,
        range_min: [-1.0; 2],
        range_max: [1.0; 2],
    }];
    rows.spell_procs_per_minute = vec![SpellProcsPerMinuteRecord {
        id: 65535,
        base_proc_rate: f32::from_bits(0x7FC0_1234),
        flags: 0,
    }];
    rows.spell_procs_per_minute_mods = vec![
        SpellProcsPerMinuteModRecord {
            id: 2,
            r#type: 0,
            param: 0,
            coeff: 0.0,
            field_12_1_5_69594_003: 0,
            spell_procs_per_minute_id: 65535,
        },
        SpellProcsPerMinuteModRecord {
            id: 1,
            r#type: 0,
            param: 0,
            coeff: 0.0,
            field_12_1_5_69594_003: 0,
            spell_procs_per_minute_id: 65535,
        },
        SpellProcsPerMinuteModRecord {
            id: 3,
            r#type: 0,
            param: 0,
            coeff: 0.0,
            field_12_1_5_69594_003: 0,
            spell_procs_per_minute_id: 0,
        },
    ];
    let raw = catalog(rows);
    let result = SpellLoadPlan::build(raw.clone()).unwrap();
    let seed = result.get(1, 0).unwrap().constructor_seed();
    let fields = seed.fields();
    assert_eq!(fields.attributes, [i32::MIN; 17].map(|word| word as u32));
    assert_eq!(
        fields.speed.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        fields.launch_delay.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        fields.min_duration.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(fields.school_mask, u32::from(u8::MAX));
    assert_eq!(fields.icon_file_data_id, i32::MIN as u32);
    assert_eq!(fields.active_icon_file_data_id, i32::MIN as u32);
    assert_eq!(fields.content_tuning_id, i32::MIN as u32);
    assert_eq!(
        fields.show_future_spell_player_condition_id,
        i32::MIN as u32
    );
    assert_eq!(fields.min_scaling_level, u32::MAX);
    assert_eq!(fields.max_scaling_level, u32::MAX);
    assert_eq!(fields.proc_flags, [i32::MIN; 2].map(|word| word as u32));
    assert_eq!(fields.proc_chance, u32::from(u8::MAX));
    assert_eq!(fields.proc_charges, i32::MIN as u32);
    assert_eq!(fields.proc_cooldown, i32::MIN as u32);
    assert_eq!(fields.stack_amount, u32::from(u16::MAX));
    assert_eq!(fields.caster_aura_state, i32::MIN as u32);
    assert_eq!(fields.target_aura_state, i32::MIN as u32);
    assert_eq!(fields.exclude_caster_aura_state, i32::MIN as u32);
    assert_eq!(fields.exclude_target_aura_state, i32::MIN as u32);
    assert_eq!(fields.caster_aura_spell, i32::MIN as u32);
    assert_eq!(fields.target_aura_spell, i32::MIN as u32);
    assert_eq!(fields.exclude_caster_aura_spell, i32::MIN as u32);
    assert_eq!(fields.exclude_target_aura_spell, i32::MIN as u32);
    assert_eq!(fields.caster_aura_type, i16::MIN as u32);
    assert_eq!(fields.target_aura_type, i16::MIN as u32);
    assert_eq!(fields.exclude_caster_aura_type, i16::MIN as u32);
    assert_eq!(fields.exclude_target_aura_type, i16::MIN as u32);
    assert_eq!(fields.requires_spell_focus, u32::from(u16::MAX));
    assert_eq!(fields.facing_caster_flags, i32::MIN as u32);
    assert_eq!(fields.required_areas_id, i32::from(u16::MAX));
    assert_eq!(fields.category_id, i16::MIN as u32);
    assert_eq!(fields.dispel, i8::MIN as u32);
    assert_eq!(fields.mechanic, i8::MIN as u32);
    assert_eq!(fields.start_recovery_category, i16::MIN as u32);
    assert_eq!(fields.damage_class, i8::MIN as u32);
    assert_eq!(fields.prevention_type, i32::MIN as u32);
    assert_eq!(fields.charge_category_id, i16::MIN as u32);
    assert_eq!(fields.spell_family_name, i32::MIN as u32);
    assert_eq!(fields.spell_family_flags, [u32::MAX; 4]);
    assert_eq!(fields.recovery_time, i32::MIN as u32);
    assert_eq!(fields.category_recovery_time, i32::MIN as u32);
    assert_eq!(fields.start_recovery_time, i32::MIN as u32);
    assert_eq!(fields.cooldown_aura_spell_id, i32::MIN as u32);
    assert_eq!(fields.equipped_item_class, i32::MIN);
    assert_eq!(fields.equipped_item_subclass_mask, i32::MIN);
    assert_eq!(fields.equipped_item_inventory_type_mask, i32::MIN);
    assert_eq!(fields.interrupt_flags, i32::MIN as u32);
    assert_eq!(
        fields.aura_interrupt_flags,
        [i32::MIN; 2].map(|word| word as u32)
    );
    assert_eq!(
        fields.channel_interrupt_flags,
        [i32::MIN; 2].map(|word| word as u32)
    );
    assert_eq!(fields.max_level, i16::MIN as u32);
    assert_eq!(fields.base_level, i32::MIN as u32);
    assert_eq!(fields.spell_level, i32::MIN as u32);
    assert_eq!(fields.reagents, [i32::MIN; 8]);
    assert_eq!(fields.reagent_counts, [i16::MIN; 8]);
    assert_eq!(fields.stances, 0x8000_0000_8000_0000_u64);
    assert_eq!(fields.stances_not, 0x8000_0000_8000_0000_u64);
    assert_eq!(
        fields.cone_angle.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        fields.width.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(fields.targets, i32::MIN as u32);
    assert_eq!(fields.target_creature_type, i16::MIN as u32);
    assert_eq!(fields.max_affected_targets, u32::from(u8::MAX));
    assert_eq!(fields.max_target_level, u32::MAX);
    assert_eq!(fields.totem_category, [u16::MAX; 2]);
    assert_eq!(fields.totems, [i32::MIN; 2]);
    assert_eq!(fields.proc_base_ppm.to_bits(), 0x7FC0_1234);
    assert_eq!(seed.spell_id(), 1);
    assert_eq!(seed.difficulty(), 0);
    assert!(std::ptr::eq(seed.name(), raw.spell_name(1).unwrap()));
    assert!(std::ptr::eq(
        seed.cast_time().unwrap(),
        raw.spell_cast_times(65535).unwrap()
    ));
    assert!(std::ptr::eq(
        seed.duration().unwrap(),
        raw.spell_duration(65535).unwrap()
    ));
    assert!(std::ptr::eq(
        seed.range().unwrap(),
        raw.spell_range(65535).unwrap()
    ));
    assert_eq!(
        seed.ppm_modifiers()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [1, 2]
    );
}

#[test]
fn constructor_keeps_sparse_blank_effect_gaps_and_all_effect_values() {
    let mut rows = named(&[1]);
    let mut row = effect(1, 1, 0, 2);
    row.effect_aura = 664_i16;
    row.effect = 360_u32;
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
    row.scaling_class = i32::MIN;
    row.target_node_graph = i32::MIN;
    row.effect_misc_value = [i32::MIN; 2];
    row.effect_radius_index = [0_u32, 99];
    row.effect_spell_class_mask = [u32::MAX; 4];
    row.implicit_target = [152_i16; 2];
    rows.spell_effects = vec![row];
    rows.spell_radii = vec![SpellRadiusRecord {
        id: 0,
        radius: 1.0,
        radius_per_level: 2.0,
        radius_min: 3.0,
        radius_max: 4.0,
    }];
    let raw = catalog(rows);
    let result = SpellLoadPlan::build(raw.clone()).unwrap();
    let seed = result.get(1, 0).unwrap().constructor_seed();
    assert_eq!(seed.effects().len(), 3);
    for (index, effect) in seed.effects()[..2].iter().enumerate() {
        assert_eq!(effect.index, index as u32);
        assert_eq!(effect.effect, 0);
        assert_eq!(effect.aura, 0);
        assert_eq!(effect.aura_period, 0);
        assert_eq!(effect.base_points, 0.0);
        assert_eq!(effect.real_points_per_level, 0.0);
        assert_eq!(effect.points_per_resource, 0.0);
        assert_eq!(effect.amplitude, 0.0);
        assert_eq!(effect.chain_amplitude, 0.0);
        assert_eq!(effect.bonus_coefficient, 0.0);
        assert_eq!(effect.misc_values, [0; 2]);
        assert_eq!(effect.mechanic, 0);
        assert_eq!(effect.position_facing, 0.0);
        assert_eq!(effect.implicit_targets, [0; 2]);
        assert_eq!(effect.chain_targets, 0);
        assert_eq!(effect.item_type, 0);
        assert_eq!(effect.trigger_spell, 0);
        assert_eq!(effect.class_mask, [0; 4]);
        assert_eq!(effect.bonus_coefficient_from_ap, 0.0);
        assert_eq!(effect.scaling_class, 0);
        assert_eq!(effect.scaling_coefficient, 0.0);
        assert_eq!(effect.scaling_variance, 0.0);
        assert_eq!(effect.scaling_resource_coefficient, 0.0);
        assert_eq!(effect.attributes, 0);
        assert!(effect.radii.iter().all(Option::is_none));
    }
    let effect = &seed.effects()[2];
    assert_eq!(effect.index, 2);
    assert_eq!(effect.effect, (360_u32));
    assert_eq!(effect.aura, (664_i16) as u32);
    assert_eq!(effect.aura_period, i32::MIN as u32);
    assert_eq!(
        effect.base_points.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        effect.real_points_per_level.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        effect.points_per_resource.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        effect.amplitude.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        effect.chain_amplitude.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        effect.bonus_coefficient.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(effect.misc_values, [i32::MIN; 2]);
    assert_eq!(effect.mechanic, i32::MIN as u32);
    assert_eq!(
        effect.position_facing.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        effect.implicit_targets,
        [152_i16; 2].map(|target| target as u32)
    );
    assert_eq!(effect.chain_targets, i32::MIN);
    assert_eq!(effect.item_type, i32::MIN as u32);
    assert_eq!(effect.trigger_spell, i32::MIN as u32);
    assert_eq!(effect.class_mask, [u32::MAX; 4]);
    assert_eq!(
        effect.bonus_coefficient_from_ap.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(effect.scaling_class, i32::MIN);
    assert_eq!(
        effect.scaling_coefficient.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        effect.scaling_variance.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(
        effect.scaling_resource_coefficient.to_bits(),
        (f32::from_bits(0x7FC0_1234)).to_bits()
    );
    assert_eq!(effect.attributes, i32::MIN as u32);
    assert!(std::ptr::eq(
        effect.radii[0].unwrap(),
        raw.spell_radius(0).unwrap()
    ));
    assert!(effect.radii[1].is_none());
}

#[test]
fn constructor_missing_dependencies_are_null_and_missing_misc_does_not_lookup_zero() {
    let mut rows = named(&[1, 2]);
    let mut misc = spell_misc(1, 1);
    misc.casting_time_index = 7;
    misc.duration_index = 8;
    misc.range_index = 9;
    rows.spell_misc = vec![misc];
    rows.spell_labels = vec![SpellLabelRecord {
        id: 1,
        label_id: 1,
        spell_id: 2,
    }];
    rows.spell_cast_times = vec![SpellCastTimesRecord {
        id: 0,
        base: 100,
        minimum: 0,
    }];
    let mut options = spell_aura_options(1, 1);
    options.spell_procs_per_minute_id = 7;
    rows.spell_aura_options = vec![options];
    rows.spell_procs_per_minute_mods = vec![SpellProcsPerMinuteModRecord {
        id: 1,
        r#type: 0,
        param: 0,
        coeff: 1.0,
        field_12_1_5_69594_003: 0,
        spell_procs_per_minute_id: 7,
    }];
    let result = plan(rows);
    for id in [1, 2] {
        let seed = result.get(id, 0).unwrap().constructor_seed();
        assert!(seed.effects().is_empty());
        assert!(seed.cast_time().is_none());
        assert!(seed.duration().is_none());
        assert!(seed.range().is_none());
        assert!(seed.ppm_modifiers().is_empty());
        assert_eq!(seed.fields().proc_base_ppm, 0.0);
        assert_eq!(seed.fields().equipped_item_class, -1);
        assert_eq!(seed.fields().required_areas_id, -1);
        assert_eq!(seed.fields().attributes, [0; 17]);
        assert_eq!(seed.fields().reagents, [0; 8]);
        assert!(seed.powers().iter().all(Option::is_none));
        assert!(seed.visuals().is_empty());
        assert!(seed.reagent_currencies().is_empty());
        assert!(seed.empower_thresholds_ms().is_empty());
    }
}

#[test]
fn constructor_vectors_use_selected_sources_and_labels_deduplicate_only_after_join() {
    let mut rows = named(&[1]);
    rows.spell_labels = vec![
        SpellLabelRecord {
            id: 1,
            spell_id: 1,
            label_id: u32::MAX,
        },
        SpellLabelRecord {
            id: 2,
            spell_id: 1,
            label_id: u32::MAX,
        },
    ];
    rows.spell_empowers = vec![SpellEmpowerRecord {
        id: 1,
        spell_id: 1,
        unused1000: 0,
    }];
    rows.spell_empower_stages = vec![SpellEmpowerStageRecord {
        id: 1,
        stage: 1,
        duration_ms: -1,
        spell_empower_id: 1,
    }];
    rows.spell_powers = vec![spell_power(1, 1)];
    rows.spell_reagents_currencies = vec![SpellReagentsCurrencyRecord {
        id: 1,
        spell_id: 1,
        currency_types_id: 1,
        currency_count: -1,
        override_recraft_currency_count: 0,
        order_source: 0,
    }];
    rows.spell_x_spell_visuals = vec![visual(1, 0, 0, 0)];
    let raw = catalog(rows);
    let result = SpellLoadPlan::build(raw.clone()).unwrap();
    let input = result.get(1, 0).unwrap();
    assert_eq!(input.labels().count(), 2);
    let seed = input.constructor_seed();
    assert_eq!(
        seed.labels().iter().copied().collect::<Vec<_>>(),
        [u32::MAX]
    );
    assert_eq!(seed.empower_thresholds_ms(), [-1]);
    assert!(std::ptr::eq(
        seed.powers()[0].unwrap(),
        raw.spell_power(1).unwrap()
    ));
    assert!(std::ptr::eq(
        seed.reagent_currencies()[0],
        raw.spell_reagents_currency(1).unwrap()
    ));
    assert!(std::ptr::eq(
        seed.visuals()[0],
        raw.spell_x_spell_visual(1).unwrap()
    ));
}
