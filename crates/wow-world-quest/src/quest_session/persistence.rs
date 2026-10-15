// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest status save and the login quest-status load, moved from the World
//! `handlers/quest/persistence.rs` (#1363 slice 6a).

use super::*;

pub async fn save_represented_quest_status_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest_id: u32,
) {
    if let Some(status) = host
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| {
            state
                .statuses_like_cpp()
                .get(&quest_id)
                .map(|status| status.status)
        })
    {
        save_quest_to_db(host, quest_id, status).await;
    }
}

pub async fn save_changed_represented_quest_statuses_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest_ids: &mut Vec<u32>,
) {
    let owner = host.core().quest_objective_access_like_cpp();
    crate::save_changed_quest_statuses_like_cpp(
        &owner,
        host.quest_state(),
        host.hub_ref().catalogs,
        host.lifecycle(),
        quest_ids,
        host.world_test_consumer_like_cpp(),
    )
    .await;
}

pub async fn save_represented_quest_statuses_completed_after_like_cpp<
    H: QuestSessionHostLikeCpp,
>(
    host: &mut H,
    completion_evidence_start: usize,
) {
    let completed_quest_ids: Vec<_> = host
        .quest_state()
        .represented_quest_complete_status_updates_from_like_cpp(completion_evidence_start)
        .iter()
        .filter_map(|evidence| {
            (evidence.new_status == QUEST_STATUS_COMPLETE_LIKE_CPP).then_some(evidence.quest_id)
        })
        .collect();
    for quest_id in completed_quest_ids {
        save_represented_quest_status_like_cpp(host, quest_id).await;
    }
}

pub async fn save_quest_to_db<H: QuestSessionHostLikeCpp>(host: &H, quest_id: u32, status: u8) {
    let owner = host.core().quest_objective_access_like_cpp();
    crate::save_quest_to_db_like_cpp(
        &owner,
        host.quest_state(),
        host.hub_ref().catalogs,
        host.lifecycle(),
        quest_id,
        status,
        host.world_test_consumer_like_cpp(),
    )
    .await;
}

