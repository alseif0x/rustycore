//! Represented active quest status and quest-log lifecycle on the canonical owner.
//!
//! a5f8da2e Player.cpp: AddQuest (14398), CompleteQuest (14496),
//! RewardQuest (14625), RemoveActiveQuest (15575), FindQuestSlot (15897).
//! Catalog admission, per-slot snapshots, saves, COMMIT and publication stay APP.
//! Duplicate removal/compaction preserves the existing Rust load reconciliation.

use super::{PlayerQuestGameplayState, PlayerQuestStatusRecord};
use crate::{QuestObjectiveRulesLikeCpp, represented_can_complete_quest_after_objective_like_cpp};
use wow_constants::quest::{
    MAX_QUEST_LOG_SIZE, QUEST_STATE_COMPLETE, QUEST_STATE_FAIL, QUEST_STATE_OBJECTIVE_FLAG_BASE,
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};

#[cfg(test)]
mod tests;

impl PlayerQuestGameplayState {
    /// Prepare at the original application point, before entering the insertion writer.
    pub fn prepare_quest_start(
        quest_id: u32,
        slot: u8,
        accept_time_secs: i64,
        end_time_secs: i64,
        objective_count: usize,
    ) -> PlayerQuestStatusRecord {
        PlayerQuestStatusRecord {
            quest_id, status: QUEST_STATUS_INCOMPLETE_LIKE_CPP, explored: false,
            accept_time_secs, end_time_secs, objective_counts: vec![0; objective_count], slot,
        }
    }

    pub fn complete_quest(&mut self, quest_id: u32) -> Option<u8> {
        let status = self.status_mut_like_cpp(quest_id)?;
        (status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP).then(|| {
            let old_status = status.status;
            status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
            old_status
        })
    }

    pub fn settle_rewarded_quest(&mut self, quest_id: u32, repeatable: bool) {
        self.remove_status_like_cpp(quest_id);
        if !repeatable {
            self.set_rewarded_like_cpp(quest_id, true);
        }
    }

    pub fn clear_quest_end_time(&mut self, quest_id: u32) -> bool {
        let Some(status) = self.status_mut_like_cpp(quest_id) else { return false; };
        if status.end_time_secs <= 0 { return false; }
        status.end_time_secs = 0;
        true
    }

    pub fn mark_quest_explored(&mut self, quest_id: u32) -> (bool, bool) {
        let Some(status) = self.status_mut_like_cpp(quest_id) else { return (false, false); };
        let should_send = !status.explored && status.status != QUEST_STATUS_FAILED_LIKE_CPP;
        if should_send { status.explored = true; }
        (true, should_send)
    }

    /// This probe deliberately reads status before rewarded membership, unlike credits.
    pub fn can_complete_started_quest<'a>(
        &self,
        quest_id: u32,
        ignored_objective_id: u32,
        rules: impl FnOnce() -> QuestObjectiveRulesLikeCpp<'a>,
    ) -> bool {
        let Some(status) = self.statuses_like_cpp().get(&quest_id) else { return false; };
        let quest_already_rewarded = self.rewarded_quest_ids_like_cpp().contains(&quest_id);
        represented_can_complete_quest_after_objective_like_cpp(
            status, &rules(), ignored_objective_id, quest_already_rewarded,
        )
    }

    pub fn slot_has_active_entry(&self, slot: u8) -> bool {
        slot < MAX_QUEST_LOG_SIZE && self.statuses_like_cpp().values().any(|status| {
            status.slot == slot && matches!(status.status,
                QUEST_STATUS_INCOMPLETE_LIKE_CPP | QUEST_STATUS_COMPLETE_LIKE_CPP | QUEST_STATUS_FAILED_LIKE_CPP)
        })
    }

    pub fn quest_id_at_slot(&self, slot: u8) -> Option<u32> {
        if slot >= MAX_QUEST_LOG_SIZE { return None; }
        let mut matching_quest_id = None;
        for status in self.statuses_like_cpp().values().filter(|status| {
            status.slot == slot && matches!(status.status,
                QUEST_STATUS_INCOMPLETE_LIKE_CPP | QUEST_STATUS_COMPLETE_LIKE_CPP | QUEST_STATUS_FAILED_LIKE_CPP)
        }) {
            if matching_quest_id.is_some() { return None; }
            matching_quest_id = Some(status.quest_id);
        }
        matching_quest_id
    }

    pub fn slot_for_quest(&self, quest_id: u32) -> Option<u8> {
        self.statuses_like_cpp().get(&quest_id).and_then(|status| {
            (status.slot < MAX_QUEST_LOG_SIZE && matches!(status.status,
                QUEST_STATUS_INCOMPLETE_LIKE_CPP | QUEST_STATUS_COMPLETE_LIKE_CPP | QUEST_STATUS_FAILED_LIKE_CPP))
                .then_some(status.slot)
        })
    }

    /// Project from the initial snapshot; the application supplies the later per-slot ID.
    pub fn quest_log_entry<'a>(
        &self,
        quest_id: u32,
        rules: impl FnOnce(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
    ) -> (u32, u32, i64, [u16; 24]) {
        let Some(status) = self.statuses_like_cpp().get(&quest_id) else {
            return (0, 0, 0, [0; 24]);
        };
        let quest = rules(status.quest_id);
        let mut state_flags: u32 = match status.status {
            QUEST_STATUS_COMPLETE_LIKE_CPP => QUEST_STATE_COMPLETE,
            QUEST_STATUS_FAILED_LIKE_CPP => QUEST_STATE_FAIL,
            _ => 0,
        };
        let mut progress = [0u16; 24];
        for (i, slot_progress) in progress.iter_mut().enumerate() {
            let count = status.objective_counts.get(i).copied().unwrap_or(0);
            let stores_flag = quest.is_some_and(|quest| {
                quest.objectives().iter().any(|objective| {
                    objective.storage_index == i as i8 && objective.is_storing_flag_like_cpp()
                })
            });
            if stores_flag {
                if count != 0 { state_flags |= QUEST_STATE_OBJECTIVE_FLAG_BASE << i; }
                continue;
            }
            *slot_progress = count.min(u16::MAX as i32) as u16;
        }
        (status.quest_id, state_flags, status.end_time_secs, progress)
    }

    pub fn plan_rewarded_active_duplicates(
        &self,
        mut repeatable: impl FnMut(u32) -> Option<bool>,
    ) -> Vec<u32> {
        let mut duplicate_quest_ids = self.statuses_like_cpp().keys()
            .filter(|quest_id| {
                self.rewarded_quest_ids_like_cpp().contains(quest_id)
                    && repeatable(**quest_id).is_some_and(|repeatable| !repeatable)
            }).copied().collect::<Vec<_>>();
        duplicate_quest_ids.sort_unstable();
        duplicate_quest_ids.dedup();
        duplicate_quest_ids
    }

    pub fn remove_rewarded_active_duplicates(&mut self, duplicate_ids: &[u32]) {
        for quest_id in duplicate_ids { self.remove_status_like_cpp(*quest_id); }
        let mut remaining_slots = self.statuses_like_cpp().iter()
            .map(|(quest_id, status)| (*quest_id, status.slot)).collect::<Vec<_>>();
        remaining_slots.sort_by_key(|(_, slot)| *slot);
        for (slot, (quest_id, _)) in remaining_slots.into_iter().enumerate() {
            if let Some(status) = self.status_mut_like_cpp(quest_id) {
                status.slot = u8::try_from(slot).unwrap_or(MAX_QUEST_LOG_SIZE.saturating_sub(1));
            }
        }
    }
}
