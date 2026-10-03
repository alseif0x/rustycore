use super::*;

#[test]
fn all_scalar_families_join_by_spell_and_last_storage_id_without_cloning_rows() {
    let mut rows = named(&[1, 2]);
    rows.spell_aura_options = vec![spell_aura_options(9, 1), spell_aura_options(2, 1)];
    rows.spell_aura_restrictions =
        vec![spell_aura_restrictions(9, 1), spell_aura_restrictions(2, 1)];
    rows.spell_casting_requirements = vec![
        spell_casting_requirements(9, 1),
        spell_casting_requirements(2, 1),
    ];
    rows.spell_categories = vec![spell_categories(9, 1), spell_categories(2, 1)];
    rows.spell_class_options = vec![spell_class_options(9, 1), spell_class_options(2, 1)];
    rows.spell_cooldowns = vec![spell_cooldowns(9, 1), spell_cooldowns(2, 1)];
    rows.spell_equipped_items = vec![spell_equipped_items(9, 1), spell_equipped_items(2, 1)];
    rows.spell_interrupts = vec![spell_interrupts(9, 1), spell_interrupts(2, 1)];
    rows.spell_levels = vec![spell_levels(9, 1), spell_levels(2, 1)];
    rows.spell_misc = vec![spell_misc(9, 1), spell_misc(2, 1)];
    rows.spell_reagents = vec![spell_reagents(9, 1), spell_reagents(2, 1)];
    rows.spell_scaling = vec![spell_scaling(9, 1), spell_scaling(2, 1)];
    rows.spell_shapeshifts = vec![spell_shapeshift(9, 1), spell_shapeshift(2, 1)];
    rows.spell_target_restrictions = vec![
        spell_target_restrictions(9, 1),
        spell_target_restrictions(2, 1),
    ];
    rows.spell_totems = vec![spell_totems(9, 1), spell_totems(2, 1)];
    let raw = catalog(rows);
    let result = SpellLoadPlan::build(raw.clone()).unwrap();
    let input = result.get(1, 0).unwrap();
    assert_eq!(input.aura_options().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.aura_options().unwrap(),
        raw.spell_aura_options(9).unwrap()
    ));
    assert_eq!(input.aura_restrictions().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.aura_restrictions().unwrap(),
        raw.spell_aura_restrictions(9).unwrap()
    ));
    assert_eq!(input.casting_requirements().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.casting_requirements().unwrap(),
        raw.spell_casting_requirements(9).unwrap()
    ));
    assert_eq!(input.categories().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.categories().unwrap(),
        raw.spell_categories(9).unwrap()
    ));
    assert_eq!(input.class_options().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.class_options().unwrap(),
        raw.spell_class_options(9).unwrap()
    ));
    assert_eq!(input.cooldowns().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.cooldowns().unwrap(),
        raw.spell_cooldowns(9).unwrap()
    ));
    assert_eq!(input.equipped_items().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.equipped_items().unwrap(),
        raw.spell_equipped_items(9).unwrap()
    ));
    assert_eq!(input.interrupts().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.interrupts().unwrap(),
        raw.spell_interrupts(9).unwrap()
    ));
    assert_eq!(input.levels().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.levels().unwrap(),
        raw.spell_levels(9).unwrap()
    ));
    assert_eq!(input.misc().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.misc().unwrap(),
        raw.spell_misc(9).unwrap()
    ));
    assert_eq!(input.reagents().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.reagents().unwrap(),
        raw.spell_reagents(9).unwrap()
    ));
    assert_eq!(input.scaling().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.scaling().unwrap(),
        raw.spell_scaling(9).unwrap()
    ));
    assert_eq!(input.shapeshift().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.shapeshift().unwrap(),
        raw.spell_shapeshift(9).unwrap()
    ));
    assert_eq!(input.target_restrictions().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.target_restrictions().unwrap(),
        raw.spell_target_restrictions(9).unwrap()
    ));
    assert_eq!(input.totems().unwrap().id, 9);
    assert!(std::ptr::eq(
        input.totems().unwrap(),
        raw.spell_totems(9).unwrap()
    ));
    assert!(std::ptr::eq(input.name(), raw.spell_name(1).unwrap()));
    assert_eq!(input.spell_id(), 1);
    assert_eq!(input.difficulty(), 0);
    assert_eq!(result.records().count(), 1);
    // A name alone is not a LoadHelper and does not manufacture SpellInfo.
    assert!(result.get(2, 0).is_none());
    assert!(result.get(1, 7).is_none());
}

