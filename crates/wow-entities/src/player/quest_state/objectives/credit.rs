//! Stored objective credit and threshold-loss transitions on canonical quest state.
//!
//! Reference: a5f8da2e `Player.cpp`, UpdateQuestObjectiveProgress (16181),
//! SetQuestObjectiveData (16426), and CanCompleteQuest (14123).
//! Preserve the represented matching gates, resize/clamp behavior and per-credit
//! writes. Admission, post-write snapshots, completion effects and publication
//! remain with the application.

use super::{
    PlayerQuestGameplayState, QuestObjectiveRulesLikeCpp,
    represented_can_complete_quest_after_objective_like_cpp,
};
use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP, QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};

#[cfg(test)]
mod tests;

impl PlayerQuestGameplayState {
    pub fn plan_value_objective_credits<'a>(
        &self,
        mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
        objective_type: u8,
        object_id: i32,
        victim_same_faction: Option<bool>,
    ) -> Vec<(u32, usize, i32, u32)> {
        let mut matching = Vec::new();
        for status in self.statuses_like_cpp().values() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_by_id(status.quest_id) else {
                continue;
            };
            for objective in quest.objectives() {
                if objective.obj_type != objective_type || objective.object_id != object_id {
                    continue;
                }
                if objective_type == QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP
                    && (objective.flags & QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP)
                        != 0
                    && victim_same_faction == Some(false)
                {
                    continue;
                }
                let Ok(index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                matching.push((status.quest_id, index, objective.amount, objective.id));
            }
        }
        matching
    }

    pub fn plan_flag_objective_credits<'a>(
        &self,
        mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
        objective_type: u8,
        object_id: i32,
    ) -> Vec<(u32, usize, u32)> {
        let mut matching = Vec::new();
        for status in self.statuses_like_cpp().values() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_by_id(status.quest_id) else {
                continue;
            };
            for objective in quest.objectives() {
                if objective.obj_type != objective_type
                    || objective.object_id != object_id
                    || !objective.is_storing_flag_like_cpp()
                {
                    continue;
                }
                let Ok(index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                matching.push((status.quest_id, index, objective.id));
            }
        }
        matching
    }

    pub fn apply_value_objective_credit(
        &mut self,
        quest_id: u32,
        storage_index: usize,
        required: i32,
        add_count: i32,
    ) -> Option<i32> {
        let status = self.status_mut_like_cpp(quest_id)?;
        if status.objective_counts.len() <= storage_index {
            status.objective_counts.resize(storage_index + 1, 0);
        }
        if add_count >= 0 && status.objective_counts[storage_index] >= required {
            return None;
        }
        status.objective_counts[storage_index] = status.objective_counts[storage_index]
            .saturating_add(add_count)
            .clamp(0, required);
        Some(status.objective_counts[storage_index])
    }

    pub fn apply_flag_objective_credit(
        &mut self,
        quest_id: u32,
        storage_index: usize,
        add_count: i32,
    ) -> Option<(bool, bool)> {
        let status = self.status_mut_like_cpp(quest_id)?;
        if status.objective_counts.len() <= storage_index {
            status.objective_counts.resize(storage_index + 1, 0);
        }
        let before = status.objective_counts[storage_index] != 0;
        status.objective_counts[storage_index] = i32::from(add_count > 0);
        Some((before, status.objective_counts[storage_index] != 0))
    }

    pub fn reopen_quest_after_threshold_loss(&mut self, quest_id: u32) -> bool {
        let Some(status) = self.status_mut_like_cpp(quest_id) else {
            return false;
        };
        if status.status != QUEST_STATUS_COMPLETE_LIKE_CPP {
            return false;
        }
        status.status = QUEST_STATUS_INCOMPLETE_LIKE_CPP;
        true
    }

    pub fn can_complete_after_objective<'a>(
        &self,
        quest_id: u32,
        objective_id: u32,
        quest_rules: impl FnOnce() -> QuestObjectiveRulesLikeCpp<'a>,
    ) -> bool {
        let quest_already_rewarded = self.rewarded_quest_ids_like_cpp().contains(&quest_id);
        self.statuses_like_cpp()
            .get(&quest_id)
            .is_some_and(|status| {
                represented_can_complete_quest_after_objective_like_cpp(
                    status,
                    &quest_rules(),
                    objective_id,
                    quest_already_rewarded,
                )
            })
    }
}
