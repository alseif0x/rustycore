// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn group_invite_policy_for_test_like_cpp(&self) -> GroupInvitePolicyLikeCpp {
        self.config.group_invite_policy_for_test_like_cpp()
    }
    #[cfg(test)]
    pub fn set_party_raid_warnings_like_cpp(&mut self, enabled: bool) {
        self.config.set_party_raid_warnings_like_cpp(enabled)
    }
    #[cfg(test)]
    pub fn set_allow_gm_group_like_cpp(&mut self, enabled: bool) {
        self.config.set_allow_gm_group_like_cpp(enabled)
    }
    #[cfg(test)]
    pub fn set_party_level_req_like_cpp(&mut self, level: u32) {
        self.config.set_party_level_req_like_cpp(level)
    }
    #[cfg(test)]
    pub(crate) fn represented_subgroup_like_cpp(&self) -> Option<u8> {
        let (state, hub) = crate::session::split_social_ref(self);
        state.represented_subgroup_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn represented_silence_party_talker_like_cpp(
        &self,
    ) -> &[RepresentedSilencePartyTalkerLikeCpp] {
        self.social.represented_silence_party_talker_like_cpp()
    }
}
