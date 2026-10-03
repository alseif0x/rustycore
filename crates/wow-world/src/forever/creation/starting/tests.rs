use super::*;
use wow_data::forever_initialization::{
    CLASS_HASH, ClassRecord, InitializationRecords, RACE_HASH, RaceRecord,
};
use wow_persistence::forever::creation::templates::{TemplateClassRow, TemplateRow, TemplateRows};
use wow_persistence::forever::permissions::DefaultPermissionRows;

fn config() -> StartingConfig {
    StartingConfig {
        normal_level: 1,
        death_knight_level: 8,
        demon_hunter_level: 8,
        allied_level: 10,
        gm_level: 1,
        normal_money: 11,
        death_knight_money: 22,
        demon_hunter_money: 33,
        evoker_money: 44,
        allied_money: 55,
    }
}
fn policy() -> StartingPolicy {
    build(config(), 90).unwrap()
}
fn build(config: StartingConfig, max_level: u8) -> Result<StartingPolicy, SourceError> {
    let rows = [
        (0, 0),
        (2, 2),
        (100, 100),
        (255, 255),
        (70170, 10),
        (u32::MAX, 80),
    ];
    let templates = CharacterTemplates::load(
        TemplateRows {
            classes: rows
                .iter()
                .map(|&(id, _)| TemplateClassRow {
                    template_id: id,
                    faction_group: 3,
                    class: 1,
                })
                .collect(),
            templates: rows
                .into_iter()
                .map(|(id, level)| TemplateRow {
                    id,
                    level,
                    name: "Synthetic".into(),
                    description: "Fixture".into(),
                })
                .collect(),
        },
        &initialization(),
    )
    .unwrap();
    StartingPolicy::from_config(config, max_level, Arc::new(templates))
}
fn permissions(roots: &[u32]) -> DefaultAccountPermissions {
    DefaultAccountPermissions::load(DefaultPermissionRows {
        known: vec![10, 17, 41, 195],
        links: vec![(195, 10), (195, 17), (195, 41)],
        roots: roots.to_vec(),
    })
}
fn race(id: u32, flags: i32) -> RaceRecord {
    RaceRecord {
        id,
        flags,
        faction: 0,
        cinematic: 0,
        resurrection_sickness_spell: 0,
        starting_level: 99,
        base_language: 0,
        creature_type: 0,
        alliance: 0,
        neutral_race: 0,
    }
}
fn records() -> InitializationRecords {
    InitializationRecords {
        classes: (1..=15)
            .map(|id| ClassRecord {
                id,
                flags: 0,
                starting_level: 99,
                cinematic: 0,
                default_spec: 0,
                strength_bonus: 0,
                primary_stat_priority: 0,
                display_power: 0,
                ranged_attack_per_agility: 0,
                attack_per_agility: 0,
                attack_per_strength: 0,
                spell_class_set: 0,
            })
            .collect(),
        races: [1, 24, 25, 26, 52, 70, 95]
            .into_iter()
            .map(|id| race(id, 0))
            .collect(),
        ..Default::default()
    }
}
fn initialization() -> InitializationCatalog {
    records()
        .finish(Default::default(), Default::default(), &Default::default())
        .unwrap()
}
fn values(
    policy: &StartingPolicy,
    race: u8,
    class: u8,
    permissions: &DefaultAccountPermissions,
    template: Option<i32>,
) -> (u8, u64) {
    let values = policy
        .select(race, class, &initialization(), permissions, template)
        .unwrap();
    (values.level(), values.money())
}

#[test]
fn normal_hero_and_pandaren_rules_do_not_use_db2_starting_level() {
    let policy = policy();
    let ordinary = permissions(&[]);
    assert_eq!(values(&policy, 1, 1, &ordinary, None), (1, 11));
    assert_eq!(values(&policy, 1, 6, &ordinary, None), (8, 22));
    assert_eq!(values(&policy, 1, 12, &ordinary, None), (8, 33));
    assert_eq!(values(&policy, 1, 13, &ordinary, None), (1, 44));
    for race in [25, 26] {
        assert_eq!(values(&policy, race, 6, &ordinary, None), (10, 55));
        assert_eq!(values(&policy, race, 1, &ordinary, None), (1, 11));
    }
    assert_eq!(values(&policy, 24, 6, &ordinary, None), (8, 22));
}

#[test]
fn dracthyr_has_allied_level_but_not_allied_money_without_the_flag() {
    let policy = policy();
    let ordinary = permissions(&[]);
    for race in [52, 70] {
        assert_eq!(values(&policy, race, 1, &ordinary, None), (10, 11));
        assert_eq!(values(&policy, race, 13, &ordinary, None), (10, 44));
    }
}

