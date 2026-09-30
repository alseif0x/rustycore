//! Quest hydration operation at the application boundary.
use super::*;

impl WorldSession {


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

        let mut hydration = wow_entities::PlayerQuestGameplayState::begin_status_hydration();

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
                hydration.reject_active_row();
                continue;
            };
            hydration.hydrate_active_status(
                &mut loaded_quests, quest_id, status, explored, accept_time_secs, end_time_secs,
                |quest_id| {
                    let store = self.quests.store.as_ref();
                    store.and_then(|s| s.get(quest_id)).map_or(0, |q| q.objectives.len())
                },
            );
        }

        match port.load_objectives_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(objective_rows) => {
                for row in objective_rows {
                    let quest_id = row.quest_id.unwrap_or(0);
                    let storage_index = row.storage_index.unwrap_or(0);
                    let data = row.count.unwrap_or(0);
                    loaded_quests.hydrate_objective_count(quest_id, storage_index, data, |id| {
                        self.quests.store.as_ref().and_then(|store| store.get(id))
                            .map(|quest| quest.objectives.as_slice())
                    });
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

        match port.load_rewarded_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rewarded_rows) => {
                hydration.rewarded_rows_loaded();
                for row in rewarded_rows {
                    hydration.hydrate_rewarded_row(&mut loaded_quests, row.quest_id, |id| {
                        self.represented_quest_can_increase_rewarded_counters_like_cpp(id)
                    });
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

        let mut stale_rewarded_active_rows = hydration.finish(&mut loaded_quests);
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
