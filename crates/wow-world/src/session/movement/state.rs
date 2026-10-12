//! Represented player movement state at the Session boundary.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

#[cfg(test)]
pub(crate) use wow_world_core::session::MovementTransportMembershipLikeCpp;

#[path = "state/spline_progression.rs"]
mod spline_progression;

impl WorldSession {
    pub(crate) fn remove_represented_rest_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        self.remove_player_rest_flag_like_cpp(rest_flag)
    }

    pub(in crate::session) fn remove_represented_active_talent_side_effects_like_cpp(
        &mut self,
        talent_id: u32,
        rank: u8,
    ) {
        if let Some(spell_id) = self.represented_talent_spell_id_like_cpp(talent_id, rank) {
            self.remove_known_spell_like_cpp(spell_id);
            for trigger_spell in self.represented_direct_learn_spell_triggers_like_cpp(spell_id) {
                self.remove_known_spell_like_cpp(trigger_spell);
            }
        }

        if let Some((overriden_spell_id, new_spell_id)) =
            self.represented_talent_override_spell_pair_like_cpp(talent_id)
        {
            self.remove_represented_override_spell_like_cpp(overriden_spell_id, new_spell_id);
        }
    }

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

    pub(crate) fn set_player_transport_position_like_cpp(&mut self, position: Option<Position>) {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            if let (Some(state), Some(position)) = (
                self.fixtures
                    .vehicles
                    .player_transport_login_state_like_cpp
                    .as_mut(),
                position,
            ) {
                state.info.x = position.x;
                state.info.y = position.y;
                state.info.z = position.z;
                state.info.o = position.orientation;
            }
            return;
        }
        let _ = self.core.with_owned_player_mut_like_cpp(|player| {
            if let Some(position) = position {
                player.set_transport_position_like_cpp(position);
            }
        });
    }

    pub(crate) fn represented_visibility_source_position_like_cpp(&self) -> Option<Position> {
        let player_position = crate::session::hub_ref(self).player_position_like_cpp()?;
        let Some(player_guid) = self.player_guid() else {
            return Some(player_position);
        };
        let Some(seer_guid) = self.current_seer_guid_like_cpp() else {
            return Some(player_position);
        };
        if seer_guid.is_empty() || seer_guid == player_guid {
            return Some(player_position);
        }

        let Some(key) = self.core.current_canonical_player_map_key_like_cpp() else {
            return Some(player_position);
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
            return Some(player_position);
        };
        let Ok(manager) = manager.lock() else {
            return Some(player_position);
        };
        manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|managed| {
                managed.map().with_world_object_by_kinds_like_cpp(
                    seer_guid,
                    wow_entities::represented_seer_kinds_like_cpp(),
                    |object| object.position(),
                )
            })
            .or(Some(player_position))
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/movement/state/f3_shims.rs"]
mod f3_shims;
