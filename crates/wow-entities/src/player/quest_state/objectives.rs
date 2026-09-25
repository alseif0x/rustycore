// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical item-objective transitions from Player::ItemAddedQuestCheck.
//! No catalog, Session, clock, persistence, or packet writer is retained.

use super::PlayerQuestGameplayState;
use crate::{
    QuestObjectiveRulesLikeCpp, represented_can_complete_quest_after_objective_like_cpp,
    represented_quest_objective_completable_like_cpp,
};
use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP, QUEST_OBJECTIVE_ITEM_LIKE_CPP,
    QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq)]
pub struct QuestItemObjectiveProgressLikeCpp {
    pub changed_quest_ids: Vec<u32>,
    pub quests_to_complete: Vec<u32>,
    pub objective_updates: Vec<(i32, bool)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct QuestBoundItemObjectiveProgressLikeCpp {
    pub updated_counts: Vec<(u32, i32)>,
    pub quests_to_complete: Vec<u32>,
}

impl PlayerQuestGameplayState {
    /// Apply the existing filtered item-progress transition to canonical status records.
    /// Completion side effects are returned to the caller, not executed here.
    pub fn apply_item_objective_progress_like_cpp<'a>(
        &mut self,
        mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
        objective_ids: &[i32],
        count: i32,
        bound_item_requirement: Option<bool>,
    ) -> QuestItemObjectiveProgressLikeCpp {
        let rewarded_quest_ids = self.rewarded_quest_ids_like_cpp().clone();
        let mut changed_quest_ids = Vec::new();
        let mut quests_to_complete = Vec::new();
        let mut objective_updates = Vec::new();
        'quests: for status in self.statuses_mut_like_cpp() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }

            let Some(quest) = quest_by_id(status.quest_id) else {
                continue;
            };

            for (objective_index, objective) in quest.objectives_like_cpp().iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP {
                    continue;
                }
                let is_bound =
                    (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP) != 0;
                if bound_item_requirement.is_some_and(|required_bound| required_bound != is_bound) {
                    continue;
                }
                if !objective_ids.contains(&objective.object_id) {
                    continue;
                }
                if !represented_quest_objective_completable_like_cpp(
                    status,
                    &quest,
                    objective_index,
                ) {
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
                status.objective_counts[storage_index] =
                    current.saturating_add(count).clamp(0, objective.amount);
                let new_count = status.objective_counts[storage_index];
                if !changed_quest_ids.contains(&status.quest_id) {
                    changed_quest_ids.push(status.quest_id);
                }
                if count > 0 {
                    objective_updates.push((new_count, is_bound));
                }
                let quest_already_rewarded = rewarded_quest_ids.contains(&status.quest_id);
                if new_count >= objective.amount
                    && represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        &quest,
                        objective.id,
                        quest_already_rewarded,
                    )
                {
                    quests_to_complete.push(status.quest_id);
                }
                if is_bound {
                    break 'quests;
                }
            }
        }
        QuestItemObjectiveProgressLikeCpp {
            changed_quest_ids,
            quests_to_complete,
            objective_updates,
        }
    }

    /// Credit the first matching bound objective in the caller's existing quest order.
    /// The caller retains admission, order selection and completion/publication phases.
    pub fn apply_bound_item_objective_progress_like_cpp<'a>(
        &mut self,
        mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
        ordered_quest_ids: Vec<u32>,
        object_id: i32,
        count_i32: i32,
    ) -> QuestBoundItemObjectiveProgressLikeCpp {
        let rewarded_quest_ids = self.rewarded_quest_ids_like_cpp().clone();
        let mut updated_counts = Vec::new();
        let mut quests_to_complete = Vec::new();
        'quests: for quest_id in ordered_quest_ids {
            let Some(status) = self.status_mut_like_cpp(quest_id) else {
                continue;
            };
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }

            let Some(quest) = quest_by_id(status.quest_id) else {
                continue;
            };

            for (objective_index, objective) in quest.objectives_like_cpp().iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP {
                    continue;
                }
                if (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP) == 0 {
                    continue;
                }
                if objective.object_id != object_id {
                    continue;
                }
                if !represented_quest_objective_completable_like_cpp(
                    status,
                    &quest,
                    objective_index,
                ) {
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
                let new_count = current.saturating_add(count_i32).clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                updated_counts.push((status.quest_id, new_count));
                let quest_already_rewarded = rewarded_quest_ids.contains(&status.quest_id);
                if new_count >= objective.amount
                    && represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        &quest,
                        objective.id,
                        quest_already_rewarded,
                    )
                {
                    quests_to_complete.push(status.quest_id);
                }
                // C++ `UpdateQuestObjectiveProgress` stops after the first
                // credited quest-bound Item objective.
                break 'quests;
            }
        }
        QuestBoundItemObjectiveProgressLikeCpp {
            updated_counts,
            quests_to_complete,
        }
    }
}
