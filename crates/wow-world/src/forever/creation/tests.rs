use super::*;

fn definition(race: u8, class: u8) -> StartDefinition {
    StartDefinition {
        race,
        class,
        map: 0,
        position: [1., 2., 3., 4.],
        npe_map: Some(12),
        npe_position: [Some(1.), None, Some(3.), Some(4.)],
        npe_transport: Some(27),
        intro_movie: Some(3),
        intro_scene: None,
        npe_intro_scene: Some(8),
    }
}

fn rows() -> CreationWorldRows {
    CreationWorldRows {
        definitions: vec![definition(1, 1)],
        race_stats: vec![RaceStats {
            race: 1,
            modifiers: [3, -1, 2, 4, 0],
        }],
        class_stats: vec![ClassLevelStats {
            class: 1,
            level: 1,
            stats: [70_000, 11, -12, 13, 14],
        }],
        ..Default::default()
    }
}

fn load(rows: CreationWorldRows) -> Result<WorldSources, SourceError> {
    let tables = wow_data::forever_game_tables::InitialGameTables::parse_strs(
        "L\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\n",
        "L\tHealth\n", "L\tTotal\tKill\tJunk\tStats\tDivisor\n1\t400\t0\t0\t0\t0\n2\t500\t0\t0\t0\t0\n3\t600\t0\t0\t0\t0\n",
    ).unwrap();
    WorldSources::from_rows(rows, |id| id == 1, |id| id == 1, &tables, 3)
}

#[test]
fn signed_target_stats_are_not_narrowed_to_the_legacy_uint16_projection() {
    let sources = load(rows()).unwrap();
    assert_eq!(
        sources.primary_stats(1, 1, 1),
        Ok([70_003, 10, -10, 17, 14])
    );
    assert_eq!(
        sources.primary_stats(1, 1, 3),
        sources.primary_stats(1, 1, 1)
    );
    assert_eq!(sources.counts(), [1, 0, 0, 0, 0, 1, 1, 0, 0]);
}

#[test]
fn composed_world_sources_retain_skill_tiers_without_fabricating_missing_rows() {
    let mut input = rows();
    let mut values = [0; 16];
    values[0] = 70000;
    input
        .skill_tiers
        .push(wow_persistence::forever::creation::SkillTierRow { id: 8, values });
    let sources = load(input).unwrap();
    assert_eq!(sources.skill_tier_value(8, 15), Some(70000));
    assert_eq!(sources.skill_tier_value(9, 0), None);
    assert_eq!(sources.counts()[8], 1);
}

#[test]
fn gap_fallback_uses_combined_strength_not_raw_class_strength() {
    let mut rows = rows();
    rows.class_stats.push(ClassLevelStats {
        class: 1,
        level: 2,
        stats: [-3, 50, 50, 50, 50],
    });
    rows.class_stats.push(ClassLevelStats {
        class: 1,
        level: 3,
        stats: [0, 60, 60, 60, 60],
    });
    let sources = load(rows).unwrap();
    assert_eq!(
        sources.primary_stats(1, 1, 2),
        Ok([70_003, 10, -10, 17, 14])
    );
    assert_eq!(sources.primary_stats(1, 1, 3), Ok([3, 59, 62, 64, 60]));
}

#[test]
fn class_rows_above_cap_are_ignored_before_source_level_extrapolation() {
    let mut input = rows();
    input.class_stats.push(ClassLevelStats {
        class: 1,
        level: 4,
        stats: [999_999; 5],
    });
    let sources = load(input).unwrap();
    assert_eq!(sources.counts()[6], 1);
    assert_eq!(sources.primary_stats(1, 1, 4), Ok([70_005, 10, -8, 17, 14]));
}

#[test]
fn unknown_race_modifiers_do_not_become_zero() {
    let mut rows = rows();
    rows.race_stats[0].race = 2;
    let sources = load(rows).unwrap();
    assert_eq!(
        sources.primary_stats(1, 1, 1),
        Err(SourceError::MissingRaceStats)
    );
}

#[test]
fn missing_or_zero_combined_level_one_is_not_fabricated() {
    for missing in [false, true] {
        let mut rows = rows();
        if missing {
            rows.class_stats[0].level = 2;
        } else {
            rows.class_stats[0].stats[0] = -3;
        }
        let sources = load(rows).unwrap();
        assert_eq!(
            sources.primary_stats(1, 1, 2),
            Err(SourceError::MissingLevelOne)
        );
    }
}

