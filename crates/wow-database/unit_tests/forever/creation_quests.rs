use super::*;
use crate::SqlParam;
use std::collections::BTreeMap;
use wow_persistence::forever::creation::save::NewSpell;

fn status(value: u8) -> NewQuestStatus {
    NewQuestStatus {
        status: value,
        explored: false,
        accept_time: 0,
        end_time: 0,
        objectives: Vec::new(),
        spawn_tracking: Vec::new(),
    }
}

fn save(keep_abandoned: bool) -> NewQuestSave {
    NewQuestSave {
        keep_abandoned,
        statuses: BTreeMap::new(),
        rewarded: BTreeMap::new(),
        daily: None,
        weekly: None,
        seasonal: None,
        monthly: None,
    }
}

#[test]
fn absent_and_missing_default_statuses_are_noops_and_abandoned_status_obeys_policy() {
    for keep_abandoned in [false, true] {
        let mut save = save(keep_abandoned);
        assert_eq!(statements(17, &save).count(), 0);
        save.statuses.insert(1, NewQuestChange::Default(None));
        assert_eq!(statements(17, &save).count(), 0);
        save.statuses
            .insert(2, NewQuestChange::Default(Some(status(0))));
        let rows: Vec<_> = statements(17, &save).collect();
        if keep_abandoned {
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[0].sql(), STATUS_REPLACE);
            assert_eq!(rows[0].params()[1], SqlParam::U32(2));
        } else {
            assert!(rows.is_empty());
        }
        assert_eq!(
            status_statements(
                17,
                3,
                &NewQuestChange::Default(Some(status(1))),
                keep_abandoned
            )
            .len(),
            3
        );
    }
}

#[test]
fn explicit_status_delete_always_removes_all_three_groups_in_order_and_scope() {
    for keep_abandoned in [false, true] {
        let rows = status_statements(u64::MAX, u32::MAX, &NewQuestChange::Delete, keep_abandoned);
        assert_eq!(
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
            [
                "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?",
                "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?",
                "DELETE FROM character_queststatus_objectives_spawn_tracking WHERE guid = ? AND quest = ?",
            ]
        );
        for row in rows {
            assert_eq!(
                row.params(),
                &[SqlParam::U64(u64::MAX), SqlParam::U32(u32::MAX)]
            );
        }
    }
}

#[test]
fn status_row_preserves_u8_bool_and_signed_i64_time_bounds() {
    for (explored, accept, end) in [(false, i64::MIN, i64::MAX), (true, i64::MAX, i64::MIN)] {
        let mut data = status(u8::MAX);
        data.explored = explored;
        data.accept_time = accept;
        data.end_time = end;
        let row = status_statement(u64::MAX, u32::MAX, &data);
        assert_eq!(
            row.sql(),
            "REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)"
        );
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U32(u32::MAX),
                SqlParam::U8(u8::MAX),
                SqlParam::Bool(explored),
                SqlParam::I64(accept),
                SqlParam::I64(end),
            ]
        );
    }
}

#[test]
fn objectives_skip_only_zero_counts_and_spawn_tracking_keeps_supplied_order_and_zero_ids() {
    let mut data = status(1);
    data.objectives = vec![
        NewQuestObjective {
            index: 200,
            count: i32::MIN,
        },
        NewQuestObjective { index: 1, count: 0 },
        NewQuestObjective {
            index: 2,
            count: i32::MAX,
        },
        NewQuestObjective {
            index: u8::MAX,
            count: -1,
        },
    ];
    data.spawn_tracking = vec![
        NewQuestSpawnTracking {
            index: u8::MAX,
            id: 0,
        },
        NewQuestSpawnTracking {
            index: 2,
            id: u32::MAX,
        },
        NewQuestSpawnTracking { index: 1, id: 7 },
    ];
    let rows = status_statements(17, 23, &NewQuestChange::Default(Some(data)), false);
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            STATUS_REPLACE,
            OBJECTIVE_DELETE,
            "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)",
            "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)",
            "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)",
            SPAWN_DELETE,
            "REPLACE INTO character_queststatus_objectives_spawn_tracking (guid, quest, objective, spawnTrackingId) VALUES (?, ?, ?, ?)",
            "REPLACE INTO character_queststatus_objectives_spawn_tracking (guid, quest, objective, spawnTrackingId) VALUES (?, ?, ?, ?)",
            "REPLACE INTO character_queststatus_objectives_spawn_tracking (guid, quest, objective, spawnTrackingId) VALUES (?, ?, ?, ?)",
        ]
    );
    for (row, (index, count)) in rows[2..5]
        .iter()
        .zip([(200, i32::MIN), (2, i32::MAX), (255, -1)])
    {
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(17),
                SqlParam::U32(23),
                SqlParam::U8(index),
                SqlParam::I32(count),
            ]
        );
    }
    for (row, (index, id)) in rows[6..].iter().zip([(255, 0), (2, u32::MAX), (1, 7)]) {
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(17),
                SqlParam::U32(23),
                SqlParam::U8(index),
                SqlParam::U32(id),
            ]
        );
    }
    for row in &rows {
        assert_eq!(&row.params()[..2], &[SqlParam::U64(17), SqlParam::U32(23)]);
    }
}

