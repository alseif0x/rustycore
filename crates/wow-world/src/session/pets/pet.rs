//! Represented pet state at the Session boundary.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[allow(dead_code)]
    pub(crate) fn set_represented_pet_mode_state_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
        react_state: u8,
        command_state: u8,
    ) {
        wow_world_lifecycle::set_represented_pet_mode_state_with_spell_like_cpp(
            self,
            pet_guid,
            react_state,
            command_state,
            0,
        );
    }
    #[cfg(test)]
    pub(crate) fn represented_sign_petitions_like_cpp(&self) -> &[RepresentedSignPetitionLikeCpp] {
        self.social.represented_sign_petitions_for_test_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_decline_petitions_like_cpp(
        &self,
    ) -> &[RepresentedDeclinePetitionLikeCpp] {
        self.social
            .represented_decline_petitions_for_test_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_query_petitions_like_cpp(
        &self,
    ) -> &[RepresentedQueryPetitionLikeCpp] {
        self.social.represented_query_petitions_for_test_like_cpp()
    }
    #[cfg(test)]
    pub(in crate::session) fn record_represented_tapper_pet_killed_unit_hooks_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) {
        let (Some(player_guid), Some(pet_guid)) = (
            self.player_guid(),
            crate::session::hub_ref(self)
                .player_pet_guid_state_like_cpp()
                .flatten(),
        ) else {
            return;
        };
        let tapper_has_current_player = self
            .core
            .mutate_world_creature(creature_guid, |creature| {
                creature.tap_list().contains(&player_guid)
            })
            .unwrap_or(true);
        if !tapper_has_current_player {
            return;
        }
        self.world_entities
            .record_represented_creature_kill_event_like_cpp(
                RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi {
                    tapper_guid: player_guid,
                    pet_guid,
                    victim_guid: creature_guid,
                },
            );
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/pet/f3_shims.rs"]
mod f3_shims;
