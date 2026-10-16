//! Represented player movement state at the Session boundary.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

#[path = "state/spline_progression.rs"]
mod spline_progression;

impl WorldSession {
    pub(crate) fn remove_represented_feign_death_if_needed_like_cpp(&mut self) -> bool {
        self.player_aura_application_cx_like_cpp()
            .remove_represented_feign_death_if_needed_like_cpp()
    }

    #[cfg(test)]
    pub(crate) fn represented_calendar_remove_events_like_cpp(
        &self,
    ) -> &[RepresentedCalendarRemoveEventLikeCpp] {
        self.social
            .represented_calendar_remove_events_for_test_like_cpp()
    }

    pub(crate) fn set_player_moved_unit_guid_like_cpp(&mut self, guid: ObjectGuid) {
        crate::session::hub_mut(self).set_player_moved_unit_guid_like_cpp(guid)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/movement/state/f3_shims.rs"]
mod f3_shims;
