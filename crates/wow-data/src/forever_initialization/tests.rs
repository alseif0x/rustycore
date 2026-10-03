use super::*;

fn identity_class(id: u32) -> ClassRecord {
    ClassRecord {
        id,
        flags: -1,
        starting_level: -2,
        cinematic: 3,
        default_spec: 4,
        strength_bonus: 5,
        primary_stat_priority: -6,
        display_power: -7,
        ranged_attack_per_agility: 8,
        attack_per_agility: 9,
        attack_per_strength: 10,
        spell_class_set: 11,
    }
}
fn identity_race(id: u32) -> RaceRecord {
    RaceRecord {
        id,
        flags: 0,
        faction: -1,
        cinematic: -2,
        resurrection_sickness_spell: -3,
        starting_level: -4,
        base_language: -5,
        creature_type: 6,
        alliance: -7,
        neutral_race: -8,
    }
}

#[test]
fn final_class_race_records_include_overlay_then_removal_not_baseline_presence() {
    let mut official_class = identity_class(1);
    official_class.display_power = 3;
    let mut custom_class = official_class;
    custom_class.display_power = 1;
    let mut official_race = identity_race(1);
    official_race.faction = 123;
    let mut custom_race = official_race;
    custom_race.flags = 0x0008_0000;
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (CLASS_HASH, 2, 2),
        (RACE_HASH, 2, 2),
    ]);
    let catalog = InitializationRecords {
        classes: vec![identity_class(1), identity_class(2)],
        races: vec![identity_race(1), identity_race(2)],
        ..Default::default()
    }
    .finish(
        InitializationRecords {
            classes: vec![official_class],
            races: vec![official_race],
            ..Default::default()
        },
        InitializationRecords {
            classes: vec![custom_class],
            races: vec![custom_race],
            ..Default::default()
        },
        &removals,
    )
    .unwrap();
    assert_eq!(catalog.identity_counts(), [1, 1]);
    assert_eq!(catalog.class(1).unwrap().display_power, 1);
    assert_eq!(catalog.race(1).unwrap().faction, 123);
    assert!(catalog.race(1).unwrap().is_allied());
    assert!(catalog.class(2).is_none() && catalog.race(2).is_none());
    assert_eq!(catalog.race(1).unwrap().neutral_race, -8);
}

#[test]
fn identity_batches_reject_duplicate_ids_and_class_byte_overflow() {
    for rows in [
        InitializationRecords {
            classes: vec![identity_class(1), identity_class(1)],
            ..Default::default()
        },
        InitializationRecords {
            races: vec![identity_race(1), identity_race(1)],
            ..Default::default()
        },
        InitializationRecords {
            classes: vec![identity_class(256)],
            ..Default::default()
        },
    ] {
        assert!(
            rows.finish(Default::default(), Default::default(), &Default::default())
                .is_err()
        );
    }
    assert!(!identity_race(1).is_allied());
}

fn map(id: u32, instance_type: i8) -> MapRecord {
    MapRecord {
        id,
        instance_type,
        expansion: 0,
        parent_map: -1,
        flags: [-1, 2, i32::MIN],
    }
}
fn power(id: u32, power_type: i8) -> PowerRecord {
    PowerRecord {
        id,
        power_type,
        min_power: -7,
        max_base_power: 100,
        center_power: 0,
        default_power: 50,
        display_modifier: 1,
        regen_interrupt_ms: 2000,
        regen_peace: 1.5,
        regen_combat: 0.5,
        flags: -1,
    }
}
fn spec(id: u32, class: u8, order_index: i8) -> SpecializationRecord {
    SpecializationRecord {
        id,
        class,
        order_index,
        pet_talent_type: -1,
        role: 2,
        flags: 0,
        primary_stat_priority: -1,
        mastery_spells: [-1, 7],
    }
}
fn finish(rows: InitializationRecords) -> InitializationCatalog {
    rows.finish(Default::default(), Default::default(), &Default::default())
        .unwrap()
}