#[test]
fn signed_parent_spell_ids_preserve_cpp_uint32_key_conversion() {
    let mut rows = named(&[u32::MAX]);
    rows.spell_casting_requirements = vec![spell_casting_requirements(3, u32::MAX)];
    rows.spell_class_options = vec![spell_class_options(3, u32::MAX)];
    rows.spell_equipped_items = vec![spell_equipped_items(3, u32::MAX)];
    rows.spell_reagents = vec![spell_reagents(3, u32::MAX)];
    rows.spell_scaling = vec![spell_scaling(3, u32::MAX)];
    rows.spell_shapeshifts = vec![spell_shapeshift(3, u32::MAX)];
    rows.spell_totems = vec![spell_totems(3, u32::MAX)];
    let result = plan(rows);
    let input = result.get(u32::MAX, 0).unwrap();
    assert_eq!(input.casting_requirements().unwrap().spell_id, -1);
    assert_eq!(input.class_options().unwrap().spell_id, -1);
    assert_eq!(input.equipped_items().unwrap().spell_id, -1);
    assert_eq!(input.reagents().unwrap().spell_id, -1);
    assert_eq!(input.scaling().unwrap().spell_id, -1);
    assert_eq!(input.shapeshift().unwrap().spell_id, -1);
    assert_eq!(input.totems().unwrap().spell_id, -1);
    assert!(result.get(0, 0).is_none());
}

#[test]
fn fallback_fills_every_scalar_and_slot_but_preserves_own_values() {
    let mut rows = named(&[1]);
    rows.difficulties = vec![difficulty(2, 3), difficulty(3, 0)];
    rows.spell_aura_options = vec![spell_aura_options(1, 1)];
    rows.spell_aura_restrictions = vec![spell_aura_restrictions(1, 1)];
    rows.spell_casting_requirements = vec![spell_casting_requirements(1, 1)];
    rows.spell_categories = vec![spell_categories(1, 1)];
    rows.spell_class_options = vec![spell_class_options(1, 1)];
    rows.spell_cooldowns = vec![spell_cooldowns(1, 1)];
    rows.spell_equipped_items = vec![spell_equipped_items(1, 1)];
    rows.spell_interrupts = vec![spell_interrupts(1, 1)];
    rows.spell_levels = vec![spell_levels(1, 1)];
    rows.spell_misc = vec![spell_misc(1, 1)];
    rows.spell_reagents = vec![spell_reagents(1, 1)];
    rows.spell_scaling = vec![spell_scaling(1, 1)];
    rows.spell_shapeshifts = vec![spell_shapeshift(1, 1)];
    rows.spell_target_restrictions = vec![spell_target_restrictions(1, 1)];
    rows.spell_totems = vec![spell_totems(1, 1)];
    let mut own_misc = spell_misc(3, 1);
    own_misc.difficulty_id = 2;
    rows.spell_misc.push(own_misc);
    rows.spell_effects = vec![
        effect(20, 1, 2, 0),
        effect(10, 1, 0, 0),
        effect(11, 1, 0, 1),
        effect(12, 1, 3, 31),
    ];
    rows.spell_powers = vec![spell_power(1, 1), spell_power(2, 1), spell_power(4, 1)];
    rows.spell_powers[1].order_index = 2;
    // Override BOTH difficulty and index by Power.ID, not by SpellID.
    rows.spell_power_difficulties = vec![SpellPowerDifficultyRecord {
        id: 4,
        difficulty_id: 2,
        order_index: 1,
    }];
    let result = plan(rows);
    let input = result.get(1, 2).unwrap();
    assert_eq!(input.aura_options().unwrap().id, 1);
    assert_eq!(input.aura_restrictions().unwrap().id, 1);
    assert_eq!(input.casting_requirements().unwrap().id, 1);
    assert_eq!(input.categories().unwrap().id, 1);
    assert_eq!(input.class_options().unwrap().id, 1);
    assert_eq!(input.cooldowns().unwrap().id, 1);
    assert_eq!(input.equipped_items().unwrap().id, 1);
    assert_eq!(input.interrupts().unwrap().id, 1);
    assert_eq!(input.levels().unwrap().id, 1);
    assert_eq!(input.reagents().unwrap().id, 1);
    assert_eq!(input.scaling().unwrap().id, 1);
    assert_eq!(input.shapeshift().unwrap().id, 1);
    assert_eq!(input.target_restrictions().unwrap().id, 1);
    assert_eq!(input.totems().unwrap().id, 1);
    assert_eq!(input.misc().unwrap().id, 3);
    let effects = input.effect_slots();
    assert_eq!(effects[0].unwrap().id, 20);
    assert_eq!(effects[1].unwrap().id, 11);
    assert_eq!(effects[31].unwrap().id, 12);
    assert!(effects[2..31].iter().all(Option::is_none));
    let powers = input.power_slots();
    assert_eq!(powers[0].unwrap().id, 1);
    assert_eq!(powers[1].unwrap().id, 4);
    assert_eq!(powers[2].unwrap().id, 2);
    assert!(powers[3].is_none() && powers[4].is_none());
}

