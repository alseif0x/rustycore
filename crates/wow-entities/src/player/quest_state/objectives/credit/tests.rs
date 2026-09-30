use super::*;
use crate::{PlayerQuestStatusRecord, QuestObjective};
use std::collections::BTreeMap;
use wow_constants::quest::{QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, QUEST_OBJECTIVE_MONSTER_LIKE_CPP};

fn objective(id: u32, quest_id: u32, obj_type: u8, storage_index: i8) -> QuestObjective {
    QuestObjective {
        id, quest_id, obj_type, order: 0, storage_index, object_id: 42,
        amount: 3, flags: 0, flags2: 0, progress_bar_weight: 0.0, description: String::new(),
    }
}

fn status(quest_id: u32, status: u8, counts: Vec<i32>) -> PlayerQuestStatusRecord {
    PlayerQuestStatusRecord {
        quest_id, status, explored: false, accept_time_secs: 0, end_time_secs: 0,
        objective_counts: counts, slot: 0,
    }
}

fn state() -> PlayerQuestGameplayState {
    let mut state = PlayerQuestGameplayState::default();
    state.insert_status_like_cpp(1, status(1, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0]));
    state
}

#[test]
fn value_matching_queries_only_incomplete_statuses_in_btree_and_objective_order() {
    let mut state = PlayerQuestGameplayState::default();
    for (id, phase) in [(30, QUEST_STATUS_COMPLETE_LIKE_CPP), (20, QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        (15, QUEST_STATUS_INCOMPLETE_LIKE_CPP), (10, QUEST_STATUS_INCOMPLETE_LIKE_CPP)]
    {
        state.insert_status_like_cpp(id, status(id, phase, vec![]));
    }
    let definitions = BTreeMap::from([
        (10, vec![objective(102, 10, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 2),
            objective(101, 10, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 1),
            objective(101, 10, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 1)]),
        (20, vec![objective(201, 20, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 0)]),
    ]);
    let mut queries = Vec::new();
    let matching = state.plan_value_objective_credits(
        |id| {
            queries.push(id);
            definitions.get(&id).map(|rows| QuestObjectiveRulesLikeCpp::new(id, 0, 0, false, rows))
        },
        QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42, None,
    );
    assert_eq!(queries, vec![10, 15, 20]);
    assert_eq!(matching, vec![(10, 2, 3, 102), (10, 1, 3, 101), (10, 1, 3, 101), (20, 0, 3, 201)]);
    assert!(state.statuses_like_cpp()[&10].objective_counts.is_empty());
}

#[test]
fn value_matching_preserves_type_identity_storage_and_existing_admission_gaps() {
    let state = state();
    let mut rows = vec![
        objective(10, 1, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, -1),
        objective(11, 1, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 0),
        objective(12, 1, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 1),
    ];
    rows[1].object_id = 99;
    let rules = QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &rows);
    assert!(state.plan_value_objective_credits(|_| Some(rules), QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42, None).is_empty());
    // The represented value matcher does not add an IsStoringValue admission gate.
    assert_eq!(state.plan_value_objective_credits(|_| Some(rules), QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 42, None),
        vec![(1, 1, 3, 12)]);
    assert!(state.plan_value_objective_credits(|_| None, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42, None).is_empty());
}

#[test]
fn player_kill_matching_blocks_only_known_other_faction_for_flagged_objectives() {
    let state = state();
    let mut rows = vec![objective(10, 1, QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP, 0),
        objective(11, 1, QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP, 1)];
    rows[0].flags = QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP;
    let rules = QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &rows);
    for faction in [None, Some(true)] {
        assert_eq!(state.plan_value_objective_credits(|_| Some(rules), QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP, 42, faction),
            vec![(1, 0, 3, 10), (1, 1, 3, 11)]);
    }
    assert_eq!(state.plan_value_objective_credits(|_| Some(rules), QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP, 42, Some(false)),
        vec![(1, 1, 3, 11)]);
}

#[test]
fn flag_matching_preserves_duplicate_order_and_requires_storing_flag_type() {
    let mut state = state();
    state.insert_status_like_cpp(2, status(2, QUEST_STATUS_COMPLETE_LIKE_CPP, vec![]));
    let rows = vec![objective(12, 1, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 2),
        objective(11, 1, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 1),
        objective(11, 1, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 1),
        objective(13, 1, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, -1),
        objective(14, 1, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 0)];
    let rules = QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &rows);
    let mut queries = Vec::new();
    assert_eq!(state.plan_flag_objective_credits(|id| {
        queries.push(id); Some(rules)
    }, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 42), vec![(1, 2, 12), (1, 1, 11), (1, 1, 11)]);
    assert_eq!(queries, vec![1]);
    assert!(state.plan_flag_objective_credits(|_| Some(rules), QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42).is_empty());
    assert!(state.plan_flag_objective_credits(|_| Some(rules), QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 99).is_empty());
}

