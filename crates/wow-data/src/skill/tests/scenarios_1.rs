//! Skill tier and pet spell stores regression scenarios, part 1 of 2.
//!
//! Moved out of the skill.rs root under #638; every test is unchanged.

use super::*;

#[test]
fn signed_skill_identifier_0x8000_is_rejected_without_unsigned_reinterpretation() {
    let signed_raw_0x8000 = i128::from(i16::MIN);
    assert_eq!(u16::from_ne_bytes(i16::MIN.to_ne_bytes()), 0x8000);

    for source in [
        SkillStoreLoadSourceLikeCpp::Wdc4,
        SkillStoreLoadSourceLikeCpp::OfficialSql,
        SkillStoreLoadSourceLikeCpp::CustomSql,
    ] {
        let mut ability = ability_source(ability(1, 100, 1_000), source);
        ability.skill_line = signed_raw_0x8000;
        assert_eq!(
            skill_line_ability_skill_key_from_source_like_cpp(&ability),
            None,
            "the signed DB2/SQL bit pattern must not become skill 32768"
        );

        let mut diagnostics = Vec::new();
        assert!(skill_line_ability_from_source_like_cpp(ability, &mut diagnostics).is_none());
        assert_eq!(
            diagnostics,
            [
                SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                    source,
                    record_id: 1,
                    spell: 1_000,
                    skill_line: signed_raw_0x8000,
                    skillup_skill_line_id: 0,
                }
            ]
        );

        let mut race_class = race_class_source(race_class_info(2, 100, 0, 1, 0, 0), source);
        race_class.skill_id = signed_raw_0x8000;

        let mut diagnostics = Vec::new();
        assert!(skill_race_class_info_from_source_like_cpp(race_class, &mut diagnostics).is_none());
        assert_eq!(
            diagnostics,
            [
                SkillStoreLoadDiagnosticLikeCpp::InvalidSkillRaceClassInfoIdentifier {
                    source,
                    record_id: 2,
                    race_mask: 1,
                    skill_id: signed_raw_0x8000,
                    class_mask: 1,
                }
            ]
        );
    }
}