#[test]
fn duplicate_slots_last_id_and_vectors_keep_source_lower_bound_order() {
    let mut rows = named(&[1]);
    rows.spell_effects = vec![effect(9, 1, 0, 31), effect(2, 1, 0, 31)];
    rows.spell_powers = vec![spell_power(9, 1), spell_power(2, 1)];
    rows.spell_empowers = vec![SpellEmpowerRecord {
        id: 1,
        spell_id: 1,
        unused1000: 0,
    }];
    rows.spell_empower_stages = vec![
        SpellEmpowerStageRecord {
            id: 3,
            stage: 1,
            duration_ms: 30,
            spell_empower_id: 1,
        },
        SpellEmpowerStageRecord {
            id: 1,
            stage: 2,
            duration_ms: 10,
            spell_empower_id: 1,
        },
        SpellEmpowerStageRecord {
            id: 2,
            stage: 1,
            duration_ms: 20,
            spell_empower_id: 1,
        },
        // Missing parent never manufactures a helper.
        SpellEmpowerStageRecord {
            id: 4,
            stage: 0,
            duration_ms: 40,
            spell_empower_id: 99,
        },
    ];
    rows.spell_labels = vec![
        SpellLabelRecord {
            id: 8,
            spell_id: 1,
            label_id: 55,
        },
        SpellLabelRecord {
            id: 1,
            spell_id: 1,
            label_id: 55,
        },
    ];
    rows.spell_reagents_currencies = vec![
        SpellReagentsCurrencyRecord {
            id: 8,
            spell_id: 1,
            currency_types_id: 8,
            currency_count: 1,
            override_recraft_currency_count: 0,
            order_source: 0,
        },
        SpellReagentsCurrencyRecord {
            id: 1,
            spell_id: 1,
            currency_types_id: 1,
            currency_count: 1,
            override_recraft_currency_count: 0,
            order_source: 0,
        },
    ];
    rows.spell_x_spell_visuals = vec![
        visual(1, 0, 1, 0),
        visual(2, 0, 1, u32::MAX),
        visual(3, 0, 1, u32::MAX),
        visual(4, 0, -1, 2),
        visual(5, 0, 2, 0),
    ];
    let result = plan(rows);
    let input = result.get(1, 0).unwrap();
    assert_eq!(input.effect_slots()[31].unwrap().id, 9);
    assert_eq!(input.power_slots()[0].unwrap().id, 9);
    assert_eq!(
        input.empower_stages().map(|r| r.id).collect::<Vec<_>>(),
        [3, 2, 1]
    );
    assert_eq!(input.labels().map(|r| r.id).collect::<Vec<_>>(), [1, 8]);
    assert_eq!(
        input.reagent_currencies().map(|r| r.id).collect::<Vec<_>>(),
        [1, 8]
    );
    assert_eq!(
        input.visuals().map(|r| r.id).collect::<Vec<_>>(),
        [5, 3, 2, 1, 4]
    );
}

