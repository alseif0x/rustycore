use super::{
    QuestObjective, QuestObjectiveRulesLikeCpp, ThresholdQuestObjectiveChangeLikeCpp,
    apply_quest_item_added_bound_to_statuses_like_cpp,
    apply_quest_item_added_non_bound_to_statuses_like_cpp,
    apply_quest_item_removed_to_statuses_like_cpp, plan_threshold_quest_objective_changes_like_cpp,
    player_has_incomplete_quest_objective_for_object_id_like_cpp,
    represented_quest_objective_completable_like_cpp,
    represented_quest_objective_complete_like_cpp,
};
use crate::PlayerQuestStatusRecord;
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
    QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP, QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP,
    QUEST_OBJECTIVE_ITEM_LIKE_CPP, QUEST_OBJECTIVE_MONSTER_LIKE_CPP,
    QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP, QUEST_STATUS_COMPLETE_LIKE_CPP,
    QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};

fn objective(
    id: u32,
    quest_id: u32,
    obj_type: u8,
    storage_index: i8,
    object_id: i32,
    amount: i32,
    flags: u32,
    flags2: u32,
    progress_bar_weight: f32,
) -> QuestObjective {
    QuestObjective {
        id,
        quest_id,
        obj_type,
        order: 0,
        storage_index,
        object_id,
        amount,
        flags,
        flags2,
        progress_bar_weight,
        description: String::new(),
    }
}

fn status(
    quest_id: u32,
    slot: u8,
    status: u8,
    objective_counts: Vec<i32>,
) -> PlayerQuestStatusRecord {
    PlayerQuestStatusRecord {
        quest_id,
        status,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts,
        slot,
    }
}

#[test]
fn definition_view_borrows_the_original_objectives_in_the_original_order() {
    let objectives = [
        objective(90, 9, QUEST_OBJECTIVE_ITEM_LIKE_CPP, 0, 20, 1, 0, 0, 0.0),
        objective(91, 9, QUEST_OBJECTIVE_ITEM_LIKE_CPP, 1, 21, 2, 0, 0, 0.0),
    ];
    let view = QuestObjectiveRulesLikeCpp::new(9, 0, 0, false, &objectives);
    assert!(std::ptr::eq(view.objectives.as_ptr(), objectives.as_ptr()));
    assert_eq!(view.objectives[0].id, 90);
    assert_eq!(view.objectives[1].id, 91);
}

#[test]
fn item_admission_uses_status_counts_and_fail_closed_catalog_lookup() {
    let quest_id = 93;
    let objectives = vec![objective(
        930,
        quest_id,
        QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        0,
        77,
        2,
        0,
        0,
        0.0,
    )];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let incomplete = status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![1]);
    let complete = status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![2]);
    let mut statuses = BTreeMap::from([(quest_id, incomplete)]);
    assert!(
        player_has_incomplete_quest_objective_for_object_id_like_cpp(
            &statuses,
            |_| Some(quest),
            77,
        )
    );
    statuses.insert(quest_id, complete);
    assert!(
        !player_has_incomplete_quest_objective_for_object_id_like_cpp(
            &statuses,
            |_| Some(quest),
            77,
        )
    );
    assert!(
        !player_has_incomplete_quest_objective_for_object_id_like_cpp(&statuses, |_| None, 77,)
    );
}

#[test]
fn threshold_planner_preserves_direction_and_skips_already_complete_positive_changes() {
    let quest_id = 94;
    let objectives = vec![objective(940, quest_id, 16, 0, 12, 10, 0, 0, 0.0)];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let statuses = BTreeMap::from([(
        quest_id,
        status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0]),
    )]);
    assert_eq!(
        plan_threshold_quest_objective_changes_like_cpp(
            &statuses,
            |_| Some(quest),
            16,
            12,
            8,
            11,
            false,
        ),
        vec![ThresholdQuestObjectiveChangeLikeCpp {
            quest_id,
            objective_id: 940,
            required: 10,
            objective_was_complete: false,
            objective_is_now_complete: true,
        }]
    );
    assert!(
        plan_threshold_quest_objective_changes_like_cpp(
            &statuses,
            |_| Some(quest),
            16,
            12,
            11,
            12,
            false,
        )
        .is_empty()
    );
}

