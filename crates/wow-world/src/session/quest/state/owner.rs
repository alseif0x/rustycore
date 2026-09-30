//! owner operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(crate) fn player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerQuestGameplayState> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.player_quest_gameplay_fixture_like_cpp());
        }
        self.with_owned_player_like_cpp(|player| player.gameplay_state().quests.clone())
    }
    /// C++ `Player::SetQuestStatus` (`Player.cpp:15557`) installing one record.
    pub(crate) fn insert_represented_quest_status_like_cpp(
        &mut self,
        quest_id: u32,
        status: wow_entities::PlayerQuestStatusRecord,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.insert_status_like_cpp(quest_id, status);
        })
        .is_some()
    }

    /// C++ `Player::RemoveActiveQuest` (`Player.cpp:15575`).
    pub(crate) fn remove_represented_quest_status_like_cpp(&mut self, quest_id: u32) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.remove_status_like_cpp(quest_id);
        })
        .is_some()
    }

    /// C++ `Player::m_RewardedQuests` gaining or losing one quest.
    pub(crate) fn set_represented_quest_rewarded_like_cpp(
        &mut self,
        quest_id: u32,
        rewarded: bool,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_rewarded_like_cpp(quest_id, rewarded);
        })
        .is_some()
    }

    /// Settle one rewarded quest: C++ `Player::RewardQuest` removes the active
    /// entry and records a non-repeatable quest as rewarded.
    pub(crate) fn settle_represented_rewarded_quest_like_cpp(
        &mut self,
        quest_id: u32,
        repeatable: bool,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.settle_rewarded_quest(quest_id, repeatable);
        })
        .is_some()
    }

    /// C++ `Player::CompleteQuest` moving one incomplete quest to complete and
    /// reporting the status it replaced.
    pub(crate) fn complete_represented_quest_status_like_cpp(
        &mut self,
        quest_id: u32,
    ) -> Option<u8> {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.complete_quest(quest_id)
        })
        .flatten()
    }

    /// Clear one quest's timer, as C++ does when its timed window ends.
    pub(crate) fn clear_represented_quest_end_time_like_cpp(&mut self, quest_id: u32) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.clear_quest_end_time(quest_id)
        })
        .unwrap_or(false)
    }

    /// Mark one quest explored, reporting whether the record existed and
    /// whether the client must be told.
    pub(crate) fn mark_represented_quest_explored_like_cpp(
        &mut self,
        quest_id: u32,
    ) -> Option<(bool, bool)> {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.mark_quest_explored(quest_id)
        })
    }

    /// Ensure one seasonal event exists, without changing its quests.
    pub(crate) fn ensure_represented_seasonal_event_like_cpp(&mut self, event_id: u16) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.ensure_seasonal_event_like_cpp(event_id);
        })
        .is_some()
    }

    /// C++ records a rewarded quest's recurrence under its own bucket
    /// (`SetDailyQuestStatus` and its weekly/monthly/seasonal siblings).
    pub(crate) fn record_represented_quest_recurrence_like_cpp(
        &mut self,
        recurrence: RepresentedQuestRecurrenceLikeCpp,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| match recurrence {
            RepresentedQuestRecurrenceLikeCpp::Daily {
                quest_id,
                now_secs,
                is_df_quest,
            } => {
                state.set_last_daily_quest_time_secs_like_cpp(now_secs);
                if is_df_quest {
                    state.set_df_quest_like_cpp(quest_id, true);
                } else {
                    state.set_daily_like_cpp(quest_id, true);
                }
            }
            RepresentedQuestRecurrenceLikeCpp::Weekly { quest_id } => {
                state.set_weekly_like_cpp(quest_id, true);
            }
            RepresentedQuestRecurrenceLikeCpp::Monthly { quest_id } => {
                state.set_monthly_like_cpp(quest_id, true);
            }
            RepresentedQuestRecurrenceLikeCpp::Seasonal {
                event_id,
                quest_id,
                completed_at,
            } => {
                state.set_seasonal_like_cpp(event_id, quest_id, completed_at);
            }
        })
        .is_some()
    }

    /// Install the authoritative loaded quest statuses and rewarded sets
    /// (`Player::_LoadQuestStatus` and `_LoadQuestStatusRewarded`).
    pub(crate) fn install_represented_loaded_quest_statuses_like_cpp(
        &mut self,
        loaded: &wow_entities::PlayerQuestGameplayState,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_statuses_like_cpp(
                loaded.statuses_snapshot_like_cpp(),
                loaded.status_authority_complete_like_cpp(),
            );
            state.replace_rewarded_quest_ids_like_cpp(loaded.rewarded_quest_ids_like_cpp().clone());
            state.replace_rewarded_quest_rows_like_cpp(
                loaded.rewarded_quest_rows_like_cpp().clone(),
            );
        })
        .is_some()
    }

    /// Install the loaded daily bucket and its timestamp.
    pub(crate) fn install_represented_loaded_daily_quests_like_cpp(
        &mut self,
        df_quest_ids: std::collections::BTreeSet<u32>,
        daily_quest_ids: std::collections::BTreeSet<u32>,
        last_daily_time_secs: i64,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_df_quest_ids_like_cpp(df_quest_ids);
            state.replace_daily_quest_ids_like_cpp(daily_quest_ids);
            state.set_last_daily_quest_time_secs_like_cpp(last_daily_time_secs);
        })
        .is_some()
    }

    /// Install the loaded weekly bucket.
    pub(crate) fn install_represented_loaded_weekly_quests_like_cpp(
        &mut self,
        weekly_quest_ids: std::collections::BTreeSet<u32>,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_weekly_quest_ids_like_cpp(weekly_quest_ids);
        })
        .is_some()
    }

    /// Install the loaded monthly bucket.
    pub(crate) fn install_represented_loaded_monthly_quests_like_cpp(
        &mut self,
        monthly_quest_ids: std::collections::BTreeSet<u32>,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_monthly_quest_ids_like_cpp(monthly_quest_ids);
        })
        .is_some()
    }

    /// Execute one admitted application writer on the canonical Player quest state.
    ///
    /// Objective and lifecycle rules live in PlayerQuestGameplayState's private
    /// children and consume borrowed model views. This boundary retains the
    /// existing ownership checks, per-writer execution and fixture compatibility;
    /// application snapshots, catalog reads, saves and publication stay separate.
    pub(crate) fn mutate_player_quest_gameplay_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut PlayerQuestGameplayState) -> R,
    ) -> Option<R> {
        let mut mutate = Some(mutate);
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            let mut fixture = self.player_quest_gameplay_fixture_like_cpp();
            let result = mutate.take().expect("test quest mutation executes once")(&mut fixture);
            self.apply_player_quest_gameplay_fixture_like_cpp(fixture);
            return Some(result);
        }
        let canonical = self.mutate_canonical_player_like_cpp(|player| {
            mutate.take().expect("Player quest mutation executes once")(
                &mut player.gameplay_state_mut().quests,
            )
        });
        if canonical.is_some() {
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(state) =
                self.with_owned_player_like_cpp(|player| player.gameplay_state().quests.clone())
            {
                self.apply_player_quest_core_compatibility_like_cpp(&state);
            }
            return canonical;
        }
        None
    }
}