#[test]
fn official_custom_then_final_removal_precede_all_derived_indexes() {
    let baseline = InitializationRecords {
        maps: vec![map(0, 0)],
        powers: vec![power(1, 0)],
        specializations: vec![spec(7, 1, 4)],
        unknown_map_records: 8,
        ..Default::default()
    };
    let official = InitializationRecords {
        maps: vec![map(0, 1)],
        powers: vec![power(1, 1)],
        specializations: vec![spec(7, 2, 4)],
        ..Default::default()
    };
    let custom = InitializationRecords {
        maps: vec![map(0, 0)],
        powers: vec![power(1, 3)],
        specializations: vec![spec(7, 3, 0)],
        ..Default::default()
    };
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (MAP_HASH, 0, 2),
        (MAP_HASH, 0, 1),
        (POWER_HASH, 1, 2),
    ]);
    let catalog = baseline.finish(official, custom, &removals).unwrap();
    assert!(!catalog.map(0).unwrap().instanceable());
    assert_eq!(catalog.map(0).unwrap().flags, [-1, 2, i32::MIN]);
    assert_eq!(catalog.map(0).unwrap().parent_map, -1);
    for kind in [0, 1, 3] {
        assert!(catalog.power(kind).is_none());
    }
    assert!(catalog.default_specialization(1).is_none());
    assert!(catalog.default_specialization(2).is_none());
    assert_eq!(catalog.default_specialization(3).map(|r| r.id), Some(7));
    assert_eq!(catalog.counts(), [1, 0, 1, 0, 0, 8]);
    assert!(catalog.map(999).is_none()); // never fabricate an unknown baseline map
}

#[test]
fn duplicate_power_type_retains_first_ascending_id_and_unknown_enums_are_skipped() {
    let catalog = finish(InitializationRecords {
        powers: vec![power(10, 0), power(4, 0), power(3, -2), power(1, 26)],
        ..Default::default()
    });
    assert_eq!(catalog.power(0).map(|r| r.id), Some(4));
    assert_eq!(catalog.power(0).map(|r| r.min_power), Some(-7));
    assert!(catalog.power(-2).is_none());
    assert!(catalog.power(26).is_none());
    assert_eq!(catalog.counts()[1], 4); // underlying store retains skipped enum rows
}

#[test]
fn initial_specialization_precedes_classic_order_fallback_not_lowest_id_or_recommended() {
    let catalog = finish(InitializationRecords {
        specializations: vec![spec(20, 1, 2), spec(90, 1, 0), spec(95, 1, 0)],
        ..Default::default()
    });
    assert_eq!(catalog.default_specialization(1).map(|r| r.id), Some(95));
    let rows = InitializationRecords {
        specializations: vec![spec(95, 1, 0), spec(100, 1, 4)],
        ..Default::default()
    };
    assert_eq!(
        finish(rows).default_specialization(1).map(|r| r.id),
        Some(100)
    );
    let rows = InitializationRecords {
        specializations: vec![spec(95, 1, 0), spec(100, 1, 4)],
        ..Default::default()
    };
    let removed =
        Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(SPECIALIZATION_HASH, 100, 2)]);
    assert_eq!(
        rows.finish(Default::default(), Default::default(), &removed)
            .unwrap()
            .default_specialization(1)
            .map(|r| r.id),
        Some(95)
    );
}

#[test]
fn pet_override_is_not_a_player_default_and_unsafe_indexes_fail_closed() {
    let mut pet = spec(1, 0, 0);
    pet.flags = PET_OVERRIDE_SPEC;
    let catalog = finish(InitializationRecords {
        specializations: vec![pet],
        ..Default::default()
    });
    assert!(catalog.default_specialization(0).is_none());
    assert!(catalog.default_specialization(16).is_none());
    for invalid in [spec(1, 16, 0), spec(1, 1, -1), spec(1, 1, 5)] {
        assert!(
            InitializationRecords {
                specializations: vec![invalid],
                ..Default::default()
            }
            .finish(Default::default(), Default::default(), &Default::default())
            .is_err()
        );
    }
    pet.class = 1;
    assert!(
        InitializationRecords {
            specializations: vec![pet],
            ..Default::default()
        }
        .finish(Default::default(), Default::default(), &Default::default())
        .is_err()
    );
}

#[test]
fn repeated_ids_are_rejected_in_every_table_and_each_startup_batch() {
    for table in 0..5 {
        for batch in 0..3 {
            let mut duplicate = InitializationRecords::default();
            match table {
                0 => duplicate.maps = vec![map(1, 0); 2],
                1 => duplicate.powers = vec![power(1, 0); 2],
                2 => duplicate.specializations = vec![spec(1, 1, 0); 2],
                3 => {
                    duplicate.class_powers = vec![
                        ClassPowerRecord {
                            id: 1,
                            class: 1,
                            power_type: 0
                        };
                        2
                    ]
                }
                _ => duplicate.movies = vec![movie(1); 2],
            }
            let result = match batch {
                0 => duplicate.finish(Default::default(), Default::default(), &Default::default()),
                1 => InitializationRecords::default().finish(
                    duplicate,
                    Default::default(),
                    &Default::default(),
                ),
                _ => InitializationRecords::default().finish(
                    Default::default(),
                    duplicate,
                    &Default::default(),
                ),
            };
            assert!(result.is_err());
        }
    }
}

