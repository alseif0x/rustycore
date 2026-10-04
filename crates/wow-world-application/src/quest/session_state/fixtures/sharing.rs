// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private quest-sharing evidence fixture operations for quest state.

use super::super::{contracts, SessionQuestState};

impl SessionQuestState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_pending_quest_sharing_like_cpp(
        &self,
    ) -> Option<contracts::RepresentedPendingQuestSharingLikeCpp> {
        self.fixtures.represented_pending_quest_sharing_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_represented_pending_quest_sharing_like_cpp(
        &mut self,
        pending: Option<contracts::RepresentedPendingQuestSharingLikeCpp>,
    ) {
        self.fixtures.represented_pending_quest_sharing_like_cpp = pending;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_push_result_responses_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestPushResultResponseLikeCpp] {
        &self
            .fixtures
            .represented_quest_push_result_responses_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_push_result_response_like_cpp(
        &mut self,
        response: contracts::RepresentedQuestPushResultResponseLikeCpp,
    ) {
        self.fixtures
            .represented_quest_push_result_responses_like_cpp
            .push(response);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_quest_push_result_sender_mismatch_count_like_cpp(&self) -> u32 {
        self.fixtures
            .represented_quest_push_result_sender_mismatch_count_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_push_result_sender_mismatch_like_cpp(&mut self) {
        self.fixtures
            .represented_quest_push_result_sender_mismatch_count_like_cpp = self
            .fixtures
            .represented_quest_push_result_sender_mismatch_count_like_cpp
            .saturating_add(1);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_confirm_accepts_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestConfirmAcceptLikeCpp] {
        &self.fixtures.represented_quest_confirm_accepts_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_confirm_accept_like_cpp(
        &mut self,
        evidence: contracts::RepresentedQuestConfirmAcceptLikeCpp,
    ) {
        self.fixtures
            .represented_quest_confirm_accepts_like_cpp
            .push(evidence);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_push_quest_to_party_outcomes_like_cpp(
        &self,
    ) -> &[contracts::RepresentedPushQuestToPartyOutcomeLikeCpp] {
        &self
            .fixtures
            .represented_push_quest_to_party_outcomes_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_push_quest_to_party_outcome_like_cpp(
        &mut self,
        outcome: contracts::RepresentedPushQuestToPartyOutcomeLikeCpp,
    ) {
        self.fixtures
            .represented_push_quest_to_party_outcomes_like_cpp
            .push(outcome);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_quest_push_result_responses_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestPushResultResponseLikeCpp] {
        self.fixture_represented_quest_push_result_responses_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_quest_confirm_accepts_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestConfirmAcceptLikeCpp] {
        self.fixture_represented_quest_confirm_accepts_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_push_quest_to_party_outcomes_like_cpp(
        &self,
    ) -> &[contracts::RepresentedPushQuestToPartyOutcomeLikeCpp] {
        self.fixture_represented_push_quest_to_party_outcomes_like_cpp()
    }
}
