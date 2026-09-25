use super::super::PlayerQuestGameplayState;
use crate::{PlayerQuestStatusRecord, QuestObjective, QuestObjectiveRulesLikeCpp};
use std::collections::BTreeMap;
use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP, QUEST_OBJECTIVE_ITEM_LIKE_CPP,
    QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};

fn item_objective(
    id: u32,
    quest_id: u32,
    object_id: i32,
    amount: i32,
    storage_index: i8,
    flags2: u32,
) -> QuestObjective {
    QuestObjective {
        id,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        order: 0,
        storage_index,
        object_id,
        amount,
        flags: 0,
        flags2,
        progress_bar_weight: 0.0,
        description: String::new(),
    }
}

fn status(quest_id: u32, slot: u8, objective_counts: Vec<i32>) -> PlayerQuestStatusRecord {
    PlayerQuestStatusRecord {
        quest_id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts,
        slot,
    }
}

fn state_with_statuses(
    statuses: BTreeMap<u32, PlayerQuestStatusRecord>,
) -> PlayerQuestGameplayState {
    let mut state = PlayerQuestGameplayState::default();
    state.replace_statuses_like_cpp(statuses, true);
    state
}

#[test]
fn filtered_non_bound_progress_skips_bound_objectives() {
    let bound_id = 8101;
    let non_bound_id = 8102;
    let bound_objectives = vec![item_objective(
        bound_id * 10,
        bound_id,
        500,
        2,
        0,
        QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
    )];
    let non_bound_objectives = vec![item_objective(
        non_bound_id * 10,
        non_bound_id,
        500,
        2,
        0,
        0,
    )];
    let bound_quest = QuestObjectiveRulesLikeCpp::new(bound_id, 0, 0, false, &bound_objectives);
    let non_bound_quest =
        QuestObjectiveRulesLikeCpp::new(non_bound_id, 0, 0, false, &non_bound_objectives);
    let mut state = state_with_statuses(BTreeMap::from([
        (bound_id, status(bound_id, 0, vec![0])),
        (non_bound_id, status(non_bound_id, 1, vec![0])),
    ]));

    let outcome = state.apply_item_objective_progress_like_cpp(
        |id| match id {
            8101 => Some(bound_quest),
            8102 => Some(non_bound_quest),
            _ => None,
        },
        &[500],
        1,
        Some(false),
    );

    assert_eq!(outcome.changed_quest_ids, vec![non_bound_id]);
    assert!(outcome.quests_to_complete.is_empty());
    assert_eq!(outcome.objective_updates, vec![(1, false)]);
    assert_eq!(
        state.statuses_like_cpp()[&bound_id].objective_counts,
        vec![0]
    );
    assert_eq!(
        state.statuses_like_cpp()[&non_bound_id].objective_counts,
        vec![1]
    );
}

#[test]
fn no_filter_stops_after_first_bound_quest_in_btree_order() {
    let first_id = 8110;
    let later_id = 8111;
    let first_objectives = vec![item_objective(
        first_id * 10,
        first_id,
        501,
        2,
        0,
        QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
    )];
    let later_objectives = vec![item_objective(later_id * 10, later_id, 501, 2, 0, 0)];
    let first_quest = QuestObjectiveRulesLikeCpp::new(first_id, 0, 0, false, &first_objectives);
    let later_quest = QuestObjectiveRulesLikeCpp::new(later_id, 0, 0, false, &later_objectives);
    let mut state = state_with_statuses(BTreeMap::from([
        (later_id, status(later_id, 0, vec![0])),
        (first_id, status(first_id, 4, vec![0])),
    ]));

    let outcome = state.apply_item_objective_progress_like_cpp(
        |id| match id {
            8110 => Some(first_quest),
            8111 => Some(later_quest),
            _ => None,
        },
        &[501],
        1,
        None,
    );

    assert_eq!(outcome.changed_quest_ids, vec![first_id]);
    assert_eq!(outcome.objective_updates, vec![(1, true)]);
    assert!(outcome.quests_to_complete.is_empty());
    assert_eq!(
        state.statuses_like_cpp()[&first_id].objective_counts,
        vec![1]
    );
    assert_eq!(
        state.statuses_like_cpp()[&later_id].objective_counts,
        vec![0]
    );
}

