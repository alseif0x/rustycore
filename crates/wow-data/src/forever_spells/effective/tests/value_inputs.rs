use super::*;
use crate::forever_spells::value_input_fixtures as rows;

fn expected_stat(id: u32, marker: u32) -> ExpectedStatRecord {
    let mut row = rows::expected_stat(id);
    row.expansion_id = marker as i32;
    row
}
fn expected_stat_mod(id: u32, marker: u32) -> ExpectedStatModRecord {
    let mut row = rows::expected_stat_mod(id);
    row.creature_health_mod = f32::from_bits(marker);
    row
}
fn content_tuning(id: u32, marker: u32) -> ContentTuningRecord {
    let mut row = rows::content_tuning(id);
    row.flags = marker as i32;
    row
}
fn content_tuning_x_expected(id: u32, marker: u32) -> ContentTuningXExpectedRecord {
    let mut row = rows::content_tuning_x_expected(id);
    row.expected_stat_mod_id = marker as i32;
    row
}
fn rand_prop_points(id: u32, marker: u32) -> RandPropPointsRecord {
    let mut row = rows::rand_prop_points(id);
    row.damage_replace_stat_f = f32::from_bits(marker);
    row
}
fn mythic_plus_season(id: u32, marker: u32) -> MythicPlusSeasonRecord {
    let mut row = rows::mythic_plus_season(id);
    row.milestone_season = marker as i32;
    row
}
fn batch(id: u32, marker: u32) -> SpellRecords {
    SpellRecords {
        expected_stats: vec![expected_stat(id, marker)],
        expected_stat_mods: vec![expected_stat_mod(id, marker)],
        content_tunings: vec![content_tuning(id, marker)],
        content_tuning_x_expected: vec![content_tuning_x_expected(id, marker)],
        rand_prop_points: vec![rand_prop_points(id, marker)],
        mythic_plus_seasons: vec![mythic_plus_season(id, marker)],
        ..Default::default()
    }
}

