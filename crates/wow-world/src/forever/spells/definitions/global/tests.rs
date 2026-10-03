//! Synthetic prescribed traversals, not a claim of native hash-order evidence.
use super::super::traversal::OwnedInputs;
use super::*;
use crate::forever::spells::{ServerSpellCounts, SpellEffectValues, SpellLoadCounts};
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};

fn catalog(rows: SpellRecords) -> SpellCatalog {
    rows.finish(
        SpellRecords::default(),
        SpellRecords::default(),
        6,
        SpellLocaleRecords::default(),
        SpellLocaleRecords::default(),
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
    )
    .unwrap()
}
fn definition(effect: u32) -> Definition {
    let mut result = Definition::empty_server(Vec::new());
    result.effects.push(SpellEffectValues {
        effect,
        ..Default::default()
    });
    result
}
fn seeds(
    rows: SpellRecords,
    definitions: impl IntoIterator<Item = (Key, Definition)>,
) -> SpellDefinitionSeeds {
    SpellDefinitionSeeds {
        catalog: Arc::new(catalog(rows)),
        definitions: definitions.into_iter().collect(),
        languages: BTreeMap::new(),
        battle_pets_by_spell: BTreeMap::new(),
        client_counts: SpellLoadCounts::default(),
        server_counts: ServerSpellCounts::default(),
        id_corrections: None,
        global_corrections: None,
        traversal_inputs: Some(OwnedInputs::new(Vec::new(), Vec::new())),
        source_order: None,
        skill_line_abilities: None,
        sql_custom_attributes: None,
        custom_attributes: None,
        diminishing: None,
        immunities: None,
        target_caps: None,
        ranks: None,
        required: None,
        learn_skills: None,
        specific: None,
        learn_spells: None,
        value_game_tables: None,
    }
}
fn ready_input(
    rows: SpellRecords,
    definitions: impl IntoIterator<Item = (Key, Definition)>,
) -> SpellDefinitionSeeds {
    seeds(rows, definitions).with_id_corrections().unwrap()
}
fn prescribed(seeds: SpellDefinitionSeeds) -> SpellDefinitionSeeds {
    // Synthetic test order only; production must use its verified producer.
    let keys: Vec<_> = seeds.definitions.keys().copied().collect();
    seeds
        .with_global_corrections(SpellTraversal::new(keys.clone(), keys))
        .unwrap()
}
fn ranges() -> SpellRecords {
    SpellRecords {
        spell_ranges: [40.0, 20.0, 10.0]
            .into_iter()
            .enumerate()
            .map(|(i, max)| SpellRangeRecord {
                id: i as u32 + 1,
                display_name: SpellText::default(),
                display_name_short: SpellText::default(),
                flags: 0,
                range_min: [i as f32; 2],
                range_max: [max, 1000.0],
            })
            .collect(),
        ..Default::default()
    }
}
fn trajectory(range: u32, trigger: u32) -> Definition {
    let mut result = definition(3);
    result.range = Some(range);
    result.effects[0].implicit_targets = [89, 0];
    result.effects[0].trigger_spell = trigger;
    result
}

#[test]
fn global_phase_requires_id_phase_and_rejects_reapplication() {
    assert!(matches!(
        seeds(SpellRecords::default(), [])
            .with_global_corrections(SpellTraversal::new(vec![], vec![])),
        Err(SpellDefinitionError::GlobalCorrectionsRequireIdCorrections)
    ));
    let result = prescribed(ready_input(SpellRecords::default(), []));
    assert_eq!(
        result
            .global_correction_counts()
            .unwrap()
            .summon_properties_missing,
        3
    );
    assert!(result.traversal_inputs().is_none());
    assert!(matches!(
        result.with_global_corrections(SpellTraversal::new(vec![], vec![])),
        Err(SpellDefinitionError::GlobalCorrectionsAlreadyApplied)
    ));
}

#[test]
fn both_orders_require_the_exact_unique_canonical_key_set() {
    let valid = vec![(1, 0), (2, 0)];
    for (primary, secondary) in [
        (vec![(1, 0)], valid.clone()),
        (vec![(1, 0), (1, 0)], valid.clone()),
        (vec![(1, 0), (3, 0)], valid.clone()),
        (valid.clone(), vec![(1, 0)]),
        (valid.clone(), vec![(1, 0), (1, 0)]),
        (valid, vec![(1, 0), (3, 0)]),
    ] {
        let input = ready_input(
            SpellRecords::default(),
            [((1, 0), definition(0)), ((2, 0), definition(0))],
        );
        assert!(matches!(
            input.with_global_corrections(SpellTraversal::new(primary, secondary)),
            Err(SpellDefinitionError::InvalidCorrectionTraversal)
        ));
    }
}

