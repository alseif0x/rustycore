// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_duel_accepts_like_cpp(&self) -> &[RepresentedDuelAcceptedLikeCpp] {
        self.social.represented_duel_accepts_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_duel_cancels_like_cpp(&self) -> &[RepresentedDuelCancelledLikeCpp] {
        self.social.represented_duel_cancels_like_cpp()
    }
}

impl WorldSession {
    pub(crate) fn resolved_represented_duel_arbiter_guid_like_cpp(
        &self,
    ) -> Option<Option<ObjectGuid>> {
        let (state, hub) = crate::session::split_social_ref(self);
        wow_world_application::resolved_represented_duel_arbiter_guid_like_cpp(hub, state)
    }
}
