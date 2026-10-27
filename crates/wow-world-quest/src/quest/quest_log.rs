// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read and initialize the Player-owned native QuestLog update fields.
//! Target Player.cpp:15963–16001 and :18613–18748.

use super::SessionQuestState;
use wow_world_core::session::{QuestObjectiveAccessLikeCpp, SessionCatalogs};

const QUEST_STATE_COMPLETE_LIKE_CPP: u32 = 0x0001;
const QUEST_STATE_FAIL_LIKE_CPP: u32 = 0x0002;
const QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP: u32 = 256;
const MAX_QUEST_LOG_SIZE_LIKE_CPP: u8 = 25;
const MAX_QUEST_COUNTS_LIKE_CPP: usize = 24;

pub fn get_quest_slot_quest_id_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    _quest_state: &SessionQuestState,
    slot: u8,
    _world_test_consumer: bool,
) -> Option<u32> {
    (slot < MAX_QUEST_LOG_SIZE_LIKE_CPP).then(|| owner.quest_slot_quest_id_like_cpp(slot))?
}

pub fn quest_log_create_entries_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    _quest_state: &SessionQuestState,
    _catalogs: &SessionCatalogs,
    _world_test_consumer: bool,
) -> Vec<(u32, u32, i64, [u16; MAX_QUEST_COUNTS_LIKE_CPP])> {
    let Some(rows) = owner.quest_log_values_snapshot_like_cpp() else {
        return Vec::new();
    };
    rows.into_iter()
        .map(|row| {
            (
                row.quest_id() as u32,
                row.state_flags(),
                row.end_time(),
                *row.objective_progress(),
            )
        })
        .collect()
}

/// Install durable quest status and objective data into the native rows before CREATE.
pub(crate) fn initialize_quest_log_from_loaded_state_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    state: &wow_entities::PlayerQuestGameplayState,
    catalogs: &SessionCatalogs,
    objective_rows: &[(u32, u8, i32)],
) {
    let mut occupied = [false; MAX_QUEST_LOG_SIZE_LIKE_CPP as usize];

    for status in state.statuses_like_cpp().values() {
        let slot = status.slot;
        if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP
            || status.status == wow_conditions::QUEST_STATUS_NONE_LIKE_CPP
            || catalogs
                .quests
                .store
                .as_ref()
                .and_then(|store| store.get(status.quest_id))
                .is_none()
            || owner.set_quest_slot_like_cpp(slot, status.quest_id) != Some(true)
        {
            continue;
        }
        occupied[usize::from(slot)] = true;
        let quest = catalogs
            .quests
            .store
            .as_ref()
            .and_then(|store| store.get(status.quest_id));
        let end_time = quest.is_some_and(|quest| {
            quest.limit_time_secs > 0
                && !state
                    .rewarded_quest_ids_like_cpp()
                    .contains(&status.quest_id)
        });
        let _ = owner.set_quest_slot_end_time_like_cpp(
            slot,
            if end_time { status.end_time_secs } else { 0 },
        );
        match status.status {
            wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP => {
                let _ = owner.set_quest_slot_state_like_cpp(slot, QUEST_STATE_COMPLETE_LIKE_CPP);
            }
            wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP => {
                let _ = owner.set_quest_slot_state_like_cpp(slot, QUEST_STATE_FAIL_LIKE_CPP);
            }
            _ => {}
        }
    }

    for &(quest_id, storage_index, data) in objective_rows {
        let Some(status) = state.status_like_cpp(quest_id) else {
            continue;
        };
        let Some(quest) = catalogs
            .quests
            .store
            .as_ref()
            .and_then(|store| store.get(quest_id))
        else {
            continue;
        };
        let Some(objective) = quest
            .objectives
            .iter()
            .find(|objective| u8::try_from(objective.storage_index).ok() == Some(storage_index))
        else {
            continue;
        };
        if usize::from(storage_index) >= MAX_QUEST_COUNTS_LIKE_CPP {
            continue;
        }
        if objective.is_storing_flag_like_cpp() {
            if data != 0 {
                let flag = QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP
                    .checked_shl(u32::from(storage_index))
                    .unwrap_or(0);
                let _ = owner.set_quest_slot_state_like_cpp(status.slot, flag);
            }
        } else {
            let _ = owner.set_quest_slot_counter_like_cpp(status.slot, storage_index, data as u16);
        }
    }

    for (slot, occupied) in occupied.into_iter().enumerate() {
        if !occupied {
            let _ = owner.set_quest_slot_like_cpp(slot as u8, 0);
        }
    }
}

/// Apply only objective and completion fields changed by an item-status operation.
pub(crate) fn apply_quest_log_status_delta_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    before: &wow_entities::PlayerQuestGameplayState,
    after: &wow_entities::PlayerQuestGameplayState,
    catalogs: &SessionCatalogs,
    changed_quest_ids: &[u32],
) {
    let Some(rows) = owner.quest_log_values_snapshot_like_cpp() else {
        return;
    };
    let quest_store = catalogs.quests.store.as_ref();

    for &quest_id in changed_quest_ids {
        let previous = before.statuses_like_cpp().get(&quest_id);
        let Some(current) = after.statuses_like_cpp().get(&quest_id) else {
            continue;
        };

        if let Some(previous) = previous {
            if previous.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                && current.status == wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
            {
                let _ = owner
                    .set_quest_slot_state_like_cpp(current.slot, QUEST_STATE_COMPLETE_LIKE_CPP);
            } else if previous.status == wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
                && current.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
            {
                if let Some(slot) = rows
                    .iter()
                    .position(|row| row.quest_id() as u32 == quest_id)
                {
                    let _ = owner.remove_quest_slot_state_like_cpp(
                        slot as u8,
                        QUEST_STATE_COMPLETE_LIKE_CPP,
                    );
                }
            }

            let quest = quest_store.and_then(|store| store.get(quest_id));
            for index in 0..MAX_QUEST_COUNTS_LIKE_CPP {
                let old_count = previous.objective_counts.get(index).copied().unwrap_or(0);
                let new_count = current.objective_counts.get(index).copied().unwrap_or(0);
                if old_count == new_count {
                    continue;
                }
                if current.slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
                    continue;
                }
                let stores_flag = quest.is_some_and(|quest| {
                    quest.objectives.iter().any(|objective| {
                        objective.storage_index == index as i8
                            && objective.is_storing_flag_like_cpp()
                    })
                });
                if stores_flag {
                    let state_flag = QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP << index;
                    if new_count != 0 {
                        let _ = owner.set_quest_slot_state_like_cpp(current.slot, state_flag);
                    } else {
                        let _ = owner.remove_quest_slot_state_like_cpp(current.slot, state_flag);
                    }
                } else {
                    let _ = owner.set_quest_slot_counter_like_cpp(
                        current.slot,
                        index as u8,
                        new_count as u16,
                    );
                }
            }
        }
    }
}