#[test]
fn effective_rank_projection_preserves_only_rank_fields_and_final_removals() {
    const ABILITY_TABLE_HASH: u32 = 0xA100_0001;
    const RACE_CLASS_TABLE_HASH: u32 = 0xB200_0002;
    let skill_lines =
        SkillLineStore::from_entries([skill_line(100, SKILL_CATEGORY_PROFESSION_LIKE_CPP)]);

    let mut unrelated_invalid =
        ability_source(ability(1, 100, 3), SkillStoreLoadSourceLikeCpp::CustomSql);
    unrelated_invalid.supercedes_spell = 2;
    unrelated_invalid.race_mask = i128::from(i64::MAX) + 1;

    let mut invalid_supercedes =
        ability_source(ability(2, 100, 4), SkillStoreLoadSourceLikeCpp::CustomSql);
    invalid_supercedes.supercedes_spell = i128::from(i32::MAX) + 1;

    let mut invalid_spell =
        ability_source(ability(3, 100, 5), SkillStoreLoadSourceLikeCpp::CustomSql);
    invalid_spell.spell = i128::from(i32::MAX) + 1;
    invalid_spell.supercedes_spell = 2;

    let mut one_signed_endpoint =
        ability_source(ability(4, 100, 6), SkillStoreLoadSourceLikeCpp::CustomSql);
    one_signed_endpoint.spell = i128::from(i32::MAX) + 1;
    one_signed_endpoint.supercedes_spell = -1;

    let mut no_rank = ability_source(ability(5, 100, 7), SkillStoreLoadSourceLikeCpp::CustomSql);
    no_rank.race_mask = i128::from(i64::MAX) + 1;

    let mut removed_rank =
        ability_source(ability(6, 100, 8), SkillStoreLoadSourceLikeCpp::CustomSql);
    removed_rank.supercedes_spell = 7;
    let removals =
        Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(ABILITY_TABLE_HASH, 6, 2)]);

    let mut signed_endpoints =
        ability_source(ability(7, 100, -2), SkillStoreLoadSourceLikeCpp::CustomSql);
    signed_endpoints.supercedes_spell = -1;

    let mut base_replaced_rank =
        ability_source(ability(8, 100, 20), SkillStoreLoadSourceLikeCpp::Wdc4);
    base_replaced_rank.supercedes_spell = 19;
    let mut official_replaced_rank = ability_source(
        ability(8, 100, 30),
        SkillStoreLoadSourceLikeCpp::OfficialSql,
    );
    official_replaced_rank.supercedes_spell = 29;
    let mut final_overlay_rank =
        ability_source(ability(8, 100, 40), SkillStoreLoadSourceLikeCpp::CustomSql);
    final_overlay_rank.supercedes_spell = 39;
    final_overlay_rank.race_mask = i128::from(i64::MAX) + 1;

    let mut no_representable_endpoint =
        ability_source(ability(9, 100, 50), SkillStoreLoadSourceLikeCpp::CustomSql);
    no_representable_endpoint.spell = i128::from(i32::MAX) + 1;
    no_representable_endpoint.supercedes_spell = i128::from(i32::MAX) + 2;

    let outcome = compose_effective_skill_store_like_cpp(
        [base_replaced_rank],
        [official_replaced_rank],
        [
            unrelated_invalid,
            invalid_supercedes,
            invalid_spell,
            one_signed_endpoint,
            no_rank,
            removed_rank,
            signed_endpoints,
            final_overlay_rank,
            no_representable_endpoint,
        ],
        ABILITY_TABLE_HASH,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        RACE_CLASS_TABLE_HASH,
        &removals,
        &skill_lines,
    );

    assert_eq!(
        outcome.store.skill_line_ability_rank_rows_like_cpp(),
        [
            SkillLineAbilityRankRowLikeCpp::Edge {
                record_id: 1,
                spell_id: 3,
                supercedes_spell_id: 2,
            },
            SkillLineAbilityRankRowLikeCpp::Indeterminate {
                record_id: 2,
                spell_raw: 4,
                supercedes_spell_raw: i128::from(i32::MAX) + 1,
            },
            SkillLineAbilityRankRowLikeCpp::Indeterminate {
                record_id: 3,
                spell_raw: i128::from(i32::MAX) + 1,
                supercedes_spell_raw: 2,
            },
            SkillLineAbilityRankRowLikeCpp::Indeterminate {
                record_id: 4,
                spell_raw: i128::from(i32::MAX) + 1,
                supercedes_spell_raw: -1,
            },
            SkillLineAbilityRankRowLikeCpp::Edge {
                record_id: 7,
                spell_id: u32::MAX - 1,
                supercedes_spell_id: u32::MAX,
            },
            SkillLineAbilityRankRowLikeCpp::Edge {
                record_id: 8,
                spell_id: 40,
                supercedes_spell_id: 39,
            },
            SkillLineAbilityRankRowLikeCpp::Indeterminate {
                record_id: 9,
                spell_raw: i128::from(i32::MAX) + 1,
                supercedes_spell_raw: i128::from(i32::MAX) + 2,
            },
        ]
    );
    assert!(
        matches!(
            outcome
                .store
                .skill_line_ability_coverage_by_spell_like_cpp(3),
            SkillLineAbilityCoverageLikeCpp::Indeterminate(_)
        ),
        "the richer acquisition row remains invalid even though its rank edge is valid"
    );
}

