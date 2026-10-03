use super::*;
use crate::forever_spells::value_input_fixtures as rows;

fn stat(id: u32, expansion: i32, level: u32, value: f32) -> ExpectedStatRecord {
    ExpectedStatRecord {
        id,
        expansion_id: expansion,
        lvl: level,
        content_set_id: -17,
        creature_health: value,
        player_health: value,
        creature_auto_attack_dps: value,
        creature_armor: value,
        player_mana: value,
        player_primary_stat: value,
        player_secondary_stat: value,
        armor_constant: value,
        creature_spell_damage: value,
    }
}
fn modifier(id: u32, value: f32) -> ExpectedStatModRecord {
    ExpectedStatModRecord {
        id,
        creature_health_mod: value,
        player_health_mod: value,
        creature_auto_attack_dps_mod: value,
        creature_armor_mod: value,
        player_mana_mod: value,
        player_primary_stat_mod: value,
        player_secondary_stat_mod: value,
        armor_constant_mod: value,
        creature_spell_damage_mod: value,
    }
}
fn relation(
    id: u32,
    modifier: i32,
    tuning: u32,
    min: i32,
    max: i32,
) -> ContentTuningXExpectedRecord {
    ContentTuningXExpectedRecord {
        id,
        expected_stat_mod_id: modifier,
        content_tuning_id: tuning,
        min_mythic_plus_season_id: min,
        max_mythic_plus_season_id: max,
    }
}
fn catalog(base: SpellRecords) -> SpellCatalog {
    base.finish(
        Default::default(),
        Default::default(),
        6,
        Default::default(),
        Default::default(),
        &Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .unwrap()
}
fn evaluate(c: &SpellCatalog, expansion: i32, tuning: u32, class: u32, season: i32) -> f32 {
    c.evaluate_expected_stat(
        ExpectedStatType::PlayerHealth,
        1,
        expansion,
        tuning,
        class,
        season,
    )
    .unwrap()
}

#[test]
fn indexes_use_final_last_ascending_id_without_content_set_filter_and_source_fallback() {
    let c = SpellRecords {
        expected_stats: vec![
            stat(9, 3, 1, 9.0),
            stat(1, 3, 1, 1.0),
            stat(7, 3, 1, 7.0),
            stat(3, -2, 1, 3.0),
        ],
        ..Default::default()
    }
    .finish(
        SpellRecords {
            expected_stats: vec![stat(7, 3, 1, 70.0)],
            ..Default::default()
        },
        SpellRecords {
            expected_stats: vec![stat(7, 3, 1, 71.0)],
            ..Default::default()
        },
        6,
        Default::default(),
        Default::default(),
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(SPELL_TABLE_HASHES[42], 9, 2)]),
    )
    .unwrap();
    assert_eq!(evaluate(&c, 3, 0, 0, 0), 71.0);
    assert_eq!(evaluate(&c, 3, 0, 1, 0), 71.0); // Missing class mod is genuine.
    assert_eq!(evaluate(&c, 4, 0, 0, 0), 3.0);
    assert_eq!(c.expected_stat_by_level[&(1, 3)], 7);
    assert!(c.expected_stat(9).is_none());
    assert_eq!(
        c.evaluate_expected_stat(ExpectedStatType::None, 1, 3, 0, 0, 0)
            .unwrap(),
        0.0
    );
    assert_eq!(
        c.evaluate_expected_stat(ExpectedStatType::None, 2, 3, 0, 0, 0)
            .unwrap(),
        1.0
    );
}

#[test]
fn all_nine_stat_fields_and_four_class_modifiers_use_source_order() {
    let mut baseline = rows::expected_stat(1);
    baseline.lvl = 1;
    baseline.expansion_id = -2;
    baseline.creature_health = 1.0;
    baseline.player_health = 2.0;
    baseline.creature_auto_attack_dps = 3.0;
    baseline.creature_armor = 4.0;
    baseline.player_mana = 5.0;
    baseline.player_primary_stat = 6.0;
    baseline.player_secondary_stat = 7.0;
    baseline.armor_constant = 8.0;
    baseline.creature_spell_damage = 9.0;
    let mut content = modifier(9, 2.0);
    content.player_health_mod = 3.0;
    content.creature_auto_attack_dps_mod = 4.0;
    content.creature_armor_mod = 5.0;
    content.player_mana_mod = 6.0;
    content.player_primary_stat_mod = 7.0;
    content.player_secondary_stat_mod = 8.0;
    content.armor_constant_mod = 9.0;
    content.creature_spell_damage_mod = 10.0;
    let c = catalog(SpellRecords {
        expected_stats: vec![baseline],
        expected_stat_mods: vec![
            modifier(1, 10.0),
            modifier(2, 20.0),
            modifier(3, 30.0),
            modifier(4, 40.0),
            content,
        ],
        content_tuning_x_expected: vec![relation(1, 9, 0, 0, 0)],
        ..Default::default()
    });
    for (kind, value) in [
        (ExpectedStatType::CreatureHealth, 1.0),
        (ExpectedStatType::PlayerHealth, 2.0),
        (ExpectedStatType::CreatureAutoAttackDps, 3.0),
        (ExpectedStatType::CreatureArmor, 4.0),
        (ExpectedStatType::PlayerMana, 5.0),
        (ExpectedStatType::PlayerPrimaryStat, 6.0),
        (ExpectedStatType::PlayerSecondaryStat, 7.0),
        (ExpectedStatType::ArmorConstant, 8.0),
        (ExpectedStatType::CreatureSpellDamage, 9.0),
    ] {
        for (class, multiplier) in [
            (0, 1.0),
            (1, 40.0),
            (2, 20.0),
            (4, 30.0),
            (8, 10.0),
            (15, 1.0),
            (u32::MAX, 1.0),
        ] {
            assert_eq!(
                c.evaluate_expected_stat(kind, 1, -2, 0, class, 0).unwrap(),
                value * (value + 1.0) * multiplier
            );
        }
    }
}