#[test]
fn partial_npe_and_optional_intro_sources_are_preserved_not_admitted() {
    let sources = load(rows()).unwrap();
    let definition = sources.definition(1, 1).unwrap();
    assert_eq!(definition.npe_position[1], None);
    assert_eq!(definition.npe_map, Some(12));
    assert_eq!(definition.npe_transport, Some(27));
    assert_eq!(definition.intro_movie, Some(3));
    assert_eq!(definition.intro_scene, None);
    assert_eq!(definition.npe_intro_scene, Some(8));
    assert!(sources.definition(2, 1).is_none());
    assert_eq!(
        sources.primary_stats(2, 1, 1),
        Err(SourceError::MissingDefinition)
    );
}

#[test]
fn required_batches_duplicates_bad_levels_and_overflow_fail_closed() {
    for required in 0..3 {
        let mut empty = rows();
        match required {
            0 => empty.definitions.clear(),
            1 => empty.race_stats.clear(),
            _ => empty.class_stats.clear(),
        }
        assert!(matches!(load(empty), Err(SourceError::EmptyRequiredSource)));
    }
    let mut duplicate = rows();
    duplicate.definitions.push(definition(1, 1));
    assert!(matches!(
        load(duplicate),
        Err(SourceError::DuplicateIdentity)
    ));
    let mut duplicate = rows();
    duplicate.race_stats.push(duplicate.race_stats[0]);
    assert!(matches!(
        load(duplicate),
        Err(SourceError::DuplicateIdentity)
    ));
    let mut duplicate = rows();
    duplicate.class_stats.push(duplicate.class_stats[0]);
    assert!(matches!(
        load(duplicate),
        Err(SourceError::DuplicateIdentity)
    ));
    let mut zero = rows();
    zero.class_stats[0].level = 0;
    assert!(matches!(load(zero), Err(SourceError::InvalidLevel)));
    let mut overflow = rows();
    overflow.class_stats[0].stats[0] = i32::MAX;
    assert_eq!(
        load(overflow).unwrap().primary_stats(1, 1, 1),
        Err(SourceError::StatsOverflow)
    );
    assert_eq!(
        load(rows()).unwrap().primary_stats(1, 1, 0),
        Err(SourceError::InvalidLevel)
    );
}

#[test]
fn only_real_target_race_class_ids_enter_the_source_indexes() {
    let mut rows = rows();
    rows.definitions.push(definition(2, 1));
    rows.definitions.push(definition(1, 2));
    rows.race_stats.push(RaceStats {
        race: 2,
        modifiers: [12; 5],
    });
    rows.class_stats.push(ClassLevelStats {
        class: 2,
        level: 1,
        stats: [12; 5],
    });
    rows.actions.push(StartAction {
        race: 1,
        class: 1,
        button: 257,
        action: 19,
        kind: 256,
    });
    rows.item_overrides.push(ItemOverride {
        race: 0,
        class: 0,
        item: 1,
        amount: -1,
    });
    rows.custom_spells.push(CustomSpell {
        race_mask: 0,
        class_mask: 0,
        spell: 1,
    });
    rows.cast_spells.push(CastSpell {
        spell: rows.custom_spells[0],
        create_mode: 1,
    });
    rows.xp_overrides.push(XpOverride {
        level: 1,
        experience: 400,
    });
    let sources = load(rows).unwrap();
    assert_eq!(sources.counts(), [1, 1, 1, 1, 1, 1, 1, 1, 0]);
    assert!(sources.definition(2, 1).is_none());
    assert!(sources.definition(1, 2).is_none());
    // The source batch retains raw values until semantic validation, rather
    // than silently truncating action types/buttons or deleting removals.
    assert_eq!(sources.remaining.actions[0].button, 257);
    assert_eq!(sources.remaining.actions[0].kind, 256);
    assert_eq!(sources.remaining.items[0].amount, -1);
}

fn initialization(instance_type: Option<i8>, removed: bool) -> InitializationCatalog {
    use wow_data::forever_initialization::{InitializationRecords, MAP_HASH, MapRecord};
    let removals = wow_data::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(
        removed.then_some((MAP_HASH, 0, 2)),
    );
    InitializationRecords {
        maps: instance_type
            .into_iter()
            .map(|instance_type| MapRecord {
                id: 0,
                instance_type,
                expansion: 0,
                parent_map: -1,
                flags: [0; 3],
            })
            .collect(),
        unknown_map_records: 8,
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &removals)
    .unwrap()
}