#[test]
fn effective_skill_store_composes_collisions_sql_only_rows_and_removals_by_record_id() {
    const ABILITY_TABLE_HASH: u32 = 0xA100_0001;
    const RACE_CLASS_TABLE_HASH: u32 = 0xB200_0002;
    let skill_lines = SkillLineStore::from_entries([
        skill_line(100, SKILL_CATEGORY_PROFESSION_LIKE_CPP),
        skill_line(200, SKILL_CATEGORY_SECONDARY_LIKE_CPP),
    ]);
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (ABILITY_TABLE_HASH, 30, 2),
        (ABILITY_TABLE_HASH, 40, 2),
        (ABILITY_TABLE_HASH, 40, 1),
        (RACE_CLASS_TABLE_HASH, 60, 2),
        (RACE_CLASS_TABLE_HASH, 55, 2),
        (RACE_CLASS_TABLE_HASH, 55, 1),
    ]);

    let outcome = compose_effective_skill_store_like_cpp(
        [
            ability_source(ability(20, 100, 1000), SkillStoreLoadSourceLikeCpp::Wdc4),
            ability_source(ability(10, 100, 900), SkillStoreLoadSourceLikeCpp::Wdc4),
        ],
        [
            ability_source(
                ability(20, 100, 2000),
                SkillStoreLoadSourceLikeCpp::OfficialSql,
            ),
            ability_source(
                ability(30, 100, 3000),
                SkillStoreLoadSourceLikeCpp::OfficialSql,
            ),
        ],
        [
            ability_source(
                ability(20, 100, 4000),
                SkillStoreLoadSourceLikeCpp::CustomSql,
            ),
            ability_source(
                ability(40, 200, 5000),
                SkillStoreLoadSourceLikeCpp::CustomSql,
            ),
        ],
        ABILITY_TABLE_HASH,
        [race_class_source(
            race_class_info(50, 100, 1, 1, 0, 0),
            SkillStoreLoadSourceLikeCpp::Wdc4,
        )],
        [
            race_class_source(
                race_class_info(50, 100, 2, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::OfficialSql,
            ),
            race_class_source(
                race_class_info(60, 100, 4, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::OfficialSql,
            ),
        ],
        [
            race_class_source(
                race_class_info(50, 100, 8, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::CustomSql,
            ),
            race_class_source(
                race_class_info(55, 200, 16, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::CustomSql,
            ),
        ],
        RACE_CLASS_TABLE_HASH,
        &removals,
        &skill_lines,
    );

    assert_eq!(
        outcome
            .store
            .skill_line_abilities_like_cpp()
            .iter()
            .map(|record| (record.id, record.spell))
            .collect::<Vec<_>>(),
        vec![(10, 900), (20, 4000), (40, 5000)],
        "custom replaces official/base, SQL-only survives, removed rows vanish, and final IDs sort"
    );
    assert_eq!(
        outcome
            .store
            .skill_race_class_info_candidates_like_cpp(100, 1, 1)
            .iter()
            .map(|record| (record.id, record.flags))
            .collect::<Vec<_>>(),
        vec![(50, 8)]
    );
    assert_eq!(
        outcome
            .store
            .skill_race_class_info_candidates_like_cpp(200, 1, 1)
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![55]
    );
    assert_eq!(
        outcome.report,
        SkillStoreEffectiveLoadReportLikeCpp {
            skill_line_ability_wdc4_rows: 2,
            skill_line_ability_official_sql_rows: 2,
            skill_line_ability_custom_sql_rows: 2,
            skill_line_ability_removed_rows: 1,
            skill_line_ability_effective_rows: 3,
            skill_line_ability_indexed_rows: 3,
            skill_line_ability_invalid_rows: 0,
            skill_race_class_info_wdc4_rows: 1,
            skill_race_class_info_official_sql_rows: 2,
            skill_race_class_info_custom_sql_rows: 2,
            skill_race_class_info_removed_rows: 1,
            skill_race_class_info_effective_rows: 2,
            skill_race_class_info_indexed_rows: 2,
            skill_race_class_info_invalid_rows: 0,
            skill_race_class_info_missing_skill_line_rows: 0,
            diagnostics_in_record_order_like_cpp: Vec::new(),
        }
    );
}

#[test]
fn final_invalid_overlay_replaces_stale_payload_and_valid_custom_can_repair_it() {
    const ABILITY_TABLE_HASH: u32 = 0xA100_0001;
    const RACE_CLASS_TABLE_HASH: u32 = 0xB200_0002;
    let skill_lines = SkillLineStore::from_entries([
        skill_line(100, SKILL_CATEGORY_PROFESSION_LIKE_CPP),
        skill_line(200, SKILL_CATEGORY_SECONDARY_LIKE_CPP),
    ]);

    let mut invalid_final_ability = ability_source(
        ability(1, 100, 1000),
        SkillStoreLoadSourceLikeCpp::CustomSql,
    );
    invalid_final_ability.skill_line = -1;
    let mut repaired_official_ability = ability_source(
        ability(2, 100, 2000),
        SkillStoreLoadSourceLikeCpp::OfficialSql,
    );
    repaired_official_ability.skill_line = -2;
    let mut invalid_skillup_ability = ability_source(
        ability(3, 100, 3000),
        SkillStoreLoadSourceLikeCpp::CustomSql,
    );
    invalid_skillup_ability.skillup_skill_line_id = -1;
    let mut invalid_payload_ability = ability_source(
        ability(4, 200, 2222),
        SkillStoreLoadSourceLikeCpp::CustomSql,
    );
    invalid_payload_ability.race_mask = i128::MAX;

    let mut invalid_final_race_class = race_class_source(
        race_class_info(10, 100, 0, 1, 0, 0),
        SkillStoreLoadSourceLikeCpp::CustomSql,
    );
    invalid_final_race_class.skill_id = -3;
    let mut repaired_official_race_class = race_class_source(
        race_class_info(11, 100, 0, 1, 0, 0),
        SkillStoreLoadSourceLikeCpp::OfficialSql,
    );
    repaired_official_race_class.skill_id = -4;

    let outcome = compose_effective_skill_store_like_cpp(
        [
            ability_source(ability(1, 100, 1000), SkillStoreLoadSourceLikeCpp::Wdc4),
            ability_source(ability(2, 100, 2000), SkillStoreLoadSourceLikeCpp::Wdc4),
        ],
        [repaired_official_ability],
        [
            invalid_final_ability,
            ability_source(
                ability(2, 200, 2222),
                SkillStoreLoadSourceLikeCpp::CustomSql,
            ),
            invalid_skillup_ability,
            invalid_payload_ability,
        ],
        ABILITY_TABLE_HASH,
        [
            race_class_source(
                race_class_info(10, 100, 0, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::Wdc4,
            ),
            race_class_source(
                race_class_info(11, 100, 0, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::Wdc4,
            ),
        ],
        [repaired_official_race_class],
        [
            invalid_final_race_class,
            race_class_source(
                race_class_info(11, 200, 0, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::CustomSql,
            ),
        ],
        RACE_CLASS_TABLE_HASH,
        &Db2HotfixRemovalStoreLikeCpp::default(),
        &skill_lines,
    );

    assert_eq!(
        outcome
            .store
            .skill_line_abilities_like_cpp()
            .iter()
            .map(|record| (record.id, record.skill_line, record.spell))
            .collect::<Vec<_>>(),
        vec![(2, 200, 2222)],
        "an invalid final custom row must not uncover the stale WDC4 record"
    );
    assert!(matches!(
        outcome
            .store
            .skill_line_ability_coverage_by_spell_like_cpp(1000),
        SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics)
            if diagnostics.iter().any(|diagnostic| matches!(
                diagnostic,
                SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                    record_id: 1,
                    ..
                }
            ))
    ));
    assert!(matches!(
        outcome
            .store
            .skill_line_ability_coverage_by_spell_like_cpp(2222),
        SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics)
            if diagnostics.iter().any(|diagnostic| matches!(
                diagnostic,
                SkillStoreLoadDiagnosticLikeCpp::InvalidSourceField {
                    record_id: 4,
                    field: "RaceMask",
                    ..
                }
            ))
    ));
    assert!(matches!(
        outcome
            .store
            .skill_line_ability_coverage_by_skill_like_cpp(200),
        SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics)
            if diagnostics.iter().any(|diagnostic| matches!(
                diagnostic,
                SkillStoreLoadDiagnosticLikeCpp::InvalidSourceField {
                    record_id: 4,
                    field: "RaceMask",
                    ..
                }
            ))
    ));
    assert_eq!(
        outcome
            .store
            .skill_line_ability_coverage_by_spell_like_cpp(9999),
        SkillLineAbilityCoverageLikeCpp::CoveredZero
    );
    assert_eq!(
        outcome
            .store
            .skill_line_ability_coverage_by_skill_like_cpp(999),
        SkillLineAbilityCoverageLikeCpp::CoveredZero
    );
    assert_eq!(outcome.store.race_class_count(), 1);
    assert_eq!(
        outcome
            .store
            .skill_race_class_info_candidates_like_cpp(200, 1, 1)[0]
            .id,
        11
    );
    assert!(
        outcome
            .report
            .diagnostics_in_record_order_like_cpp
            .contains(
                &SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                    source: SkillStoreLoadSourceLikeCpp::CustomSql,
                    record_id: 1,
                    spell: 1000,
                    skill_line: -1,
                    skillup_skill_line_id: 0,
                }
            )
    );
    assert!(
        outcome
            .report
            .diagnostics_in_record_order_like_cpp
            .contains(
                &SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                    source: SkillStoreLoadSourceLikeCpp::CustomSql,
                    record_id: 3,
                    spell: 3000,
                    skill_line: 100,
                    skillup_skill_line_id: -1,
                }
            )
    );
    assert!(
        outcome
            .report
            .diagnostics_in_record_order_like_cpp
            .contains(
                &SkillStoreLoadDiagnosticLikeCpp::InvalidSkillRaceClassInfoIdentifier {
                    source: SkillStoreLoadSourceLikeCpp::CustomSql,
                    record_id: 10,
                    race_mask: 1,
                    skill_id: -3,
                    class_mask: 1,
                }
            )
    );
}

