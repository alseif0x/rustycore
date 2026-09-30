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

        self.invalidate_represented_character_pet_empty_authority_like_cpp();

        if !self.update_player_pet_lifecycle_state_like_cpp(|state| {
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
    /// C++ `Player::RemovePet(nullptr, PET_SAVE_NOT_IN_SLOT, true)`.
    ///
    /// Represented boundary: clears the active represented pet link, resets
    /// `PetStable::CurrentPetIndex`, and removes the live typed pet from the
    /// canonical map if present. Reagent return and exact `Pet::SavePetToDB`
    /// remain owned by the future full pet/inventory persistence runtime.
    pub(crate) fn remove_represented_pet_not_in_slot_like_cpp(&mut self) {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let pet_guid = self.player_pet_guid_state_like_cpp().flatten();
        if let Some(pet_guid) = pet_guid
            && let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone)
            && let Ok(mut manager) = manager.lock()
        {
            let mut removed = false;
            manager.do_for_all_maps_mut(|managed| {
                if removed {
                    return;
                }
                match managed.map_mut().remove_from_map_like_cpp(pet_guid, false) {
                    Ok(_) => removed = true,
                    Err(wow_map::RemoveFromMapError::ObjectNotFound { .. }) => {}
                    Err(_) => {}
                }
            });
        }

        if self.player_pet_guid_state_like_cpp().flatten().is_some() {
            let _ = self.set_player_pet_guid_like_cpp(None);
            #[cfg(test)]
            {
                self.represented_pet_created_by_spell_like_cpp = 0;
                self.represented_pet_react_state_like_cpp =
                    wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP;
                self.represented_pet_command_state_like_cpp =
                    wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP;
            }
            let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
                state.temporary_mount_react_state = None;
            });
            #[cfg(test)]
            {
                self.represented_pet_movement_speed_rates_like_cpp =
                    [1.0; UnitMoveTypeLikeCpp::COUNT];
            }
        }
        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.stable.current_pet_index = None;
        });
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.player_pet_guid_state_like_cpp().flatten()
    }
}
