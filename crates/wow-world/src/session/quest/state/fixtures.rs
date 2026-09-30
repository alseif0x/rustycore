//! fixtures operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) fn player_quest_gameplay_fixture_like_cpp(&self) -> PlayerQuestGameplayState {
        let objective_counts_by_quest = self
            .quest_test_fixture_like_cpp
            .player_quests
            .values()
            .map(|status| (status.quest_id, status.objective_counts.clone()))
            .collect();
        let mut state = PlayerQuestGameplayState::default();
        state.replace_statuses_like_cpp(
            self.quest_test_fixture_like_cpp
                .player_quests
                .iter()
                .map(|(&quest_id, status)| (quest_id, status.clone()))
                .collect(),
            self.quest_test_fixture_like_cpp
                .player_quest_status_authority_complete_like_cpp,
        );
        state.replace_rewarded_quest_ids_like_cpp(
            self.quest_test_fixture_like_cpp
                .rewarded_quests
                .iter()
                .copied()
                .collect(),
        );
        state.replace_daily_quest_ids_like_cpp(
            self.quest_test_fixture_like_cpp
                .daily_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
        );
        state.replace_weekly_quest_ids_like_cpp(
            self.quest_test_fixture_like_cpp
                .weekly_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
        );
        state.replace_monthly_quest_ids_like_cpp(
            self.quest_test_fixture_like_cpp
                .monthly_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
        );
        state.replace_seasonal_quests_like_cpp(
            self.quest_test_fixture_like_cpp
                .seasonal_quests_like_cpp
                .clone(),
            self.quest_test_fixture_like_cpp
                .seasonal_quest_changed_like_cpp,
        );
        state.replace_df_quest_ids_like_cpp(
            self.quest_test_fixture_like_cpp
                .df_quests_like_cpp
                .iter()
                .copied()
                .collect(),
        );
        state.set_last_daily_quest_time_secs_like_cpp(
            self.quest_test_fixture_like_cpp
                .last_daily_quest_time_like_cpp,
        );
        state.replace_rewarded_quest_rows_like_cpp(
            self.quest_test_fixture_like_cpp
                .represented_rewarded_quest_rows_like_cpp
                .clone(),
        );
        state.replace_objective_counts_by_quest_like_cpp(objective_counts_by_quest);
        state
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) fn apply_player_quest_gameplay_fixture_like_cpp(&mut self, state: PlayerQuestGameplayState) {
        self.apply_player_quest_core_compatibility_like_cpp(&state);
        self.quest_test_fixture_like_cpp
            .daily_quests_completed_like_cpp =
            state.daily_quest_ids_like_cpp().iter().copied().collect();
        self.quest_test_fixture_like_cpp
            .weekly_quests_completed_like_cpp =
            state.weekly_quest_ids_like_cpp().iter().copied().collect();
        self.quest_test_fixture_like_cpp
            .monthly_quests_completed_like_cpp =
            state.monthly_quest_ids_like_cpp().iter().copied().collect();
        self.quest_test_fixture_like_cpp.seasonal_quests_like_cpp =
            state.seasonal_quests_snapshot_like_cpp();
        self.quest_test_fixture_like_cpp.df_quests_like_cpp =
            state.df_quest_ids_like_cpp().iter().copied().collect();
        self.quest_test_fixture_like_cpp
            .last_daily_quest_time_like_cpp = state.last_daily_quest_time_secs_like_cpp();
        self.quest_test_fixture_like_cpp
            .seasonal_quest_changed_like_cpp = state.seasonal_quest_changed_like_cpp();
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) fn apply_player_quest_core_compatibility_like_cpp(&mut self, state: &PlayerQuestGameplayState) {
        self.quest_test_fixture_like_cpp.player_quests = state
            .statuses_like_cpp()
            .iter()
            .map(|(&quest_id, status)| (quest_id, status.clone()))
            .collect();
        self.quest_test_fixture_like_cpp.rewarded_quests = state
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect();
        self.quest_test_fixture_like_cpp
            .player_quest_status_authority_complete_like_cpp =
            state.status_authority_complete_like_cpp();
        self.quest_test_fixture_like_cpp
            .represented_rewarded_quest_rows_like_cpp =
            state.rewarded_quest_rows_like_cpp().clone();
    }
}