#[test]
fn missing_skill_lines_and_conflicting_race_class_payloads_fail_closed_with_diagnostics() {
    let skill_lines =
        SkillLineStore::from_entries([skill_line(100, SKILL_CATEGORY_PROFESSION_LIKE_CPP)]);
    let outcome = compose_effective_skill_store_like_cpp(
        [],
        [],
        [],
        1,
        [
            race_class_source(
                race_class_info(1, 999, 0, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::Wdc4,
            ),
            race_class_source(
                race_class_info(2, 100, 0, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::Wdc4,
            ),
            race_class_source(
                race_class_info(3, 100, 1, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::Wdc4,
            ),
        ],
        [],
        [],
        2,
        &Db2HotfixRemovalStoreLikeCpp::default(),
        &skill_lines,
    );

    assert_eq!(outcome.store.race_class_count(), 2);
    assert!(matches!(
        outcome
            .store
            .skill_race_class_info_coverage_by_skill_like_cpp(999),
        SkillRaceClassInfoCoverageLikeCpp::Indeterminate(diagnostics)
            if diagnostics == [SkillStoreLoadDiagnosticLikeCpp::MissingEffectiveSkillLine {
                record_id: 1,
                skill_id: 999,
            }]
    ));
    assert!(matches!(
        outcome
            .store
            .skill_race_class_info_coverage_by_skill_like_cpp(100),
        SkillRaceClassInfoCoverageLikeCpp::Indeterminate(diagnostics)
            if diagnostics == [SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                skill_id: 100,
                first_record_id: 2,
                second_record_id: 3,
            }]
    ));
    assert!(matches!(
        outcome
            .store
            .skill_race_class_info_coverage_for_player_like_cpp(100, 1, 1),
        SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(diagnostics)
            if diagnostics == [SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                skill_id: 100,
                first_record_id: 2,
                second_record_id: 3,
            }]
    ));
    assert!(
        outcome
            .store
            .skill_race_class_info_like_cpp(100, 1, 1)
            .is_none(),
        "an unordered C++ first-match conflict must fail closed"
    );
    assert!(
        outcome
            .store
            .default_starting_skill_info_like_cpp(
                1,
                1,
                80,
                &skill_lines,
                &SkillTiersStoreLikeCpp::default(),
            )
            .is_empty(),
        "the starting-skill consumer must not bypass the ambiguity guard"
    );
    assert!(
        !outcome.store.starting_skills.contains_key(&(1, 1))
            || outcome.store.starting_skills[&(1, 1)]
                .iter()
                .all(|record| record.skill_id != 999),
        "the #163 fail-closed hardening also excludes missing SkillLine rows from starting skills"
    );
    assert!(
        outcome
            .report
            .diagnostics_in_record_order_like_cpp
            .contains(
                &SkillStoreLoadDiagnosticLikeCpp::MissingEffectiveSkillLine {
                    record_id: 1,
                    skill_id: 999,
                }
            )
    );
    assert!(
        outcome
            .report
            .diagnostics_in_record_order_like_cpp
            .contains(&SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                skill_id: 100,
                first_record_id: 2,
                second_record_id: 3,
            })
    );
}