#[test]
fn seasons_use_inclusive_min_exclusive_max_and_missing_season_does_not_suppress_modifier() {
    let mut minimum = rows::mythic_plus_season(1);
    minimum.milestone_season = 5;
    let mut maximum = rows::mythic_plus_season(2);
    maximum.milestone_season = 10;
    let c = catalog(SpellRecords {
        expected_stats: vec![stat(1, -2, 1, 3.0)],
        expected_stat_mods: vec![modifier(8, 2.0), modifier(9, 5.0)],
        content_tuning_x_expected: vec![
            relation(1, 8, 0, 1, 2),
            relation(2, 9, 0, -1, -2),
            relation(3, 123, 0, 0, 0),
        ],
        mythic_plus_seasons: vec![minimum, maximum],
        ..Default::default()
    });
    assert_eq!(c.expected_mods_by_content_tuning[&0], [1, 2]);
    for (season, value) in [(4, 15.0), (5, 30.0), (9, 30.0), (10, 15.0)] {
        assert_eq!(evaluate(&c, -2, 0, 0, season), value);
    }
    assert_eq!(evaluate(&c, -2, 1, 0, 5), 3.0);
}

#[test]
fn modifier_product_finishes_before_baseline_multiplication_without_reassociation() {
    let c = catalog(SpellRecords {
        expected_stats: vec![stat(1, -2, 1, 3.1415927)],
        expected_stat_mods: vec![modifier(8, 3.1415927), modifier(9, 2.7182817)],
        content_tuning_x_expected: vec![relation(2, 9, 0, 0, 0), relation(1, 8, 0, 0, 0)],
        ..Default::default()
    });
    assert_eq!(c.expected_mods_by_content_tuning[&0], [1, 2]);
    let source = 3.1415927f32 * (3.1415927f32 * 2.7182817f32);
    let regrouped = (3.1415927f32 * 3.1415927f32) * 2.7182817f32;
    assert_ne!(source.to_bits(), regrouped.to_bits());
    assert_eq!(evaluate(&c, -2, 0, 0, 0).to_bits(), source.to_bits());
}

#[test]
fn random_properties_cover_all_inventory_bits_qualities_and_wand_subclass() {
    let mut row = rows::rand_prop_points(1);
    row.good_f = [1.0, 2.0, 3.0, 4.0, 5.0];
    row.superior_f = [11.0, 12.0, 13.0, 14.0, 15.0];
    row.epic_f = [21.0, 22.0, 23.0, 24.0, 25.0];
    let c = catalog(SpellRecords {
        rand_prop_points: vec![row],
        ..Default::default()
    });
    for inv in 0..=255u32 {
        for subclass in [0, 19, u32::MAX] {
            let index = match inv {
                1 | 4 | 5 | 7 | 15 | 17 | 20 | 25 => Some(0),
                26 => Some(if subclass == 19 { 3 } else { 0 }),
                13 | 21 | 22 => Some(3),
                3 | 6 | 8 | 10 | 12 => Some(1),
                2 | 9 | 11 | 14 | 16 | 23 => Some(2),
                28 => Some(4),
                _ => None,
            };
            for quality in 0..=8u32 {
                let expected = index.map_or(0.0, |i| match quality {
                    2 => 1.0 + i as f32,
                    3 | 7 => 11.0 + i as f32,
                    4 | 5 | 6 => 21.0 + i as f32,
                    _ => 0.0,
                });
                assert_eq!(
                    c.random_property_points(1, quality, inv, subclass).unwrap(),
                    expected
                );
            }
        }
    }
    assert_eq!(c.random_property_points(2, 3, 5, 0).unwrap(), 0.0);
    assert_eq!(c.random_property_points(1, 3, u32::MAX, 0).unwrap(), 0.0);
}

#[test]
fn unknown_baseline_is_not_complete_absence_or_a_numeric_default() {
    for index in [42, 43, 45, 47] {
        let mut rows = SpellRecords::default();
        rows.unknown_baseline_records[index] = 1;
        let c = catalog(rows);
        assert!(
            c.evaluate_expected_stat(ExpectedStatType::None, 1, -2, 0, 0, 0)
                .is_err()
        );
    }
    let mut rows = SpellRecords::default();
    rows.unknown_baseline_records[46] = 1;
    assert!(catalog(rows).random_property_points(1, 3, 5, 0).is_err());
}

#[test]
fn expected_modifier_membership_is_built_after_final_modifier_and_relation_removals() {
    let c = SpellRecords {
        expected_stats: vec![stat(1, -2, 1, 3.0)],
        expected_stat_mods: vec![modifier(8, 2.0), modifier(9, 5.0)],
        content_tuning_x_expected: vec![relation(1, 8, 0, 0, 0), relation(2, 9, 0, 0, 0)],
        ..Default::default()
    }
    .finish(
        Default::default(),
        Default::default(),
        6,
        Default::default(),
        Default::default(),
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (SPELL_TABLE_HASHES[43], 8, 2),
            (SPELL_TABLE_HASHES[45], 2, 2),
        ]),
    )
    .unwrap();
    assert!(c.expected_mods_by_content_tuning.get(&0).is_none());
    assert!(c.expected_stat_mod(8).is_none());
    assert!(c.content_tuning_x_expected(2).is_none());
    assert_eq!(evaluate(&c, -2, 0, 0, 0), 3.0);
}
