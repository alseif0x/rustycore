use crate::SessionSocialLimits;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::{RepresentedCalendarAddEventLikeCpp, RepresentedCalendarCommunityInviteLikeCpp};
use wow_world_core::session::HubMut;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::HubRef;

impl SessionSocialLimits {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_calendar_community_invites_like_cpp(
        &self,
    ) -> &[RepresentedCalendarCommunityInviteLikeCpp] {
        &self
            .calendar_test_fixture_like_cpp
            .represented_calendar_community_invites_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_calendar_add_events_like_cpp(
        &self,
    ) -> &[RepresentedCalendarAddEventLikeCpp] {
        &self
            .calendar_test_fixture_like_cpp
            .represented_calendar_add_events_like_cpp
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub fn set_represented_arena_team_id_invited_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        arena_team_id: u32,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_arena_team_id_invited_like_cpp(arena_team_id)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && hub.core.player_handle_like_cpp.is_none() {
            return hub
                .mutate_player_battleground_state_like_cpp(|state| {
                    state.set_arena_team_id_invited_like_cpp(arena_team_id);
                })
                .is_some();
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_arena_team_id_invited_like_cpp(&self, hub: HubRef<'_>) -> u32 {
        hub.player_battleground_state_snapshot_like_cpp()
            .expect("test Player battleground owner must resolve")
            .arena_team_id_invited_like_cpp()
    }
}
