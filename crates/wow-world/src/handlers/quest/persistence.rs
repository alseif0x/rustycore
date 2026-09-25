// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest status persistence and load.

use super::*;

mod item_mutations;

impl WorldSession {
    pub(super) async fn quest_poi_store_like_cpp(&mut self) -> Arc<HashMap<i32, QuestPoiData>> {
        if let Some(store) = &self.quest_poi_store_like_cpp {
            return Arc::clone(store);
        }

        let Some(port) = self.quest_poi_persistence_port_like_cpp() else {
            warn!(
                "QuestPOIQuery: quest POI persistence port unavailable; sending empty C++ response"
            );
            let store = Arc::new(HashMap::new());
            self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
            return store;
        };

        let store = match port.load_quest_poi_rows_like_cpp().await {
            wow_persistence::QuestPoiLoadOutcomeLikeCpp::Loaded { points, blobs } => {
                Arc::new(build_quest_poi_store_like_cpp(points, blobs))
            }
            wow_persistence::QuestPoiLoadOutcomeLikeCpp::Failed { stage, reason } => {
                warn!(?stage, error = %reason, "QuestPOIQuery: failed to load quest POI store like C++");
                Arc::new(HashMap::new())
            }
        };

        self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
        store
    }

    pub(crate) async fn save_represented_quest_status_like_cpp(&self, quest_id: u32) {
        if let Some(status) = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| {
                state
                    .statuses_like_cpp()
                    .get(&quest_id)
                    .map(|status| status.status)
            })
        {
            self.save_quest_to_db(quest_id, status).await;
        }
    }

    pub(crate) async fn save_changed_represented_quest_statuses_like_cpp(
        &self,
        quest_ids: &mut Vec<u32>,
    ) {
        quest_ids.sort_unstable();
        quest_ids.dedup();
        for quest_id in quest_ids.drain(..) {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_quest_statuses_for_save_like_cpp(&self) -> Vec<(u32, u8)> {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return Vec::new();
        };
        let mut quests = state
            .statuses_like_cpp()
            .iter()
            .filter_map(|(quest_id, status)| {
                let store = self.quests.store.as_ref();
                if state.rewarded_quest_ids_like_cpp().contains(quest_id)
                    && store
                        .and_then(|store| store.get(*quest_id))
                        .is_some_and(|quest| !quest.is_repeatable())
                {
                    return None;
                }

                Some((*quest_id, status.status))
            })
            .collect::<Vec<_>>();
        quests.sort_by_key(|(quest_id, _)| *quest_id);
        quests
    }

    pub(super) async fn save_represented_quest_statuses_completed_after_like_cpp(
        &mut self,
        completion_evidence_start: usize,
    ) {
        let completed_quest_ids: Vec<_> = self.represented_quest_complete_status_updates_like_cpp
            [completion_evidence_start..]
            .iter()
            .filter_map(|evidence| {
                (evidence.new_status == QUEST_STATUS_COMPLETE_LIKE_CPP).then_some(evidence.quest_id)
            })
            .collect();
        for quest_id in completed_quest_ids {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
    }

    /// Project the quest's durable status without writing it.
    ///
    /// Shared by the standalone save below and by the quest-reward operation,
    /// which carries the same row inside its own closing transaction rather
    /// than committing it separately.
    pub(crate) fn plan_quest_status_save_like_cpp(
        &self,
        quest_id: u32,
        status: u8,
    ) -> Option<wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp> {
        let owner_guid = self.player_guid()?.counter() as u64;
        let quest_state = self.player_quest_gameplay_snapshot_like_cpp();
        let mut projection = match quest_state
            .as_ref()
            .and_then(|state| state.statuses_like_cpp().get(&quest_id))
        {
            Some(saved) => self.represented_quest_status_persistence_like_cpp(saved),
            None if status == QUEST_STATUS_REWARDED_LIKE_CPP => {
                wow_persistence::QuestStatusPersistenceLikeCpp {
                    quest_id,
                    status,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objectives: Vec::new(),
                }
            }
            None => {
                warn!(
                    account = self.account_id,
                    quest_id,
                    "Quest status save skipped because canonical Player quest state is unavailable"
                );
                return None;
            }
        };
        projection.status = status;
        Some(
            wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp::Save {
                owner_guid,
                status: projection,
            },
        )
    }

    pub(super) async fn save_quest_to_db(&self, quest_id: u32, status: u8) {
        let port = match self.player_quest_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        let Some(request) = self.plan_quest_status_save_like_cpp(quest_id, status) else {
            return;
        };

        match port.persist_status_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                quest_id,
                error = %reason,
                "Failed to save quest status"
            ),
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                quest_id,
                error = %reason,
                "Quest status save commit outcome is unknown"
            ),
        }
    }

    /// Delete a quest from the characters database (abandon).
    pub(super) async fn delete_quest_from_db(&self, quest_id: u32) {
        let owner_guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let port = match self.player_quest_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        match port
            .persist_status_like_cpp(
                wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp::Delete {
                    owner_guid,
                    quest_id,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                quest_id,
                error = %reason,
                "Failed to delete quest"
            ),
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                quest_id,
                error = %reason,
                "Quest deletion commit outcome is unknown"
            ),
        }
    }

    /// Load all active quests for this player from the characters DB.
    pub(crate) async fn load_player_quests(&mut self) {
        self.begin_player_quest_status_authority_load_like_cpp();

        let owner_guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let port = match self.player_quest_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let active_rows = match port.load_active_statuses_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
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
                let store = self.quests.store.as_ref();
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
                        self.quests
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
                            status.objective_counts[index] = if objective.is_storing_flag_like_cpp()
                            {
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
                    account = self.account_id,
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
                    if self
                        .represented_quest_can_increase_rewarded_counters_like_cpp(quest_id)
                        .is_some_and(|can_increase| can_increase)
                    {
                        loaded_quests.set_rewarded_like_cpp(quest_id, true);
                    }
                }
            }
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "Failed to load rewarded quest status"
                );
            }
        }

        loaded_quests.set_status_authority_complete_like_cpp(
            quest_status_rows_coherent_like_cpp && rewarded_rows_coherent_like_cpp,
        );
        if self.install_represented_loaded_quest_statuses_like_cpp(&loaded_quests) == false {
            warn!(
                account = self.account_id,
                "Failed to install loaded quest status into canonical Player owner"
            );
            return;
        }

        stale_rewarded_active_rows
            .extend(self.remove_represented_active_rewarded_duplicates_like_cpp());
        stale_rewarded_active_rows.sort_unstable();
        stale_rewarded_active_rows.dedup();
        for quest_id in stale_rewarded_active_rows {
            info!(
                account = self.account_id,
                quest_id,
                "QuestLoad: migrating stale active rewarded quest status before deleting active row like C++"
            );
            self.save_quest_to_db(quest_id, QUEST_STATUS_REWARDED_LIKE_CPP)
                .await;
        }

        let mut loaded_df = std::collections::BTreeSet::new();
        let mut loaded_daily = std::collections::BTreeSet::new();
        let mut loaded_last_daily_time = 0;
        match port.load_daily_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(daily_rows) => {
                for row in daily_rows {
                    let quest_id = row.quest_id.unwrap_or(0);
                    let completed_time = row.completed_time.unwrap_or(0);
                    let store = self.quests.store.as_ref();
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
                    account = self.account_id,
                    error = %reason,
                    "Failed to load daily quest status"
                );
            }
        }
        let _ = self.install_represented_loaded_daily_quests_like_cpp(
            loaded_df,
            loaded_daily,
            loaded_last_daily_time,
        );

        let mut loaded_weekly = std::collections::BTreeSet::new();
        match port.load_weekly_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(weekly_rows) => {
                for row in weekly_rows {
                    let quest_id = row.quest_id.unwrap_or(0);
                    if self
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
                    account = self.account_id,
                    error = %reason,
                    "Failed to load weekly quest status"
                );
            }
        }
        let _ = self.install_represented_loaded_weekly_quests_like_cpp(loaded_weekly);

        let mut loaded_monthly = std::collections::BTreeSet::new();
        match port.load_monthly_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(monthly_rows) => {
                for row in monthly_rows {
                    let quest_id = row.quest_id.unwrap_or(0);
                    if self
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
                    account = self.account_id,
                    error = %reason,
                    "Failed to load monthly quest status"
                );
            }
        }
        let _ = self.install_represented_loaded_monthly_quests_like_cpp(loaded_monthly);

        let seasonal_rows = match port.load_seasonal_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) => rows
                .into_iter()
                .map(|row| {
                    let quest_id = row.quest_id.unwrap_or_else(|| {
                        warn!(
                            account = self.account_id,
                            "Failed to read seasonal quest id"
                        );
                        0
                    });
                    let event_id = row.event_id.unwrap_or_else(|| {
                        warn!(
                            account = self.account_id,
                            quest_id, "Failed to read seasonal quest event id"
                        );
                        u32::MAX
                    });
                    let completed_time = row.completed_time.unwrap_or_else(|| {
                        warn!(
                            account = self.account_id,
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
                    account = self.account_id,
                    error = %reason,
                    "Failed to load seasonal quest status"
                );
                Vec::new()
            }
        };

        let quest_store = self.quests.store.as_ref().map(Arc::clone);
        let quest_v2_store = self.quests.v2_store.as_ref().map(Arc::clone);
        let seasonal_outcome = self.load_seasonal_quest_status_like_cpp(
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
                account = self.account_id,
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

        let recurrence = self.player_quest_gameplay_snapshot_like_cpp();
        info!(
            account = self.account_id,
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
            seasonal_completed_bit_no_change_or_noop =
                seasonal_outcome.completed_bit_no_change_or_noop,
            "Loaded player quests"
        );
    }
}