#[test]
fn effective_indices_use_cpp_race_bits_and_full_race_class_ranges() {
    let mut race_52 = race_class_info(1, 100, 0, 1, 0, 0);
    race_52.race_mask = 1_i64 << 16;
    race_52.class_mask = 1_i32 << (13 - 1);
    let mut race_70 = race_class_info(2, 100, 0, 1, 0, 0);
    race_70.race_mask = 1_i64 << 15;
    race_70.class_mask = -1;
    let store =
        SkillStore::from_skill_line_abilities_and_race_class_like_cpp([], [race_52, race_70]);

    assert_eq!(
        store
            .skill_race_class_info_candidates_like_cpp(100, 52, 13)
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        store
            .skill_race_class_info_candidates_like_cpp(100, 70, 14)
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(store.starting_skills.contains_key(&(52, 13)));
    assert!(store.starting_skills.contains_key(&(70, 14)));
    assert!(!store.starting_skills.contains_key(&(70, 15)));
}

#[test]
fn removal_lookup_preserves_unsigned_record_id_bit_pattern_like_cpp() {
    let table_hash = 0xA100_0001;
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(table_hash, -1, 2)]);
    assert!(record_removed_like_cpp(&removals, table_hash, u32::MAX));
}

#[test]
fn skill_rewarded_spells_match_cpp_filters() {
    let store = SkillStore::from_skill_line_abilities_like_cpp([
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ..ability(1, 756, 822)
        },
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ..ability(2, 756, 28877)
        },
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
            min_skill_line_rank: 450,
            ..ability(3, 756, 999)
        },
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            race_mask: 1,
            ..ability(4, 756, 1000)
        },
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            class_mask: 1,
            ..ability(5, 756, 1001)
        },
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ..ability(6, 756, 1002)
        },
    ]);

    let spells = store.skill_rewarded_spells_like_cpp(
        756,
        400,
        10,
        5,
        80,
        |spell_id| match spell_id {
            1002 => Some((81, 81)),
            _ => Some((0, 0)),
        },
        |_| false,
    );

    assert_eq!(spells, vec![822, 28877]);

    let changes = store.skill_rewarded_spell_changes_like_cpp(
        756,
        400,
        10,
        5,
        80,
        |spell_id| match spell_id {
            1002 => Some((81, 81)),
            _ => Some((0, 0)),
        },
        |_| false,
    );
    assert_eq!(changes.learn, vec![822, 28877]);
    assert_eq!(
        changes.remove,
        vec![999],
        "C++ removes LEARNED_ON_SKILL_VALUE spells below MinSkillLineRank"
    );
}

