// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player update-field values for the independent native QuestLog.

use super::Player;

pub(super) const MAX_QUEST_LOG_SIZE_LIKE_CPP: usize = 25;
pub(super) const MAX_QUEST_COUNTS_LIKE_CPP: usize = 24;

const QUEST_LOG_PARENT_BIT: usize = 35;
const QUEST_LOG_FIRST_SLOT_BIT: usize = 36;
const QUEST_LOG_ROW_ROOT: u32 = 1 << 0;
const QUEST_LOG_END_TIME: u32 = 1 << 1;
const QUEST_LOG_QUEST_ID: u32 = 1 << 2;
const QUEST_LOG_STATE_FLAGS: u32 = 1 << 3;
const QUEST_LOG_OBJECTIVES: u32 = 1 << 4;
const QUEST_LOG_COUNTERS_FIRST_BIT: u32 = 5;
const QUEST_LOG_ALL_ROW_BITS: u32 = QUEST_LOG_ROW_ROOT
    | QUEST_LOG_END_TIME
    | QUEST_LOG_QUEST_ID
    | QUEST_LOG_STATE_FLAGS
    | QUEST_LOG_OBJECTIVES
    | (((1_u32 << MAX_QUEST_COUNTS_LIKE_CPP) - 1) << QUEST_LOG_COUNTERS_FIRST_BIT);

/// Read-only native values for one C++ `UF::QuestLog` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuestLogValuesLikeCpp {
    end_time: i64,
    quest_id: i32,
    state_flags: u32,
    objective_progress: [u16; MAX_QUEST_COUNTS_LIKE_CPP],
}

impl QuestLogValuesLikeCpp {
    pub const fn end_time(&self) -> i64 {
        self.end_time
    }

    pub const fn quest_id(&self) -> i32 {
        self.quest_id
    }

    pub const fn state_flags(&self) -> u32 {
        self.state_flags
    }

    pub const fn objective_progress(&self) -> &[u16; MAX_QUEST_COUNTS_LIKE_CPP] {
        &self.objective_progress
    }

    /// C++ `Player::GetQuestSlotObjectiveData`'s persisted objective reader
    /// (`Player.cpp:15929-15948`), including its raw-StorageIndex flag mask.
    pub fn objective_data_for_save_like_cpp(
        &self,
        storage_index: u8,
        is_storing_flag: bool,
    ) -> Option<u16> {
        if usize::from(storage_index) >= MAX_QUEST_COUNTS_LIKE_CPP {
            return Some(0);
        }
        if is_storing_flag {
            return Some(u16::from(self.state_flags & u32::from(storage_index) != 0));
        }
        self.objective_progress
            .get(usize::from(storage_index))
            .copied()
    }
}

/// Immutable row and nested mask captured for one Player update publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuestLogRowUpdateLikeCpp {
    values: QuestLogValuesLikeCpp,
    change_mask: u32,
}

impl QuestLogRowUpdateLikeCpp {
    pub(super) const fn new(values: QuestLogValuesLikeCpp, change_mask: u32) -> Self {
        Self {
            values,
            change_mask,
        }
    }

    pub const fn values(&self) -> &QuestLogValuesLikeCpp {
        &self.values
    }

    pub const fn change_mask(&self) -> u32 {
        self.change_mask
    }
}

impl Player {
    /// Snapshot canonical QuestLog rows for login and self-create consumers.
    pub fn quest_log_values_like_cpp(
        &self,
    ) -> [QuestLogValuesLikeCpp; MAX_QUEST_LOG_SIZE_LIKE_CPP] {
        self.data.quest_log
    }

    /// Read the live QuestID stored in a native update-field slot.
    pub fn quest_slot_quest_id_like_cpp(&self, slot: u8) -> Option<u32> {
        let row = self.data.quest_log.get(usize::from(slot))?;
        (row.quest_id != 0).then_some(row.quest_id as u32)
    }

    /// C++ `FindQuestSlot`; match the live update-field QuestID array.
    pub fn find_quest_slot_like_cpp(&self, quest_id: u32) -> Option<u8> {
        self.data
            .quest_log
            .iter()
            .position(|row| row.quest_id as u32 == quest_id)
            .and_then(|slot| u8::try_from(slot).ok())
    }

    /// C++ `Player::GetQuestSlotObjectiveData`'s persisted objective reader
    /// (`Player.cpp:15929-15948`), including its raw-StorageIndex flag mask.
    pub fn quest_slot_objective_data_for_save_like_cpp(
        &self,
        slot: u8,
        storage_index: i8,
        is_storing_flag: bool,
    ) -> Option<u16> {
        let row = self.data.quest_log.get(usize::from(slot))?;
        if !(0..MAX_QUEST_COUNTS_LIKE_CPP as i8).contains(&storage_index) {
            return Some(0);
        }
        Some(row.objective_data_for_save_like_cpp(storage_index as u8, is_storing_flag)?)
    }

