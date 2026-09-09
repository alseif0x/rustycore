//! Misc scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_objective_negative_storage_index_does_not_alias_slot_zero_like_cpp() {
    let quest_id = 7101;
    let mut quest = quest_template(quest_id);
    let objective = QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: -1,
        object_id: 55,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    };
    quest.objectives = vec![objective.clone()];
    let status = PlayerQuestStatus {
        quest_id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![1],
        slot: 0,
    };

    assert!(
        !WorldSession::represented_quest_objective_complete_like_cpp(&status, &quest, &objective)
    );
}
#[test]
fn represented_progress_bar_part_objective_stops_when_progress_bar_complete_like_cpp() {
    let quest_id = 7120;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![
        QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: 99,
            amount: 2,
            flags: QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL,
            flags2: 0,
            progress_bar_weight: 50.0,
            description: String::new(),
        },
        QuestObjective {
            id: quest_id * 10 + 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL,
            order: 1,
            storage_index: 1,
            object_id: 0,
            amount: 100,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        },
    ];
    let status = PlayerQuestStatus {
        quest_id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![2, 0],
        slot: 0,
    };

    assert!(!WorldSession::represented_quest_objective_completable_like_cpp(&status, &quest, 0));
}