#[test]
fn threshold_planner_supports_max_reputation_inverse() {
    let quest_id = 95;
    let objectives = vec![objective(950, quest_id, 7, 0, 22, 50, 0, 0, 0.0)];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let statuses = BTreeMap::from([(
        quest_id,
        status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0]),
    )]);
    let changes = plan_threshold_quest_objective_changes_like_cpp(
        &statuses,
        |_| Some(quest),
        7,
        22,
        60,
        40,
        true,
    );
    assert_eq!(changes[0].objective_was_complete, false);
    assert_eq!(changes[0].objective_is_now_complete, true);
}

#[test]
fn represented_quest_objective_completable_accepts_cpp_storing_value_previous_types() {
    let quest_id = 7100;
    let objectives = vec![
        objective(
            quest_id * 10,
            quest_id,
            QUEST_OBJECTIVE_MONSTER_LIKE_CPP,
            0,
            44,
            1,
            0,
            0,
            0.0,
        ),
        objective(
            quest_id * 10 + 1,
            quest_id,
            QUEST_OBJECTIVE_ITEM_LIKE_CPP,
            1,
            55,
            1,
            QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP,
            0,
            0.0,
        ),
    ];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let status = status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![1, 0]);

    assert!(represented_quest_objective_completable_like_cpp(
        &status, &quest, 1
    ));
}

#[test]
fn represented_objective_negative_storage_index_does_not_alias_slot_zero_like_cpp() {
    let quest_id = 7101;
    let objectives = vec![objective(
        quest_id * 10,
        quest_id,
        QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        -1,
        55,
        1,
        0,
        0,
        0.0,
    )];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let status = status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![1]);

    assert!(!represented_quest_objective_complete_like_cpp(
        &status,
        &quest,
        &objectives[0],
    ));
}

#[test]
fn represented_progress_bar_part_objective_stops_when_progress_bar_complete_like_cpp() {
    let quest_id = 7120;
    let objectives = vec![
        objective(
            quest_id * 10,
            quest_id,
            QUEST_OBJECTIVE_ITEM_LIKE_CPP,
            0,
            99,
            2,
            QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP,
            0,
            50.0,
        ),
        objective(
            quest_id * 10 + 1,
            quest_id,
            QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP,
            1,
            0,
            100,
            0,
            0,
            0.0,
        ),
    ];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let status = status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![2, 0]);

    assert!(!represented_quest_objective_completable_like_cpp(
        &status, &quest, 0
    ));
}

#[test]
fn bound_item_progress_uses_lowest_quest_slot_first() {
    let first_quest_id = 7201;
    let second_quest_id = 7202;
    let first_objectives = vec![objective(
        first_quest_id * 10,
        first_quest_id,
        QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        0,
        900,
        1,
        0,
        QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
        0.0,
    )];
    let second_objectives = vec![objective(
        second_quest_id * 10,
        second_quest_id,
        QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        0,
        900,
        1,
        0,
        QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
        0.0,
    )];
    let first_quest =
        QuestObjectiveRulesLikeCpp::new(first_quest_id, 0, 0, false, &first_objectives);
    let second_quest =
        QuestObjectiveRulesLikeCpp::new(second_quest_id, 0, 0, false, &second_objectives);
    let mut player_quests = HashMap::from([
        (
            first_quest_id,
            status(first_quest_id, 4, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0]),
        ),
        (
            second_quest_id,
            status(
                second_quest_id,
                1,
                QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                vec![0],
            ),
        ),
    ]);

    let changed = apply_quest_item_added_bound_to_statuses_like_cpp(
        |id| match id {
            7201 => Some(first_quest),
            7202 => Some(second_quest),
            _ => None,
        },
        &HashSet::new(),
        &mut player_quests,
        900,
        0,
        1,
    );

    assert_eq!(changed, Some((second_quest_id, 1)));
    assert_eq!(player_quests[&second_quest_id].objective_counts, vec![1]);
    assert_eq!(player_quests[&first_quest_id].objective_counts, vec![0]);
}

