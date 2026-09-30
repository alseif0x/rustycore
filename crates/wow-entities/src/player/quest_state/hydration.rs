//! Reduction of loaded quest statuses on the application's existing staging state.
//!
//! a5f8da2e Player.cpp: _LoadQuestStatus (18613),
//! _LoadQuestStatusObjectives (18709), _LoadQuestStatusRewarded (18747).
//! Retains Rust's missing-catalog acceptance, NONE slots, duplicate overwrite,
//! raw times/counts and authority flag. SQL, saves, canonical install and the
//! recurrence stages remain application operations.

use super::{PlayerQuestGameplayState, PlayerQuestStatusRecord};
use wow_constants::quest::{
    MAX_QUEST_LOG_SIZE, QUEST_STATUS_INCOMPLETE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};
use wow_data_model::quest::QuestObjective;

#[cfg(test)]
mod tests;

/// Load bookkeeping only: this accumulator owns no Player state or catalog.
pub struct QuestStatusHydration {
    next_active_slot: u8,
    active_rows_coherent: bool,
    rewarded_rows_coherent: bool,
    stale_rewarded_active_rows: Vec<u32>,
}

impl PlayerQuestGameplayState {
    /// Begin bookkeeping for the already existing temporary load state.
    pub fn begin_status_hydration() -> QuestStatusHydration {
        QuestStatusHydration {
            next_active_slot: 0,
            active_rows_coherent: true,
            rewarded_rows_coherent: false,
            stale_rewarded_active_rows: Vec::new(),
        }
    }

    /// Resolve metadata after status_mut, including when the status is absent.
    pub fn hydrate_objective_count<'a>(
        &mut self,
        quest_id: u32,
        storage_index: u8,
        data: i32,
        objectives: impl FnOnce(u32) -> Option<&'a [QuestObjective]>,
    ) {
        if let (Some(status), Some(objectives)) = (
            self.status_mut_like_cpp(quest_id),
            objectives(quest_id),
        ) {
            if let Some(objective) = objectives.iter().find(|objective| {
                u8::try_from(objective.storage_index).ok() == Some(storage_index)
            }) {
                let index = usize::from(storage_index);
                if status.objective_counts.len() <= index {
                    status.objective_counts.resize(index + 1, 0);
                }
                status.objective_counts[index] = if objective.is_storing_flag_like_cpp() {
                    i32::from(data != 0)
                } else {
                    data
                };
            }
        }
    }
}

impl QuestStatusHydration {
    /// The application rejected a row missing any of its five required values.
    pub fn reject_active_row(&mut self) {
        self.active_rows_coherent = false;
    }

    pub fn hydrate_active_status(
        &mut self,
        state: &mut PlayerQuestGameplayState,
        quest_id: u32,
        status: u8,
        explored: u8,
        accept_time_secs: i64,
        end_time_secs: i64,
        objective_count: impl FnOnce(u32) -> usize,
    ) {
        let status = if status < 7 { status } else { QUEST_STATUS_INCOMPLETE_LIKE_CPP };
        let explored = explored != 0;

        if status == QUEST_STATUS_REWARDED_LIKE_CPP {
            state.set_rewarded_like_cpp(quest_id, true);
            self.stale_rewarded_active_rows.push(quest_id);
        } else if self.next_active_slot < MAX_QUEST_LOG_SIZE {
            let slot = self.next_active_slot;
            self.next_active_slot = self.next_active_slot.saturating_add(1);
            let obj_count = objective_count(quest_id);
            if state.statuses_like_cpp().contains_key(&quest_id) {
                self.active_rows_coherent = false;
            }
            state.insert_status_like_cpp(
                quest_id,
                PlayerQuestStatusRecord {
                    quest_id, status, explored, accept_time_secs, end_time_secs,
                    objective_counts: vec![0; obj_count], slot,
                },
            );
        }
    }

    /// A successful rewarded query is coherent even when its result is empty.
    pub fn rewarded_rows_loaded(&mut self) {
        self.rewarded_rows_coherent = true;
    }

    pub fn hydrate_rewarded_row(
        &mut self,
        state: &mut PlayerQuestGameplayState,
        quest_id: Option<u32>,
        can_increase_rewarded_counters: impl FnOnce(u32) -> Option<bool>,
    ) {
        let Some(quest_id) = quest_id else {
            self.rewarded_rows_coherent = false;
            return;
        };
        state.set_rewarded_row_like_cpp(quest_id, true);
        if can_increase_rewarded_counters(quest_id).is_some_and(|can_increase| can_increase) {
            state.set_rewarded_like_cpp(quest_id, true);
        }
    }

    /// Close authority before the application's existing canonical install.
    pub fn finish(self, state: &mut PlayerQuestGameplayState) -> Vec<u32> {
        state.set_status_authority_complete_like_cpp(
            self.active_rows_coherent && self.rewarded_rows_coherent,
        );
        self.stale_rewarded_active_rows
    }
}