fn appearance(genders: &[u8]) -> AppearanceCatalog {
    use wow_data::forever_appearance::{AppearanceRecords, Model, RaceModel};
    AppearanceRecords {
        models: genders
            .iter()
            .map(|&sex| Model {
                id: u32::from(sex) + 1,
                display: 100 + u32::from(sex),
            })
            .collect(),
        race_models: genders
            .iter()
            .map(|&sex| RaceModel {
                id: u32::from(sex) + 1,
                race: 1,
                model: u32::from(sex) + 1,
                sex: i32::from(sex),
            })
            .collect(),
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &Default::default())
    .unwrap()
}

#[test]
fn normal_start_admits_known_noninstance_map_and_both_models_without_admitting_npe() {
    let sources = load(rows()).unwrap();
    let start = sources
        .normal_start(1, 1, &initialization(Some(0), false), &appearance(&[0, 1]))
        .unwrap();
    assert_eq!(start.map, 0);
    assert_eq!(
        [
            start.position.x,
            start.position.y,
            start.position.z,
            start.position.orientation
        ],
        [1., 2., 3., 4.]
    );
    assert_eq!(start.gender_displays, [100, 101]);
    // This normal-location view does not convert partially populated NPE
    // fields into an admitted NPE, nor discard pending intro validation.
    let raw = sources.definition(1, 1).unwrap();
    assert_eq!(raw.npe_position[1], None);
    assert_eq!(raw.npe_transport, Some(27));
    assert_eq!(raw.intro_movie, Some(3));
}

#[test]
fn normal_start_requires_real_effective_map_both_genders_and_definition() {
    let sources = load(rows()).unwrap();
    let appearance = appearance(&[0, 1]);
    for initialization in [initialization(None, false), initialization(Some(0), true)] {
        assert!(matches!(
            sources.normal_start(1, 1, &initialization, &appearance),
            Err(SourceError::MissingMap)
        ));
    }
    for instance_type in 1..=5 {
        assert!(matches!(
            sources.normal_start(
                1,
                1,
                &initialization(Some(instance_type), false),
                &appearance
            ),
            Err(SourceError::InstanceableStart)
        ));
    }
    let maps = initialization(Some(0), false);
    for genders in [vec![], vec![0], vec![1]] {
        assert!(matches!(
            sources.normal_start(1, 1, &maps, &self::appearance(&genders)),
            Err(SourceError::MissingGenderModel)
        ));
    }
    assert!(matches!(
        sources.normal_start(2, 1, &maps, &appearance),
        Err(SourceError::MissingDefinition)
    ));
}

#[test]
fn normal_start_bounds_all_three_axes_but_only_requires_finite_orientation() {
    let maps = initialization(Some(0), false);
    let appearance = appearance(&[0, 1]);
    let limit = Position::MAP_HALFSIZE_LIKE_CPP - 0.5;
    for axis in 0..3 {
        for value in [limit, -limit] {
            let mut rows = rows();
            rows.definitions[0].position[axis] = value;
            assert!(
                load(rows)
                    .unwrap()
                    .normal_start(1, 1, &maps, &appearance)
                    .is_ok()
            );
        }
        for value in [
            limit + 1.,
            -limit - 1.,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let mut rows = rows();
            rows.definitions[0].position[axis] = value;
            assert!(matches!(
                load(rows).unwrap().normal_start(1, 1, &maps, &appearance),
                Err(SourceError::InvalidStartPosition)
            ));
        }
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut rows = rows();
        rows.definitions[0].position[3] = value;
        assert!(matches!(
            load(rows).unwrap().normal_start(1, 1, &maps, &appearance),
            Err(SourceError::InvalidStartPosition)
        ));
    }
}

#[test]
fn normal_start_keeps_source_orientation_edge_cases_not_euclidean_remainder() {
    let maps = initialization(Some(0), false);
    let appearance = appearance(&[0, 1]);
    let tau = std::f32::consts::TAU;
    for (raw, normalized) in [
        (tau, 0.),
        (-tau, tau),
        (-0., -0.),
        (-1., tau - 1.),
        (100_000., 100_000. % tau),
    ] {
        let mut rows = rows();
        rows.definitions[0].position[3] = raw;
        let start = load(rows)
            .unwrap()
            .normal_start(1, 1, &maps, &appearance)
            .unwrap();
        assert_eq!(start.position.orientation.to_bits(), normalized.to_bits());
    }
}
