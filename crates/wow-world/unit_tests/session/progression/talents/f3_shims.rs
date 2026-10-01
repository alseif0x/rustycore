// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session::progression::talents) fn represented_spent_talent_points_count_like_cpp(
        &self,
    ) -> Option<u32> {
        crate::session::hub_ref(self).represented_spent_talent_points_count_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn mutate_player_talent_runtime_for_test_like_cpp<R>(
        &mut self,
        apply: impl FnOnce(&mut wow_entities::PlayerTalentRuntimeState) -> R,
    ) -> Option<R> {
        crate::session::hub_mut(self).mutate_player_talent_runtime_for_test_like_cpp(apply)
    }
    #[cfg(test)]
    pub(crate) fn represented_update_talent_data_packet_like_cpp(
        &self,
    ) -> wow_packet::packets::misc::UpdateTalentData {
        crate::session::hub_ref(self).represented_update_talent_data_packet_like_cpp()
    }
    pub(crate) fn represented_talent_reset_cost_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).represented_talent_reset_cost_like_cpp()
    }
    pub(crate) fn represented_talent_reset_time_secs_like_cpp(&self) -> Option<u64> {
        crate::session::hub_ref(self).represented_talent_reset_time_secs_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_reset_script_hooks_like_cpp(
        &self,
    ) -> &[RepresentedTalentResetScriptHookLikeCpp] {
        self.fixtures
            .progression
            .represented_talent_reset_script_hooks_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_respec_criteria_events_like_cpp(
        &self,
    ) -> &[RepresentedTalentRespecCriteriaEventLikeCpp] {
        self.fixtures
            .progression
            .represented_talent_respec_criteria_events_like_cpp()
    }
    pub(crate) fn reset_represented_glyphs_like_cpp(&mut self) {
        crate::session::hub_mut(self).reset_represented_glyphs_like_cpp()
    }
    pub(in crate::session) fn represented_talent_info_like_cpp(
        &self,
        talent_id: u32,
        rank: u8,
    ) -> Option<wow_packet::packets::misc::TalentInfoLikeCpp> {
        crate::session::hub_ref(self).represented_talent_info_like_cpp(talent_id, rank)
    }
    pub(crate) fn represented_next_reset_talents_cost_like_cpp(
        &self,
        now_secs: u64,
    ) -> Option<u32> {
        crate::session::hub_ref(self).represented_next_reset_talents_cost_like_cpp(now_secs)
    }
    pub(crate) fn player_talent_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerTalentRuntimeState> {
        crate::session::hub_ref(self).player_talent_runtime_snapshot_like_cpp()
    }
    pub(crate) fn set_represented_talent_reset_state_like_cpp(
        &mut self,
        reset_cost: u32,
        reset_time_secs: u64,
    ) -> bool {
        crate::session::hub_mut(self)
            .set_represented_talent_reset_state_like_cpp(reset_cost, reset_time_secs)
    }
}
