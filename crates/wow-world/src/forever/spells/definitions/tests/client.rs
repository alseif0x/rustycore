use super::*;
#[test]
fn client_materialization_moves_every_constructor_value_and_keeps_canonical_dependencies() {
    let raw = catalog(client_rows());
    let expected_plan = SpellLoadPlan::build(raw.clone()).unwrap();
    let expected = expected_plan.get(1, 0).unwrap().constructor_seed();
    let actual = SpellLoadPlan::build(raw.clone())
        .unwrap()
        .with_server_spells(ServerSpellRows::default())
        .unwrap();
    let definition = actual.get_exact(1, 0).unwrap();
    assert!(!definition.is_server_defined());
    assert_eq!(definition.spell_id(), 1);
    assert_eq!(definition.difficulty(), 0);
    assert_eq!(definition.fields().attributes, expected.fields().attributes);
    assert_eq!(
        definition.fields().speed.to_bits(),
        expected.fields().speed.to_bits()
    );
    assert_eq!(
        definition.fields().launch_delay.to_bits(),
        expected.fields().launch_delay.to_bits()
    );
    assert_eq!(
        definition.fields().min_duration.to_bits(),
        expected.fields().min_duration.to_bits()
    );
    assert_eq!(
        definition.fields().school_mask,
        expected.fields().school_mask
    );
    assert_eq!(
        definition.fields().icon_file_data_id,
        expected.fields().icon_file_data_id
    );
    assert_eq!(
        definition.fields().active_icon_file_data_id,
        expected.fields().active_icon_file_data_id
    );
    assert_eq!(
        definition.fields().content_tuning_id,
        expected.fields().content_tuning_id
    );
    assert_eq!(
        definition.fields().show_future_spell_player_condition_id,
        expected.fields().show_future_spell_player_condition_id
    );
    assert_eq!(
        definition.fields().min_scaling_level,
        expected.fields().min_scaling_level
    );
    assert_eq!(
        definition.fields().max_scaling_level,
        expected.fields().max_scaling_level
    );
    assert_eq!(definition.fields().proc_flags, expected.fields().proc_flags);
    assert_eq!(
        definition.fields().proc_chance,
        expected.fields().proc_chance
    );
    assert_eq!(
        definition.fields().proc_charges,
        expected.fields().proc_charges
    );
    assert_eq!(
        definition.fields().proc_cooldown,
        expected.fields().proc_cooldown
    );
    assert_eq!(
        definition.fields().stack_amount,
        expected.fields().stack_amount
    );
    assert_eq!(
        definition.fields().caster_aura_state,
        expected.fields().caster_aura_state
    );
    assert_eq!(
        definition.fields().target_aura_state,
        expected.fields().target_aura_state
    );
    assert_eq!(
        definition.fields().exclude_caster_aura_state,
        expected.fields().exclude_caster_aura_state
    );
    assert_eq!(
        definition.fields().exclude_target_aura_state,
        expected.fields().exclude_target_aura_state
    );
    assert_eq!(
        definition.fields().caster_aura_spell,
        expected.fields().caster_aura_spell
    );
    assert_eq!(
        definition.fields().target_aura_spell,
        expected.fields().target_aura_spell
    );
    assert_eq!(
        definition.fields().exclude_caster_aura_spell,
        expected.fields().exclude_caster_aura_spell
    );
    assert_eq!(
        definition.fields().exclude_target_aura_spell,
        expected.fields().exclude_target_aura_spell
    );
    assert_eq!(
        definition.fields().caster_aura_type,
        expected.fields().caster_aura_type
    );
    assert_eq!(
        definition.fields().target_aura_type,
        expected.fields().target_aura_type
    );
    assert_eq!(
        definition.fields().exclude_caster_aura_type,
        expected.fields().exclude_caster_aura_type
    );
    assert_eq!(
        definition.fields().exclude_target_aura_type,
        expected.fields().exclude_target_aura_type
    );
    assert_eq!(
        definition.fields().requires_spell_focus,
        expected.fields().requires_spell_focus
    );
    assert_eq!(
        definition.fields().facing_caster_flags,
        expected.fields().facing_caster_flags
    );
    assert_eq!(
        definition.fields().required_areas_id,
        expected.fields().required_areas_id
    );
    assert_eq!(
        definition.fields().category_id,
        expected.fields().category_id
    );
    assert_eq!(definition.fields().dispel, expected.fields().dispel);
    assert_eq!(definition.fields().mechanic, expected.fields().mechanic);
    assert_eq!(
        definition.fields().start_recovery_category,
        expected.fields().start_recovery_category
    );
    assert_eq!(
        definition.fields().damage_class,
        expected.fields().damage_class
    );
    assert_eq!(
        definition.fields().prevention_type,
        expected.fields().prevention_type
    );
    assert_eq!(
        definition.fields().charge_category_id,
        expected.fields().charge_category_id
    );
    assert_eq!(
        definition.fields().spell_family_name,
        expected.fields().spell_family_name
    );
    assert_eq!(
        definition.fields().spell_family_flags,
        expected.fields().spell_family_flags
    );
    assert_eq!(
        definition.fields().recovery_time,
        expected.fields().recovery_time
    );
    assert_eq!(
        definition.fields().category_recovery_time,
        expected.fields().category_recovery_time
    );
    assert_eq!(
        definition.fields().start_recovery_time,
        expected.fields().start_recovery_time
    );
    assert_eq!(
        definition.fields().cooldown_aura_spell_id,
        expected.fields().cooldown_aura_spell_id
    );
    assert_eq!(
        definition.fields().equipped_item_class,
        expected.fields().equipped_item_class
    );
    assert_eq!(
        definition.fields().equipped_item_subclass_mask,
        expected.fields().equipped_item_subclass_mask
    );
    assert_eq!(
        definition.fields().equipped_item_inventory_type_mask,
        expected.fields().equipped_item_inventory_type_mask
    );
    assert_eq!(
        definition.fields().interrupt_flags,
        expected.fields().interrupt_flags
    );
    assert_eq!(
        definition.fields().aura_interrupt_flags,
        expected.fields().aura_interrupt_flags
    );
    assert_eq!(
        definition.fields().channel_interrupt_flags,
        expected.fields().channel_interrupt_flags
    );
    assert_eq!(definition.fields().max_level, expected.fields().max_level);
    assert_eq!(definition.fields().base_level, expected.fields().base_level);
    assert_eq!(
        definition.fields().spell_level,
        expected.fields().spell_level
    );
    assert_eq!(definition.fields().reagents, expected.fields().reagents);
    assert_eq!(
        definition.fields().reagent_counts,
        expected.fields().reagent_counts
    );
    assert_eq!(definition.fields().stances, expected.fields().stances);
    assert_eq!(
        definition.fields().stances_not,
        expected.fields().stances_not
    );
    assert_eq!(
        definition.fields().cone_angle.to_bits(),
        expected.fields().cone_angle.to_bits()
    );
    assert_eq!(
        definition.fields().width.to_bits(),
        expected.fields().width.to_bits()
    );
    assert_eq!(definition.fields().targets, expected.fields().targets);
    assert_eq!(
        definition.fields().target_creature_type,
        expected.fields().target_creature_type
    );
    assert_eq!(
        definition.fields().max_affected_targets,
        expected.fields().max_affected_targets
    );
    assert_eq!(
        definition.fields().max_target_level,
        expected.fields().max_target_level
    );
    assert_eq!(
        definition.fields().totem_category,
        expected.fields().totem_category
    );
    assert_eq!(definition.fields().totems, expected.fields().totems);
    assert_eq!(
        definition.fields().proc_base_ppm.to_bits(),
        expected.fields().proc_base_ppm.to_bits()
    );
    assert_eq!(definition.name_bytes(6), Some(&[0xFF][..]));
    assert_eq!(definition.name_bytes(0), None);
    assert_eq!(definition.name_bytes(12), None);
    assert_eq!(definition.effect_count(), 3);
    for (index, expected) in expected.effects().iter().enumerate() {
        let effect = definition.effect(index).unwrap();
        let values = effect.values();
        assert_eq!(values.index, expected.index);
        assert_eq!(values.effect, expected.effect);
        assert_eq!(values.aura, expected.aura);
        assert_eq!(values.aura_period, expected.aura_period);
        assert_eq!(values.base_points.to_bits(), expected.base_points.to_bits());
        assert_eq!(
            values.real_points_per_level.to_bits(),
            expected.real_points_per_level.to_bits()
        );
        assert_eq!(
            values.points_per_resource.to_bits(),
            expected.points_per_resource.to_bits()
        );
        assert_eq!(values.amplitude.to_bits(), expected.amplitude.to_bits());
        assert_eq!(
            values.chain_amplitude.to_bits(),
            expected.chain_amplitude.to_bits()
        );
        assert_eq!(
            values.bonus_coefficient.to_bits(),
            expected.bonus_coefficient.to_bits()
        );
        assert_eq!(values.misc_values, expected.misc_values);
        assert_eq!(values.mechanic, expected.mechanic);
        assert_eq!(
            values.position_facing.to_bits(),
            expected.position_facing.to_bits()
        );
        assert_eq!(values.implicit_targets, expected.implicit_targets);
        assert_eq!(values.chain_targets, expected.chain_targets);
        assert_eq!(values.item_type, expected.item_type);
        assert_eq!(values.trigger_spell, expected.trigger_spell);
        assert_eq!(values.class_mask, expected.class_mask);
        assert_eq!(
            values.bonus_coefficient_from_ap.to_bits(),
            expected.bonus_coefficient_from_ap.to_bits()
        );
        assert_eq!(values.scaling_class, expected.scaling_class);
        assert_eq!(
            values.scaling_coefficient.to_bits(),
            expected.scaling_coefficient.to_bits()
        );
        assert_eq!(
            values.scaling_variance.to_bits(),
            expected.scaling_variance.to_bits()
        );
        assert_eq!(
            values.scaling_resource_coefficient.to_bits(),
            expected.scaling_resource_coefficient.to_bits()
        );
        assert_eq!(values.attributes, expected.attributes);
        assert_eq!(
            values.radius_ids,
            expected.radii.map(|row| row.map(|row| row.id))
        );
    }
    assert!(definition.effect(3).is_none());
    assert_eq!(definition.effects().count(), 3);
    assert!(std::ptr::eq(
        definition.effect(2).unwrap().radii()[0].unwrap(),
        raw.spell_radius(0).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.cast_time().unwrap(),
        raw.spell_cast_times(65535).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.duration().unwrap(),
        raw.spell_duration(65535).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.range().unwrap(),
        raw.spell_range(65535).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.powers()[0].unwrap(),
        raw.spell_power(1).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.ppm_modifiers().next().unwrap(),
        raw.spell_procs_per_minute_mod(1).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.reagent_currencies().next().unwrap(),
        raw.spell_reagents_currency(1).unwrap()
    ));
    assert!(std::ptr::eq(
        definition.visuals().next().unwrap(),
        raw.spell_x_spell_visual(1).unwrap()
    ));
    assert_eq!(definition.labels(), expected.labels());
    assert_eq!(
        definition.empower_thresholds_ms(),
        expected.empower_thresholds_ms()
    );
    assert_eq!(actual.client_counts(), expected_plan.counts());
    assert_eq!(actual.server_counts(), ServerSpellCounts::default());
    assert_eq!(actual.len(), 1);
    assert!(!actual.is_empty());
    assert_eq!(actual.records().count(), 1);
}

