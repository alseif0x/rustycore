use super::*;
use crate::{SqlParam, SqlTransaction};
use wow_persistence::forever::creation::save::{
    NewQuestChange, NewRewardedQuestChange, NewSeasonalQuest, NewSpell,
};

fn save() -> NewQuestSave {
    super::super::super::tests::character().quests
}

#[test]
fn unchanged_and_changed_empty_matrix_preserves_each_periodic_groups_delete_policy() {
    for mask in 0..16 {
        let mut save = save();
        if mask & 1 != 0 {
            save.daily = Some(NewDailyQuestSave {
                quests: Vec::new(),
                dungeon_quests: BTreeSet::new(),
                last_time: 0,
            });
        }
        if mask & 2 != 0 {
            save.weekly = Some(BTreeSet::new());
        }
        if mask & 4 != 0 {
            save.seasonal = Some(Vec::new());
        }
        if mask & 8 != 0 {
            save.monthly = Some(BTreeSet::new());
        }
        let rows: Vec<_> = statements(17, &save).collect();
        let mut expected = Vec::new();
        if mask & 1 != 0 {
            expected.push("DELETE FROM character_queststatus_daily WHERE guid = ?");
        }
        if mask & 4 != 0 {
            expected.push("DELETE FROM character_queststatus_seasonal WHERE guid = ?");
        }
        assert_eq!(
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
            expected,
            "mask {mask}"
        );
        for row in rows {
            assert_eq!(row.params(), &[SqlParam::U64(17)]);
        }
    }
}

#[test]
fn daily_normal_before_dungeon_matrix_preserves_duplicates_id_bits_and_signed_time() {
    for normal in [false, true] {
        for dungeon in [false, true] {
            for last_time in [i64::MIN, i64::MAX] {
                let data = NewDailyQuestSave {
                    quests: if normal {
                        vec![-1, i32::MIN, 7, 7, 0]
                    } else {
                        Vec::new()
                    },
                    dungeon_quests: if dungeon {
                        BTreeSet::from([u32::MAX, 7, 0x8000_0001])
                    } else {
                        BTreeSet::new()
                    },
                    last_time,
                };
                let rows = daily_statements(u64::MAX, Some(&data));
                assert_eq!(
                    rows[0].sql(),
                    "DELETE FROM character_queststatus_daily WHERE guid = ?"
                );
                assert_eq!(rows[0].params(), &[SqlParam::U64(u64::MAX)]);
                let mut expected_ids = Vec::new();
                if normal {
                    expected_ids.extend([u32::MAX, 0x8000_0000, 7, 7, 0]);
                }
                if dungeon {
                    expected_ids.extend([7, 0x8000_0001, u32::MAX]);
                }
                assert_eq!(rows.len(), expected_ids.len() + 1);
                for (row, quest) in rows[1..].iter().zip(expected_ids) {
                    assert_eq!(
                        row.sql(),
                        "INSERT INTO character_queststatus_daily (guid, quest, time) VALUES (?, ?, ?)"
                    );
                    assert_eq!(
                        row.params(),
                        &[
                            SqlParam::U64(u64::MAX),
                            SqlParam::U32(quest),
                            SqlParam::I64(last_time),
                        ]
                    );
                }
            }
        }
    }
}

#[test]
fn weekly_and_monthly_nonempty_sets_delete_then_insert_ascending_scoped_ids() {
    let quests = BTreeSet::from([u32::MAX, 12, 0, 1, 12]);
    for (delete_sql, insert_sql) in [
        (
            "DELETE FROM character_queststatus_weekly WHERE guid = ?",
            "INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)",
        ),
        (
            "DELETE FROM character_queststatus_monthly WHERE guid = ?",
            "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)",
        ),
    ] {
        let rows = set_statements(29, Some(&quests), delete_sql, insert_sql);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].sql(), delete_sql);
        assert_eq!(rows[0].params(), &[SqlParam::U64(29)]);
        for (row, quest) in rows[1..].iter().zip([0, 1, 12, u32::MAX]) {
            assert_eq!(row.sql(), insert_sql);
            assert_eq!(row.params(), &[SqlParam::U64(29), SqlParam::U32(quest)]);
        }
    }
}

