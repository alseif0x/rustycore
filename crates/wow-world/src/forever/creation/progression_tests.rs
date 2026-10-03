use super::*;

#[test]
fn above_cap_stats_keep_the_source_cap_minus_one_loop_and_unchanged_spirit() {
    let initial = [100; 5];
    assert_eq!(
        extrapolate_stats(1, 3, 4, initial).unwrap(),
        [102, 100, 102, 100, 100]
    );
    assert_eq!(
        extrapolate_stats(11, 40, 41, initial).unwrap(),
        [104, 104, 104, 106, 100]
    );
    for class in [0, 6, 10, 12, 13, 14, 15] {
        assert_eq!(extrapolate_stats(class, 40, 41, initial).unwrap(), initial);
    }
    assert!(matches!(
        extrapolate_stats(1, 3, 4, [i32::MAX; 5]),
        Err(SourceError::StatsOverflow)
    ));
}

#[test]
fn source_extrapolation_thresholds_are_class_specific() {
    // One source-loop step: cap-1 to cap. This tests the exact four stat
    // thresholds without importing a 3.3.5 spirit or DeathKnight formula.
    for (class, cap, expected) in [
        (1, 25, [2, 0, 2, 1, 0]),
        (2, 35, [1, 1, 2, 0, 0]),
        (3, 35, [1, 2, 1, 0, 0]),
        (4, 18, [1, 2, 1, 0, 0]),
        (5, 24, [0, 1, 1, 2, 0]),
        (7, 36, [1, 0, 1, 1, 0]),
        (8, 26, [0, 0, 1, 2, 0]),
        (9, 40, [0, 0, 2, 2, 0]),
        (11, 40, [2, 2, 2, 3, 0]),
    ] {
        assert_eq!(
            extrapolate_stats(class, cap, cap, [0; 5]).unwrap(),
            expected
        );
    }
}

fn tables(xp: &[f32]) -> InitialGameTables {
    let base = format!(
        "L\t{}\nignored\t{}\nignored\t{}\n",
        (0..15)
            .map(|i| format!("C{i}"))
            .collect::<Vec<_>>()
            .join("\t"),
        (1..=15)
            .map(|i| format!("{}.75", i))
            .collect::<Vec<_>>()
            .join("\t"),
        (101..=115)
            .map(|i| format!("{}.75", i))
            .collect::<Vec<_>>()
            .join("\t")
    );
    let xp = format!(
        "L\tTotal\tPerKill\tJunk\tStats\tDivisor\n{}",
        xp.iter()
            .map(|total| format!("999\t{total}\t0\t0\t0\t0\n"))
            .collect::<String>()
    );
    InitialGameTables::parse_strs(&base, "L\tHealth\n", &xp).unwrap()
}

#[test]
fn source_xp_uses_physical_gt_rows_unsigned_truncation_and_lookup_bounds() {
    let tables = tables(&[400.75, 900.5, 1200.]);
    let progression = Progression::load(vec![], &tables, 3).unwrap();
    assert_eq!(progression.counts(), [4, 3]);
    assert_eq!(progression.override_count(), 0);
    assert_eq!(
        (0..=4)
            .map(|level| progression.xp(level))
            .collect::<Vec<_>>(),
        [0, 400, 900, 1200, 0]
    );
}

#[test]
fn official_gt_then_sql_then_gap_fallback_includes_sql_level_zero_but_not_cap() {
    let tables = tables(&[400., 0., 0., 700.]);
    let progression = Progression::load(
        vec![
            XpOverride {
                level: 0,
                experience: 7,
            },
            XpOverride {
                level: 1,
                experience: 0,
            },
            XpOverride {
                level: 3,
                experience: 999,
            }, // exactly cap is ignored
            XpOverride {
                level: 255,
                experience: 999,
            },
        ],
        &tables,
        3,
    )
    .unwrap();
    assert_eq!(progression.override_count(), 4);
    assert_eq!(
        (0..=4)
            .map(|level| progression.xp(level))
            .collect::<Vec<_>>(),
        [7, 12_007, 24_007, 0, 700]
    );
    // Missing/zero at cap is retained, not filled by the below-cap loop.
    assert_eq!(progression.xp(3), 0);
}

#[test]
fn xp_gap_addition_preserves_defined_source_uint32_wrapping() {
    let progression = Progression::load(
        vec![XpOverride {
            level: 0,
            experience: u32::MAX - 100,
        }],
        &tables(&[0., 0.]),
        3,
    )
    .unwrap();
    assert_eq!(progression.xp(1), 11_899);
    assert_eq!(progression.xp(2), 23_899);
}

#[test]
fn invalid_caps_unknown_vector_coverage_duplicates_and_undefined_float_casts_fail_closed() {
    for max in [0, 124] {
        assert!(matches!(
            Progression::load(vec![], &tables(&[400.]), max),
            Err(SourceError::InvalidLevelCap)
        ));
    }
    assert!(matches!(
        Progression::load(vec![], &tables(&[400.]), 3),
        Err(SourceError::InsufficientXpCoverage)
    ));
    let duplicate = vec![
        XpOverride {
            level: 1,
            experience: 5,
        },
        XpOverride {
            level: 1,
            experience: 6,
        },
    ];
    assert!(matches!(
        Progression::load(duplicate, &tables(&[400.]), 2),
        Err(SourceError::DuplicateIdentity)
    ));
    assert!(matches!(
        Progression::load(vec![], &tables(&[-1.]), 2),
        Err(SourceError::InvalidGameTableValue)
    ));
    for value in [-1., f32::NAN, f32::INFINITY, 4_294_967_296.] {
        assert_eq!(unsigned(value), Err(SourceError::InvalidGameTableValue));
    }
    assert_eq!(unsigned(-0.), Ok(0));
    assert_eq!(unsigned(4_294_967_040.), Ok(4_294_967_040));
}

#[test]
fn base_mana_clamps_to_configured_cap_and_does_not_fabricate_absent_rows() {
    let tables = tables(&[400., 900.]);
    let capped = Progression::load(vec![], &tables, 1).unwrap();
    assert_eq!(capped.base_mana(&tables, 1, 2), Ok(9));
    let progression = Progression::load(vec![], &tables, 3).unwrap();
    assert_eq!(progression.base_mana(&tables, 1, 1), Ok(9));
    assert_eq!(progression.base_mana(&tables, 1, 2), Ok(109));
    assert_eq!(
        progression.base_mana(&tables, 1, 3),
        Err(SourceError::MissingManaRow)
    );
    assert_eq!(
        progression.base_mana(&tables, 1, 0),
        Err(SourceError::InvalidLevel)
    );
    assert_eq!(
        progression.base_mana(&tables, 16, 1),
        Err(SourceError::InvalidClass)
    );
}