fn movie(id: u32) -> MovieRecord {
    MovieRecord {
        id,
        volume: 255,
        key_id: 0,
        audio_file: 7,
        subtitle_file: 8,
        subtitle_format: 9,
    }
}

#[test]
fn class_powers_sort_by_class_and_type_not_record_id_and_deduplicate_pairs() {
    let rows = InitializationRecords {
        class_powers: vec![
            ClassPowerRecord {
                id: 1,
                class: 1,
                power_type: 3,
            },
            ClassPowerRecord {
                id: 99,
                class: 1,
                power_type: 0,
            },
            ClassPowerRecord {
                id: 5,
                class: 1,
                power_type: 0,
            },
            ClassPowerRecord {
                id: 6,
                class: 2,
                power_type: 1,
            },
        ],
        ..Default::default()
    };
    let catalog = finish(rows);
    assert_eq!(catalog.class_power_types(1).collect::<Vec<_>>(), [0, 3]);
    assert_eq!(catalog.class_power_index(1, 0), Some(0));
    assert_eq!(catalog.class_power_index(1, 3), Some(1));
    assert_eq!(catalog.class_power_index(1, 1), None);
    assert_eq!(catalog.class_power_types(2).collect::<Vec<_>>(), [1]);
    assert_eq!(catalog.class_power_types(16).count(), 0);
    assert_eq!(catalog.class_power_ids[&1], [5, 1]);
    assert_eq!(catalog.counts()[3], 4); // original effective rows remain owned
    assert!(catalog.power(0).is_none()); // relation doesn't manufacture PowerType
}

#[test]
fn class_power_overlays_and_removals_precede_pair_sorting_and_movie_presence() {
    let baseline = InitializationRecords {
        class_powers: vec![ClassPowerRecord {
            id: 1,
            class: 1,
            power_type: 3,
        }],
        movies: vec![movie(1)],
        ..Default::default()
    };
    let official = InitializationRecords {
        class_powers: vec![ClassPowerRecord {
            id: 1,
            class: 2,
            power_type: 0,
        }],
        movies: vec![movie(2)],
        ..Default::default()
    };
    let custom = InitializationRecords {
        class_powers: vec![ClassPowerRecord {
            id: 1,
            class: 3,
            power_type: 1,
        }],
        movies: vec![MovieRecord {
            key_id: 255,
            ..movie(2)
        }],
        ..Default::default()
    };
    let removed = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(MOVIE_HASH, 1, 2)]);
    let catalog = baseline.finish(official, custom, &removed).unwrap();
    assert_eq!(catalog.class_power_types(1).count(), 0);
    assert_eq!(catalog.class_power_types(2).count(), 0);
    assert_eq!(catalog.class_power_types(3).collect::<Vec<_>>(), [1]);
    assert!(catalog.movie(1).is_none());
    assert_eq!(catalog.movie(2).unwrap().key_id, 255);
    assert_eq!(catalog.movie(2).unwrap().subtitle_format, 9);
}

#[test]
fn class_power_invalid_signed_indexes_and_more_than_ten_unique_powers_fail_closed() {
    for invalid in [
        ClassPowerRecord {
            id: 1,
            class: 16,
            power_type: 0,
        },
        ClassPowerRecord {
            id: 1,
            class: u32::MAX,
            power_type: 0,
        },
        ClassPowerRecord {
            id: 1,
            class: 1,
            power_type: -1,
        },
        ClassPowerRecord {
            id: 1,
            class: 1,
            power_type: 26,
        },
    ] {
        assert!(
            InitializationRecords {
                class_powers: vec![invalid],
                ..Default::default()
            }
            .finish(Default::default(), Default::default(), &Default::default())
            .is_err()
        );
    }
    let make = |count: u32| InitializationRecords {
        class_powers: (0..count)
            .map(|index| ClassPowerRecord {
                id: index + 1,
                class: 1,
                power_type: index as i8,
            })
            .collect(),
        ..Default::default()
    };
    assert_eq!(finish(make(10)).class_power_types(1).count(), 10);
    assert!(
        make(11)
            .finish(Default::default(), Default::default(), &Default::default())
            .is_err()
    );
}