#[test]
fn explicit_bound_order_selects_the_caller_selected_quest() {
    let first_id = 8120;
    let selected_id = 8121;
    let first_objectives = vec![item_objective(
        first_id * 10,
        first_id,
        502,
        2,
        0,
        QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
    )];
    let selected_objectives = vec![item_objective(
        selected_id * 10,
        selected_id,
        502,
        2,
        0,
        QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
    )];
    let first_quest = QuestObjectiveRulesLikeCpp::new(first_id, 0, 0, false, &first_objectives);
    let selected_quest =
        QuestObjectiveRulesLikeCpp::new(selected_id, 0, 0, false, &selected_objectives);
    let mut state = state_with_statuses(BTreeMap::from([
        (first_id, status(first_id, 0, vec![0])),
        (selected_id, status(selected_id, 1, vec![0])),
    ]));

    let outcome = state.apply_bound_item_objective_progress_like_cpp(
        |id| match id {
            8120 => Some(first_quest),
            8121 => Some(selected_quest),
            _ => None,
        },
        vec![selected_id, first_id],
        502,
        2,
    );

    assert_eq!(outcome.updated_counts, vec![(selected_id, 2)]);
    assert_eq!(outcome.quests_to_complete, vec![selected_id]);
    assert_eq!(
        state.statuses_like_cpp()[&first_id].objective_counts,
        vec![0]
    );
    assert_eq!(
        state.statuses_like_cpp()[&selected_id].objective_counts,
        vec![2]
    );
    assert_eq!(
        state.statuses_like_cpp()[&selected_id].status,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
}

#[test]
fn completion_ids_are_returned_while_status_stays_incomplete_for_caller() {
    let quest_id = 8130;
    let objectives = vec![item_objective(quest_id * 10, quest_id, 503, 1, 0, 0)];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let mut state = state_with_statuses(BTreeMap::from([(quest_id, status(quest_id, 0, vec![0]))]));

    let outcome =
        state.apply_item_objective_progress_like_cpp(|_| Some(quest), &[503], 1, Some(false));

    assert_eq!(outcome.changed_quest_ids, vec![quest_id]);
    assert_eq!(outcome.quests_to_complete, vec![quest_id]);
    assert_eq!(outcome.objective_updates, vec![(1, false)]);
    assert_eq!(
        state.statuses_like_cpp()[&quest_id].status,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
}

#[test]
fn missing_catalog_invalid_storage_and_zero_count_gate_publication() {
    let valid_id = 8140;
    let invalid_id = 8141;
    let missing_id = 8142;
    let valid_objectives = vec![item_objective(valid_id * 10, valid_id, 504, 2, 0, 0)];
    let invalid_objectives = vec![item_objective(invalid_id * 10, invalid_id, 504, 1, -1, 0)];
    let valid_quest = QuestObjectiveRulesLikeCpp::new(valid_id, 0, 0, false, &valid_objectives);
    let invalid_quest =
        QuestObjectiveRulesLikeCpp::new(invalid_id, 0, 0, false, &invalid_objectives);
    let mut state = state_with_statuses(BTreeMap::from([
        (valid_id, status(valid_id, 0, vec![0])),
        (invalid_id, status(invalid_id, 1, vec![0])),
        (missing_id, status(missing_id, 2, vec![0])),
    ]));

    let outcome = state.apply_item_objective_progress_like_cpp(
        |id| match id {
            8140 => Some(valid_quest),
            8141 => Some(invalid_quest),
            8142 => None,
            _ => None,
        },
        &[504],
        0,
        Some(false),
    );

    assert_eq!(outcome.changed_quest_ids, vec![valid_id]);
    assert!(outcome.quests_to_complete.is_empty());
    assert!(outcome.objective_updates.is_empty());
    assert_eq!(
        state.statuses_like_cpp()[&valid_id].objective_counts,
        vec![0]
    );
    assert_eq!(
        state.statuses_like_cpp()[&invalid_id].objective_counts,
        vec![0]
    );
    assert_eq!(
        state.statuses_like_cpp()[&missing_id].objective_counts,
        vec![0]
    );
}
