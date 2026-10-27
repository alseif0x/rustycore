//! Native QuestLog fixtures for canonical quest sessions.

use super::*;

pub(super) fn seed_native_quest_log_statuses_like_cpp(
    session: &WorldSession,
    statuses: &[wow_entities::PlayerQuestStatusRecord],
) {
    let owner = session.core.quest_objective_access_like_cpp();
    for status in statuses {
        if status.status == crate::conditions::QUEST_STATUS_NONE_LIKE_CPP || status.slot >= 25 {
            continue;
        }
        let Some(quest) = session
            .catalogs
            .quests
            .store
            .as_ref()
            .and_then(|store| store.get(status.quest_id))
        else {
            continue;
        };
        assert_eq!(
            owner.set_quest_slot_like_cpp(status.slot, status.quest_id),
            Some(true)
        );
        let end_time = if quest.limit_time_secs > 0 {
            status.end_time_secs
        } else {
            0
        };
        assert_eq!(
            owner.set_quest_slot_end_time_like_cpp(status.slot, end_time),
            Some(true)
        );
        match status.status {
            crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP => {
                assert_eq!(
                    owner.set_quest_slot_state_like_cpp(status.slot, 1),
                    Some(true)
                );
            }
            crate::conditions::QUEST_STATUS_FAILED_LIKE_CPP => {
                assert_eq!(
                    owner.set_quest_slot_state_like_cpp(status.slot, 2),
                    Some(true)
                );
            }
            _ => {}
        }
        for (index, count) in status.objective_counts.iter().copied().enumerate().take(24) {
            let stores_flag = quest.objectives.iter().any(|objective| {
                objective.storage_index == index as i8 && objective.is_storing_flag_like_cpp()
            });
            if stores_flag {
                if count != 0 {
                    assert_eq!(
                        owner.set_quest_slot_state_like_cpp(status.slot, 256_u32 << index),
                        Some(true)
                    );
                }
            } else {
                assert_eq!(
                    owner.set_quest_slot_counter_like_cpp(status.slot, index as u8, count as u16),
                    Some(true)
                );
            }
        }
    }
}

pub(in crate::session::tests) fn publish_quest_log_values_after_map_tick_like_cpp(
    session: &mut WorldSession,
) -> usize {
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .unwrap_or_else(shared_canonical_map_manager);
    session.set_canonical_map_manager(Arc::clone(&manager));
    let (map_id, instance_id) = session.core.current_runtime_map_key_like_cpp();
    insert_session_player_into_canonical_map_like_cpp(
        session,
        &manager,
        u32::from(map_id),
        instance_id,
    );
    let Ok(mut manager) = manager.lock() else {
        return 0;
    };
    let _ = manager.update(1);
    drop(manager);
    session.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp()
}
