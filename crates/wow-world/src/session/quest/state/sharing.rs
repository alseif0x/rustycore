//! sharing operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    #[cfg_attr(
        not(any(test, feature = "test-fixtures")),
        allow(unused_variables)
    )]
    pub(crate) fn record_represented_adventure_map_start_quest_like_cpp(
        &mut self,
        request: RepresentedAdventureMapStartQuestLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_adventure_map_start_quest_requests_like_cpp
            .push(request);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_adventure_map_start_quest_requests_like_cpp(
        &self,
    ) -> &[RepresentedAdventureMapStartQuestLikeCpp] {
        &self.represented_adventure_map_start_quest_requests_like_cpp
    }
    pub(crate) fn set_represented_pending_quest_sharing_like_cpp(
        &mut self,
        sender_guid: ObjectGuid,
        quest_id: u32,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            self.quest_test_fixture_like_cpp
                .represented_pending_quest_sharing_like_cpp =
                Some(RepresentedPendingQuestSharingLikeCpp {
                    sender_guid,
                    quest_id,
                });
            self.sync_player_registry_state_like_cpp();
            return;
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_pending_share_like_cpp(Some((sender_guid, quest_id)));
        });
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn clear_represented_pending_quest_sharing_like_cpp(&mut self) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            self.quest_test_fixture_like_cpp
                .represented_pending_quest_sharing_like_cpp = None;
            self.sync_player_registry_state_like_cpp();
            return;
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_pending_share_like_cpp(None);
        });
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn represented_pending_quest_sharing_like_cpp(
        &self,
    ) -> Option<RepresentedPendingQuestSharingLikeCpp> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            return self
                .quest_test_fixture_like_cpp
                .represented_pending_quest_sharing_like_cpp;
        }
        self.player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.pending_share_like_cpp())
            .map(
                |(sender_guid, quest_id)| RepresentedPendingQuestSharingLikeCpp {
                    sender_guid,
                    quest_id,
                },
            )
    }
    pub(crate) fn set_represented_df_quest_like_cpp_for_test(
        &mut self,
        quest_id: u32,
        present: bool,
    ) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            if present {
                state.set_df_quest_like_cpp(quest_id, true);
            } else {
                state.set_df_quest_like_cpp(quest_id, false);
            }
        });
        self.sync_player_registry_state_like_cpp();
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_timed_quest_removals_like_cpp(&self) -> &[u32] {
        &self
            .quest_test_fixture_like_cpp
            .represented_timed_quest_removals_like_cpp
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_push_result_responses_like_cpp(
        &self,
    ) -> &[RepresentedQuestPushResultResponseLikeCpp] {
        &self
            .quest_test_fixture_like_cpp
            .represented_quest_push_result_responses_like_cpp
    }
    pub(crate) fn record_represented_quest_push_result_response_like_cpp(
        &mut self,
        response: RepresentedQuestPushResultResponseLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.quest_test_fixture_like_cpp
            .represented_quest_push_result_responses_like_cpp
            .push(response);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = response;
    }
    pub(crate) fn record_represented_quest_push_result_sender_mismatch_like_cpp(&mut self) {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.quest_test_fixture_like_cpp
                .represented_quest_push_result_sender_mismatch_count_like_cpp = self
                .quest_test_fixture_like_cpp
                .represented_quest_push_result_sender_mismatch_count_like_cpp
                .saturating_add(1);
        }
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_confirm_accepts_like_cpp(
        &self,
    ) -> &[RepresentedQuestConfirmAcceptLikeCpp] {
        &self
            .quest_test_fixture_like_cpp
            .represented_quest_confirm_accepts_like_cpp
    }
    pub(crate) fn record_represented_quest_confirm_accept_like_cpp(
        &mut self,
        evidence: RepresentedQuestConfirmAcceptLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.quest_test_fixture_like_cpp
            .represented_quest_confirm_accepts_like_cpp
            .push(evidence);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = evidence;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_push_quest_to_party_outcomes_like_cpp(
        &self,
    ) -> &[RepresentedPushQuestToPartyOutcomeLikeCpp] {
        &self
            .quest_test_fixture_like_cpp
            .represented_push_quest_to_party_outcomes_like_cpp
    }
    pub(crate) fn record_represented_push_quest_to_party_outcome_like_cpp(
        &mut self,
        outcome: RepresentedPushQuestToPartyOutcomeLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.quest_test_fixture_like_cpp
            .represented_push_quest_to_party_outcomes_like_cpp
            .push(outcome);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = outcome;
    }
}