#[test]
fn seasonal_supplied_order_event_narrowing_and_signed_completed_times_are_preserved() {
    let groups = vec![
        NewSeasonalQuestGroup {
            event: 0x1234_5678,
            quests: vec![
                NewSeasonalQuest {
                    quest: u32::MAX,
                    completed_time: i64::MIN,
                },
                NewSeasonalQuest {
                    quest: 2,
                    completed_time: i64::MAX,
                },
            ],
        },
        NewSeasonalQuestGroup {
            event: 5,
            quests: Vec::new(),
        },
        NewSeasonalQuestGroup {
            event: u32::MAX,
            quests: vec![
                NewSeasonalQuest {
                    quest: 90,
                    completed_time: -1,
                },
                NewSeasonalQuest {
                    quest: 1,
                    completed_time: 0,
                },
            ],
        },
        NewSeasonalQuestGroup {
            event: 0x0001_0000,
            quests: vec![NewSeasonalQuest {
                quest: 3,
                completed_time: i64::MAX,
            }],
        },
    ];
    let rows = seasonal_statements(u64::MAX, Some(&groups));
    assert_eq!(rows.len(), 6);
    assert_eq!(
        rows[0].sql(),
        "DELETE FROM character_queststatus_seasonal WHERE guid = ?"
    );
    assert_eq!(rows[0].params(), &[SqlParam::U64(u64::MAX)]);
    for (row, (quest, event, time)) in rows[1..].iter().zip([
        (u32::MAX, 0x5678, i64::MIN),
        (2, 0x5678, i64::MAX),
        (90, 65535, -1),
        (1, 65535, 0),
        (3, 0, i64::MAX),
    ]) {
        assert_eq!(
            row.sql(),
            "INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) VALUES (?, ?, ?, ?)"
        );
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U32(quest),
                SqlParam::U32(event),
                SqlParam::I64(time),
            ]
        );
    }
}

#[test]
fn production_prefix_orders_status_reward_daily_weekly_seasonal_monthly_before_spells() {
    let mut character = super::super::super::tests::character();
    character.items = vec![super::super::super::items::tests::item()];
    character.quests.statuses.insert(23, NewQuestChange::Delete);
    character
        .quests
        .rewarded
        .insert(7, NewRewardedQuestChange::Default);
    character.quests.daily = Some(NewDailyQuestSave {
        quests: vec![1],
        dungeon_quests: BTreeSet::new(),
        last_time: -1,
    });
    character.quests.weekly = Some(BTreeSet::from([3]));
    character.quests.seasonal = Some(vec![NewSeasonalQuestGroup {
        event: 2,
        quests: vec![NewSeasonalQuest {
            quest: 4,
            completed_time: 5,
        }],
    }]);
    character.quests.monthly = Some(BTreeSet::from([6]));
    character.spells = vec![NewSpell {
        spell: 42,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
    }];
    let mut transaction = SqlTransaction::new();
    super::super::super::append_creation_prefix(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let start = sqls
        .iter()
        .position(|sql| *sql == "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?")
        .unwrap();
    let inventory = sqls
        .iter()
        .position(|sql| sql.starts_with("REPLACE INTO character_inventory ("))
        .unwrap();
    assert_eq!(start - inventory, 7);
    assert_eq!(
        &sqls[start..],
        &[
            "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?",
            "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?",
            "DELETE FROM character_queststatus_objectives_spawn_tracking WHERE guid = ? AND quest = ?",
            "INSERT IGNORE INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, 1)",
            "DELETE FROM character_queststatus_daily WHERE guid = ?",
            "INSERT INTO character_queststatus_daily (guid, quest, time) VALUES (?, ?, ?)",
            "DELETE FROM character_queststatus_weekly WHERE guid = ?",
            "INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)",
            "DELETE FROM character_queststatus_seasonal WHERE guid = ?",
            "INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_queststatus_monthly WHERE guid = ?",
            "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)",
            "DELETE FROM character_glyphs WHERE guid = ?",
            "DELETE FROM character_talent WHERE guid = ?",
            "DELETE FROM character_pvp_talent WHERE guid = ?",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?",
            "DELETE FROM character_spell_cooldown WHERE guid = ?",
            "DELETE FROM character_spell_charges WHERE guid = ?",
        ]
    );
}
