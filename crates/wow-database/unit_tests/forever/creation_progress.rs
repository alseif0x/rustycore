//! Borrowed frozen-snapshot checks, not canonical post-commit flag-application proof.
use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{
    NewCompletedAchievement, NewSkill, NewStoredAuraLocation, NewStoredAuraLocationState,
    NewStoredAuraPosition,
};

fn achievements() -> NewAchievementSave {
    NewAchievementSave {
        completed: Vec::new(),
        progress: Vec::new(),
    }
}

fn quest_criteria() -> NewQuestCriteriaSave {
    NewQuestCriteriaSave {
        completed_objectives: Vec::new(),
        progress: Vec::new(),
    }
}

#[test]
fn completed_achievement_changed_matrix_preserves_reversed_delete_scope_and_signed_date() {
    // Scalar limits check bindings, not canonical achievement construction.
    for changed in [false, true] {
        for date in [i64::MIN, i64::MAX] {
            let mut data = achievements();
            data.completed = vec![NewCompletedAchievement {
                achievement: u32::MAX,
                changed,
                date,
            }];
            let rows = achievement_statements(u64::MAX, &data);
            if !changed {
                assert!(rows.is_empty());
                continue;
            }
            assert_eq!(
                rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
                [
                    "DELETE FROM character_achievement WHERE achievement = ? AND guid = ?",
                    "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)",
                ]
            );
            assert_eq!(
                rows[0].params(),
                &[SqlParam::U32(u32::MAX), SqlParam::U64(u64::MAX)]
            );
            assert_eq!(
                rows[1].params(),
                &[
                    SqlParam::U64(u64::MAX),
                    SqlParam::U32(u32::MAX),
                    SqlParam::I64(date),
                ]
            );
            assert!(data.completed[0].changed);
        }
    }
}

#[test]
fn both_criteria_writers_skip_unchanged_delete_changed_zero_and_insert_changed_nonzero() {
    for (delete_sql, insert_sql) in [
        (
            "DELETE FROM character_achievement_progress WHERE guid = ? AND criteria = ?",
            "INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)",
        ),
        (
            "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ? AND criteriaId = ?",
            "INSERT INTO character_queststatus_objectives_criteria_progress (guid, criteriaId, counter, date) VALUES (?, ?, ?, ?)",
        ),
    ] {
        for changed in [false, true] {
            for counter in [0, u64::MAX] {
                for date in [i64::MIN, i64::MAX] {
                    let data = [NewCriteriaProgress {
                        criteria: u32::MAX,
                        changed,
                        counter,
                        date,
                    }];
                    let rows = criteria_statements(u64::MAX, &data, delete_sql, insert_sql);
                    if !changed {
                        assert!(rows.is_empty());
                        continue;
                    }
                    assert_eq!(rows.len(), if counter == 0 { 1 } else { 2 });
                    assert_eq!(rows[0].sql(), delete_sql);
                    assert_eq!(
                        rows[0].params(),
                        &[SqlParam::U64(u64::MAX), SqlParam::U32(u32::MAX)]
                    );
                    if counter != 0 {
                        assert_eq!(rows[1].sql(), insert_sql);
                        assert_eq!(
                            rows[1].params(),
                            &[
                                SqlParam::U64(u64::MAX),
                                SqlParam::U32(u32::MAX),
                                SqlParam::U64(counter),
                                SqlParam::I64(date),
                            ]
                        );
                    }
                    assert!(data[0].changed);
                }
            }
        }
    }
}