#[test]
fn every_value_input_composes_ordered_overlays_and_borrows_canonical_rows() {
    let mut official = batch(7, 2);
    let mut custom = batch(7, 4);
    official.expected_stats.push(expected_stat(7, 3));
    custom.expected_stats.push(expected_stat(8, 5));
    official.expected_stat_mods.push(expected_stat_mod(7, 3));
    custom.expected_stat_mods.push(expected_stat_mod(8, 5));
    official.content_tunings.push(content_tuning(7, 3));
    custom.content_tunings.push(content_tuning(8, 5));
    official
        .content_tuning_x_expected
        .push(content_tuning_x_expected(7, 3));
    custom
        .content_tuning_x_expected
        .push(content_tuning_x_expected(8, 5));
    official.rand_prop_points.push(rand_prop_points(7, 3));
    custom.rand_prop_points.push(rand_prop_points(8, 5));
    official.mythic_plus_seasons.push(mythic_plus_season(7, 3));
    custom.mythic_plus_seasons.push(mythic_plus_season(8, 5));
    let result = batch(7, 1)
        .finish(
            official,
            custom,
            6,
            Default::default(),
            Default::default(),
            &removals([]),
        )
        .unwrap();
    assert_eq!(result.expected_stat(7).unwrap().expansion_id as u32, 4);
    assert_eq!(result.expected_stat(8).unwrap().expansion_id as u32, 5);
    assert_eq!(
        result
            .expected_stat_mod(7)
            .unwrap()
            .creature_health_mod
            .to_bits(),
        4
    );
    assert_eq!(
        result
            .expected_stat_mod(8)
            .unwrap()
            .creature_health_mod
            .to_bits(),
        5
    );
    assert_eq!(result.content_tuning(7).unwrap().flags as u32, 4);
    assert_eq!(result.content_tuning(8).unwrap().flags as u32, 5);
    assert_eq!(
        result
            .content_tuning_x_expected(7)
            .unwrap()
            .expected_stat_mod_id as u32,
        4
    );
    assert_eq!(
        result
            .content_tuning_x_expected(8)
            .unwrap()
            .expected_stat_mod_id as u32,
        5
    );
    assert_eq!(
        result
            .rand_prop_points(7)
            .unwrap()
            .damage_replace_stat_f
            .to_bits(),
        4
    );
    assert_eq!(
        result
            .rand_prop_points(8)
            .unwrap()
            .damage_replace_stat_f
            .to_bits(),
        5
    );
    assert_eq!(
        result.mythic_plus_season(7).unwrap().milestone_season as u32,
        4
    );
    assert_eq!(
        result.mythic_plus_season(8).unwrap().milestone_season as u32,
        5
    );
    assert_eq!(
        result
            .expected_stat_records()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [7, 8]
    );
    assert!(std::ptr::eq(
        result.expected_stat(7).unwrap(),
        result.expected_stat_records().next().unwrap()
    ));
    assert!(result.expected_stat(9).is_none());
    assert_eq!(
        result
            .expected_stat_mod_records()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [7, 8]
    );
    assert!(std::ptr::eq(
        result.expected_stat_mod(7).unwrap(),
        result.expected_stat_mod_records().next().unwrap()
    ));
    assert!(result.expected_stat_mod(9).is_none());
    assert_eq!(
        result
            .content_tuning_records()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [7, 8]
    );
    assert!(std::ptr::eq(
        result.content_tuning(7).unwrap(),
        result.content_tuning_records().next().unwrap()
    ));
    assert!(result.content_tuning(9).is_none());
    assert_eq!(
        result
            .content_tuning_x_expected_records()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [7, 8]
    );
    assert!(std::ptr::eq(
        result.content_tuning_x_expected(7).unwrap(),
        result.content_tuning_x_expected_records().next().unwrap()
    ));
    assert!(result.content_tuning_x_expected(9).is_none());
    assert_eq!(
        result
            .rand_prop_points_records()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [7, 8]
    );
    assert!(std::ptr::eq(
        result.rand_prop_points(7).unwrap(),
        result.rand_prop_points_records().next().unwrap()
    ));
    assert!(result.rand_prop_points(9).is_none());
    assert_eq!(
        result
            .mythic_plus_season_records()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [7, 8]
    );
    assert!(std::ptr::eq(
        result.mythic_plus_season(7).unwrap(),
        result.mythic_plus_season_records().next().unwrap()
    ));
    assert!(result.mythic_plus_season(9).is_none());
}

#[test]
fn all_value_input_removals_use_native_hash_and_same_width_signed_ids() {
    let result = batch(u32::MAX, 1)
        .finish(
            batch(u32::MAX, 2),
            batch(u32::MAX, 3),
            6,
            Default::default(),
            Default::default(),
            &removals(SPELL_TABLE_HASHES[42..48].iter().map(|&h| (h, -1, 2))),
        )
        .unwrap();
    assert!(
        result.counts()[42..48]
            .iter()
            .all(|(_, n, unknown)| *n == 0 && *unknown == 0)
    );
}

#[test]
fn value_input_duplicate_baselines_and_false_sql_coverage_are_rejected() {
    assert!(
        finish(
            SpellRecords {
                expected_stats: vec![rows::expected_stat(7), rows::expected_stat(7)],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                expected_stat_mods: vec![rows::expected_stat_mod(7), rows::expected_stat_mod(7)],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                content_tunings: vec![rows::content_tuning(7), rows::content_tuning(7)],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                content_tuning_x_expected: vec![
                    rows::content_tuning_x_expected(7),
                    rows::content_tuning_x_expected(7)
                ],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                rand_prop_points: vec![rows::rand_prop_points(7), rows::rand_prop_points(7)],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                mythic_plus_seasons: vec![rows::mythic_plus_season(7), rows::mythic_plus_season(7)],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    for index in 42..48 {
        let mut batch = SpellRecords::default();
        batch.unknown_baseline_records[index] = 1;
        assert!(finish(Default::default(), batch, Default::default()).is_err());
    }
}
