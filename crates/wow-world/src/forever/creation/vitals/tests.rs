use super::*;
use wow_data::forever_initialization::{
    ClassPowerRecord, ClassRecord, InitializationRecords, PowerRecord, RaceRecord,
};
use wow_persistence::forever::creation::{
    ClassLevelStats, CreationWorldRows, RaceStats, StartDefinition,
};

fn tables(hp: &str, mana: f32) -> InitialGameTables {
    let columns = (0..15)
        .map(|i| format!("C{i}"))
        .collect::<Vec<_>>()
        .join("\t");
    let row = (0..15)
        .map(|_| mana.to_string())
        .collect::<Vec<_>>()
        .join("\t");
    InitialGameTables::parse_strs(&format!("L\t{columns}\n1\t{row}\n2\t{row}\n3\t{row}\n"),hp,"L\tTotal\tKill\tJunk\tStats\tDivisor\n1\t400\t0\t0\t0\t0\n2\t500\t0\t0\t0\t0\n3\t600\t0\t0\t0\t0\n").unwrap()
}
fn sources(tables: &InitialGameTables, stats: [i32; 5]) -> WorldSources {
    WorldSources::from_rows(
        CreationWorldRows {
            definitions: (1..=15)
                .map(|class| StartDefinition {
                    race: 1,
                    class,
                    map: 0,
                    position: [0.; 4],
                    npe_map: None,
                    npe_position: [None; 4],
                    npe_transport: None,
                    intro_movie: None,
                    intro_scene: None,
                    npe_intro_scene: None,
                })
                .collect(),
            race_stats: vec![RaceStats {
                race: 1,
                modifiers: [0; 5],
            }],
            class_stats: (1..=15)
                .map(|class| ClassLevelStats {
                    class,
                    level: 1,
                    stats,
                })
                .collect(),
            ..Default::default()
        },
        |id| id == 1,
        |id| (1..=15).contains(&id),
        tables,
        3,
    )
    .unwrap()
}
fn initialization_records(
    class: u8,
    display: i8,
    powers: &[(i8, i32, i32)],
) -> InitializationRecords {
    InitializationRecords {
        classes: vec![ClassRecord {
            id: u32::from(class),
            flags: 0,
            starting_level: 90,
            cinematic: 0,
            default_spec: 0,
            strength_bonus: 0,
            primary_stat_priority: 0,
            display_power: display,
            ranged_attack_per_agility: 0,
            attack_per_agility: 1,
            attack_per_strength: 1,
            spell_class_set: 0,
        }],
        races: vec![RaceRecord {
            id: 1,
            flags: 0,
            faction: 123,
            cinematic: 0,
            resurrection_sickness_spell: 0,
            starting_level: 80,
            base_language: 0,
            creature_type: 0,
            alliance: 0,
            neutral_race: 0,
        }],
        powers: powers
            .iter()
            .enumerate()
            .map(|(id, &(kind, maximum, flags))| PowerRecord {
                id: id as u32 + 1,
                power_type: kind,
                min_power: 0,
                max_base_power: maximum,
                center_power: 0,
                default_power: 77,
                display_modifier: 10,
                regen_interrupt_ms: 0,
                regen_peace: 0.,
                regen_combat: 0.,
                flags,
            })
            .collect(),
        class_powers: powers
            .iter()
            .enumerate()
            .rev()
            .map(|(id, &(kind, _, _))| ClassPowerRecord {
                id: id as u32 + 1,
                power_type: kind,
                class: u32::from(class),
            })
            .collect(),
        ..Default::default()
    }
}
fn initialization(class: u8, display: i8, powers: &[(i8, i32, i32)]) -> InitializationCatalog {
    initialization_records(class, display, powers)
        .finish(Default::default(), Default::default(), &Default::default())
        .unwrap()
}

#[test]
fn health_armor_xp_and_mana_are_source_derived_not_classic_legacy_defaults() {
    let tables = tables("L\tHealth\n1\t2.75\n", 109.9);
    let sources = sources(&tables, [20, 13, 21, 12, 9]);
    let result = sources
        .pre_equipment_vitals(
            1,
            1,
            1,
            &initialization(1, 1, &[(1, 1000, 0), (0, 9999, 0)]),
            &tables,
        )
        .unwrap();
    assert_eq!(result.stats(), &[20, 13, 21, 12, 9]);
    assert_eq!(result.armor(), 26);
    assert_eq!(result.full_health(), 57); // uint32 truncation of f32 21*2.75
    assert_eq!(result.base_mana(), 109); // GT, not PowerType MaxBasePower
    assert_eq!((result.experience(), result.next_level_xp()), (0, 400));
    assert_eq!(result.display_power(), 1);
    assert_eq!(
        result
            .powers()
            .iter()
            .map(|p| (p.kind(), p.index(), p.maximum(), p.current()))
            .collect::<Vec<_>>(),
        [(0, 0, 109, 109), (1, 1, 1000, 0)]
    );
}

#[test]
fn source_health_missing_row_fallback_and_zero_maximum_clamp_are_retained() {
    let tables = tables("L\tHealth\n", 100.);
    let result = sources(&tables, [1, 2, 3, 4, 5])
        .pre_equipment_vitals(1, 1, 1, &initialization(1, 1, &[(1, 1000, 0)]), &tables)
        .unwrap();
    assert_eq!(result.full_health(), 30);
    let result = sources(&tables, [1, 2, 0, 4, 5])
        .pre_equipment_vitals(1, 1, 1, &initialization(1, 1, &[(1, 1000, 0)]), &tables)
        .unwrap();
    assert_eq!(result.full_health(), 1);
}

