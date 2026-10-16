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

    pub(crate) fn remove_represented_at_login_flag_like_cpp(
        &mut self,
        flags: u16,
        persist: bool,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.remove_at_login_flags_like_cpp(flags));
        #[cfg(test)]
        let removed = canonical.or_else(|| {
            self.core.player_handle_like_cpp.is_none().then(|| {
                self.mutate_player_persistent_capability_state_like_cpp(|state| {
                    let removed = (state.at_login_flags & flags) != 0;
                    state.at_login_flags &= !flags;
                    removed
                })
                .expect("handle-less fixture capability owner")
            })
        });
        #[cfg(not(test))]
        let removed = canonical;
        let Some(removed) = removed else {
            return false;
        };
        if !removed {
            return false;
        }
        if persist {
            #[cfg(test)]
            self.lifecycle
                .record_at_login_flag_removal_for_test_like_cpp(
                    RepresentedAtLoginFlagRemovalLikeCpp {
                        flags,
                        persist,
                        db_statement_unrepresented: true,
                    },
                );
        }
        true
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/movement/state/f3_shims.rs"]
mod f3_shims;
