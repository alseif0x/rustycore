// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private represented quest state kept by the World session.

use std::collections::VecDeque;

use wow_data::quest::QuestTemplate;
use wow_data::quest_xp::QuestXpStore;

pub(super) mod contracts;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;

pub struct SessionQuestState {
    min_quest_scaled_xp_ratio_like_cpp: u32,
    quest_high_level_hide_diff_like_cpp: u32,
    quest_low_level_hide_diff_like_cpp: u32,
    represented_quest_complete_status_updates_like_cpp:
        Vec<contracts::RepresentedQuestCompleteStatusUpdateLikeCpp>,
    represented_quest_objective_progress_draining_like_cpp: bool,
    represented_quest_objective_progress_events_like_cpp:
        VecDeque<contracts::RepresentedQuestObjectiveProgressEventLikeCpp>,
    movement_visibility_refresh_requests_like_cpp: u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: fixtures::QuestTestFixtureLikeCpp,
}

impl SessionQuestState {
    pub fn new() -> Self {
        Self {
            min_quest_scaled_xp_ratio_like_cpp: 0,
            quest_high_level_hide_diff_like_cpp: 7,
            quest_low_level_hide_diff_like_cpp: 4,
            represented_quest_complete_status_updates_like_cpp: Vec::new(),
            represented_quest_objective_progress_draining_like_cpp: false,
            represented_quest_objective_progress_events_like_cpp: VecDeque::new(),
            movement_visibility_refresh_requests_like_cpp: 0,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: fixtures::QuestTestFixtureLikeCpp::default(),
        }
    }

    pub fn min_quest_scaled_xp_ratio_like_cpp(&self) -> u32 {
        self.min_quest_scaled_xp_ratio_like_cpp
    }

    pub fn set_min_quest_scaled_xp_ratio_like_cpp(&mut self, ratio: u32) {
        self.min_quest_scaled_xp_ratio_like_cpp = if ratio > 100 { 0 } else { ratio };
    }

    pub fn quest_high_level_hide_diff_like_cpp(&self) -> u32 {
        self.quest_high_level_hide_diff_like_cpp
    }

    pub fn set_quest_high_level_hide_diff_like_cpp(&mut self, value: u32) {
        self.quest_high_level_hide_diff_like_cpp = value;
    }

    pub fn quest_low_level_hide_diff_like_cpp(&self) -> u32 {
        self.quest_low_level_hide_diff_like_cpp
    }

    pub fn set_quest_low_level_hide_diff_like_cpp(&mut self, value: u32) {
        self.quest_low_level_hide_diff_like_cpp = value;
    }

    pub fn player_quest_level_like_cpp(&self, player_level: u8, quest: &QuestTemplate) -> i32 {
        if quest.quest_level > 0 {
            quest.quest_level
        } else {
            i32::from(player_level).min(quest.quest_max_scaling_level)
        }
    }

    pub fn calculate_quest_xp_like_cpp(
        &self,
        xp_store: Option<&QuestXpStore>,
        player_level: u8,
        difficulty: u32,
        quest_level: i32,
        xp_multiplier: f32,
    ) -> u32 {
        if let Some(store) = xp_store {
            store.calculate_xp(
                quest_level,
                player_level,
                difficulty,
                xp_multiplier,
                self.min_quest_scaled_xp_ratio_like_cpp,
            )
        } else {
            const XP_TABLE: [u32; 10] = [0, 50, 100, 200, 400, 650, 1000, 1500, 2500, 4000];
            XP_TABLE[difficulty.min(9) as usize]
        }
    }

    pub fn record_represented_quest_complete_status_update_like_cpp(
        &mut self,
        evidence: contracts::RepresentedQuestCompleteStatusUpdateLikeCpp,
    ) {
        self.represented_quest_complete_status_updates_like_cpp
            .push(evidence);
    }

