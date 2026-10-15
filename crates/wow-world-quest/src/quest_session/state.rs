// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest log slots and the represented quest-status installers, moved from the
//! World `handlers/quest/state.rs` and `session/quest/state.rs` (#1363 slice 6a).

use super::*;

pub fn remove_represented_active_rewarded_duplicates_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
) -> Vec<u32> {
    let Some(state) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return Vec::new();
    };
    let mut duplicate_quest_ids = state
        .statuses_like_cpp()
        .keys()
        .filter(|quest_id| {
            let store = host.hub_ref().catalogs.quests.store.as_ref();
            state.rewarded_quest_ids_like_cpp().contains(quest_id)
                && store
                    .and_then(|store| store.get(**quest_id))
                    .is_some_and(|quest| !quest.is_repeatable())
        })
        .copied()
        .collect::<Vec<_>>();
    duplicate_quest_ids.sort_unstable();
    duplicate_quest_ids.dedup();

    if !duplicate_quest_ids.is_empty() {
        host.invalidate_player_quest_status_authority_like_cpp();
    }

    if !duplicate_quest_ids.is_empty() {
        let duplicate_ids = duplicate_quest_ids.clone();
        let _ = host.mutate_player_quest_gameplay_like_cpp(|state| {
            for quest_id in &duplicate_ids {
                state.remove_status_like_cpp(*quest_id);
            }
            let mut remaining_slots = state
                .statuses_like_cpp()
                .iter()
                .map(|(quest_id, status)| (*quest_id, status.slot))
                .collect::<Vec<_>>();
            remaining_slots.sort_by_key(|(_, slot)| *slot);
            for (slot, (quest_id, _)) in remaining_slots.into_iter().enumerate() {
                if let Some(status) = state.status_mut_like_cpp(quest_id) {
                    status.slot =
                        u8::try_from(slot).unwrap_or(MAX_QUEST_LOG_SIZE_LIKE_CPP.saturating_sub(1));
                }
            }
        });
    }

    duplicate_quest_ids
}

pub fn first_free_quest_slot_like_cpp<H: QuestSessionHostLikeCpp>(host: &H) -> Option<u8> {
    (0..MAX_QUEST_LOG_SIZE_LIKE_CPP).find(|&slot| !quest_slot_has_active_entry_like_cpp(host, slot))
}

fn quest_slot_has_active_entry_like_cpp<H: QuestSessionHostLikeCpp>(host: &H, slot: u8) -> bool {
    // C++ `QuestSlotOffset` stores the quest id independently from the status fields;
    // represented active slots are INCOMPLETE, COMPLETE, or FAILED.
    slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
        && host
            .player_quest_gameplay_snapshot_like_cpp()
            .is_some_and(|state| {
                state.statuses_like_cpp().values().any(|status| {
                    status.slot == slot
                        && matches!(
                            status.status,
                            QUEST_STATUS_INCOMPLETE_LIKE_CPP
                                | QUEST_STATUS_COMPLETE_LIKE_CPP
                                | QUEST_STATUS_FAILED_LIKE_CPP
                        )
                })
            })
}

/// C++ `Player::SetQuestStatus` (`Player.cpp:15557`) installing one record.
pub fn insert_represented_quest_status_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    quest_id: u32,
    status: wow_entities::PlayerQuestStatusRecord,
) -> bool {
    host.mutate_player_quest_gameplay_like_cpp(|state| {
        state.insert_status_like_cpp(quest_id, status);
    })
    .is_some()
}

/// C++ `Player::m_RewardedQuests` gaining or losing one quest.
pub fn set_represented_quest_rewarded_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    quest_id: u32,
    rewarded: bool,
) -> bool {
    host.mutate_player_quest_gameplay_like_cpp(|state| {
        state.set_rewarded_like_cpp(quest_id, rewarded);
    })
    .is_some()
}

/// Mark one quest explored, reporting whether the record existed and
/// whether the client must be told.
pub fn mark_represented_quest_explored_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    quest_id: u32,
    failed_status: u8,
) -> Option<(bool, bool)> {
    host.mutate_player_quest_gameplay_like_cpp(|state| {
        let Some(status) = state.status_mut_like_cpp(quest_id) else {
            return (false, false);
        };
        let should_send = !status.explored && status.status != failed_status;
        if should_send {
            status.explored = true;
        }
        (true, should_send)
    })
}

/// Install the authoritative loaded quest statuses and rewarded sets
/// (`Player::_LoadQuestStatus` and `_LoadQuestStatusRewarded`).
pub(crate) fn install_represented_loaded_quest_statuses_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    loaded: &wow_entities::PlayerQuestGameplayState,
) -> bool {
    host.mutate_player_quest_gameplay_like_cpp(|state| {
        state.replace_statuses_like_cpp(
            loaded.statuses_snapshot_like_cpp(),
            loaded.status_authority_complete_like_cpp(),
        );
        state.replace_rewarded_quest_ids_like_cpp(loaded.rewarded_quest_ids_like_cpp().clone());
        state.replace_rewarded_quest_rows_like_cpp(loaded.rewarded_quest_rows_like_cpp().clone());
    })
    .is_some()
}

/// Install the loaded daily bucket and its timestamp.
pub(crate) fn install_represented_loaded_daily_quests_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    df_quest_ids: std::collections::BTreeSet<u32>,
    daily_quest_ids: std::collections::BTreeSet<u32>,
    last_daily_time_secs: i64,
) -> bool {
    host.mutate_player_quest_gameplay_like_cpp(|state| {
        state.replace_df_quest_ids_like_cpp(df_quest_ids);
        state.replace_daily_quest_ids_like_cpp(daily_quest_ids);
        state.set_last_daily_quest_time_secs_like_cpp(last_daily_time_secs);
    })
    .is_some()
}

/// Install the loaded weekly bucket.
pub(crate) fn install_represented_loaded_weekly_quests_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    weekly_quest_ids: std::collections::BTreeSet<u32>,
) -> bool {
    host.mutate_player_quest_gameplay_like_cpp(|state| {
        state.replace_weekly_quest_ids_like_cpp(weekly_quest_ids);
    })
    .is_some()
}

/// Install the loaded monthly bucket.
pub(crate) fn install_represented_loaded_monthly_quests_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    monthly_quest_ids: std::collections::BTreeSet<u32>,
) -> bool {
    host.mutate_player_quest_gameplay_like_cpp(|state| {
        state.replace_monthly_quest_ids_like_cpp(monthly_quest_ids);
    })
    .is_some()
}

pub(crate) fn player_quest_level_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> i32 {
    let (state, hub) = host.split_quest_state_ref();
    let player_level = if quest.quest_level > 0 {
        0
    } else {
        hub.player_level_like_cpp()
    };
    state.player_quest_level_like_cpp(player_level, quest)
}