#[test]
fn final_effective_allied_flag_overrides_money_and_removed_identity_is_not_a_fallback() {
    let policy = policy();
    let ordinary = permissions(&[]);
    let custom = || InitializationRecords {
        races: vec![race(95, 0x0008_0000)],
        ..Default::default()
    };
    let catalog = records()
        .finish(Default::default(), custom(), &Default::default())
        .unwrap();
    for class in [1, 6, 12, 13] {
        let result = policy.select(95, class, &catalog, &ordinary, None).unwrap();
        assert_eq!((result.level(), result.money()), (10, 55));
    }
    let removals = wow_data::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (RACE_HASH, 95, 2),
        (CLASS_HASH, 1, 2),
    ]);
    let catalog = records()
        .finish(Default::default(), custom(), &removals)
        .unwrap();
    assert!(matches!(
        policy.select(95, 6, &catalog, &ordinary, None),
        Err(SourceError::MissingRace)
    ));
    assert!(matches!(
        policy.select(1, 1, &catalog, &ordinary, None),
        Err(SourceError::MissingClass)
    ));
}

#[test]
fn gm_and_template_permissions_are_independent_and_never_lower_start_level() {
    let mut config = config();
    config.gm_level = 80;
    let policy = build(config, 30).unwrap();
    let ordinary = permissions(&[17]);
    assert_eq!(values(&policy, 1, 1, &ordinary, Some(100)), (1, 11));
    assert_eq!(values(&policy, 1, 1, &ordinary, Some(999)), (1, 11));
    let template = permissions(&[10]);
    assert_eq!(values(&policy, 1, 1, &template, Some(100)), (100, 11));
    assert_eq!(values(&policy, 1, 6, &template, Some(2)), (8, 22));
    assert_eq!(values(&policy, 1, 6, &template, Some(999)), (8, 22));
    assert_eq!(values(&policy, 1, 1, &template, Some(999)), (1, 11)); // known absent SQL template
    assert_eq!(values(&policy, 1, 1, &template, Some(70170)), (10, 11)); // ID is NOT level
    assert_eq!(values(&policy, 1, 1, &template, Some(-1)), (80, 11)); // source uint32 lookup conversion
    let gm = permissions(&[41]);
    assert_eq!(values(&policy, 1, 1, &gm, Some(100)), (80, 11));
    assert_eq!(values(&policy, 1, 1, &gm, Some(999)), (80, 11));
    let both = permissions(&[195]);
    assert_eq!(values(&policy, 1, 1, &both, Some(100)), (100, 11));
    assert_eq!(values(&policy, 1, 1, &both, Some(0)), (80, 11));
}

#[test]
fn source_config_normalization_clamps_ordinary_levels_but_not_gm_to_player_cap() {
    let mut settings = config();
    settings.normal_level = 0;
    settings.death_knight_level = u32::MAX;
    settings.demon_hunter_level = 0;
    settings.allied_level = u32::MAX;
    settings.gm_level = u32::MAX;
    settings.normal_money = u64::MAX;
    settings.death_knight_money = u64::MAX;
    settings.demon_hunter_money = u64::MAX;
    settings.evoker_money = u64::MAX;
    settings.allied_money = u64::MAX;
    let policy = build(settings, 30).unwrap();
    let ordinary = permissions(&[]);
    for (race, class, level) in [
        (1, 1, 1),
        (1, 6, 30),
        (1, 12, 1),
        (1, 13, 1),
        (52, 1, 30),
        (25, 6, 30),
    ] {
        assert_eq!(
            values(&policy, race, class, &ordinary, None),
            (level, MAX_MONEY)
        );
    }
    assert_eq!(
        values(&policy, 1, 1, &permissions(&[41]), None),
        (123, MAX_MONEY)
    );
    assert_eq!(
        values(&policy, 1, 1, &permissions(&[10]), Some(255)),
        (255, MAX_MONEY)
    );
    let mut settings = config();
    settings.normal_level = 20;
    settings.gm_level = 1;
    let policy = build(settings, 30).unwrap();
    assert_eq!(values(&policy, 1, 1, &permissions(&[41]), None), (20, 11));
    assert!(matches!(
        build(config(), 0),
        Err(SourceError::InvalidLevelCap)
    ));
    assert!(matches!(
        build(config(), 124),
        Err(SourceError::InvalidLevelCap)
    ));
}

#[test]
fn hero_level_is_a_maximum_with_the_ordinary_or_allied_start_not_a_replacement() {
    let mut config = config();
    config.normal_level = 20;
    config.allied_level = 25;
    let policy = build(config, 90).unwrap();
    let ordinary = permissions(&[]);
    assert_eq!(values(&policy, 1, 6, &ordinary, None), (20, 22));
    assert_eq!(values(&policy, 1, 12, &ordinary, None), (20, 33));
    assert_eq!(values(&policy, 25, 6, &ordinary, None), (25, 55));
    assert_eq!(values(&policy, 52, 12, &ordinary, None), (25, 33));
}