#[test]
fn value_resize_precedes_cap_guard_and_zero_changes_still_return_current() {
    let mut state = state();
    assert_eq!(state.apply_value_objective_credit(1, 3, 0, 1), None);
    assert_eq!(state.statuses_like_cpp()[&1].objective_counts, vec![0, 0, 0, 0]);
    assert_eq!(state.apply_value_objective_credit(1, 0, 3, 0), Some(0));
    assert_eq!(state.apply_value_objective_credit(1, 0, 3, -1), Some(0));
    assert_eq!(state.apply_value_objective_credit(1, 0, 3, 5), Some(3));
    assert_eq!(state.apply_value_objective_credit(1, 0, 3, 0), None);
    assert_eq!(state.apply_value_objective_credit(1, 0, 3, 1), None);
    assert_eq!(state.apply_value_objective_credit(1, 0, 3, -1), Some(2));
}

#[test]
fn value_transition_saturates_clamps_and_does_not_recheck_snapshot_status() {
    let mut state = state();
    state.status_mut_like_cpp(1).unwrap().status = QUEST_STATUS_COMPLETE_LIKE_CPP;
    state.status_mut_like_cpp(1).unwrap().objective_counts[0] = i32::MAX - 1;
    assert_eq!(state.apply_value_objective_credit(1, 0, i32::MAX, 100), Some(i32::MAX));
    assert_eq!(state.apply_value_objective_credit(1, 0, i32::MAX, i32::MIN), Some(0));
    assert_eq!(state.statuses_like_cpp()[&1].status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(state.apply_value_objective_credit(99, 0, 3, 1), None);
    assert!(!state.statuses_like_cpp().contains_key(&99));
}

#[test]
fn duplicate_candidates_keep_separate_transitions_including_later_noop() {
    let mut state = state();
    let mut rows = vec![objective(10, 1, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 0); 2];
    for row in &mut rows { row.amount = 1; }
    let matching = state.plan_value_objective_credits(
        |_| Some(QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &rows)),
        QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42, None,
    );
    assert_eq!(matching.len(), 2);
    let outcomes = matching.into_iter().map(|(quest_id, index, required, _)| {
        state.apply_value_objective_credit(quest_id, index, required, 1)
    }).collect::<Vec<_>>();
    assert_eq!(outcomes, vec![Some(1), None]);
}

#[test]
fn flag_transition_resizes_and_writes_even_without_completion_change() {
    let mut state = state();
    assert_eq!(state.apply_flag_objective_credit(1, 2, 1), Some((false, true)));
    assert_eq!(state.statuses_like_cpp()[&1].objective_counts, vec![0, 0, 1]);
    assert_eq!(state.apply_flag_objective_credit(1, 2, i32::MAX), Some((true, true)));
    state.status_mut_like_cpp(1).unwrap().objective_counts[2] = -9;
    assert_eq!(state.apply_flag_objective_credit(1, 2, 1), Some((true, true)));
    assert_eq!(state.statuses_like_cpp()[&1].objective_counts[2], 1);
    assert_eq!(state.apply_flag_objective_credit(1, 2, 0), Some((true, false)));
    assert_eq!(state.apply_flag_objective_credit(1, 2, -1), Some((false, false)));
    assert_eq!(state.apply_flag_objective_credit(99, 2, 1), None);
}

#[test]
fn threshold_loss_reopens_only_complete_status_and_preserves_other_fields() {
    let mut state = state();
    let mut complete = status(2, QUEST_STATUS_COMPLETE_LIKE_CPP, vec![7, 8]);
    complete.explored = true;
    complete.end_time_secs = 99;
    state.insert_status_like_cpp(2, complete);
    assert!(!state.reopen_quest_after_threshold_loss(1));
    assert!(!state.reopen_quest_after_threshold_loss(99));
    assert!(state.reopen_quest_after_threshold_loss(2));
    assert!(!state.reopen_quest_after_threshold_loss(2));
    let status = &state.statuses_like_cpp()[&2];
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(status.objective_counts, vec![7, 8]);
    assert!(status.explored);
    assert_eq!(status.end_time_secs, 99);
}

#[test]
fn completion_probe_defers_projection_until_status_exists_and_preserves_rewarded_gate() {
    let mut state = state();
    assert!(!state.can_complete_after_objective(99, 0, || panic!("missing status must skip projection")));
    let rows = [objective(10, 1, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 0)];
    assert!(state.can_complete_after_objective(1, 10, || QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &rows)));
    assert!(!state.can_complete_after_objective(1, 0, || QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &rows)));
    state.set_rewarded_like_cpp(1, true);
    assert!(!state.can_complete_after_objective(1, 10, || QuestObjectiveRulesLikeCpp::new(1, 0, 0, false, &rows)));
    assert!(state.can_complete_after_objective(1, 10, || QuestObjectiveRulesLikeCpp::new(1, 0, 0, true, &rows)));
}

#[test]
#[should_panic]
fn negative_required_retains_clamp_panic_when_negative_credit_reaches_write() {
    let mut state = state();
    state.apply_value_objective_credit(1, 0, -1, -1);
}

#[test]
fn negative_required_nonnegative_credit_retains_guard_after_resize() {
    let mut state = state();
    assert_eq!(state.apply_value_objective_credit(1, 2, -1, 1), None);
    assert_eq!(state.statuses_like_cpp()[&1].objective_counts, vec![0, 0, 0]);
}