#[test]
fn non_bound_item_progress_updates_multiple_quests_and_defers_incomplete_completion() {
    let complete_quest_id = 7210;
    let deferred_quest_id = 7211;
    let complete_objectives = vec![objective(
        complete_quest_id * 10,
        complete_quest_id,
        QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        0,
        901,
        1,
        0,
        0,
        0.0,
    )];
    let deferred_objectives = vec![
        objective(
            deferred_quest_id * 10,
            deferred_quest_id,
            QUEST_OBJECTIVE_ITEM_LIKE_CPP,
            0,
            901,
            1,
            0,
            0,
            0.0,
        ),
        objective(
            deferred_quest_id * 10 + 1,
            deferred_quest_id,
            QUEST_OBJECTIVE_MONSTER_LIKE_CPP,
            1,
            902,
            1,
            0,
            0,
            0.0,
        ),
    ];
    let complete_quest =
        QuestObjectiveRulesLikeCpp::new(complete_quest_id, 0, 0, false, &complete_objectives);
    let deferred_quest =
        QuestObjectiveRulesLikeCpp::new(deferred_quest_id, 0, 0, false, &deferred_objectives);
    let mut player_quests = HashMap::from([
        (
            complete_quest_id,
            status(
                complete_quest_id,
                0,
                QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                vec![0],
            ),
        ),
        (
            deferred_quest_id,
            status(
                deferred_quest_id,
                1,
                QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                vec![0, 0],
            ),
        ),
    ]);

    let changed = apply_quest_item_added_non_bound_to_statuses_like_cpp(
        |id| match id {
            7210 => Some(complete_quest),
            7211 => Some(deferred_quest),
            _ => None,
        },
        &HashSet::new(),
        &mut player_quests,
        901,
        0,
        1,
    );

    assert_eq!(changed, vec![complete_quest_id, deferred_quest_id]);
    assert_eq!(
        player_quests[&complete_quest_id].status,
        QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_eq!(
        player_quests[&deferred_quest_id].status,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert_eq!(
        player_quests[&deferred_quest_id].objective_counts,
        vec![1, 0]
    );
}

#[test]
fn removed_item_count_reopens_a_completed_quest_once() {
    let quest_id = 7220;
    let objectives = vec![objective(
        quest_id * 10,
        quest_id,
        QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        0,
        903,
        2,
        0,
        0,
        0.0,
    )];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let mut player_quests = HashMap::from([(
        quest_id,
        status(quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP, vec![2]),
    )]);

    let changed =
        apply_quest_item_removed_to_statuses_like_cpp(|_| Some(quest), &mut player_quests, 903, 1);
    assert_eq!(changed, vec![quest_id]);
    assert_eq!(
        player_quests[&quest_id].status,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert_eq!(player_quests[&quest_id].objective_counts, vec![1]);

    let unchanged =
        apply_quest_item_removed_to_statuses_like_cpp(|_| Some(quest), &mut player_quests, 903, 1);
    assert!(unchanged.is_empty());
}

#[test]
fn item_rules_leave_status_untouched_when_catalog_is_missing() {
    let quest_id = 7230;
    let mut player_quests = HashMap::from([(
        quest_id,
        status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0]),
    )]);
    let before = player_quests.clone();

    let changed = apply_quest_item_added_non_bound_to_statuses_like_cpp(
        |_| None,
        &HashSet::new(),
        &mut player_quests,
        904,
        0,
        1,
    );

    assert!(changed.is_empty());
    assert_eq!(player_quests, before);
}

#[test]
fn item_rules_skip_invalid_storage_without_aliasing_slot_zero() {
    let quest_id = 7240;
    let objectives = vec![objective(
        quest_id * 10,
        quest_id,
        QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        -1,
        905,
        1,
        0,
        0,
        0.0,
    )];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let mut player_quests = HashMap::from([(
        quest_id,
        status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0]),
    )]);

    let changed = apply_quest_item_added_non_bound_to_statuses_like_cpp(
        |_| Some(quest),
        &HashSet::new(),
        &mut player_quests,
        905,
        0,
        1,
    );

    assert!(changed.is_empty());
    assert_eq!(player_quests[&quest_id].objective_counts, vec![0]);
}

#[test]
fn item_rules_ignore_a_non_matching_entry() {
    let quest_id = 7250;
    let objectives = vec![objective(
        quest_id * 10,
        quest_id,
        QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        0,
        906,
        1,
        0,
        0,
        0.0,
    )];
    let quest = QuestObjectiveRulesLikeCpp::new(quest_id, 0, 0, false, &objectives);
    let mut player_quests = HashMap::from([(
        quest_id,
        status(quest_id, 0, QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0]),
    )]);

    let changed = apply_quest_item_added_non_bound_to_statuses_like_cpp(
        |_| Some(quest),
        &HashSet::new(),
        &mut player_quests,
        999,
        0,
        1,
    );

    assert!(changed.is_empty());
    assert_eq!(player_quests[&quest_id].objective_counts, vec![0]);
}
