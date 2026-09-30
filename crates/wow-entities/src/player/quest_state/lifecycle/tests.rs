//! Original pure quest-log scenarios with immutable model-row fixtures.

use super::*;
use crate::QuestObjective;
use std::collections::BTreeMap;
use wow_constants::quest::{
    MAX_QUEST_LOG_SIZE as MAX_QUEST_LOG_SIZE_LIKE_CPP, QUEST_OBJECTIVE_MONSTER_LIKE_CPP,
};

#[derive(Default)]
struct QuestLogFixture {
    state: PlayerQuestGameplayState,
    definitions: BTreeMap<u32, Vec<QuestObjective>>,
}

fn make_fixture() -> (QuestLogFixture, ()) {
    (QuestLogFixture::default(), ())
}

fn add_active_quest_in_slot(fixture: &mut QuestLogFixture, quest_id: u32, slot: u8) {
    add_active_quest_in_slot_with_status(fixture, quest_id, slot, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
}

fn add_active_quest_in_slot_with_status(
    fixture: &mut QuestLogFixture,
    quest_id: u32,
    slot: u8,
    status: u8,
) {
    let mut record = PlayerQuestGameplayState::prepare_quest_start(quest_id, slot, 0, 0, 0);
    record.status = status;
    fixture.state.insert_status_like_cpp(quest_id, record);
}

fn with_player_quest_status_mut_for_test<R>(
    fixture: &mut QuestLogFixture,
    quest_id: u32,
    operation: impl FnOnce(&mut PlayerQuestStatusRecord) -> R,
) -> Option<R> {
    fixture.state.status_mut_like_cpp(quest_id).map(operation)
}

fn quest_log_create_entries_for_test(fixture: &QuestLogFixture) -> Vec<(u32, u32, i64, [u16; 24])> {
    (0..MAX_QUEST_LOG_SIZE)
        .map(|slot| {
            let Some(quest_id) = fixture.state.quest_id_at_slot(slot) else {
                return (0, 0, 0, [0; 24]);
            };
            fixture.state.quest_log_entry(quest_id, |id| {
                fixture
                    .definitions
                    .get(&id)
                    .map(|rows| QuestObjectiveRulesLikeCpp::new(id, 0, 0, false, rows))
            })
        })
        .collect()
}

fn insert_rewarded_quest_for_test(fixture: &mut QuestLogFixture, quest_id: u32) {
    fixture.state.set_rewarded_like_cpp(quest_id, true);
}

fn remove_represented_active_rewarded_duplicates_for_test(
    fixture: &mut QuestLogFixture,
) -> Vec<u32> {
    let ids = fixture
        .state
        .plan_rewarded_active_duplicates(|id| fixture.definitions.get(&id).map(|_| false));
    if !ids.is_empty() {
        fixture.state.remove_rewarded_active_duplicates(&ids);
    }
    ids
}

fn contains_player_quest_status_for_test(fixture: &QuestLogFixture, quest_id: u32) -> bool {
    fixture.state.statuses_like_cpp().contains_key(&quest_id)
}

fn player_quest_status_for_test(
    fixture: &QuestLogFixture,
    quest_id: u32,
) -> Option<&PlayerQuestStatusRecord> {
    fixture.state.status_like_cpp(quest_id)
}

#[test]
fn quest_log_create_entries_preserve_explicit_slot_holes_like_cpp() {
    let (mut session, _send_rx) = make_fixture();
    add_active_quest_in_slot(&mut session, 5916, 9);
    add_active_quest_in_slot(&mut session, 5915, 2);

    let entries = quest_log_create_entries_for_test(&session);

    assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
    assert_eq!(entries[0], (0, 0, 0, [0; 24]));
    assert_eq!(entries[2].0, 5915);
    assert_eq!(entries[9].0, 5916);
}

#[test]
fn quest_log_create_entries_preserve_end_time_like_cpp() {
    let (mut session, _send_rx) = make_fixture();
    add_active_quest_in_slot(&mut session, 5917, 4);
    with_player_quest_status_mut_for_test(&mut session, 5917, |status| {
        status.end_time_secs = 123_456;
    })
    .expect("active quest");

    let entries = quest_log_create_entries_for_test(&session);

    assert_eq!(entries[4].2, 123_456);
}

#[test]
fn quest_load_removes_active_rewarded_duplicate_and_compacts_slots_like_cpp() {
    let (mut session, _send_rx) = make_fixture();
    let duplicate_quest_id = 5923;
    let active_quest_id = 5924;
    session.definitions.insert(duplicate_quest_id, Vec::new());
    session.definitions.insert(active_quest_id, Vec::new());
    add_active_quest_in_slot_with_status(
        &mut session,
        duplicate_quest_id,
        0,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
    );
    add_active_quest_in_slot_with_status(
        &mut session,
        active_quest_id,
        3,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    insert_rewarded_quest_for_test(&mut session, duplicate_quest_id);

    assert_eq!(
        remove_represented_active_rewarded_duplicates_for_test(&mut session),
        vec![duplicate_quest_id]
    );
    assert!(!contains_player_quest_status_for_test(
        &session,
        duplicate_quest_id
    ));
    assert_eq!(
        player_quest_status_for_test(&session, active_quest_id).map(|status| status.slot),
        Some(0)
    );
}

#[test]
fn quest_log_create_entries_store_flag_objectives_in_state_flags_like_cpp() {
    let (mut session, _send_rx) = make_fixture();
    let quest_id = 5918;
    let mut objectives = Vec::new();
    objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: 10,
        order: 1,
        storage_index: 1,
        object_id: 45,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.definitions.insert(quest_id, objectives);
    add_active_quest_in_slot(&mut session, quest_id, 3);
    with_player_quest_status_mut_for_test(&mut session, quest_id, |status| {
        status.objective_counts = vec![3, 1];
    })
    .expect("active quest");

    let entries = quest_log_create_entries_for_test(&session);

    assert_eq!(entries[3].1, 256 << 1);
    assert_eq!(entries[3].3[0], 3);
    assert_eq!(
        entries[3].3[1], 0,
        "C++ stores flag objectives in QuestLog.StateFlags, not ObjectiveProgress"
    );
}

#[test]
fn quest_log_create_entries_preserve_failed_state_flag_like_cpp() {
    let (mut session, _send_rx) = make_fixture();
    add_active_quest_in_slot_with_status(&mut session, 5919, 5, QUEST_STATUS_FAILED_LIKE_CPP);

    let entries = quest_log_create_entries_for_test(&session);

    assert_eq!(entries[5].1, 2);
}

#[test]
fn quest_log_create_entries_duplicate_slot_is_empty_fail_closed_like_cpp() {
    let (mut session, _send_rx) = make_fixture();
    add_active_quest_in_slot(&mut session, 5915, 2);
    add_active_quest_in_slot(&mut session, 5916, 2);

    let entries = quest_log_create_entries_for_test(&session);

    assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
    assert_eq!(entries[2], (0, 0, 0, [0; 24]));
    assert!(contains_player_quest_status_for_test(&session, 5915));
    assert!(contains_player_quest_status_for_test(&session, 5916));
}

#[test]
fn start_record_preserves_supplied_times_slot_and_objective_length() {
    let record = PlayerQuestGameplayState::prepare_quest_start(17, 255, -4, -9, 3);
    assert_eq!(
        record,
        PlayerQuestStatusRecord {
            quest_id: 17,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: -4,
            end_time_secs: -9,
            objective_counts: vec![0, 0, 0],
            slot: 255,
        }
    );
    let mut state = PlayerQuestGameplayState::default();
    assert!(
        !state.statuses_like_cpp().contains_key(&17),
        "preparation does not enter the writer"
    );
    state.insert_status_like_cpp(17, record.clone());
    assert_eq!(state.status_like_cpp(17), Some(&record));
    assert!(
        PlayerQuestGameplayState::prepare_quest_start(17, 0, 0, 0, 0)
            .objective_counts
            .is_empty()
    );
}

#[test]
fn completion_writer_guards_current_status_and_preserves_other_fields() {
    let (mut fixture, _) = make_fixture();
    add_active_quest_in_slot(&mut fixture, 17, 4);
    let before = fixture.state.status_like_cpp(17).unwrap().clone();
    assert_eq!(fixture.state.complete_quest(99), None);
    assert_eq!(
        fixture.state.complete_quest(17),
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert_eq!(fixture.state.complete_quest(17), None);
    let mut expected = before;
    expected.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
    assert_eq!(fixture.state.status_like_cpp(17), Some(&expected));
    fixture.state.status_mut_like_cpp(17).unwrap().status = QUEST_STATUS_FAILED_LIKE_CPP;
    expected.status = QUEST_STATUS_FAILED_LIKE_CPP;
    assert_eq!(fixture.state.complete_quest(17), None);
    assert_eq!(fixture.state.status_like_cpp(17), Some(&expected));
}

#[test]
fn reward_settlement_preserves_repeatable_membership_and_missing_status_behavior() {
    let (mut fixture, _) = make_fixture();
    for id in [1, 2] {
        add_active_quest_in_slot(&mut fixture, id, id as u8);
    }
    fixture.state.set_rewarded_like_cpp(2, true);
    fixture
        .state
        .set_objective_counts_for_quest_like_cpp(1, vec![7]);
    fixture.state.settle_rewarded_quest(1, false);
    fixture.state.settle_rewarded_quest(2, true);
    fixture.state.settle_rewarded_quest(3, true);
    fixture.state.settle_rewarded_quest(4, false);
    assert!(fixture.state.statuses_like_cpp().is_empty());
    assert_eq!(
        fixture
            .state
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1, 2, 4]
    );
    assert_eq!(
        fixture.state.objective_counts_by_quest_like_cpp(),
        &[(1, vec![7])],
        "the existing independent count representation is not repaired during settlement"
    );
}

#[test]
fn timer_and_explored_transitions_preserve_noops_and_failed_admission() {
    let (mut fixture, _) = make_fixture();
    add_active_quest_in_slot(&mut fixture, 1, 1);
    add_active_quest_in_slot_with_status(&mut fixture, 2, 2, QUEST_STATUS_FAILED_LIKE_CPP);
    assert!(!fixture.state.clear_quest_end_time(99));
    fixture.state.status_mut_like_cpp(1).unwrap().end_time_secs = -1;
    assert!(!fixture.state.clear_quest_end_time(1));
    assert_eq!(fixture.state.status_like_cpp(1).unwrap().end_time_secs, -1);
    fixture.state.status_mut_like_cpp(1).unwrap().end_time_secs = 12;
    assert!(fixture.state.clear_quest_end_time(1));
    assert!(!fixture.state.clear_quest_end_time(1));
    assert_eq!(fixture.state.mark_quest_explored(99), (false, false));
    assert_eq!(fixture.state.mark_quest_explored(2), (true, false));
    assert!(!fixture.state.status_like_cpp(2).unwrap().explored);
    assert_eq!(fixture.state.mark_quest_explored(1), (true, true));
    assert_eq!(fixture.state.mark_quest_explored(1), (true, false));
}

#[test]
fn completion_probe_defers_rules_until_status_exists_and_preserves_projection_before_helper_gates()
{
    let (mut fixture, _) = make_fixture();
    assert!(
        !fixture
            .state
            .can_complete_started_quest(99, 0, || panic!("missing status must skip rules"))
    );
    add_active_quest_in_slot_with_status(&mut fixture, 1, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
    let calls = std::cell::Cell::new(0);
    assert!(!fixture.state.can_complete_started_quest(1, 0, || {
        calls.set(calls.get() + 1);
        QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &[])
    }));
    fixture.state.status_mut_like_cpp(1).unwrap().status = QUEST_STATUS_INCOMPLETE_LIKE_CPP;
    fixture.state.set_rewarded_like_cpp(1, true);
    assert!(!fixture.state.can_complete_started_quest(1, 0, || {
        calls.set(calls.get() + 1);
        QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &[])
    }));
    assert_eq!(
        calls.get(),
        2,
        "present status projects rules before the helper's status/rewarded gates"
    );
    assert!(
        fixture
            .state
            .can_complete_started_quest(1, 0, || QuestObjectiveRulesLikeCpp::new(
                1,
                0,
                0,
                true,
                &[]
            ))
    );
}

#[test]
fn slot_readers_preserve_active_states_ambiguity_and_record_id_key_distinction() {
    let mut state = PlayerQuestGameplayState::default();
    for phase in [
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        QUEST_STATUS_FAILED_LIKE_CPP,
    ] {
        let mut record = PlayerQuestGameplayState::prepare_quest_start(99, 2, 0, 0, 0);
        record.status = phase;
        state.insert_status_like_cpp(1, record);
        assert!(state.slot_has_active_entry(2));
        assert_eq!(state.quest_id_at_slot(2), Some(99));
        assert_eq!(state.slot_for_quest(1), Some(2));
        assert_eq!(state.slot_for_quest(99), None);
    }
    assert_eq!(
        state.quest_log_entry(99, |_| panic!(
            "stored ID without a matching map key must skip lookup"
        )),
        (0, 0, 0, [0; 24])
    );
    state.insert_status_like_cpp(
        2,
        PlayerQuestGameplayState::prepare_quest_start(2, 2, 0, 0, 0),
    );
    assert_eq!(state.quest_id_at_slot(2), None);
    assert!(
        state.slot_has_active_entry(2),
        "ambiguous slots stay occupied"
    );
    assert_eq!(
        state.slot_for_quest(1),
        Some(2),
        "find-by-key does not add an ambiguity gate"
    );
    for phase in [0, 6] {
        state.status_mut_like_cpp(1).unwrap().status = phase;
        state.status_mut_like_cpp(2).unwrap().status = phase;
        assert!(!state.slot_has_active_entry(2));
        assert_eq!(state.quest_id_at_slot(2), None);
        assert_eq!(state.slot_for_quest(1), None);
    }
    assert_eq!(state.quest_id_at_slot(MAX_QUEST_LOG_SIZE), None);
    assert!(!state.slot_has_active_entry(MAX_QUEST_LOG_SIZE));
}

#[test]
fn entry_projection_preserves_lazy_record_id_lookup_negative_casts_and_flag_shift() {
    let mut state = PlayerQuestGameplayState::default();
    let mut record = PlayerQuestGameplayState::prepare_quest_start(99, 0, 0, 123, 24);
    record.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
    record.objective_counts[0] = -1;
    record.objective_counts[1] = i32::MIN;
    record.objective_counts[2] = i32::MAX;
    record.objective_counts[23] = -7;
    state.insert_status_like_cpp(1, record);
    assert_eq!(
        state.quest_log_entry(2, |_| panic!("absent status skips catalog lookup")),
        (0, 0, 0, [0; 24])
    );
    let rows = [QuestObjective {
        id: 1,
        quest_id: 99,
        obj_type: 10,
        order: 0,
        storage_index: 23,
        object_id: 42,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    let entry = state.quest_log_entry(1, |id| {
        assert_eq!(id, 99);
        Some(QuestObjectiveRulesLikeCpp::new(id, 0, 0, false, &rows))
    });
    assert_eq!(entry.0, 99);
    assert_eq!(
        entry.1,
        QUEST_STATE_COMPLETE | (QUEST_STATE_OBJECTIVE_FLAG_BASE << 23)
    );
    assert_eq!(entry.2, 123);
    assert_eq!(&entry.3[..3], &[u16::MAX, 0, u16::MAX]);
    assert_eq!(entry.3[23], 0);
    assert_eq!(state.quest_log_entry(1, |_| None).3[23], (-7i32) as u16);
}

#[test]
fn duplicate_planner_queries_only_rewarded_map_keys_and_preserves_missing_repeatable_rows() {
    let mut state = PlayerQuestGameplayState::default();
    for key in [40, 20, 30, 10] {
        state.insert_status_like_cpp(
            key,
            PlayerQuestGameplayState::prepare_quest_start(key + 100, 0, 0, 0, 0),
        );
    }
    for key in [10, 30, 40] {
        state.set_rewarded_like_cpp(key, true);
    }
    let mut queries = Vec::new();
    let ids = state.plan_rewarded_active_duplicates(|id| {
        queries.push(id);
        match id {
            10 => Some(false),
            30 => Some(true),
            _ => None,
        }
    });
    assert_eq!(queries, vec![10, 30, 40]);
    assert_eq!(ids, vec![10]);
    assert!(state.plan_rewarded_active_duplicates(|_| None).is_empty());
}

#[test]
fn duplicate_writer_removes_without_rechecking_and_stably_compacts_all_remaining_statuses() {
    let mut state = PlayerQuestGameplayState::default();
    for (key, slot) in [(5, 9), (4, 7), (3, 8), (2, 7)] {
        let mut record = PlayerQuestGameplayState::prepare_quest_start(key + 100, slot, 12, 34, 1);
        record.status = 0;
        state.insert_status_like_cpp(key, record);
    }
    state.remove_rewarded_active_duplicates(&[3, 99]);
    assert!(!state.statuses_like_cpp().contains_key(&3));
    assert_eq!(state.status_like_cpp(2).unwrap().slot, 0);
    assert_eq!(state.status_like_cpp(4).unwrap().slot, 1);
    assert_eq!(state.status_like_cpp(5).unwrap().slot, 2);
    assert_eq!(state.status_like_cpp(5).unwrap().quest_id, 105);
    assert_eq!(state.status_like_cpp(5).unwrap().status, 0);
    assert_eq!(state.status_like_cpp(5).unwrap().end_time_secs, 34);
    assert!(state.rewarded_quest_ids_like_cpp().is_empty());
}

#[test]
fn compaction_retains_u8_overflow_fallback_without_quest_log_size_clamp() {
    let mut state = PlayerQuestGameplayState::default();
    for id in 0..259 {
        state.insert_status_like_cpp(
            id,
            PlayerQuestGameplayState::prepare_quest_start(id, 1, 0, 0, 0),
        );
    }
    state.remove_rewarded_active_duplicates(&[0]);
    assert_eq!(state.status_like_cpp(26).unwrap().slot, 25);
    assert_eq!(state.status_like_cpp(256).unwrap().slot, 255);
    assert_eq!(
        state.status_like_cpp(257).unwrap().slot,
        MAX_QUEST_LOG_SIZE - 1
    );
    assert_eq!(
        state.status_like_cpp(258).unwrap().slot,
        MAX_QUEST_LOG_SIZE - 1
    );
}
