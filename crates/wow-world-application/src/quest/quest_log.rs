// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Fresh quest-log projection and selected slot publication.
//! Target Player.cpp:15963–15990 (`SetQuestSlot`, `SetQuestSlotCounter`,
//! `SetQuestSlotState`) identifies the fields; this retains the Rust projection
//! and full-slot packet policy, including its signed-count conversion.

use super::{SessionQuestState, objective_progress::{
    MAX_QUEST_LOG_SIZE_LIKE_CPP, current_quest_gameplay_snapshot_like_cpp,
}};
use wow_world_core::session::{QuestObjectiveAccessLikeCpp, PacketPublicationAccessLikeCpp, SessionCatalogs};

const QUEST_STATE_COMPLETE_LIKE_CPP: u32 = 0x0001;
const QUEST_STATE_FAIL_LIKE_CPP: u32 = 0x0002;
const QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP: u32 = 256;

pub fn get_quest_slot_quest_id_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    slot: u8,
    world_test_consumer: bool,
) -> Option<u32> {
    if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
        return None;
    }
    let state = current_quest_gameplay_snapshot_like_cpp(owner, quest_state, world_test_consumer)?;
    let mut matching_quest_id = None;
    for status in state.statuses_like_cpp().values().filter(|status| {
        status.slot == slot
            && matches!(status.status,
                wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                | wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
                | wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP)
    }) {
        if matching_quest_id.is_some() {
            return None;
        }
        matching_quest_id = Some(status.quest_id);
    }
    matching_quest_id
}

pub fn quest_log_create_entries_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    catalogs: &SessionCatalogs,
    world_test_consumer: bool,
) -> Vec<(u32, u32, i64, [u16; 24])> {
    let Some(state) = current_quest_gameplay_snapshot_like_cpp(owner, quest_state, world_test_consumer) else {
        return Vec::new();
    };
    (0..MAX_QUEST_LOG_SIZE_LIKE_CPP).map(|slot| {
        let Some(quest_id) = get_quest_slot_quest_id_like_cpp(owner, quest_state, slot, world_test_consumer) else {
            return (0, 0, 0, [0; 24]);
        };
        let Some(qs) = state.statuses_like_cpp().get(&quest_id) else {
            return (0, 0, 0, [0; 24]);
        };
        let store = catalogs.quests.store.as_ref();
        let quest = store.and_then(|store| store.get(qs.quest_id));
        let mut state_flags: u32 = match qs.status {
            wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP => QUEST_STATE_COMPLETE_LIKE_CPP,
            wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP => QUEST_STATE_FAIL_LIKE_CPP,
            _ => 0,
        };
        let mut obj_progress = [0u16; 24];
        for (i, slot_progress) in obj_progress.iter_mut().enumerate() {
            let count = qs.objective_counts.get(i).copied().unwrap_or(0);
            let stores_flag = quest.is_some_and(|quest| {
                quest.objectives.iter().any(|objective| {
                    objective.storage_index == i as i8 && objective.is_storing_flag_like_cpp()
                })
            });
            if stores_flag {
                if count != 0 {
                    state_flags |= QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP << i;
                }
                continue;
            }
            *slot_progress = count.min(u16::MAX as i32) as u16;
        }
        (qs.quest_id, state_flags, qs.end_time_secs, obj_progress)
    }).collect()
}

pub fn send_represented_quest_log_slot_update_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    catalogs: &SessionCatalogs,
    publication: &PacketPublicationAccessLikeCpp<'_>,
    slot: u8,
    world_test_consumer: bool,
) {
    if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
        return;
    }
    let Some(guid) = owner.player_guid_like_cpp() else {
        return;
    };
    let Some((quest_id, state_flags, end_time, objective_progress)) =
        quest_log_create_entries_like_cpp(owner, quest_state, catalogs, world_test_consumer)
            .get(slot as usize).copied()
    else {
        return;
    };
    let mut data = wow_packet::packets::update::PlayerDataValuesDeltaUpdate::default();
    data.player_data_mask[35 / 32] |= 1 << (35 % 32);
    let slot_bit = 36 + usize::from(slot);
    data.player_data_mask[slot_bit / 32] |= 1 << (slot_bit % 32);
    data.quest_log[slot as usize] = wow_packet::packets::update::QuestLogValuesUpdate {
        // C++ Player::SetQuestSlot marks QuestID, StateFlags, EndTime,
        // and every ObjectiveProgress field changed for the slot.
        quest_log_mask: 0x1FFF_FFFF,
        end_time,
        quest_id: quest_id.min(i32::MAX as u32) as i32,
        state_flags,
        objective_progress,
    };
    publication.send_packet(&wow_packet::packets::update::UpdateObject::full_player_values_update(
        guid, owner.player_map_id_like_cpp(), data,
    ));
}
