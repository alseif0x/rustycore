// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_calendar_community_invites_like_cpp(
        &self,
    ) -> &[RepresentedCalendarCommunityInviteLikeCpp] {
        self.social
            .represented_calendar_community_invites_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_calendar_add_events_like_cpp(
        &self,
    ) -> &[RepresentedCalendarAddEventLikeCpp] {
        self.social.represented_calendar_add_events_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_arena_team_id_invited_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_social_ref(self);
        state.represented_arena_team_id_invited_like_cpp(hub)
    }
}

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn set_represented_arena_team_id_invited_like_cpp(
        &mut self,
        arena_team_id: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.set_represented_arena_team_id_invited_like_cpp(&mut hub, arena_team_id)
    }
}