#[test]
fn admitted_source_indexes_serve_later_passes_without_retaining_replay_histories() {
    let input = ready_input(
        SpellRecords::default(),
        [
            ((7, -1), definition(0)),
            ((7, 2), definition(0)),
            ((3, 0), definition(0)),
        ],
    );
    assert!(input.corrected_difficulties(7).is_none());
    let result = input
        .with_global_corrections(SpellTraversal::new(
            vec![(7, 2), (3, 0), (7, -1)],
            vec![(7, -1), (3, 0), (7, 2)],
        ))
        .unwrap();
    assert_eq!(
        result
            .records()
            .map(|view| (view.spell_id(), view.difficulty()))
            .collect::<Vec<_>>(),
        vec![(7, 2), (3, 0), (7, -1)]
    );
    assert_eq!(
        result
            .corrected_difficulties(7)
            .unwrap()
            .map(|view| view.difficulty())
            .collect::<Vec<_>>(),
        vec![-1, 2]
    );
    assert_eq!(result.corrected_difficulties(999).unwrap().count(), 0);
    assert!(result.traversal_inputs().is_none());
    assert_eq!(result.len(), 3);
}

#[test]
fn raw_alias_is_rejected_without_copy_on_write_or_partial_record_mutation() {
    let input = ready_input(
        SpellRecords {
            summon_properties: vec![SummonPropertiesRecord {
                id: 121,
                control: -9,
                faction: 8,
                title: 99,
                slot: 5,
                flags: [i32::MIN, -1],
            }],
            ..Default::default()
        },
        [],
    );
    let raw = input.raw_catalog();
    assert!(matches!(
        input.with_global_corrections(SpellTraversal::new(vec![], vec![])),
        Err(SpellDefinitionError::SharedCorrectionCatalog)
    ));
    assert_eq!(raw.summon_properties(121).unwrap().title, 99);
    assert_eq!(raw.summon_properties(121).unwrap().flags, [i32::MIN, -1]);
}

#[test]
fn weak_observer_also_prevents_exclusive_raw_publication_transition() {
    let input = ready_input(SpellRecords::default(), []);
    let weak = Arc::downgrade(&input.catalog);
    assert!(matches!(
        input.with_global_corrections(SpellTraversal::new(vec![], vec![])),
        Err(SpellDefinitionError::SharedCorrectionCatalog)
    ));
    assert!(weak.upgrade().is_none()); // consumed failed owner, no admitted result
}

#[test]
fn trajectory_is_one_prescribed_pass_not_an_implicit_fixed_point_or_sorted_replay() {
    let create = || {
        ready_input(
            ranges(),
            [
                ((1, 0), trajectory(1, 2)),
                ((2, 0), trajectory(2, 3)),
                ((3, 0), trajectory(3, 99)),
            ],
        )
    };
    let forward = create()
        .with_global_corrections(SpellTraversal::new(
            vec![(1, 0), (2, 0), (3, 0)],
            vec![(1, 0), (2, 0), (3, 0)],
        ))
        .unwrap();
    assert_eq!(forward.get_exact(3, 0).unwrap().range().unwrap().id, 1);
    assert_eq!(
        forward
            .global_correction_counts()
            .unwrap()
            .trajectory_range_assignments,
        2
    );
    let reverse = create()
        .with_global_corrections(SpellTraversal::new(
            vec![(3, 0), (2, 0), (1, 0)],
            vec![(3, 0), (2, 0), (1, 0)],
        ))
        .unwrap();
    assert_eq!(reverse.get_exact(2, 0).unwrap().range().unwrap().id, 1);
    assert_eq!(reverse.get_exact(3, 0).unwrap().range().unwrap().id, 2);
    assert_eq!(
        reverse
            .global_correction_counts()
            .unwrap()
            .trajectory_range_assignments,
        2
    );
}