    pub fn represented_quest_complete_status_updates_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestCompleteStatusUpdateLikeCpp] {
        &self.represented_quest_complete_status_updates_like_cpp
    }

    pub fn represented_quest_complete_status_updates_from_like_cpp(
        &self,
        index: usize,
    ) -> &[contracts::RepresentedQuestCompleteStatusUpdateLikeCpp] {
        &self.represented_quest_complete_status_updates_like_cpp[index..]
    }

    pub fn mark_latest_tracking_event_auto_reward_like_cpp(&mut self, quest_id: u32) -> bool {
        let Some(evidence) = self
            .represented_quest_complete_status_updates_like_cpp
            .iter_mut()
            .rev()
            .find(|evidence| evidence.quest_id == quest_id)
        else {
            return false;
        };
        evidence.tracking_event_auto_reward_unrepresented = false;
        true
    }

    pub fn enqueue_represented_quest_objective_progress_like_cpp(
        &mut self,
        event: contracts::RepresentedQuestObjectiveProgressEventLikeCpp,
    ) {
        self.represented_quest_objective_progress_events_like_cpp
            .push_back(event);
    }

    pub fn begin_represented_quest_objective_progress_drain_like_cpp(&mut self) -> bool {
        if self.represented_quest_objective_progress_draining_like_cpp {
            return false;
        }
        self.represented_quest_objective_progress_draining_like_cpp = true;
        true
    }

    pub fn pop_represented_quest_objective_progress_like_cpp(
        &mut self,
    ) -> Option<contracts::RepresentedQuestObjectiveProgressEventLikeCpp> {
        self.represented_quest_objective_progress_events_like_cpp
            .pop_front()
    }

    pub fn finish_represented_quest_objective_progress_drain_like_cpp(&mut self) {
        self.represented_quest_objective_progress_draining_like_cpp = false;
    }

    pub fn consume_movement_visibility_refresh_request_like_cpp(&mut self) -> bool {
        if self.movement_visibility_refresh_requests_like_cpp == 0 {
            return false;
        }
        self.movement_visibility_refresh_requests_like_cpp = self
            .movement_visibility_refresh_requests_like_cpp
            .saturating_sub(1);
        true
    }

    pub fn record_represented_quest_push_result_response_like_cpp(
        &mut self,
        record_test_evidence: bool,
        response: contracts::RepresentedQuestPushResultResponseLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if record_test_evidence {
            self.fixture_record_quest_push_result_response_like_cpp(response);
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (record_test_evidence, response);
    }

    pub fn record_represented_quest_push_result_sender_mismatch_like_cpp(
        &mut self,
        record_test_evidence: bool,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if record_test_evidence {
            self.fixture_record_quest_push_result_sender_mismatch_like_cpp();
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = record_test_evidence;
    }

    pub fn record_represented_quest_confirm_accept_like_cpp(
        &mut self,
        record_test_evidence: bool,
        evidence: contracts::RepresentedQuestConfirmAcceptLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if record_test_evidence {
            self.fixture_record_quest_confirm_accept_like_cpp(evidence);
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (record_test_evidence, evidence);
    }

    pub fn record_represented_push_quest_to_party_outcome_like_cpp(
        &mut self,
        record_test_evidence: bool,
        outcome: contracts::RepresentedPushQuestToPartyOutcomeLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if record_test_evidence {
            self.fixture_record_push_quest_to_party_outcome_like_cpp(outcome);
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (record_test_evidence, outcome);
    }
}

/// Canonical `Player::m_QuestStatus`/`m_RewardedQuests` snapshot with the
/// represented fixture fallback the World session applied at its read point.
///
/// This is the bounded seam of #1263 F5: the owner is still the canonical
/// Player behind the Core hub, and the fixture fallback stays a
/// `test-fixtures` concern inside the crate that owns the state.
pub fn player_quest_gameplay_snapshot_like_cpp(
    hub: wow_world_core::session::HubRef<'_>,
    quest_state: &SessionQuestState,
) -> Option<wow_entities::PlayerQuestGameplayState> {
    let hydration_access = hub.core.player_registry_hydration_access_like_cpp();
    #[cfg(any(test, feature = "test-fixtures"))]
    if hydration_access.owner_handle_absent_like_cpp() {
        return Some(quest_state.player_quest_gameplay_fixture_like_cpp());
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = quest_state;
    hydration_access.owned_player_quest_gameplay_snapshot_like_cpp()
}

/// Canonical Player quest mutation with the same represented fixture fallback.
pub fn mutate_player_quest_gameplay_like_cpp<R>(
    hub: wow_world_core::session::HubMut<'_>,
    quest_state: &mut SessionQuestState,
    mutate: impl FnOnce(&mut wow_entities::PlayerQuestGameplayState) -> R,
) -> Option<R> {
    let mut mutate = Some(mutate);
    #[cfg(any(test, feature = "test-fixtures"))]
    if hub.shared().core.player_handle_like_cpp.is_none() {
        let mut fixture = quest_state.player_quest_gameplay_fixture_like_cpp();
        let result = mutate.take().expect("test quest mutation executes once")(&mut fixture);
        quest_state.apply_player_quest_gameplay_fixture_like_cpp(fixture);
        return Some(result);
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = &mut *quest_state;
    let canonical = hub.core.mutate_canonical_player_like_cpp(|player| {
        mutate.take().expect("Player quest mutation executes once")(
            &mut player.gameplay_state_mut().quests,
        )
    });
    if canonical.is_some() {
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(state) = hub
            .shared()
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().quests.clone())
        {
            quest_state.apply_player_quest_core_compatibility_like_cpp(&state);
        }
        return canonical;
    }
    None
}
