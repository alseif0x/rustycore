//! Seasonal recurrence decisions over the caller's existing quest-state snapshot.
//!
//! TrinityCore a5f8da2e, Player.cpp:24160-24189. The application retains its
//! separate owner fences, bulk writeback before bit clearing and metadata reads.

use super::PlayerQuestGameplayState;
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeasonalQuestResetReason {
    MissingEvent,
    EmptyEvent,
    RemovedOlderCompletions,
    NoOlderCompletions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeasonalQuestResetOutcome {
    pub event_id: u16,
    pub event_start_time: u64,
    pub reason: SeasonalQuestResetReason,
    pub removed_quest_ids: Vec<u32>,
    pub completed_bit_cleared: usize,
    pub completed_bit_skipped_no_quest_v2_store: usize,
    pub completed_bit_skipped_zero_unique_bit: usize,
    pub completed_bit_no_change_or_noop: usize,
    pub completed_bit_clear_unrepresented: usize,
    pub event_bucket_erased: bool,
    pub seasonal_quest_changed: bool,
}

impl SeasonalQuestResetOutcome {
    pub fn missing_event(event_id: u16, event_start_time: u64) -> Self {
        Self {
            event_id,
            event_start_time,
            reason: SeasonalQuestResetReason::MissingEvent,
            removed_quest_ids: Vec::new(),
            completed_bit_cleared: 0,
            completed_bit_skipped_no_quest_v2_store: 0,
            completed_bit_skipped_zero_unique_bit: 0,
            completed_bit_no_change_or_noop: 0,
            completed_bit_clear_unrepresented: 0,
            event_bucket_erased: false,
            seasonal_quest_changed: false,
        }
    }
}

/// One observation from the application's original metadata/bit-write operation.
pub enum SeasonalQuestBitReset {
    MissingCatalog,
    ZeroUniqueBit,
    Cleared,
    NoChangeOrNoop,
}

/// Consumed once; owns no live Player, registry, guard or persistence resource.
pub struct SeasonalQuestResetPlan {
    outcome: SeasonalQuestResetOutcome,
    updated_seasonal_quests: Option<BTreeMap<u16, BTreeMap<u32, u64>>>,
}

impl PlayerQuestGameplayState {
    pub fn plan_seasonal_reset(
        mut self,
        event_id: u16,
        event_start_time: u64,
    ) -> SeasonalQuestResetPlan {
        let mut outcome = SeasonalQuestResetOutcome::missing_event(event_id, event_start_time);
        let Some(bucket) = self.seasonal_event_quests_like_cpp(event_id) else {
            return SeasonalQuestResetPlan {
                outcome,
                updated_seasonal_quests: None,
            };
        };
        if bucket.is_empty() {
            outcome.reason = SeasonalQuestResetReason::EmptyEvent;
            return SeasonalQuestResetPlan {
                outcome,
                updated_seasonal_quests: None,
            };
        }

        let reset = self.reset_seasonal_event_like_cpp(event_id, event_start_time);
        outcome.removed_quest_ids = reset.removed_quest_ids;
        outcome.event_bucket_erased = reset.event_bucket_erased;
        outcome.reason = if outcome.removed_quest_ids.is_empty() {
            SeasonalQuestResetReason::NoOlderCompletions
        } else {
            SeasonalQuestResetReason::RemovedOlderCompletions
        };
        let seasonal_quests = self.seasonal_quests_snapshot_like_cpp();
        SeasonalQuestResetPlan {
            outcome,
            // A nonempty bucket still requests writeback if nothing was older.
            updated_seasonal_quests: Some(seasonal_quests),
        }
    }
}

impl SeasonalQuestResetPlan {
    pub fn take_updated_seasonal_quests(
        &mut self,
    ) -> Option<BTreeMap<u16, BTreeMap<u32, u64>>> {
        self.updated_seasonal_quests.take()
    }

    pub fn finish(
        mut self,
        mut reset_bit: impl FnMut(u32) -> SeasonalQuestBitReset,
    ) -> SeasonalQuestResetOutcome {
        for quest_id in &self.outcome.removed_quest_ids {
            match reset_bit(*quest_id) {
                SeasonalQuestBitReset::MissingCatalog => {
                    self.outcome.completed_bit_skipped_no_quest_v2_store += 1;
                }
                SeasonalQuestBitReset::ZeroUniqueBit => {
                    self.outcome.completed_bit_skipped_zero_unique_bit += 1;
                }
                SeasonalQuestBitReset::Cleared => {
                    self.outcome.completed_bit_cleared += 1;
                }
                SeasonalQuestBitReset::NoChangeOrNoop => {
                    self.outcome.completed_bit_no_change_or_noop += 1;
                }
            }
        }
        self.outcome
    }
}