/// Load all active quests for this player from the characters DB.
pub async fn load_player_quests<H: QuestSessionHostLikeCpp>(host: &mut H) {
    begin_player_quest_status_authority_load_like_cpp(host);

    let owner_guid = match host.player_guid() {
        Some(g) => g.counter() as u64,
        None => return,
    };
    let port = match host.lifecycle().player_quest_persistence_port_like_cpp() {
        Some(port) => port,
        None => return,
    };

    let active_rows = match port.load_active_statuses_like_cpp(owner_guid).await {
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) => rows,
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                error = %reason,
                "Failed to load quest status"
            );
            return;
        }
    };

    let mut loaded_quests = wow_entities::PlayerQuestGameplayState::default();

    let mut quest_status_rows_coherent_like_cpp = true;
    let mut next_active_slot: u8 = 0;
    let mut stale_rewarded_active_rows = Vec::new();

    for row in active_rows {
        let (
            Some(quest_id),
            Some(status),
            Some(explored),
            Some(accept_time_secs),
            Some(end_time_secs),
        ) = (
            row.quest_id,
            row.status,
            row.explored,
            row.accept_time_secs,
            row.end_time_secs,
        )
        else {
            quest_status_rows_coherent_like_cpp = false;
            continue;
        };
        let status = if status < 7 {
            status
        } else {
            QUEST_STATUS_INCOMPLETE_LIKE_CPP
        };
        let explored = explored != 0;

        if status == QUEST_STATUS_REWARDED_LIKE_CPP {
            // Rewarded (C++ QuestStatus::QUEST_STATUS_REWARDED / m_RewardedQuests).
            // Non-repeatable quests cannot be re-taken once rewarded.
            loaded_quests.set_rewarded_like_cpp(quest_id, true);
            stale_rewarded_active_rows.push(quest_id);
        } else if next_active_slot < MAX_QUEST_LOG_SIZE_LIKE_CPP {
            // Active or complete-but-not-turned-in.
            // C++ _LoadQuestStatus assigns sequential visible slots in DB row order
            // because the character DB status row has no persisted quest-log slot.
            let slot = next_active_slot;
            next_active_slot = next_active_slot.saturating_add(1);
            let store = host.hub_ref().catalogs.quests.store.as_ref();
            let obj_count = store
                .and_then(|s| s.get(quest_id))
                .map_or(0, |q| q.objectives.len());
            if loaded_quests.statuses_like_cpp().contains_key(&quest_id) {
                quest_status_rows_coherent_like_cpp = false;
            }
            loaded_quests.insert_status_like_cpp(
                quest_id,
                PlayerQuestStatus {
                    quest_id,
                    status,
                    explored,
                    accept_time_secs,
                    end_time_secs,
                    objective_counts: vec![0; obj_count],
                    slot,
                },
            );
        }
    }

    match port.load_objectives_like_cpp(owner_guid).await {
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(objective_rows) => {
            for row in objective_rows {
                let quest_id = row.quest_id.unwrap_or(0);
                let storage_index = row.storage_index.unwrap_or(0);
                let data = row.count.unwrap_or(0);
                if let (Some(status), Some(quest)) = (
                    loaded_quests.status_mut_like_cpp(quest_id),
                    host.hub_ref()
                        .catalogs
                        .quests
                        .store
                        .as_ref()
                        .and_then(|store| store.get(quest_id)),
                ) {
                    if let Some(objective) = quest.objectives.iter().find(|objective| {
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
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                error = %reason,
                "Failed to load quest objective status"
            );
        }
    }

    let mut rewarded_rows_coherent_like_cpp = false;
    match port.load_rewarded_like_cpp(owner_guid).await {
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rewarded_rows) => {
            rewarded_rows_coherent_like_cpp = true;
            for row in rewarded_rows {
                let Some(quest_id) = row.quest_id else {
                    rewarded_rows_coherent_like_cpp = false;
                    continue;
                };
                loaded_quests.set_rewarded_row_like_cpp(quest_id, true);
                if host
                    .hub_ref()
                    .catalogs
                    .represented_quest_can_increase_rewarded_counters_like_cpp(quest_id)
                    .is_some_and(|can_increase| can_increase)
                {
                    loaded_quests.set_rewarded_like_cpp(quest_id, true);
                }
            }
        }
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                error = %reason,
                "Failed to load rewarded quest status"
            );
        }
    }

    loaded_quests.set_status_authority_complete_like_cpp(
        quest_status_rows_coherent_like_cpp && rewarded_rows_coherent_like_cpp,
    );
    if install_represented_loaded_quest_statuses_like_cpp(host, &loaded_quests) == false {
        warn!(
            account = host.core().account_id,
            "Failed to install loaded quest status into canonical Player owner"
        );
        return;
    }

    stale_rewarded_active_rows.extend(remove_represented_active_rewarded_duplicates_like_cpp(host));
    stale_rewarded_active_rows.sort_unstable();
    stale_rewarded_active_rows.dedup();
    for quest_id in stale_rewarded_active_rows {
        info!(
            account = host.core().account_id,
            quest_id,
            "QuestLoad: migrating stale active rewarded quest status before deleting active row like C++"
        );
        save_quest_to_db(host, quest_id, QUEST_STATUS_REWARDED_LIKE_CPP).await;
    }

    let mut loaded_df = std::collections::BTreeSet::new();
    let mut loaded_daily = std::collections::BTreeSet::new();
    let mut loaded_last_daily_time = 0;
    match port.load_daily_like_cpp(owner_guid).await {
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(daily_rows) => {
            for row in daily_rows {
                let quest_id = row.quest_id.unwrap_or(0);
                let completed_time = row.completed_time.unwrap_or(0);
                let store = host.hub_ref().catalogs.quests.store.as_ref();
                if let Some(quest) = store.and_then(|store| store.get(quest_id)) {
                    loaded_last_daily_time = completed_time;
                    if quest.is_df_quest_like_cpp() {
                        loaded_df.insert(quest_id);
                    } else {
                        loaded_daily.insert(quest_id);
                    }
                }
            }
        }
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                error = %reason,
                "Failed to load daily quest status"
            );
        }
    }
    let _ = install_represented_loaded_daily_quests_like_cpp(
        host,
        loaded_df,
        loaded_daily,
        loaded_last_daily_time,
    );

    let mut loaded_weekly = std::collections::BTreeSet::new();
    match port.load_weekly_like_cpp(owner_guid).await {
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(weekly_rows) => {
            for row in weekly_rows {
                let quest_id = row.quest_id.unwrap_or(0);
                if host
                    .hub_ref()
                    .catalogs
                    .quests
                    .store
                    .as_ref()
                    .and_then(|store| store.get(quest_id))
                    .is_some()
                {
                    loaded_weekly.insert(quest_id);
                }
            }
        }
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                error = %reason,
                "Failed to load weekly quest status"
            );
        }
    }
    let _ = install_represented_loaded_weekly_quests_like_cpp(host, loaded_weekly);

    let mut loaded_monthly = std::collections::BTreeSet::new();
    match port.load_monthly_like_cpp(owner_guid).await {
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(monthly_rows) => {
            for row in monthly_rows {
                let quest_id = row.quest_id.unwrap_or(0);
                if host
                    .hub_ref()
                    .catalogs
                    .quests
                    .store
                    .as_ref()
                    .and_then(|store| store.get(quest_id))
                    .is_some()
                {
                    loaded_monthly.insert(quest_id);
                }
            }
        }
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                error = %reason,
                "Failed to load monthly quest status"
            );
        }
    }
    let _ = install_represented_loaded_monthly_quests_like_cpp(host, loaded_monthly);

    let seasonal_rows = match port.load_seasonal_like_cpp(owner_guid).await {
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) => rows
            .into_iter()
            .map(|row| {
                let quest_id = row.quest_id.unwrap_or_else(|| {
                    warn!(
                        account = host.core().account_id,
                        "Failed to read seasonal quest id"
                    );
                    0
                });
                let event_id = row.event_id.unwrap_or_else(|| {
                    warn!(
                        account = host.core().account_id,
                        quest_id, "Failed to read seasonal quest event id"
                    );
                    u32::MAX
                });
                let completed_time = row.completed_time.unwrap_or_else(|| {
                    warn!(
                        account = host.core().account_id,
                        quest_id, event_id, "Failed to read seasonal quest completedTime"
                    );
                    -1
                });
                SeasonalQuestStatusDbRowLikeCpp {
                    quest_id,
                    event_id,
                    completed_time,
                }
            })
            .collect(),
        wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                error = %reason,
                "Failed to load seasonal quest status"
            );
            Vec::new()
        }
    };

    let quest_store = host
        .hub_ref()
        .catalogs
        .quests
        .store
        .as_ref()
        .map(Arc::clone);
    let quest_v2_store = host
        .hub_ref()
        .catalogs
        .quests
        .v2_store
        .as_ref()
        .map(Arc::clone);
    let seasonal_outcome = load_seasonal_quest_status_like_cpp(
        host,
        seasonal_rows,
        quest_store.as_deref(),
        quest_v2_store.as_deref(),
    );

    if seasonal_outcome.skipped_no_quest_store > 0
        || seasonal_outcome.skipped_missing_quest > 0
        || seasonal_outcome.skipped_event_out_of_range > 0
        || seasonal_outcome.skipped_negative_completed_time > 0
        || seasonal_outcome.completed_bit_skipped_no_quest_v2_store > 0
        || seasonal_outcome.completed_bit_skipped_zero_unique_bit > 0
        || seasonal_outcome.completed_bit_no_change_or_noop > 0
    {
        warn!(
            account = host.core().account_id,
            rows_seen = seasonal_outcome.rows_seen,
            skipped_no_quest_store = seasonal_outcome.skipped_no_quest_store,
            skipped_missing_quest = seasonal_outcome.skipped_missing_quest,
            skipped_event_out_of_range = seasonal_outcome.skipped_event_out_of_range,
            skipped_negative_completed_time = seasonal_outcome.skipped_negative_completed_time,
            completed_bit_skipped_no_quest_v2_store =
                seasonal_outcome.completed_bit_skipped_no_quest_v2_store,
            completed_bit_skipped_zero_unique_bit =
                seasonal_outcome.completed_bit_skipped_zero_unique_bit,
            completed_bit_no_change_or_noop = seasonal_outcome.completed_bit_no_change_or_noop,
            "Skipped seasonal quest status rows during login load"
        );
    }

    let recurrence = host.player_quest_gameplay_snapshot_like_cpp();
    info!(
        account = host.core().account_id,
        active = recurrence
            .as_ref()
            .map_or(0, |state| state.statuses_like_cpp().len()),
        rewarded = recurrence
            .as_ref()
            .map_or(0, |state| state.rewarded_quest_ids_like_cpp().len()),
        df = recurrence
            .as_ref()
            .map_or(0, |state| state.df_quest_ids_like_cpp().len()),
        daily = recurrence
            .as_ref()
            .map_or(0, |state| state.daily_quest_ids_like_cpp().len()),
        weekly = recurrence
            .as_ref()
            .map_or(0, |state| state.weekly_quest_ids_like_cpp().len()),
        monthly = recurrence
            .as_ref()
            .map_or(0, |state| state.monthly_quest_ids_like_cpp().len()),
        seasonal_inserted = seasonal_outcome.inserted,
        seasonal_replaced = seasonal_outcome.replaced,
        seasonal_completed_bit_set = seasonal_outcome.completed_bit_set,
        seasonal_completed_bit_skipped_no_quest_v2_store =
            seasonal_outcome.completed_bit_skipped_no_quest_v2_store,
        seasonal_completed_bit_skipped_zero_unique_bit =
            seasonal_outcome.completed_bit_skipped_zero_unique_bit,
        seasonal_completed_bit_no_change_or_noop = seasonal_outcome.completed_bit_no_change_or_noop,
        "Loaded player quests"
    );
}