#[test]
fn reputation_uses_replist_key_order_but_value_faction_and_preserves_u16_narrowing() {
    let data = BTreeMap::from([
        (
            90,
            NewReputation {
                faction: 1,
                need_save: true,
                standing: 23,
                flags: 7,
            },
        ),
        (
            2,
            NewReputation {
                faction: 17,
                need_save: true,
                standing: -24,
                flags: 8,
            },
        ),
        (
            7,
            NewReputation {
                faction: 9,
                need_save: false,
                standing: 0,
                flags: 0,
            },
        ),
        (
            1,
            NewReputation {
                faction: 90,
                need_save: true,
                standing: -17,
                flags: 16,
            },
        ),
    ]);
    let rows = reputation_statements(29, &data);
    assert_eq!(rows.len(), 6);
    for (pair, (faction, standing, flags)) in
        rows.chunks_exact(2)
            .zip([(90, -17, 16), (17, -24, 8), (1, 23, 7)])
    {
        assert_eq!(
            pair[0].sql(),
            "DELETE FROM character_reputation WHERE guid = ? AND faction = ?"
        );
        assert_eq!(
            pair[1].sql(),
            "INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ? , ?)"
        );
        assert_eq!(
            pair[0].params(),
            &[SqlParam::U64(29), SqlParam::U16(faction)]
        );
        assert_eq!(
            pair[1].params(),
            &[
                SqlParam::U64(29),
                SqlParam::U16(faction),
                SqlParam::I32(standing),
                SqlParam::U16(flags),
            ]
        );
    }
    assert!(data[&1].need_save);
    assert!(!data[&7].need_save);
    // Encoding bounds do not establish canonical faction initialization/policy.
    for (faction, narrowed, standing, flags) in [
        (0x1234_5678, 0x5678, i32::MIN, u16::MAX),
        (u32::MAX, u16::MAX, i32::MAX, 0x8001),
    ] {
        let data = BTreeMap::from([(
            42,
            NewReputation {
                faction,
                need_save: true,
                standing,
                flags,
            },
        )]);
        let rows = reputation_statements(u64::MAX, &data);
        assert_eq!(
            rows[0].params(),
            &[SqlParam::U64(u64::MAX), SqlParam::U16(narrowed)]
        );
        assert_eq!(
            rows[1].params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U16(narrowed),
                SqlParam::I32(standing),
                SqlParam::U16(flags),
            ]
        );
    }
}

#[test]
fn completed_quest_objectives_always_delete_then_preserve_unique_supplied_order() {
    let mut data = quest_criteria();
    let empty = quest_statements(u64::MAX, &data);
    assert_eq!(empty.len(), 1);
    assert_eq!(
        empty[0].sql(),
        "DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?"
    );
    assert_eq!(empty[0].params(), &[SqlParam::U64(u64::MAX)]);
    data.completed_objectives = vec![90, 2, 17];
    let rows = quest_statements(31, &data);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].params(), &[SqlParam::U64(31)]);
    for (row, objective) in rows[1..].iter().zip([90, 2, 17]) {
        assert_eq!(
            row.sql(),
            "INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) VALUES (?, ?)"
        );
        assert_eq!(row.params(), &[SqlParam::U64(31), SqlParam::U32(objective)]);
    }
}

