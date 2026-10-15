// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest log slots and the represented quest-status installers, moved from the
//! World `handlers/quest/state.rs` and `session/quest/state.rs` (#1363 slice 6a).

use super::*;

pub const QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP: u32 = 0x0010_0000;
pub const QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP: u32 = 0x0000_1000;

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

pub async fn complete_represented_quest_after_add_with_generator_like_cpp<
    H: QuestSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    host.complete_represented_quest_after_objective_with_generator_like_cpp(
        item_guid_generator,
        quest,
        0,
    )
    .await
}

pub async fn add_quest_confirm_accept_local_state_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    let Some(slot) = crate::first_free_quest_slot_like_cpp(host) else {
        return false;
    };

    let (accept_time_secs, end_time_secs) =
        quest.accepted_and_end_time_like_cpp(wow_core::GameTime::now().as_secs() as i64);

    host.invalidate_player_quest_status_authority_like_cpp();
    let status = PlayerQuestStatus {
        quest_id: quest.id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs,
        end_time_secs,
        objective_counts: vec![0; quest.objectives.len()],
        slot,
    };
    if crate::insert_represented_quest_status_like_cpp(host, quest.id, status) == false {
        return false;
    }
    complete_represented_quest_after_add_with_generator_like_cpp(host, item_guid_generator, quest)
        .await;
    crate::save_represented_quest_status_like_cpp(host, quest.id).await;
    host.sync_player_registry_state_like_cpp();
    true
}

pub fn represented_quest_login_aura_sources_are_hit_inert_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    difficulty_id: u8,
) -> bool {
    let Some(state) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return false;
    };
    if !state.status_authority_complete_like_cpp() {
        return false;
    }
    let Some(quests) = host.hub_ref().catalogs.quests.store.as_ref() else {
        return state.rewarded_quest_rows_like_cpp().is_empty()
            && state.statuses_like_cpp().is_empty();
    };

    // C++ `_LoadQuestStatusRewarded` calls `LearnQuestRewardedSpells`
    // before filtering special quests out of `m_RewardedQuests`. Keep the
    // narrow TESTBOT proof to rows whose static reward spell is absent.
    if state.rewarded_quest_rows_like_cpp().iter().any(|quest_id| {
        quests
            .get(*quest_id)
            .is_none_or(|quest| quest.reward_spell != 0)
    }) {
        return false;
    }

    state.statuses_like_cpp().values().all(|status| {
        let Some(quest) = quests.get(status.quest_id) else {
            return false;
        };
        let recasts_accept_spell = quest.flags & QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP != 0
            && quest.flags_ex & QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP != 0
            && quest.source_spell_id != 0;
        !recasts_accept_spell
            || host.player_target_spell_is_hit_inert_like_cpp(quest.source_spell_id, difficulty_id)
    })
}
