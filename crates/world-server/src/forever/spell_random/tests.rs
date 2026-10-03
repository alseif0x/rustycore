//! ABI/composition tests; exact source draw bits require the separate oracle.
use super::*;

unsafe extern "C" {
    fn rustycore_forever_spell_select_seeded(
        seed: u32,
        weights: *const f64,
        count: usize,
        selections: *mut u32,
        draws: usize,
    ) -> i32;
    fn rustycore_forever_spell_random_seeded(
        seed: u32,
        words: *const u32,
        words_length: usize,
        minima: *const f32,
        maxima: *const f32,
        values: *mut f32,
        count: usize,
    ) -> i32;
}

#[test]
fn weighted_and_uniform_capability_checks_source_branch_inputs_without_keeping_qa_state() {
    for weights in [
        vec![0.0, 0.0],
        vec![-3.0, 0.0],
        vec![f64::NAN, 1.0],
        vec![0.2, 0.8],
        vec![0.0, 1.0],
    ] {
        let mut first = [0; 1024];
        let mut second = [0; 1024];
        for output in [&mut first, &mut second] {
            assert_eq!(
                unsafe {
                    rustycore_forever_spell_select_seeded(
                        1234,
                        weights.as_ptr(),
                        weights.len(),
                        output.as_mut_ptr(),
                        output.len(),
                    )
                },
                0
            );
        }
        assert_eq!(first, second);
        assert!(first.iter().all(|&i| i < 2));
        for _ in 0..16 {
            assert!(select(&weights).unwrap() < 2);
        }
    }
    for weights in [vec![], vec![f64::INFINITY, 1.0], vec![-1.0, 3.0]] {
        let mut sentinel = [99; 2];
        assert_eq!(
            unsafe {
                rustycore_forever_spell_select_seeded(
                    1,
                    weights.as_ptr(),
                    weights.len(),
                    sentinel.as_mut_ptr(),
                    sentinel.len(),
                )
            },
            1
        );
        assert_eq!(sentinel, [99; 2]);
        assert_eq!(
            select(&weights),
            Err(SpellDiminishingError::InvalidSelection)
        );
    }
}

fn seeded(seed: u32, words: &[u32], ranges: &[(f32, f32)]) -> Vec<u32> {
    let minima: Vec<_> = ranges.iter().map(|range| range.0).collect();
    let maxima: Vec<_> = ranges.iter().map(|range| range.1).collect();
    let mut values = vec![0.0; ranges.len()];
    // Every array owns exactly the declared number of initialized elements;
    // C++ only reads inputs, writes values, and retains no pointer/state.
    let code = unsafe {
        rustycore_forever_spell_random_seeded(
            seed,
            words.as_ptr(),
            words.len(),
            minima.as_ptr(),
            maxima.as_ptr(),
            values.as_mut_ptr(),
            values.len(),
        )
    };
    assert_eq!(code, 0);
    values
        .into_iter()
        .zip(ranges)
        .map(|(value, &(lo, hi))| {
            assert!(value.is_finite() && value >= lo && value <= hi);
            value.to_bits()
        })
        .collect()
}

#[test]
fn seeded_qa_batches_are_repeatable_and_do_not_keep_a_shared_state() {
    let choices = [
        (-0.5, 0.5),
        (0.0, 0.0),
        (-8.0, -1.0),
        (1.0, 8.0),
        (-f32::from_bits(1), f32::from_bits(1)),
        (4.0, 4.0),
    ];
    let ranges: Vec<_> = (0..2048)
        .map(|index| choices[index % choices.len()])
        .collect();
    let words: Vec<_> = (0..624u32)
        .map(|value| value.wrapping_mul(0x9e37_79b9))
        .collect();
    for seed_words in [&[][..], &[7][..], &words[..]] {
        let first = seeded(1234, seed_words, &ranges);
        assert_eq!(first, seeded(1234, seed_words, &ranges));
        // Supplying an explicit array ignores scalar seed, matching SFMT API.
        if seed_words.is_empty() {
            assert_ne!(first, seeded(1235, seed_words, &ranges));
        } else {
            assert_eq!(first, seeded(1235, seed_words, &ranges));
        }
    }
}

#[test]
fn invalid_native_batch_never_writes_a_partial_result_or_crosses_the_exception_boundary() {
    for invalid in [(1.0, -1.0), (f32::NAN, 1.0), (0.0, f32::INFINITY)] {
        let minima = [-1.0, invalid.0];
        let maxima = [1.0, invalid.1];
        let mut values = [123.5, -99.0];
        let before = values;
        let code = unsafe {
            rustycore_forever_spell_random_seeded(
                1,
                std::ptr::null(),
                0,
                minima.as_ptr(),
                maxima.as_ptr(),
                values.as_mut_ptr(),
                values.len(),
            )
        };
        assert_eq!(code, 1);
        assert_eq!(values, before);
    }
    assert_eq!(
        unsafe {
            rustycore_forever_spell_random_seeded(
                1,
                std::ptr::null(),
                1,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
            )
        },
        1
    );
}

