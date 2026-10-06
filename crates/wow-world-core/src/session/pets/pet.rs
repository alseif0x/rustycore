use crate::session::movement_protocol::UnitMoveTypeLikeCpp;
use crate::session::react_state_from_db_like_cpp;
use crate::session::state::SessionCore;
use std::sync::Arc;
use wow_core::{ObjectGuid, Position};
use wow_entities::{AccessorObjectKind, Pet, PlayerPetLifecycleStateLikeCpp};

impl SessionCore {
    pub fn with_canonical_pet_mut_like_cpp<R>(
        &self,
        pet_guid: ObjectGuid,
        mutate: impl FnOnce(&mut Pet) -> R,
    ) -> Option<R> {
        let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let mut mutate = Some(mutate);
        let mut result = None;
        manager.do_for_all_maps_mut(|managed| {
            if result.is_some() {
                return;
            }
            if let Some(pet) = managed.map_mut().get_typed_pet_mut(pet_guid) {
                result = Some(mutate.take().expect("pet mutation consumed once")(pet));
            }
        });
        result
    }

    pub(crate) fn represented_pet_position_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<Position> {
        let map_id = u32::from(self.player_map_id_like_cpp());
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let managed = manager.find_map(map_id, instance_id)?;
        managed.map().with_world_object_by_kinds_like_cpp(
            pet_guid,
            &[AccessorObjectKind::Pet],
            |object| object.position(),
        )
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PetState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_pet_speed_propagations_like_cpp(&self) -> u32 {
        self.represented_pet_speed_propagations_like_cpp
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn validate_represented_pet_action_bar_like_cpp(
        &self,
        charm_info: &mut wow_entities::CharmInfoState,
    ) {
        let Some(spell_store) = self.spell_store() else {
            return;
        };

        for button in &mut charm_info.action_bar {
            // C++ `UNIT_ACTION_BUTTON_TYPE` drops the low bit after `MAKE_UNIT_ACTION_BUTTON`
            // stores `ActiveStates << 23`; recover the just-loaded type to apply the
            // intended `LoadPetActionBar` validation without changing the packed wire shape.
            let action_type = ((*button >> 23) & 0xFF) as u8;
            if !matches!(
                action_type,
                wow_entities::ACT_DISABLED_LIKE_CPP
                    | wow_entities::ACT_ENABLED_LIKE_CPP
                    | wow_entities::ACT_PASSIVE_LIKE_CPP
            ) {
                continue;
            }

            let action = wow_entities::unit_action_button_action_like_cpp(*button);
            if spell_store
                .get(i32::try_from(action).unwrap_or(i32::MAX))
                .is_none()
            {
                *button = wow_entities::make_unit_action_button_like_cpp(
                    0,
                    wow_entities::ACT_PASSIVE_LIKE_CPP,
                );
                continue;
            }

            if self
                .spell_catalogs
                .spell_misc_store()
                .is_some_and(|store| !store.is_autocastable_like_cpp(action))
            {
                *button = wow_entities::make_unit_action_button_like_cpp(
                    action,
                    wow_entities::ACT_PASSIVE_LIKE_CPP,
                );
            }
        }
    }
}

impl crate::session::HubRef<'_> {
    fn with_canonical_pet_like_cpp<R>(
        &self,
        pet_guid: ObjectGuid,
        inspect: impl FnOnce(&Pet) -> R,
    ) -> Option<R> {
        let manager = self.core.canonical_map_manager.as_ref()?.lock().ok()?;
        let mut inspect = Some(inspect);
        let mut result = None;
        manager.do_for_all_maps(|managed| {
            if result.is_some() {
                return;
            }
            if let Some(pet) = managed.map().get_typed_pet(pet_guid) {
                result = Some(inspect.take().expect("pet inspector consumed once")(pet));
            }
        });
        result
    }

    fn canonical_pet_mode_state_like_cpp(&self) -> Option<(u8, u8)> {
        let pet_guid = self.player_pet_guid_state_like_cpp().flatten()?;
        let canonical = self.with_canonical_pet_like_cpp(pet_guid, |pet| {
            let react_state = pet.creature().react_state() as u8;
            let command_state = pet
                .creature()
                .unit()
                .subsystems()
                .control
                .charm_info
                .as_ref()
                .map_or(wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP, |info| {
                    info.command_state
                });
            (react_state, command_state)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some((
                self.fixtures.pets.represented_pet_react_state_like_cpp,
                self.fixtures.pets.represented_pet_command_state_like_cpp,
            ));
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_pet_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.player_pet_guid_state_like_cpp().flatten()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_pet_movement_speed_rate_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> f32 {
        let canonical = self
            .player_pet_guid_state_like_cpp()
            .flatten()
            .and_then(|pet_guid| {
                self.with_canonical_pet_like_cpp(pet_guid, |pet| {
                    pet.creature()
                        .unit()
                        .speed_rate_at_like_cpp(move_type.index())
                })
                .flatten()
            });
        canonical.unwrap_or(
            self.fixtures
                .pets
                .represented_pet_movement_speed_rates_like_cpp[move_type.index()],
        )
    }
}

impl crate::session::HubMut<'_> {
    pub fn disable_pet_controls_on_mount_like_cpp(&mut self, react_state: u8, command_state: u8) {
        let Some(pet_guid) = self.shared().player_pet_guid_state_like_cpp().flatten() else {
            return;
        };

        let Some((previous_react_state, _)) = self.shared().canonical_pet_mode_state_like_cpp()
        else {
            return;
        };
        if !self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.temporary_mount_react_state = Some(previous_react_state);
        }) {
            return;
        }
        if !self.update_canonical_pet_mode_state_like_cpp(react_state, command_state) {
            return;
        }
        self.core.send_packet(&wow_packet::packets::pet::PetMode {
            pet_guid,
            react_state,
            command_state,
            flag: 0,
        });
    }

    pub fn enable_pet_controls_on_dismount_like_cpp(&mut self) {
        self.core
            .enable_pet_controls_on_dismount_with_fixture_refs_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &mut crate::session::AuraDismountPetFixtureRefsLikeCpp::new(
                    &self.fixtures.pets.represented_pet_guid_like_cpp,
                    &mut self.fixtures.pets.represented_pet_react_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_command_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_stable_like_cpp,
                    &mut self
                        .fixtures
                        .pets
                        .represented_character_pet_rows_empty_authority_complete_like_cpp,
                    &mut self
                        .fixtures
                        .pets
                        .represented_temporary_unsummoned_pet_number_like_cpp,
                    &mut self.fixtures.pets.represented_old_pet_spell_like_cpp,
                    &mut self.fixtures.pets.temporary_mount_pet_react_state_like_cpp,
                ),
            );
    }