#[test]
fn fallback_vectors_take_first_nonempty_chain_and_missing_difficulty_stops() {
    let mut rows = named(&[1]);
    rows.difficulties = vec![difficulty(2, 3), difficulty(3, 0), difficulty(4, 3)];
    rows.spell_effects = vec![effect(1, 1, 2, 0), effect(2, 1, 4, 0), effect(3, 1, 9, 0)];
    rows.spell_x_spell_visuals = vec![visual(1, 0, 0, 0), visual(2, 3, 0, 0), visual(3, 4, 0, 0)];
    rows.spell_labels = vec![SpellLabelRecord {
        id: 1,
        spell_id: 1,
        label_id: 2,
    }];
    rows.spell_empowers = vec![SpellEmpowerRecord {
        id: 1,
        spell_id: 1,
        unused1000: 0,
    }];
    rows.spell_empower_stages = vec![SpellEmpowerStageRecord {
        id: 1,
        stage: 0,
        duration_ms: 1,
        spell_empower_id: 1,
    }];
    rows.spell_reagents_currencies = vec![SpellReagentsCurrencyRecord {
        id: 1,
        spell_id: 1,
        currency_types_id: 2,
        currency_count: 3,
        override_recraft_currency_count: 0,
        order_source: 0,
    }];
    let result = plan(rows);
    let child = result.get(1, 2).unwrap();
    assert_eq!(child.visuals().map(|r| r.id).collect::<Vec<_>>(), [2]);
    assert_eq!(child.labels().map(|r| r.id).collect::<Vec<_>>(), [1]);
    assert_eq!(
        child.empower_stages().map(|r| r.id).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        child.reagent_currencies().map(|r| r.id).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        result
            .get(1, 4)
            .unwrap()
            .visuals()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [3]
    );
    assert_eq!(result.get(1, 9).unwrap().visuals().count(), 0);
    assert_eq!(result.get(1, 9).unwrap().labels().count(), 0);
    // Fallback does not synthesize a new difficulty key.
    assert!(result.get(1, 8).is_none());
}

#[test]
fn hotfix_replacement_removal_and_unknown_coverage_precede_semantic_join() {
    let mut rows = named(&[1]);
    rows.spell_effects = vec![effect(1, 1, 0, 0), effect(2, 1, 0, 1)];
    rows.unknown_baseline_records[1] = 7;
    let official = SpellRecords {
        spell_effects: vec![effect(1, 1, 0, 5)],
        ..Default::default()
    };
    let custom = SpellRecords {
        spell_effects: vec![effect(1, 1, 0, 6)],
        ..Default::default()
    };
    let raw = Arc::new(
        rows.finish(
            official,
            custom,
            6,
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(
                SPELL_TABLE_HASHES[1],
                2,
                2,
            )]),
        )
        .unwrap(),
    );
    let result = SpellLoadPlan::build(raw.clone()).unwrap();
    let input = result.get(1, 0).unwrap();
    assert!(input.effect_slots()[0].is_none());
    assert!(input.effect_slots()[1].is_none());
    assert_eq!(input.effect_slots()[6].unwrap().id, 1);
    assert!(std::ptr::eq(
        input.effect_slots()[6].unwrap(),
        raw.spell_effect(1).unwrap()
    ));
    assert_eq!(raw.counts()[1].2, 7);
    assert_eq!(result.counts().spells_and_difficulties, 1);
}