#[test]
fn production_draw_admits_finite_and_degenerate_ranges_but_rejects_invalid_inputs() {
    for range in [
        (-0.5, 0.5),
        (-8.0, -1.0),
        (1.0, 8.0),
        (4.0, 4.0),
        (-0.0, 0.0),
    ] {
        for _ in 0..64 {
            let value = draw(range.0, range.1).unwrap();
            assert!(value >= range.0 && value <= range.1);
        }
    }
    for range in [(1.0, -1.0), (f32::NAN, 0.0), (0.0, f32::INFINITY)] {
        assert_eq!(
            draw(range.0, range.1),
            Err(SpellValueError::InvalidVarianceRange)
        );
    }
}

#[test]
fn production_constructor_replay_and_learning_use_the_same_native_capability() {
    use std::sync::Arc;
    use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};
    use wow_world::forever::spells::SpellLoadPlan;
    let rows = SpellRecords {
        spell_names: vec![SpellNameRecord {
            id: 900_001,
            name: SpellText::default(),
        }],
        spell_class_options: vec![SpellClassOptionsRecord {
            id: 1,
            spell_id: 900_001,
            modal_next_spell: 0,
            spell_class_set: 6,
            spell_class_mask: [0, 0, 0x20, 0],
        }],
        spell_x_spell_visuals: [52021, 52019]
            .into_iter()
            .enumerate()
            .map(|(index, visual)| SpellXSpellVisualRecord {
                id: index as u32 + 1,
                difficulty_id: 0,
                spell_visual_id: visual,
                probability: 1.0,
                flags: 0,
                priority: 0,
                spell_icon_file_id: 0,
                active_icon_file_id: 0,
                viewer_unit_condition_id: 0,
                viewer_player_condition_id: 0,
                caster_unit_condition_id: 0,
                caster_player_condition_id: 0,
                spell_id: 900_001,
            })
            .collect(),
        spell_effects: vec![SpellEffectRecord {
            id: 1,
            effect_aura: 0,
            difficulty_id: 0,
            effect_index: 0,
            effect: 63,
            effect_amplitude: 0.0,
            effect_attributes: 0,
            effect_aura_period: 0,
            effect_bonus_coefficient: 0.0,
            effect_chain_amplitude: 0.0,
            effect_chain_targets: 0,
            effect_item_type: 0,
            effect_mechanic: 0,
            effect_points_per_resource: 0.0,
            effect_pos_facing: 0.0,
            effect_real_points_per_level: 0.0,
            effect_trigger_spell: 0,
            bonus_coefficient_from_ap: 0.0,
            pvp_multiplier: 0.0,
            coefficient: 0.0,
            variance: 0.5,
            resource_coefficient: 0.0,
            group_size_base_points_coefficient: 0.0,
            effect_base_points: 12.5,
            scaling_class: 0,
            target_node_graph: 0,
            effect_misc_value: [0; 2],
            effect_radius_index: [0; 2],
            effect_spell_class_mask: [0; 4],
            implicit_target: [6, 0],
            spell_id: 900_001,
        }],
        ..Default::default()
    };
    let removals = Db2HotfixRemovalStoreLikeCpp::default();
    let catalog = rows
        .finish(
            Default::default(),
            Default::default(),
            6,
            Default::default(),
            Default::default(),
            &removals,
        )
        .unwrap();
    let definitions = SpellLoadPlan::build(Arc::new(catalog))
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    let items = wow_data::forever_birth::item_records::ItemRecords::default()
        .finish(Default::default(), Default::default(), &removals)
        .unwrap();
    let mut sample = None;
    let value = definitions
        .calculate_startup_value(900_001, 0, 0, &items, Some(120.25), &mut |lo, hi| {
            assert_eq!((lo, hi), (-0.25, 0.25));
            let value = draw(lo, hi)?;
            sample = Some(value);
            Ok(value)
        })
        .unwrap()
        .unwrap();
    assert_eq!(value.variance, sample);
    assert_eq!(value.value, 120.25 + 12.5 * f64::from(sample.unwrap()));
    assert_eq!(value.as_int().unwrap(), value.value.trunc() as i32);
    let birth = wow_data::forever_birth::BirthRecords::default()
        .finish(Default::default(), Default::default(), &removals)
        .unwrap();
    let columns = (0..24)
        .map(|i| format!("C{i}"))
        .collect::<Vec<_>>()
        .join("\t");
    let values = ["1"; 24].join("\t");
    let tables = wow_data::forever_game_tables::SpellValueGameTables::parse_strs(
        &format!("id\t{columns}\n1\t{values}\n"),
        "id\tA\tB\tC\tD\n1\t1\t1\t1\t1\n",
        "id\tA\tB\tC\tD\n1\t1\t1\t1\t1\n",
    )
    .unwrap();
    let corrected =
        super::super::spell_traversal::correct(definitions.with_id_corrections().unwrap())
            .unwrap()
            .with_skill_line_abilities(Arc::new(birth))
            .unwrap()
            .with_sql_custom_attributes(vec![])
            .unwrap()
            .with_value_game_tables(Arc::new(tables))
            .unwrap();
    let raw = corrected.raw_catalog();
    let mut draws = 0;
    let complete = corrected
        .with_custom_attributes(&items, &mut |lo, hi| {
            draws += 1;
            assert_eq!((lo, hi), (-0.25, 0.25));
            draw(lo, hi)
        })
        .unwrap();
    assert_eq!(draws, 2); // Binary calculation, then root positivity.
    let counts = complete.custom_attribute_counts().unwrap();
    assert_eq!(
        (
            counts.definitions,
            counts.effect_slots,
            counts.binary_assignments,
            counts.explicit_masks_initialized
        ),
        (1, 1, 1, 1)
    );
    assert!(Arc::ptr_eq(&raw, &complete.raw_catalog()));
    assert_eq!(
        complete.get_exact(900_001, 0).unwrap().custom_attributes() & 0x100000,
        0x100000
    );
    assert!(complete.get_exact(900_001, 0).unwrap().negative_effects()[0]);
    let mut selections = 0;
    let complete = complete
        .with_diminishing_info(&mut |weights| {
            assert_eq!(weights, [1.0, 1.0]);
            selections += 1;
            select(weights)
        })
        .unwrap();
    let counts = complete.diminishing_counts().unwrap();
    assert_eq!(counts.definitions, 1);
    assert!((1..=2).contains(&counts.visual_queries));
    assert_eq!(counts.selections, selections);
    assert_eq!(counts.visual_queries, selections);
    assert!(Arc::ptr_eq(&raw, &complete.raw_catalog()));
    assert_eq!(
        complete
            .get_exact(900_001, 0)
            .unwrap()
            .diminishing_info()
            .duration_limit_ms,
        8000
    );
    let complete = complete.with_immunity_info(vec![]).unwrap();
    assert_eq!(complete.immunity_counts().unwrap().definitions, 1);
    assert_eq!(complete.immunity_counts().unwrap().effects_with_info, 0);
    assert_eq!(
        complete
            .get_exact(900_001, 0)
            .unwrap()
            .allowed_mechanic_mask(),
        0
    );
    assert!(Arc::ptr_eq(&raw, &complete.raw_catalog()));
    let complete = complete.with_target_caps(&items).unwrap();
    let caps = complete.target_cap_counts().unwrap();
    assert_eq!(
        (
            caps.patch_groups,
            caps.requested_spells,
            caps.missing_spells,
            caps.applications
        ),
        (25, 35, 35, 0)
    );
    assert!(Arc::ptr_eq(&raw, &complete.raw_catalog()));
    let complete = complete.with_spell_ranks().unwrap();
    assert_eq!(complete.spell_rank_counts().unwrap().nodes, 0);
    let complete = complete.with_spell_required(vec![]).unwrap();
    assert_eq!(complete.required_spell_counts().unwrap().relations, 0);
    let complete = complete
        .with_learn_skills(&items, &mut |_, _| {
            panic!("Threat is not a Skill effect; no speculative native draw")
        })
        .unwrap();
    assert_eq!(
        complete.learn_skill_counts().unwrap().regular_definitions,
        1
    );
    assert_eq!(complete.learn_skill_counts().unwrap().nodes, 0);
    let complete = complete.with_specific_and_aura_state().unwrap();
    assert_eq!(complete.specific_counts().unwrap().definitions, 1);
    assert_eq!(
        complete.get_exact(900_001, 0).unwrap().aura_state(),
        wow_world::forever::spells::SpellAuraState::None
    );
    let complete = complete
        .with_learn_spells(vec![wow_persistence::forever::spells::SpellLearnRow {
            source: 900_001,
            learned: 900_001,
            active: false,
        }])
        .unwrap();
    let learned = complete.spell_learn_nodes(900_001).unwrap().next().unwrap();
    assert!(!learned.active && !learned.auto_learned);
    assert!(std::ptr::eq(
        learned,
        complete.spell_learned_by(900_001).unwrap().next().unwrap()
    ));
    assert_eq!(complete.learn_spell_counts().unwrap().nodes, 1);
    assert!(Arc::ptr_eq(&raw, &complete.raw_catalog()));
    let specs = wow_data::forever_birth::item_specs::ItemSpecRecords::default()
        .finish(Default::default(), Default::default(), &removals)
        .unwrap();
    let initialization = wow_data::forever_initialization::InitializationRecords::default()
        .finish(Default::default(), Default::default(), &removals)
        .unwrap();
    let templates = wow_world::forever::creation::NumericItemTemplates::load(
        Arc::new(items),
        &specs,
        &initialization,
        vec![],
    )
    .unwrap();
    assert!(complete.spell_is_valid(900_001, 0, &templates).unwrap());
    assert!(!complete.spell_is_valid(900_002, 0, &templates).unwrap());
    // Real constructor/native composition with an explicitly empty synthetic
    // canonical UnitCondition/ability stores and synthetic World learning SQL,
    // not Player/client/durability acceptance.
}