#[test]
fn trigger_lookup_covers_all_signed_difficulties_and_missing_trigger_is_ignored() {
    let input = ready_input(
        ranges(),
        [
            ((1, 0), trajectory(1, 7)),
            ((7, -1), trajectory(3, 999)),
            ((7, 1), trajectory(2, 999)),
        ],
    );
    let result = input
        .with_global_corrections(SpellTraversal::new(
            vec![(1, 0), (7, 1), (7, -1)],
            vec![(7, -1), (1, 0), (7, 1)],
        ))
        .unwrap();
    assert_eq!(result.get_exact(7, -1).unwrap().range().unwrap().id, 1);
    assert_eq!(result.get_exact(7, 1).unwrap().range().unwrap().id, 1);
    assert_eq!(
        result
            .global_correction_counts()
            .unwrap()
            .trajectory_range_assignments,
        2
    );
}

#[test]
fn trajectory_requires_active_effect_and_strict_negative_range_comparison() {
    let mut parent = trajectory(1, 2);
    parent.effects[0].effect = 0;
    let result = prescribed(ready_input(
        ranges(),
        [((1, 0), parent), ((2, 0), trajectory(2, 999))],
    ));
    assert_eq!(result.get_exact(2, 0).unwrap().range().unwrap().id, 2);
    let mut rows = ranges();
    rows.spell_ranges[1].range_max[0] = 40.0; // equal negative max; positive max is irrelevant
    rows.spell_ranges[1].range_max[1] = 1.0;
    let result = prescribed(ready_input(
        rows,
        [((1, 0), trajectory(1, 2)), ((2, 0), trajectory(2, 999))],
    ));
    assert_eq!(result.get_exact(2, 0).unwrap().range().unwrap().id, 2); // keep its own pointer
    let mut rows = ranges();
    rows.spell_ranges[1].range_max[0] = f32::NAN;
    let result = prescribed(ready_input(
        rows,
        [((1, 0), trajectory(1, 2)), ((2, 0), trajectory(2, 999))],
    ));
    assert_eq!(result.get_exact(2, 0).unwrap().range().unwrap().id, 2);
}

#[test]
fn movement_speed_uses_all_five_kinds_and_preserves_all_three_source_guards() {
    let mut definitions = Vec::new();
    for (index, effect) in [96, 149, 41, 42, 138].into_iter().enumerate() {
        let mut source = definition(effect);
        source.fields.speed = -0.0;
        definitions.push(((index as u32 + 1, 0), source));
    }
    let mut existing = definition(96);
    existing.fields.speed = 8.0;
    let mut family = definition(96);
    family.fields.spell_family_name = 1;
    let mut delay = definition(96);
    delay.fields.attributes[9] = 0x10;
    let mut nan = definition(96);
    nan.fields.speed = f32::from_bits(0x7fc0_0019);
    definitions.extend([
        ((20, 0), existing),
        ((21, 0), family),
        ((22, 0), delay),
        ((23, 0), nan),
    ]);
    let result = prescribed(ready_input(SpellRecords::default(), definitions));
    for id in 1..=5 {
        assert_eq!(result.get_exact(id, 0).unwrap().fields().speed, 42.0);
    }
    assert_eq!(result.get_exact(20, 0).unwrap().fields().speed, 8.0);
    assert_eq!(result.get_exact(21, 0).unwrap().fields().speed, 0.0);
    assert_eq!(result.get_exact(22, 0).unwrap().fields().speed, 0.0);
    assert_eq!(
        result.get_exact(23, 0).unwrap().fields().speed.to_bits(),
        0x7fc0_0019
    );
    assert_eq!(
        result.global_correction_counts().unwrap().movement_speeds,
        5
    );
}

#[test]
fn cone_fix_precedes_area_aura_redirect_and_does_not_require_active_effect_for_cone() {
    let mut area = definition(35);
    area.effects[0].implicit_targets = [24, 0];
    let mut blank = definition(0);
    blank.effects[0].implicit_targets = [0, 128];
    let mut line = definition(35);
    line.effects[0].implicit_targets = [133, 89];
    let result = prescribed(ready_input(
        SpellRecords::default(),
        [((1, 0), area), ((2, 0), blank), ((3, 0), line)],
    ));
    assert_eq!(result.get_exact(1, 0).unwrap().fields().cone_angle, 90.0);
    assert_eq!(
        result
            .get_exact(1, 0)
            .unwrap()
            .effect(0)
            .unwrap()
            .values()
            .implicit_targets,
        [1, 0]
    );
    assert_eq!(result.get_exact(2, 0).unwrap().fields().cone_angle, 90.0);
    assert_eq!(
        result
            .get_exact(2, 0)
            .unwrap()
            .effect(0)
            .unwrap()
            .values()
            .implicit_targets,
        [0, 128]
    );
    assert_eq!(result.get_exact(3, 0).unwrap().fields().cone_angle, 0.0);
    assert_eq!(
        result
            .get_exact(3, 0)
            .unwrap()
            .effect(0)
            .unwrap()
            .values()
            .implicit_targets,
        [133, 89]
    );
}