#[test]
fn current_power_uses_initial_login_flags_and_mana_energy_focus_not_default_power() {
    let tables = tables("L\tHealth\n1\t10\n", 100.);
    let result = sources(&tables, [20; 5])
        .pre_equipment_vitals(
            1,
            4,
            1,
            &initialization(
                4,
                3,
                &[(4, 5, 0), (3, 100, 0x20), (2, 90, 0), (7, 3, 0x2000)],
            ),
            &tables,
        )
        .unwrap();
    assert_eq!(
        result
            .powers()
            .iter()
            .map(|p| (p.kind(), p.current()))
            .collect::<Vec<_>>(),
        [(2, 90), (3, 100), (4, 0), (7, 3)]
    );
    // DefaultPower is 77, UnitsUseDefaultPowerOnInit was set on Energy;
    // neither overrides the Create phase's full energy value.
}

#[test]
fn power_initialization_retains_source_negative_max_and_runic_zero() {
    let tables = tables("L\tHealth\n1\t10\n", 100.);
    let result = sources(&tables, [20; 5])
        .pre_equipment_vitals(
            1,
            6,
            1,
            &initialization(6, 6, &[(5, 6, 0), (6, 1000, 0), (7, -2, 0)]),
            &tables,
        )
        .unwrap();
    assert_eq!(
        result
            .powers()
            .iter()
            .map(|p| (p.kind(), p.current()))
            .collect::<Vec<_>>(),
        [(5, 0), (6, 0), (7, -2)]
    );
    let result = sources(&tables, [20; 5])
        .pre_equipment_vitals(
            1,
            6,
            1,
            &initialization(6, 5, &[(5, 8, 0), (6, 1000, 0)]),
            &tables,
        )
        .unwrap();
    assert_eq!(result.powers()[0].current(), 6); // MAX_RUNES before InitStats
}

#[test]
fn corrupt_numerical_values_and_missing_effective_identity_or_power_fail_closed() {
    let tables = tables("L\tHealth\n1\t10\n", 100.);
    let initialization = initialization(1, 1, &[(1, 1000, 0)]);
    for stats in [
        [1, 2, -1, 4, 5],
        [1, i32::MAX, 3, 4, 5],
        [1, 2, i32::MAX, 4, 5],
    ] {
        assert!(
            sources(&tables, stats)
                .pre_equipment_vitals(1, 1, 1, &initialization, &tables)
                .is_err()
        );
    }
    assert!(matches!(
        sources(&tables, [20; 5]).pre_equipment_vitals(2, 1, 1, &initialization, &tables),
        Err(SourceError::MissingRace)
    ));
    assert!(matches!(
        sources(&tables, [20; 5]).pre_equipment_vitals(1, 2, 1, &initialization, &tables),
        Err(SourceError::MissingClass)
    ));
    let removals = wow_data::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(
        wow_data::forever_initialization::POWER_HASH,
        1,
        2,
    )]);
    let missing_power = initialization_records(1, 1, &[(1, 1000, 0)])
        .finish(Default::default(), Default::default(), &removals)
        .unwrap();
    assert!(matches!(
        sources(&tables, [20; 5]).pre_equipment_vitals(1, 1, 1, &missing_power, &tables),
        Err(SourceError::MissingPowerType)
    ));
}

#[test]
fn missing_class_power_mapping_and_source_xp_wrap_are_explicit() {
    let tables = tables("L\tHealth\n1\t10\n", 100.);
    let sources = sources(&tables, [20; 5]);
    let initialization = initialization(1, 1, &[(1, 1000, 0)]);
    let result = sources
        .pre_equipment_vitals(1, 1, 4, &initialization, &tables)
        .unwrap();
    assert_eq!((result.experience(), result.next_level_xp()), (u32::MAX, 0));
    let mut rows = initialization_records(1, 1, &[(1, 1000, 0)]);
    rows.class_powers.clear();
    let missing_index = rows
        .finish(Default::default(), Default::default(), &Default::default())
        .unwrap();
    assert!(matches!(
        sources.pre_equipment_vitals(1, 1, 1, &missing_index, &tables),
        Err(SourceError::MissingDisplayPowerIndex)
    ));
}

#[test]
fn mana_conversion_and_negative_displayed_rune_guard_undefined_source_values() {
    let large_mana = tables("L\tHealth\n1\t10\n", 3_000_000_000.);
    let source = sources(&large_mana, [20; 5]);
    assert!(matches!(
        source.pre_equipment_vitals(1, 1, 1, &initialization(1, 0, &[(0, 100, 0)]), &large_mana),
        Err(SourceError::InvalidGameTableValue)
    ));
    let tables = tables("L\tHealth\n1\t10\n", 100.);
    assert!(matches!(
        sources(&tables, [20; 5]).pre_equipment_vitals(
            1,
            6,
            1,
            &initialization(6, 5, &[(5, -1, 0)]),
            &tables
        ),
        Err(SourceError::InvalidGameTableValue)
    ));
}
