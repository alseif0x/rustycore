//! Shared quest row and objective catalog fixtures, unchanged from the target root.

use super::*;

pub(super) fn quest_template(id: u32) -> QuestTemplate {
    wow_world::test_fixtures::quest_template_row_for_test(id, 2, format!("Quest {id}"))
}

pub(super) fn quest_template_with_objective_count(id: u32, objective_count: usize) -> QuestTemplate {
    let mut quest = quest_template(id);
    quest.objectives = (0..objective_count)
        .map(|index| QuestObjective {
            id: id * 10 + index as u32,
            quest_id: id,
            obj_type: 0,
            order: index as u8,
            storage_index: index as i8,
            object_id: 1000 + index as i32,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    quest
}

pub(super) fn store_with_sharable_quest_objectives(id: u32, objective_count: usize) -> QuestStore {
    let mut quest = quest_template_with_objective_count(id, objective_count);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    QuestStore::from_quests_like_cpp([quest])
}

pub(super) fn store_with_sharable_timed_quest_objectives(
    id: u32,
    objective_count: usize,
    limit_time_secs: i64,
) -> QuestStore {
    let mut quest = quest_template_with_objective_count(id, objective_count);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.limit_time_secs = limit_time_secs;
    QuestStore::from_quests_like_cpp([quest])
}