#[test]
fn float_fuzzy_zero_retains_source_tolerance_nan_infinity_and_signed_zero() {
    for value in [0.0, -0.0, 0.00001, -0.00001, f32::EPSILON] {
        assert!(fuzzy_zero(value));
    }
    for value in [
        0.000011,
        -0.000011,
        1.0,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ] {
        assert!(!fuzzy_zero(value));
    }
}

#[test]
fn magnet_vehicle_flight_and_single_target_rules_preserve_unselected_words() {
    let mut valid = definition(6);
    valid.effects[0].aura = 96;
    valid.effects.push(SpellEffectValues {
        effect: 6,
        aura: 236,
        ..Default::default()
    });
    valid.fields.proc_flags = [u32::MAX, 2];
    valid.fields.attributes[0] = 1;
    valid.fields.attributes[5] = 0x20;
    valid.fields.attributes[16] = 0x8000_0000;
    valid.fields.active_icon_file_data_id = 135754;
    let mut dummy = definition(3);
    dummy.effects[0].aura = 96;
    dummy.fields.proc_flags = [1, 2];
    let mut bounded = definition(0);
    bounded.fields.attributes[5] = 0x20;
    bounded.fields.max_affected_targets = 7;
    let result = prescribed(ready_input(
        SpellRecords::default(),
        [
            ((1, 0), valid),
            ((2, 0), dummy),
            ((3, 0), bounded),
            ((4, 0), definition(0)),
        ],
    ));
    let view = result.get_exact(1, 0).unwrap();
    assert_eq!(view.fields().proc_flags, [0; 2]);
    assert_eq!(view.fields().attributes[0], 0x41);
    assert_eq!(view.fields().attributes[5], 0x0008_0020);
    assert_eq!(view.fields().attributes[16], 0x8000_0000);
    assert_eq!(view.fields().max_affected_targets, 1);
    assert_eq!(result.get_exact(2, 0).unwrap().fields().proc_flags, [1, 2]);
    assert_eq!(
        result
            .get_exact(3, 0)
            .unwrap()
            .fields()
            .max_affected_targets,
        7
    );
    assert_eq!(
        result
            .get_exact(4, 0)
            .unwrap()
            .fields()
            .max_affected_targets,
        0
    );
    let c = result.global_correction_counts().unwrap();
    assert_eq!(
        (
            c.magnet_proc_clears,
            c.vehicle_facing_flags,
            c.passive_flight_flags,
            c.single_target_limits
        ),
        (1, 1, 1, 1)
    );
}

#[test]
fn raw_summon_changes_are_in_place_exact_fields_and_replay_metadata_retires() {
    let mut rows = SpellRecords {
        summon_properties: [121, 647, 628]
            .map(|id| SummonPropertiesRecord {
                id,
                control: -9,
                faction: 8,
                title: 99,
                slot: 5,
                flags: [i32::MIN, -1],
            })
            .to_vec(),
        ..Default::default()
    };
    rows.unknown_baseline_records[33] = 7;
    let input = ready_input(rows, []);
    let raw_pointer =
        input.catalog.summon_properties(121).unwrap() as *const SummonPropertiesRecord;
    let counts_before = input.catalog.counts();
    assert!(input.traversal_inputs().is_some());
    let result = prescribed(input);
    assert!(result.traversal_inputs().is_none());
    for id in [121, 647] {
        let row = result.catalog.summon_properties(id).unwrap();
        assert_eq!(row.title, 4);
        assert_eq!(row.control, -9);
        assert_eq!((row.faction, row.slot, row.flags), (8, 5, [i32::MIN, -1]));
    }
    let row = result.catalog.summon_properties(628).unwrap();
    assert_eq!(row.title, 99);
    assert_eq!(row.control, 2);
    assert_eq!(
        raw_pointer,
        result.catalog.summon_properties(121).unwrap() as *const SummonPropertiesRecord
    );
    assert_eq!(counts_before, result.catalog.counts());
    let c = result.global_correction_counts().unwrap();
    assert_eq!(
        (c.summon_properties_applied, c.summon_properties_missing),
        (3, 0)
    );
}
