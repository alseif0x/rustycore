// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_battleground_leave_requests_like_cpp(&self) -> u32 {
        self.fixtures
            .battleground
            .represented_battleground_leave_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn temporary_pet_unsummon_requests_like_cpp(&self) -> u32 {
        self.fixtures.pets.temporary_pet_unsummon_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn movement_jump_proc_requests_like_cpp(&self) -> u32 {
        self.fixtures.movement.movement_jump_proc_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn movement_visibility_refresh_requests_like_cpp(&self) -> u32 {
        self.quest_state
            .movement_visibility_refresh_requests_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn temporary_pet_resummon_requests_like_cpp(&self) -> u32 {
        self.fixtures.pets.temporary_pet_resummon_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_timed_quest_removals_like_cpp(&self) -> &[u32] {
        self.quest_state
            .fixture_represented_timed_quest_removals_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_push_result_responses_like_cpp(
        &self,
    ) -> &[RepresentedQuestPushResultResponseLikeCpp] {
        self.quest_state
            .fixture_represented_quest_push_result_responses_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_confirm_accepts_like_cpp(
        &self,
    ) -> &[RepresentedQuestConfirmAcceptLikeCpp] {
        self.quest_state
            .fixture_represented_quest_confirm_accepts_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_push_quest_to_party_outcomes_like_cpp(
        &self,
    ) -> &[RepresentedPushQuestToPartyOutcomeLikeCpp] {
        self.quest_state
            .fixture_represented_push_quest_to_party_outcomes_like_cpp()
    }
}

impl WorldSession {
    /// Ensure one seasonal event exists, without changing its quests.
    pub(crate) fn ensure_represented_seasonal_event_like_cpp(&mut self, event_id: u16) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.ensure_seasonal_event_like_cpp(event_id);
        })
        .is_some()
    }
    pub(crate) fn spell_area_for_quest_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_quest_map_bounds_like_cpp(quest_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_area_for_quest_end_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_quest_end_map_bounds_like_cpp(quest_id))
            .unwrap_or_default()
    }
    pub(crate) fn set_represented_resurrection_request_like_cpp(
        &mut self,
        request: PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_resurrection_request_like_cpp(request)
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .combat
                .represented_resurrection_request_like_cpp = Some(request);
        }
        canonical || cfg!(test) && self.core.player_handle_like_cpp.is_none()
    }
    pub(crate) fn represented_resurrection_requested_by_like_cpp(
        &self,
        resurrecter: ObjectGuid,
    ) -> bool {
        self.player_resurrection_state_snapshot_like_cpp()
            .and_then(|state| state.request)
            .is_some_and(|request| {
                request.resurrecter != ObjectGuid::EMPTY && request.resurrecter == resurrecter
            })
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
}