#[test]
fn skill_rewarded_spells_applies_cpp_riding_exception() {
    let store = SkillStore::from_skill_line_abilities_like_cpp([
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
            num_skill_ups: 1,
            ..ability(1, SKILL_RIDING_LIKE_CPP, 333)
        },
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            num_skill_ups: 0,
            ..ability(2, SKILL_RIDING_LIKE_CPP, 444)
        },
        SkillLineAbilityRecord {
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            num_skill_ups: 1,
            ..ability(3, SKILL_RIDING_LIKE_CPP, 555)
        },
    ]);

    let spells = store.skill_rewarded_spells_like_cpp(
        SKILL_RIDING_LIKE_CPP,
        1,
        1,
        1,
        80,
        |_| Some((0, 0)),
        |_| false,
    );

    assert_eq!(spells, vec![555]);
}

#[test]
fn skill_tiers_store_replaces_duplicate_ids_like_cpp() {
    let store = SkillTiersStoreLikeCpp::from_rows_like_cpp([
        skill_tier_row(12, [75, 150, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        skill_tier_row(12, [1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    ]);

    assert_eq!(store.len(), 1);
    assert_eq!(
        store
            .get_skill_tier_like_cpp(12)
            .expect("duplicate ID should leave one C++ map entry")
            .value[5],
        6,
        "C++ _skillTiers[id] overwrites the existing entry for duplicate IDs"
    );
}

#[test]
fn skill_tier_value_falls_back_to_previous_nonzero_like_cpp() {
    let tier = SkillTiersEntryLikeCpp {
        id: 1,
        value: [75, 150, 225, 0, 0, 0, 450, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    };

    assert_eq!(tier.get_value_for_tier_index_like_cpp(0), 75);
    assert_eq!(tier.get_value_for_tier_index_like_cpp(3), 225);
    assert_eq!(tier.get_value_for_tier_index_like_cpp(6), 450);
}

#[test]
fn default_skills_filter_availability_and_min_level_and_use_cpp_ranges() {
    let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
        std::iter::empty(),
        [
            race_class_info(1, 100, 0, 1, 1, 0),
            race_class_info(2, 101, 0, 1, 1, 0),
            race_class_info(3, 102, SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP, 1, 1, 0),
            race_class_info(4, 103, 0, 1, 1, 0),
            race_class_info(5, 104, 0, 1, 1, 12),
            race_class_info(6, 105, 0, 0, 1, 0),
            race_class_info(7, 106, 0, 1, 11, 0),
        ],
    );
    let skill_lines = SkillLineStore::from_entries([
        skill_line(100, SKILL_CATEGORY_LANGUAGES_LIKE_CPP),
        skill_line(101, 0),
        skill_line(102, 0),
        skill_line(103, SKILL_CATEGORY_ARMOR_LIKE_CPP),
        skill_line(104, SKILL_CATEGORY_PROFESSION_LIKE_CPP),
        skill_line(105, 0),
        skill_line(106, 0),
    ]);
    let tiers = SkillTiersStoreLikeCpp::from_rows_like_cpp([skill_tier_row(
        12,
        [75, 150, 225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    )]);

    assert_eq!(
        store.default_starting_skill_info_like_cpp(1, 1, 10, &skill_lines, &tiers),
        vec![
            SkillInfoEntry {
                skill_id: 100,
                step: 0,
                rank: 300,
                starting_rank: 1,
                max_rank: 300,
                temp_bonus: 0,
                perm_bonus: 0,
            },
            SkillInfoEntry {
                skill_id: 101,
                step: 0,
                rank: 1,
                starting_rank: 1,
                max_rank: 50,
                temp_bonus: 0,
                perm_bonus: 0,
            },
            SkillInfoEntry {
                skill_id: 102,
                step: 0,
                rank: 50,
                starting_rank: 1,
                max_rank: 50,
                temp_bonus: 0,
                perm_bonus: 0,
            },
            SkillInfoEntry {
                skill_id: 103,
                step: 0,
                rank: 1,
                starting_rank: 1,
                max_rank: 1,
                temp_bonus: 0,
                perm_bonus: 0,
            },
            SkillInfoEntry {
                skill_id: 104,
                step: 1,
                rank: 1,
                starting_rank: 1,
                max_rank: 75,
                temp_bonus: 0,
                perm_bonus: 0,
            },
        ],
        "C++ includes default skills even without abilities, but excludes Availability != 1 and future MinLevel rows"
    );
}
