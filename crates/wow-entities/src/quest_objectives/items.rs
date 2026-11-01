// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item-driven quest objective progress rules.
//!
//! Quest definitions are resolved lazily by the caller. This preserves the
//! existing lookup order while keeping catalog, Session, and persistence types
//! outside the entity rule boundary.

use super::completion::{
    represented_can_complete_quest_after_objective_like_cpp,
    represented_quest_objective_completable_like_cpp,
};
use super::model::QuestObjectiveRulesLikeCpp;
use crate::PlayerQuestStatusRecord;
use std::collections::{HashMap, HashSet};
use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP, QUEST_OBJECTIVE_ITEM_LIKE_CPP,
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};

pub fn apply_quest_item_added_bound_to_statuses_like_cpp<'a>(
    mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
    rewarded_quests: &HashSet<u32>,
    player_quests: &mut HashMap<u32, PlayerQuestStatusRecord>,
    entry_id: u32,
    quest_log_item_id: u32,
    count: u32,
) -> Option<(u32, i32)> {
    let count = i32::try_from(count).unwrap_or(i32::MAX);
    let mut ordered_quest_ids = player_quests
        .values()
        .map(|status| (status.slot, status.quest_id))
        .collect::<Vec<_>>();
    ordered_quest_ids.sort_unstable();

    for object_id in [entry_id, quest_log_item_id] {
        if object_id == 0 {
            continue;
        }
        let object_id = i32::try_from(object_id).unwrap_or(i32::MAX);
        for &(_, quest_id) in &ordered_quest_ids {
            let Some(status) = player_quests.get_mut(&quest_id) else {
                continue;
            };
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_by_id(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP
                    || (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP) == 0
                    || objective.object_id != object_id
                    || !represented_quest_objective_completable_like_cpp(
                        status,
                        &quest,
                        objective_index,
                    )
                {
                    continue;
                }
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                if status.objective_counts.len() <= storage_index {
                    status.objective_counts.resize(storage_index + 1, 0);
                }
                let current = status.objective_counts[storage_index];
                if current >= objective.amount {
                    continue;
                }
                let new_count = current.saturating_add(count).clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                if new_count >= objective.amount
                    && represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        &quest,
                        objective.id,
                        rewarded_quests.contains(&status.quest_id),
                    )
                {
                    status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
                }
                return Some((status.quest_id, new_count));
            }
        }
    }
    None
}

pub fn apply_quest_item_added_non_bound_to_statuses_like_cpp<'a>(
    mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
    rewarded_quests: &HashSet<u32>,
    player_quests: &mut HashMap<u32, PlayerQuestStatusRecord>,
    entry_id: u32,
    quest_log_item_id: u32,
    count: u32,
) -> Vec<u32> {
    let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
    let mut objective_ids = vec![entry_object_id];
    if quest_log_item_id != 0 {
        objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
    }
    let count = i32::try_from(count).unwrap_or(i32::MAX);
    let mut changed_quest_ids = Vec::new();
    let mut quests_to_complete = Vec::new();

    for status in player_quests.values_mut() {
        if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
            continue;
        }
        let Some(quest) = quest_by_id(status.quest_id) else {
            continue;
        };
        for (objective_index, objective) in quest.objectives.iter().enumerate() {
            if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP
                || (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP) != 0
                || !objective_ids.contains(&objective.object_id)
                || !represented_quest_objective_completable_like_cpp(
                    status,
                    &quest,
                    objective_index,
                )
            {
                continue;
            }
            let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                continue;
            };
            if status.objective_counts.len() <= storage_index {
                status.objective_counts.resize(storage_index + 1, 0);
            }
            let current = status.objective_counts[storage_index];
            if current >= objective.amount {
                continue;
            }
            let new_count = current.saturating_add(count).clamp(0, objective.amount);
            status.objective_counts[storage_index] = new_count;
            changed_quest_ids.push(status.quest_id);
            if new_count >= objective.amount
                && represented_can_complete_quest_after_objective_like_cpp(
                    status,
                    &quest,
                    objective.id,
                    rewarded_quests.contains(&status.quest_id),
                )
            {
                quests_to_complete.push(status.quest_id);
            }
        }
    }
    for quest_id in quests_to_complete {
        if let Some(status) = player_quests.get_mut(&quest_id) {
            status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
        }
    }
    changed_quest_ids.sort_unstable();
    changed_quest_ids.dedup();
    changed_quest_ids
}

pub fn apply_quest_item_removed_to_statuses_like_cpp<'a>(
    mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
    player_quests: &mut HashMap<u32, PlayerQuestStatusRecord>,
    entry_id: u32,
    new_non_bank_item_count: u32,
) -> Vec<u32> {
    let Ok(object_id) = i32::try_from(entry_id) else {
        return Vec::new();
    };
    let new_item_count = i32::try_from(new_non_bank_item_count).unwrap_or(i32::MAX);
    let mut changed_quest_ids = Vec::new();

    for status in player_quests.values_mut() {
        let Some(quest) = quest_by_id(status.quest_id) else {
            continue;
        };
        for (objective_index, objective) in quest.objectives.iter().enumerate() {
            if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP
                || objective.object_id != object_id
                || !represented_quest_objective_completable_like_cpp(
                    status,
                    &quest,
                    objective_index,
                )
            {
                continue;
            }
            let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                continue;
            };
            if new_item_count >= objective.amount {
                continue;
            }
            if status.objective_counts.len() <= storage_index {
                status.objective_counts.resize(storage_index + 1, 0);
            }
            if status.objective_counts[storage_index] == new_item_count
                && status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
            {
                continue;
            }
            status.objective_counts[storage_index] = new_item_count.max(0);
            status.status = QUEST_STATUS_INCOMPLETE_LIKE_CPP;
            changed_quest_ids.push(status.quest_id);
        }
    }
    changed_quest_ids.sort_unstable();
    changed_quest_ids.dedup();
    changed_quest_ids
}