#[test]
fn entire_writer_orders_progress_groups_and_preserves_frozen_snapshot_flags() {
    let achievements = NewAchievementSave {
        completed: vec![
            NewCompletedAchievement {
                achievement: 90,
                changed: true,
                date: 1,
            },
            NewCompletedAchievement {
                achievement: 2,
                changed: true,
                date: 2,
            },
        ],
        progress: vec![NewCriteriaProgress {
            criteria: 7,
            changed: true,
            counter: 0,
            date: 3,
        }],
    };
    let reputation = BTreeMap::from([(
        1,
        NewReputation {
            faction: 4,
            need_save: true,
            standing: -1,
            flags: 1,
        },
    )]);
    let quest_criteria = NewQuestCriteriaSave {
        completed_objectives: vec![30, 3],
        progress: vec![
            NewCriteriaProgress {
                criteria: 9,
                changed: true,
                counter: 5,
                date: -1,
            },
            NewCriteriaProgress {
                criteria: 1,
                changed: false,
                counter: 7,
                date: 1,
            },
        ],
    };
    let rows = statements(37, &achievements, &reputation, &quest_criteria);
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            ACHIEVEMENT_DELETE,
            ACHIEVEMENT_INSERT,
            ACHIEVEMENT_DELETE,
            ACHIEVEMENT_INSERT,
            ACHIEVEMENT_PROGRESS_DELETE,
            REPUTATION_DELETE,
            REPUTATION_INSERT,
            OBJECTIVES_DELETE,
            OBJECTIVE_INSERT,
            OBJECTIVE_INSERT,
            QUEST_PROGRESS_DELETE,
            QUEST_PROGRESS_INSERT,
        ]
    );
    assert_eq!(rows[0].params(), &[SqlParam::U32(90), SqlParam::U64(37)]);
    assert_eq!(rows[2].params(), &[SqlParam::U32(2), SqlParam::U64(37)]);
    assert_eq!(rows[8].params(), &[SqlParam::U64(37), SqlParam::U32(30)]);
    assert_eq!(rows[9].params(), &[SqlParam::U64(37), SqlParam::U32(3)]);
    for row in &rows {
        let owner_index = usize::from(row.sql() == ACHIEVEMENT_DELETE);
        assert_eq!(row.params()[owner_index], SqlParam::U64(37));
    }
    let mut transaction = SqlTransaction::new();
    append(
        &mut transaction,
        37,
        &achievements,
        &reputation,
        &quest_criteria,
    );
    assert_eq!(
        transaction.sqls_for_test(),
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
    );
    // Source owners clear Changed/needSave; their real flag application is
    // absent here. These assertions concern only the borrowed DTO snapshot.
    assert!(
        achievements
            .completed
            .iter()
            .all(|achievement| achievement.changed)
    );
    assert!(achievements.progress[0].changed);
    assert!(reputation[&1].need_save);
    assert!(quest_criteria.progress[0].changed);
    assert!(!quest_criteria.progress[1].changed);
}

#[test]
fn production_through_cache_places_all_progress_groups_after_skills_and_locations_before_cache() {
    let mut character = super::super::tests::character();
    character.skills = vec![NewSkill {
        skill: 1,
        value: 1,
        maximum: 2,
        profession_slot: -1,
    }];
    character.stored_aura_locations = vec![NewStoredAuraLocation {
        spell: 9,
        state: NewStoredAuraLocationState::Changed(NewStoredAuraPosition {
            map: 42,
            position: [1.0, 2.0, 3.0, 4.0],
        }),
    }];
    character.achievements.completed = vec![NewCompletedAchievement {
        achievement: 90,
        changed: true,
        date: 1,
    }];
    character.achievements.progress = vec![NewCriteriaProgress {
        criteria: 7,
        changed: true,
        counter: 0,
        date: 2,
    }];
    character.reputation.insert(
        10,
        NewReputation {
            faction: 3,
            need_save: true,
            standing: -1,
            flags: 1,
        },
    );
    character.quest_criteria.completed_objectives = vec![4];
    character.quest_criteria.progress = vec![NewCriteriaProgress {
        criteria: 5,
        changed: true,
        counter: 1,
        date: 3,
    }];
    let mut transaction = SqlTransaction::new();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let cache = super::super::visual_statement(character.guid, &character.visual_items);
    let start = sqls
        .iter()
        .position(|sql| sql.starts_with("INSERT INTO character_skills ("))
        .unwrap();
    assert_eq!(
        &sqls[start..],
        &[
            "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?,?,?,?,?)",
            "DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?",
            "INSERT INTO character_aura_stored_location (Guid, Spell, MapId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?, ?)",
            "DELETE FROM character_achievement WHERE achievement = ? AND guid = ?",
            "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)",
            "DELETE FROM character_achievement_progress WHERE guid = ? AND criteria = ?",
            "DELETE FROM character_reputation WHERE guid = ? AND faction = ?",
            "INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ? , ?)",
            "DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?",
            "INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) VALUES (?, ?)",
            "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ? AND criteriaId = ?",
            "INSERT INTO character_queststatus_objectives_criteria_progress (guid, criteriaId, counter, date) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_select_screen_equipment_cache WHERE guid = ?",
            cache.sql(),
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?",
        ]
    );
}