    fn update_canonical_pet_mode_state_like_cpp(
        &mut self,
        react_state: u8,
        command_state: u8,
    ) -> bool {
        let Some(pet_guid) = self.shared().player_pet_guid_state_like_cpp().flatten() else {
            return false;
        };
        let canonical = self
            .core
            .with_canonical_pet_mut_like_cpp(pet_guid, |pet| {
                pet.creature_mut()
                    .set_react_state(react_state_from_db_like_cpp(react_state));
                pet.creature_mut()
                    .unit_mut()
                    .subsystems_mut()
                    .control
                    .init_charm_info()
                    .command_state = command_state;
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            self.fixtures.pets.represented_pet_react_state_like_cpp = react_state;
            self.fixtures.pets.represented_pet_command_state_like_cpp = command_state;
            return true;
        }
        canonical
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_player_pet_guid_like_cpp(&mut self, pet_guid: Option<ObjectGuid>) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_pet_guid_like_cpp(pet_guid);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.pets.represented_pet_guid_like_cpp = pet_guid;
            return true;
        }
        canonical
    }

    pub fn update_player_pet_lifecycle_state_like_cpp(
        &mut self,
        update: impl FnOnce(&mut PlayerPetLifecycleStateLikeCpp),
    ) -> bool {
        if self.core.player_handle_like_cpp.is_some() {
            return self
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    update(player.pet_lifecycle_state_mut_like_cpp())
                })
                .is_some();
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let mut state = self
                .shared()
                .player_pet_lifecycle_state_snapshot_like_cpp()
                .unwrap_or_default();
            update(&mut state);
            self.fixtures.pets.represented_pet_stable_like_cpp = state.stable;
            self.fixtures
                .pets
                .represented_character_pet_rows_empty_authority_complete_like_cpp =
                state.character_rows_empty_authority_complete;
            self.fixtures
                .pets
                .represented_temporary_unsummoned_pet_number_like_cpp =
                state.temporary_unsummoned_pet_number;
            self.fixtures.pets.represented_old_pet_spell_like_cpp = state.old_pet_spell;
            self.fixtures.pets.temporary_mount_pet_react_state_like_cpp =
                state.temporary_mount_react_state;
            true
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            let _ = update;
            false
        }
    }

    pub fn invalidate_represented_character_pet_empty_authority_like_cpp(&mut self) {
        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.character_rows_empty_authority_complete = false;
        });
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }

    /// C++ `Player::RemovePet(nullptr, PET_SAVE_NOT_IN_SLOT, true)`.
    ///
    /// Represented boundary: clears the active represented pet link, resets
    /// `PetStable::CurrentPetIndex`, and removes the live typed pet from the
    /// canonical map if present. Reagent return and exact `Pet::SavePetToDB`
    /// remain owned by the future full pet/inventory persistence runtime.
    pub fn remove_represented_pet_not_in_slot_like_cpp(&mut self) {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let pet_guid = self.shared().player_pet_guid_state_like_cpp().flatten();
        if let Some(pet_guid) = pet_guid
            && let Some(manager) = self.core.canonical_map_manager.as_ref().map(Arc::clone)
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

        if self
            .shared()
            .player_pet_guid_state_like_cpp()
            .flatten()
            .is_some()
        {
            let _ = self.set_player_pet_guid_like_cpp(None);
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                self.fixtures.pets.represented_pet_created_by_spell_like_cpp = 0;
                self.fixtures.pets.represented_pet_react_state_like_cpp =
                    wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP;
                self.fixtures.pets.represented_pet_command_state_like_cpp =
                    wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP;
            }
            let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
                state.temporary_mount_react_state = None;
            });
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                self.fixtures
                    .pets
                    .represented_pet_movement_speed_rates_like_cpp =
                    [1.0; UnitMoveTypeLikeCpp::COUNT];
            }
        }
        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.stable.current_pet_index = None;
        });
    }

    pub(crate) fn propagate_represented_player_speed_to_pet_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let (_presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.propagate_represented_player_speed_to_pet_like_cpp(move_type, rate);
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_pet_guid_state_like_cpp(&self) -> Option<Option<ObjectGuid>> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().pet_guid);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.pets.represented_pet_guid_like_cpp);
        }
        canonical
    }

    pub fn player_pet_lifecycle_state_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerPetLifecycleStateLikeCpp> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.pet_lifecycle_state_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(PlayerPetLifecycleStateLikeCpp {
                stable: self.fixtures.pets.represented_pet_stable_like_cpp.clone(),
                character_rows_empty_authority_complete: self
                    .fixtures
                    .pets
                    .represented_character_pet_rows_empty_authority_complete_like_cpp,
                temporary_unsummoned_pet_number: self
                    .fixtures
                    .pets
                    .represented_temporary_unsummoned_pet_number_like_cpp,
                old_pet_spell: self.fixtures.pets.represented_old_pet_spell_like_cpp,
                temporary_mount_react_state: self
                    .fixtures
                    .pets
                    .temporary_mount_pet_react_state_like_cpp,
            });
        }
        canonical
    }
}
