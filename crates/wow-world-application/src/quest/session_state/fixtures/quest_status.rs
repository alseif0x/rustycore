// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private quest status and compatibility-state fixture operations for quest state.

use super::super::{SessionQuestState, contracts};

impl SessionQuestState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_quest_objective_progress_event_count_like_cpp(&self) -> usize {
        self.represented_quest_objective_progress_events_like_cpp
            .len()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_quest_objective_progress_events_are_empty_like_cpp(&self) -> bool {
        self.represented_quest_objective_progress_events_like_cpp
            .is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn movement_visibility_refresh_requests_like_cpp(&self) -> u32 {
        self.movement_visibility_refresh_requests_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_quest_status_like_cpp(
        &self,
        quest_id: u32,
    ) -> Option<&wow_entities::PlayerQuestStatusRecord> {
        self.fixtures.player_quests.get(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_contains_player_quest_status_like_cpp(&self, quest_id: u32) -> bool {
        self.fixtures.player_quests.contains_key(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_quest_status_mut_like_cpp(
        &mut self,
        quest_id: u32,
    ) -> Option<&mut wow_entities::PlayerQuestStatusRecord> {
        self.fixtures.player_quests.get_mut(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_quest_statuses_snapshot_like_cpp(
        &self,
    ) -> Vec<(u32, wow_entities::PlayerQuestStatusRecord)> {
        self.fixtures
            .player_quests
            .iter()
            .map(|(&quest_id, status)| (quest_id, status.clone()))
            .collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_quest_status_values_snapshot_like_cpp(
        &self,
    ) -> Vec<wow_entities::PlayerQuestStatusRecord> {
        self.fixtures.player_quests.values().cloned().collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_quest_status_values_like_cpp(
        &self,
    ) -> impl Iterator<Item = &wow_entities::PlayerQuestStatusRecord> {
        self.fixtures.player_quests.values()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_quest_status_count_like_cpp(&self) -> usize {
        self.fixtures.player_quests.len()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_quest_statuses_are_empty_like_cpp(&self) -> bool {
        self.fixtures.player_quests.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_insert_player_quest_status_like_cpp(
        &mut self,
        quest_id: u32,
        status: wow_entities::PlayerQuestStatusRecord,
    ) -> Option<wow_entities::PlayerQuestStatusRecord> {
        self.fixtures.player_quests.insert(quest_id, status)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_remove_player_quest_status_like_cpp(
        &mut self,
        quest_id: u32,
    ) -> Option<wow_entities::PlayerQuestStatusRecord> {
        self.fixtures.player_quests.remove(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_clear_player_quest_statuses_like_cpp(&mut self) {
        self.fixtures.player_quests.clear();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_quest_status_authority_complete_like_cpp(&self) -> bool {
        self.fixtures
            .player_quest_status_authority_complete_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_player_quest_status_authority_complete_like_cpp(&mut self, complete: bool) {
        self.fixtures
            .player_quest_status_authority_complete_like_cpp = complete;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_has_rewarded_quest_like_cpp(&self, quest_id: u32) -> bool {
        self.fixtures.rewarded_quests.contains(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_rewarded_quest_ids_snapshot_like_cpp(&self) -> Vec<u32> {
        self.fixtures.rewarded_quests.iter().copied().collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_rewarded_quest_like_cpp(&mut self, quest_id: u32, rewarded: bool) -> bool {
        if rewarded {
            self.fixtures.rewarded_quests.insert(quest_id)
        } else {
            self.fixtures.rewarded_quests.remove(&quest_id)
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_clear_rewarded_quests_like_cpp(&mut self) {
        self.fixtures.rewarded_quests.clear();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_daily_quest_ids_snapshot_like_cpp(&self) -> Vec<u32> {
        self.fixtures
            .daily_quests_completed_like_cpp
            .iter()
            .copied()
            .collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_has_daily_quest_like_cpp(&self, quest_id: u32) -> bool {
        self.fixtures
            .daily_quests_completed_like_cpp
            .contains(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_daily_quest_like_cpp(&mut self, quest_id: u32) -> bool {
        self.fixtures
            .daily_quests_completed_like_cpp
            .insert(quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_weekly_quest_ids_snapshot_like_cpp(&self) -> Vec<u32> {
        self.fixtures
            .weekly_quests_completed_like_cpp
            .iter()
            .copied()
            .collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_has_weekly_quest_like_cpp(&self, quest_id: u32) -> bool {
        self.fixtures
            .weekly_quests_completed_like_cpp
            .contains(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_weekly_quest_like_cpp(&mut self, quest_id: u32) -> bool {
        self.fixtures
            .weekly_quests_completed_like_cpp
            .insert(quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_monthly_quest_ids_snapshot_like_cpp(&self) -> Vec<u32> {
        self.fixtures
            .monthly_quests_completed_like_cpp
            .iter()
            .copied()
            .collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_has_monthly_quest_like_cpp(&self, quest_id: u32) -> bool {
        self.fixtures
            .monthly_quests_completed_like_cpp
            .contains(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_monthly_quest_like_cpp(&mut self, quest_id: u32) -> bool {
        self.fixtures
            .monthly_quests_completed_like_cpp
            .insert(quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_df_quest_ids_snapshot_like_cpp(&self) -> Vec<u32> {
        self.fixtures.df_quests_like_cpp.iter().copied().collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_has_df_quest_like_cpp(&self, quest_id: u32) -> bool {
        self.fixtures.df_quests_like_cpp.contains(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_df_quest_like_cpp(&mut self, quest_id: u32, present: bool) -> bool {
        if present {
            self.fixtures.df_quests_like_cpp.insert(quest_id)
        } else {
            self.fixtures.df_quests_like_cpp.remove(&quest_id)
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_last_daily_quest_time_like_cpp(&self) -> i64 {
        self.fixtures.last_daily_quest_time_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_last_daily_quest_time_like_cpp(&mut self, value: i64) {
        self.fixtures.last_daily_quest_time_like_cpp = value;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_seasonal_quests_snapshot_like_cpp(
        &self,
    ) -> std::collections::BTreeMap<u16, std::collections::BTreeMap<u32, u64>> {
        self.fixtures.seasonal_quests_like_cpp.clone()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_seasonal_quests_are_empty_like_cpp(&self) -> bool {
        self.fixtures.seasonal_quests_like_cpp.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_seasonal_quest_time_like_cpp(
        &self,
        event_id: u16,
        quest_id: u32,
    ) -> Option<u64> {
        self.fixtures
            .seasonal_quests_like_cpp
            .get(&event_id)
            .and_then(|quests| quests.get(&quest_id).copied())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_seasonal_quest_times_snapshot_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<std::collections::BTreeMap<u32, u64>> {
        self.fixtures
            .seasonal_quests_like_cpp
            .get(&event_id)
            .cloned()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_seasonal_quest_changed_like_cpp(&self) -> bool {
        self.fixtures.seasonal_quest_changed_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_seasonal_quest_changed_like_cpp(&mut self, changed: bool) {
        self.fixtures.seasonal_quest_changed_like_cpp = changed;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_rewarded_quest_rows_snapshot_like_cpp(&self) -> Vec<u32> {
        self.fixtures
            .represented_rewarded_quest_rows_like_cpp
            .iter()
            .copied()
            .collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_has_represented_rewarded_quest_row_like_cpp(&self, quest_id: u32) -> bool {
        self.fixtures
            .represented_rewarded_quest_rows_like_cpp
            .contains(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_represented_rewarded_quest_row_like_cpp(
        &mut self,
        quest_id: u32,
        present: bool,
    ) {
        if present {
            self.fixtures
                .represented_rewarded_quest_rows_like_cpp
                .insert(quest_id);
        } else {
            self.fixtures
                .represented_rewarded_quest_rows_like_cpp
                .remove(&quest_id);
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_clear_represented_rewarded_quest_rows_like_cpp(&mut self) {
        self.fixtures
            .represented_rewarded_quest_rows_like_cpp
            .clear();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_completed_bits_like_cpp(
        &self,
    ) -> impl Iterator<Item = u32> + '_ {
        self.fixtures
            .represented_quest_completed_bits_like_cpp
            .iter()
            .copied()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_completed_bits_snapshot_like_cpp(
        &self,
    ) -> std::collections::BTreeSet<u32> {
        self.fixtures
            .represented_quest_completed_bits_like_cpp
            .iter()
            .copied()
            .collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_has_represented_quest_completed_bit_like_cpp(&self, quest_id: u32) -> bool {
        self.fixtures
            .represented_quest_completed_bits_like_cpp
            .contains(&quest_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_completed_bits_are_empty_like_cpp(&self) -> bool {
        self.fixtures
            .represented_quest_completed_bits_like_cpp
            .is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_represented_quest_completed_bit_like_cpp(
        &mut self,
        quest_id: u32,
        present: bool,
    ) {
        if present {
            self.fixtures
                .represented_quest_completed_bits_like_cpp
                .insert(quest_id);
        } else {
            self.fixtures
                .represented_quest_completed_bits_like_cpp
                .remove(&quest_id);
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_known_titles_snapshot_like_cpp(&self) -> Vec<u32> {
        self.fixtures
            .represented_known_titles_like_cpp
            .iter()
            .copied()
            .collect()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_has_represented_known_title_like_cpp(&self, title_id: u32) -> bool {
        self.fixtures
            .represented_known_titles_like_cpp
            .contains(&title_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_represented_known_title_like_cpp(&mut self, title_id: u32, known: bool) {
        if known {
            self.fixtures
                .represented_known_titles_like_cpp
                .insert(title_id);
        } else {
            self.fixtures
                .represented_known_titles_like_cpp
                .remove(&title_id);
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_replace_represented_known_titles_like_cpp(
        &mut self,
        title_ids: impl IntoIterator<Item = u32>,
    ) {
        self.fixtures.represented_known_titles_like_cpp = title_ids.into_iter().collect();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_chosen_title_like_cpp(&self) -> i32 {
        self.fixtures.represented_chosen_title_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_represented_chosen_title_like_cpp(&mut self, title_id: i32) {
        self.fixtures.represented_chosen_title_like_cpp = title_id;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_auto_accept_acknowledged_quests_like_cpp(&self) -> &[u32] {
        &self
            .fixtures
            .represented_auto_accept_acknowledged_quests_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_auto_accept_acknowledged_quest_like_cpp(&mut self, quest_id: u32) {
        self.fixtures
            .represented_auto_accept_acknowledged_quests_like_cpp
            .push(quest_id);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_timed_quest_removal_like_cpp(&mut self, quest_id: u32) {
        self.fixtures
            .represented_timed_quest_removals_like_cpp
            .push(quest_id);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_quest_gameplay_fixture_like_cpp(&self) -> wow_entities::PlayerQuestGameplayState {
        let objective_counts_by_quest = self
            .fixtures
            .player_quests
            .values()
            .map(|status| (status.quest_id, status.objective_counts.clone()))
            .collect();
        let mut state = wow_entities::PlayerQuestGameplayState::default();
        state.replace_statuses_like_cpp(
            self.fixtures
                .player_quests
                .iter()
                .map(|(&quest_id, status)| (quest_id, status.clone()))
                .collect(),
            self.fixtures
                .player_quest_status_authority_complete_like_cpp,
        );
        state.replace_rewarded_quest_ids_like_cpp(
            self.fixtures.rewarded_quests.iter().copied().collect(),
        );
        state.replace_daily_quest_ids_like_cpp(
            self.fixtures
                .daily_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
        );
        state.replace_weekly_quest_ids_like_cpp(
            self.fixtures
                .weekly_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
        );
        state.replace_monthly_quest_ids_like_cpp(
            self.fixtures
                .monthly_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
        );
        state.replace_seasonal_quests_like_cpp(
            self.fixtures.seasonal_quests_like_cpp.clone(),
            self.fixtures.seasonal_quest_changed_like_cpp,
        );
        state.replace_df_quest_ids_like_cpp(
            self.fixtures.df_quests_like_cpp.iter().copied().collect(),
        );
        state.set_last_daily_quest_time_secs_like_cpp(self.fixtures.last_daily_quest_time_like_cpp);
        state.replace_rewarded_quest_rows_like_cpp(
            self.fixtures
                .represented_rewarded_quest_rows_like_cpp
                .clone(),
        );
        state.replace_objective_counts_by_quest_like_cpp(objective_counts_by_quest);
        state
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn apply_player_quest_gameplay_fixture_like_cpp(
        &mut self,
        state: wow_entities::PlayerQuestGameplayState,
    ) {
        self.apply_player_quest_core_compatibility_like_cpp(&state);
        self.fixtures.daily_quests_completed_like_cpp =
            state.daily_quest_ids_like_cpp().iter().copied().collect();
        self.fixtures.weekly_quests_completed_like_cpp =
            state.weekly_quest_ids_like_cpp().iter().copied().collect();
        self.fixtures.monthly_quests_completed_like_cpp =
            state.monthly_quest_ids_like_cpp().iter().copied().collect();
        self.fixtures.seasonal_quests_like_cpp = state.seasonal_quests_snapshot_like_cpp();
        self.fixtures.df_quests_like_cpp = state.df_quest_ids_like_cpp().iter().copied().collect();
        self.fixtures.last_daily_quest_time_like_cpp = state.last_daily_quest_time_secs_like_cpp();
        self.fixtures.seasonal_quest_changed_like_cpp = state.seasonal_quest_changed_like_cpp();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn apply_player_quest_core_compatibility_like_cpp(
        &mut self,
        state: &wow_entities::PlayerQuestGameplayState,
    ) {
        self.fixtures.player_quests = state
            .statuses_like_cpp()
            .iter()
            .map(|(&quest_id, status)| (quest_id, status.clone()))
            .collect();
        self.fixtures.rewarded_quests = state
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect();
        self.fixtures
            .player_quest_status_authority_complete_like_cpp =
            state.status_authority_complete_like_cpp();
        self.fixtures.represented_rewarded_quest_rows_like_cpp =
            state.rewarded_quest_rows_like_cpp().clone();
    }
}