#[test]
fn rewarded_default_is_insert_ignore_and_delete_policy_distinguishes_force_delete() {
    for keep_abandoned in [false, true] {
        for (change, expected) in [
            (
                NewRewardedQuestChange::Default,
                Some(
                    "INSERT IGNORE INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, 1)",
                ),
            ),
            (
                NewRewardedQuestChange::Delete,
                if keep_abandoned {
                    None
                } else {
                    Some("DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?")
                },
            ),
            (
                NewRewardedQuestChange::ForceDelete,
                Some("DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?"),
            ),
        ] {
            let row = rewarded_statement(u64::MAX, u32::MAX, &change, keep_abandoned);
            assert_eq!(row.as_ref().map(|row| row.sql()), expected);
            if let Some(row) = row {
                assert_eq!(
                    row.params(),
                    &[SqlParam::U64(u64::MAX), SqlParam::U32(u32::MAX)]
                );
            }
        }
    }
}

#[test]
fn ordered_status_queue_finishes_before_ordered_reward_queue_regardless_of_insertion_order() {
    for ids in [[90, 10], [10, 90]] {
        let mut save = save(false);
        for quest in ids {
            save.statuses.insert(
                quest,
                if quest == 10 {
                    NewQuestChange::Default(Some(status(1)))
                } else {
                    NewQuestChange::Delete
                },
            );
        }
        for quest in ids.map(|quest| if quest == 10 { 1 } else { 80 }) {
            save.rewarded.insert(
                quest,
                if quest == 1 {
                    NewRewardedQuestChange::Default
                } else {
                    NewRewardedQuestChange::ForceDelete
                },
            );
        }
        let rows: Vec<_> = statements(29, &save).collect();
        assert_eq!(
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
            [
                STATUS_REPLACE,
                OBJECTIVE_DELETE,
                SPAWN_DELETE,
                STATUS_DELETE,
                OBJECTIVE_DELETE,
                SPAWN_DELETE,
                REWARDED_INSERT,
                REWARDED_DELETE,
            ]
        );
        for (row, quest) in rows.iter().zip([10, 10, 10, 90, 90, 90, 1, 80]) {
            assert_eq!(
                &row.params()[..2],
                &[SqlParam::U64(29), SqlParam::U32(quest)]
            );
        }
        let mut transaction = crate::forever::create::tests::isolated_transaction();
        append(&mut transaction, 29, &save);
        assert_eq!(
            transaction.sqls_for_test(),
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn quest_queues_follow_all_inventory_items_and_precede_spells_in_the_production_prefix() {
    let mut character = super::super::tests::character();
    let first = super::super::items::tests::item();
    let mut second = super::super::items::tests::item();
    second.guid = 43;
    second.slot = 20;
    character.items = vec![first, second];
    character
        .quests
        .statuses
        .insert(23, NewQuestChange::Default(Some(status(1))));
    character
        .quests
        .rewarded
        .insert(7, NewRewardedQuestChange::Default);
    character.spells = vec![NewSpell {
        spell: 42,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
    }];
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_creation_prefix(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let inventory: Vec<_> = sqls
        .iter()
        .enumerate()
        .filter_map(|(index, sql)| {
            sql.starts_with("REPLACE INTO character_inventory (")
                .then_some(index)
        })
        .collect();
    assert_eq!(inventory.len(), 2);
    assert_eq!(inventory[1] - inventory[0], 7);
    let quest_start = sqls.iter().position(|sql| *sql == STATUS_REPLACE).unwrap();
    assert_eq!(quest_start - inventory[1], 7);
    assert_eq!(
        sqls[quest_start - 1],
        "DELETE FROM item_instance_modifiers WHERE itemGuid = ?"
    );
    assert_eq!(
        &sqls[quest_start..],
        &[
            STATUS_REPLACE,
            OBJECTIVE_DELETE,
            SPAWN_DELETE,
            REWARDED_INSERT,
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
