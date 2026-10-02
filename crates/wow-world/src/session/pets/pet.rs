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
        self.set_represented_pet_mode_state_with_spell_like_cpp(
            pet_guid,
            react_state,
            command_state,
            0,
        );
    }
    pub(crate) fn apply_represented_login_pet_talent_reset_like_cpp(&mut self) -> bool {
        const AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP: u16 = 0x010;

        if !self
            .resolved_represented_at_login_flags_like_cpp()
            .is_some_and(|flags| (flags & AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP) != 0)
        {
            return false;
        }

        crate::session::hub_mut(self)
            .invalidate_represented_character_pet_empty_authority_like_cpp();

        if !crate::session::hub_mut(self).update_player_pet_lifecycle_state_like_cpp(|state| {
            for pet in state.stable.active_pets.iter_mut().flatten() {
                pet.specialization_id = 0;
            }
            for pet in state.stable.stabled_pets.iter_mut().flatten() {
                pet.specialization_id = 0;
            }
            for pet in &mut state.stable.unslotted_pets {
                pet.specialization_id = 0;
            }
        }) {
            return false;
        }
        self.lifecycle
            .pet_load_query_holder_rows_like_cpp
            .spells
            .clear();
        true
    }
    #[cfg(test)]
    pub(crate) fn represented_sign_petitions_like_cpp(&self) -> &[RepresentedSignPetitionLikeCpp] {
        &self.social.represented_sign_petitions_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_decline_petitions_like_cpp(
        &self,
    ) -> &[RepresentedDeclinePetitionLikeCpp] {
        &self.social.represented_decline_petitions_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_query_petitions_like_cpp(
        &self,
    ) -> &[RepresentedQueryPetitionLikeCpp] {
        &self.social.represented_query_petitions_like_cpp
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
                creature.creature.tap_list().contains(&player_guid)
            })
            .unwrap_or(true);
        if !tapper_has_current_player {
            return;
        }
        self.world_entities
            .represented_creature_kill_events_like_cpp
            .push(RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi {
                tapper_guid: player_guid,
                pet_guid,
                victim_guid: creature_guid,
            });
    }
}

impl crate::session::PetsCx<'_> {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_sign_petition_like_cpp(
        &mut self,
        petition_guid: ObjectGuid,
        choice: u8,
    ) {
        #[cfg(test)]
        self.social
            .represented_sign_petitions_like_cpp
            .push(RepresentedSignPetitionLikeCpp {
                petition_guid,
                choice,
            });
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_decline_petition_like_cpp(
        &mut self,
        petition_guid: ObjectGuid,
    ) {
        #[cfg(test)]
        self.social
            .represented_decline_petitions_like_cpp
            .push(RepresentedDeclinePetitionLikeCpp { petition_guid });
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_query_petition_like_cpp(
        &mut self,
        petition_id: u32,
        item_guid: ObjectGuid,
    ) {
        #[cfg(test)]
        self.social
            .represented_query_petitions_like_cpp
            .push(RepresentedQueryPetitionLikeCpp {
                petition_id,
                item_guid,
            });
    }
}


#[cfg(test)]
#[path = "../../../unit_tests/session/pets/pet/f3_shims.rs"]
mod f3_shims;