    /// C++ `Player::SetQuestSlot`: reset the independent update-field row.
    pub fn set_quest_slot_like_cpp(&mut self, slot: u8, quest_id: u32) -> bool {
        let Some(current) = self.data.quest_log.get(usize::from(slot)).copied() else {
            return false;
        };
        let next = QuestLogValuesLikeCpp {
            end_time: 0,
            quest_id: quest_id as i32,
            state_flags: 0,
            objective_progress: [0; MAX_QUEST_COUNTS_LIKE_CPP],
        };
        let changed = current != next;
        self.mark_quest_log_nested_bits_like_cpp(slot, QUEST_LOG_ALL_ROW_BITS);
        if changed {
            self.data.quest_log[usize::from(slot)] = next;
            self.mark_player_data_array(
                QUEST_LOG_PARENT_BIT,
                QUEST_LOG_FIRST_SLOT_BIT,
                usize::from(slot),
            );
        }
        true
    }

    /// C++ `Player::SetQuestSlotCounter`; objective values wrap to their u16 field width.
    pub fn set_quest_slot_counter_like_cpp(&mut self, slot: u8, counter: u8, count: u16) -> bool {
        let Some(row) = self.data.quest_log.get(usize::from(slot)) else {
            return false;
        };
        let Some(value) = row.objective_progress.get(usize::from(counter)) else {
            return false;
        };
        let changed = *value != count;
        let counter_bit = QUEST_LOG_COUNTERS_FIRST_BIT + u32::from(counter);
        self.mark_quest_log_nested_bits_like_cpp(slot, QUEST_LOG_OBJECTIVES | (1 << counter_bit));
        if changed {
            self.data.quest_log[usize::from(slot)].objective_progress[usize::from(counter)] = count;
            self.mark_player_data_array(
                QUEST_LOG_PARENT_BIT,
                QUEST_LOG_FIRST_SLOT_BIT,
                usize::from(slot),
            );
        }
        true
    }

    /// C++ `Player::SetQuestSlotState`; add only the requested state bits.
    pub fn set_quest_slot_state_like_cpp(&mut self, slot: u8, state: u32) -> bool {
        let Some(row) = self.data.quest_log.get(usize::from(slot)) else {
            return false;
        };
        let next = row.state_flags | state;
        let changed = next != row.state_flags;
        self.mark_quest_log_nested_bits_like_cpp(slot, QUEST_LOG_STATE_FLAGS);
        if changed {
            self.data.quest_log[usize::from(slot)].state_flags = next;
            self.mark_player_data_array(
                QUEST_LOG_PARENT_BIT,
                QUEST_LOG_FIRST_SLOT_BIT,
                usize::from(slot),
            );
        }
        true
    }

    /// C++ `Player::RemoveQuestSlotState`; retain unrelated state bits.
    pub fn remove_quest_slot_state_like_cpp(&mut self, slot: u8, state: u32) -> bool {
        let Some(row) = self.data.quest_log.get(usize::from(slot)) else {
            return false;
        };
        let next = row.state_flags & !state;
        let changed = next != row.state_flags;
        self.mark_quest_log_nested_bits_like_cpp(slot, QUEST_LOG_STATE_FLAGS);
        if changed {
            self.data.quest_log[usize::from(slot)].state_flags = next;
            self.mark_player_data_array(
                QUEST_LOG_PARENT_BIT,
                QUEST_LOG_FIRST_SLOT_BIT,
                usize::from(slot),
            );
        }
        true
    }

    /// C++ `Player::SetQuestSlotEndTime` casts to uint32 before storing in int64.
    pub fn set_quest_slot_end_time_like_cpp(&mut self, slot: u8, end_time: i64) -> bool {
        let Some(row) = self.data.quest_log.get(usize::from(slot)) else {
            return false;
        };
        let next = i64::from(end_time as u32);
        let changed = row.end_time != next;
        self.mark_quest_log_nested_bits_like_cpp(slot, QUEST_LOG_END_TIME);
        if changed {
            self.data.quest_log[usize::from(slot)].end_time = next;
            self.mark_player_data_array(
                QUEST_LOG_PARENT_BIT,
                QUEST_LOG_FIRST_SLOT_BIT,
                usize::from(slot),
            );
        }
        true
    }

    pub(super) fn quest_log_rows_for_update_like_cpp(
        &self,
    ) -> [QuestLogRowUpdateLikeCpp; MAX_QUEST_LOG_SIZE_LIKE_CPP] {
        std::array::from_fn(|slot| {
            QuestLogRowUpdateLikeCpp::new(
                self.data.quest_log[slot],
                self.quest_log_change_masks[slot],
            )
        })
    }

    pub(super) fn reset_quest_log_change_masks_like_cpp(&mut self) {
        self.quest_log_change_masks = [0; MAX_QUEST_LOG_SIZE_LIKE_CPP];
    }

    fn mark_quest_log_nested_bits_like_cpp(&mut self, slot: u8, nested_bits: u32) {
        let slot = usize::from(slot);
        self.quest_log_change_masks[slot] |= QUEST_LOG_ROW_ROOT | nested_bits;
        self.player_data_changes.set(QUEST_LOG_PARENT_BIT);
        self.player_data_changes
            .set(QUEST_LOG_FIRST_SLOT_BIT + slot);
    }
}

#[cfg(test)]
#[path = "../player_tests/quest_log.rs"]
mod tests;