#[test]
fn raw_catalog_and_hotfix_identity_are_not_replaced_by_internal_server_definitions() {
    let mut raw_rows = client_rows();
    raw_rows.unknown_baseline_records[0] = 3;
    let raw = catalog(raw_rows);
    let server_rows = ServerSpellRows {
        spells: vec![server_spell(7, 0)],
        effects: vec![],
    };
    let result = SpellLoadPlan::build(raw.clone())
        .unwrap()
        .with_server_spells(server_rows)
        .unwrap();
    assert_eq!(result.len(), 2);
    assert!(result.get_exact(7, 0).unwrap().is_server_defined());
    assert!(raw.spell_name(7).is_none());
    assert_eq!(raw.counts()[0], ("SpellName", 1, 3));
    assert_eq!(raw.spell_effect(1).unwrap().spell_id, 1);
}

#[test]
fn source_side_indices_survive_plan_consumption_but_server_effects_do_not_register_languages() {
    let mut raw_rows = client_rows();
    raw_rows.spell_effects[0].effect = 39;
    raw_rows.spell_effects[0].effect_misc_value[0] = -1;
    let mut summon = raw_rows.spell_effects[0];
    summon.id = 2;
    summon.effect_index = 3;
    summon.effect = 28;
    summon.effect_misc_value = [10, 1];
    raw_rows.spell_effects.push(summon);
    raw_rows.summon_properties = vec![SummonPropertiesRecord {
        id: 1,
        control: 0,
        faction: 0,
        title: 0,
        slot: 5,
        flags: [0x0020_0000, 0],
    }];
    raw_rows.battle_pet_species = vec![BattlePetSpeciesRecord {
        id: 1,
        description: SpellText::default(),
        source_text: SpellText::default(),
        creature_id: 10,
        summon_spell_id: 0,
        icon_file_data_id: 0,
        pet_type_enum: 0,
        flags: 0,
        source_type_enum: 0,
        card_ui_model_scene_id: 0,
        loadout_ui_model_scene_id: 0,
        covenant_id: 0,
    }];
    let raw = catalog(raw_rows);
    let mut effect = server_effect(7, 0);
    effect.effect = 39;
    effect.effect_misc_value[0] = 1;
    let result = SpellLoadPlan::build(raw.clone())
        .unwrap()
        .with_server_spells(ServerSpellRows {
            spells: vec![server_spell(7, 0)],
            effects: vec![effect],
        })
        .unwrap();
    assert_eq!(
        result
            .language_spell_registrations(u32::MAX)
            .collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(result.language_spell_registrations(1).count(), 0);
    assert!(std::ptr::eq(
        result.battle_pet_species_for_spell(1).unwrap(),
        raw.battle_pet_species(1).unwrap()
    ));
    assert!(result.battle_pet_species_for_spell(7).is_none());
}
