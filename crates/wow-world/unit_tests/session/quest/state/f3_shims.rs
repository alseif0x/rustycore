// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session::quest::state) fn apply_player_quest_gameplay_fixture_like_cpp(
        &mut self,
        state: PlayerQuestGameplayState,
    ) {
        self.quest_state
            .apply_player_quest_gameplay_fixture_like_cpp(state)
    }
    #[cfg(test)]
    pub(in crate::session::quest::state) fn apply_player_quest_core_compatibility_like_cpp(
        &mut self,
        state: &PlayerQuestGameplayState,
    ) {
        self.quest_state
            .apply_player_quest_core_compatibility_like_cpp(state)
    }
    #[cfg(test)]
    pub(crate) fn represented_battleground_leave_requests_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        state.represented_battleground_leave_requests_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn temporary_pet_unsummon_requests_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        state.temporary_pet_unsummon_requests_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn movement_jump_proc_requests_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        state.movement_jump_proc_requests_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn movement_visibility_refresh_requests_like_cpp(&self) -> u32 {
        self.quest_state
            .movement_visibility_refresh_requests_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn temporary_pet_resummon_requests_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        state.temporary_pet_resummon_requests_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn represented_timed_quest_removals_like_cpp(&self) -> &[u32] {
        self.quest_state.represented_timed_quest_removals_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_push_result_responses_like_cpp(
        &self,
    ) -> &[RepresentedQuestPushResultResponseLikeCpp] {
        self.quest_state
            .represented_quest_push_result_responses_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_confirm_accepts_like_cpp(
        &self,
    ) -> &[RepresentedQuestConfirmAcceptLikeCpp] {
        self.quest_state
            .represented_quest_confirm_accepts_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_push_quest_to_party_outcomes_like_cpp(
        &self,
    ) -> &[RepresentedPushQuestToPartyOutcomeLikeCpp] {
        self.quest_state
            .represented_push_quest_to_party_outcomes_like_cpp()
    }
}
