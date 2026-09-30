use super::*;

impl WorldSession {
    pub(in crate::session) fn player_pet_guid_state_like_cpp(&self) -> Option<Option<ObjectGuid>> {
        let canonical = self.with_owned_player_like_cpp(|player| player.gameplay_state().pet_guid);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_pet_guid_like_cpp);
        }
        #[cfg(all(not(test), feature = "test-fixtures"))]
        if canonical.is_none() && self.character_lifecycle_handleless_fixture() {
            return Some(self.represented_pet_guid_like_cpp);
        }
        canonical
    }
    pub(in crate::session) fn set_player_pet_guid_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_pet_guid_like_cpp(pet_guid);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.represented_pet_guid_like_cpp = pet_guid;
            return true;
        }
        #[cfg(all(not(test), feature = "test-fixtures"))]
        if self.character_lifecycle_handleless_fixture() {
            self.represented_pet_guid_like_cpp = pet_guid;
            return true;
        }
        canonical
    }
    pub(in crate::session) fn disable_pet_controls_on_mount_like_cpp(
        &mut self,
        react_state: u8,
        command_state: u8,
    ) {
        let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() else {
            return;
        };

        let Some((previous_react_state, _)) = self.canonical_pet_mode_state_like_cpp() else {
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
        self.send_packet(&wow_packet::packets::pet::PetMode {
            pet_guid,
            react_state,
            command_state,
            flag: 0,
        });
    }
    pub(in crate::session) fn enable_pet_controls_on_dismount_like_cpp(&mut self) {
        if let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() {
            if let Some((current_react_state, command_state)) =
                self.canonical_pet_mode_state_like_cpp()
            {
                let react_state = self
                    .player_pet_lifecycle_state_snapshot_like_cpp()
                    .and_then(|state| state.temporary_mount_react_state)
                    .unwrap_or(current_react_state);
                if self.update_canonical_pet_mode_state_like_cpp(react_state, command_state) {
                    self.send_packet(&wow_packet::packets::pet::PetMode {
                        pet_guid,
                        react_state,
                        command_state,
                        flag: 0,
                    });
                }
            }
        }

        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.temporary_mount_react_state = None;
        });
    }
    pub(super) fn with_canonical_pet_like_cpp<R>(
        &self,
        pet_guid: ObjectGuid,
        inspect: impl FnOnce(&Pet) -> R,
    ) -> Option<R> {
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
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
    pub(in crate::session) fn with_canonical_pet_mut_like_cpp<R>(
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
    pub(super) fn canonical_pet_mode_state_like_cpp(&self) -> Option<(u8, u8)> {
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
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.represented_pet_react_state_like_cpp,
                self.represented_pet_command_state_like_cpp,
            ));
        }
        canonical
    }
    pub(super) fn update_canonical_pet_mode_state_like_cpp(
        &mut self,
        react_state: u8,
        command_state: u8,
    ) -> bool {
        let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() else {
            return false;
        };
        let canonical = self
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
        #[cfg(test)]
        if !canonical && self.player_handle_like_cpp.is_none() {
            self.represented_pet_react_state_like_cpp = react_state;
            self.represented_pet_command_state_like_cpp = command_state;
            return true;
        }
        canonical
    }
    pub(in crate::session) fn player_pet_lifecycle_state_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerPetLifecycleStateLikeCpp> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.pet_lifecycle_state_like_cpp().clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(PlayerPetLifecycleStateLikeCpp {
                stable: self.represented_pet_stable_like_cpp.clone(),
                character_rows_empty_authority_complete: self
                    .represented_character_pet_rows_empty_authority_complete_like_cpp,
                temporary_unsummoned_pet_number: self
                    .represented_temporary_unsummoned_pet_number_like_cpp,
                old_pet_spell: self.represented_old_pet_spell_like_cpp,
                temporary_mount_react_state: self.temporary_mount_pet_react_state_like_cpp,
            });
        }
        canonical
    }
    pub(in crate::session) fn update_player_pet_lifecycle_state_like_cpp(
        &mut self,
        update: impl FnOnce(&mut PlayerPetLifecycleStateLikeCpp),
    ) -> bool {
        if self.player_handle_like_cpp.is_some() {
            return self
                .with_owned_player_mut_like_cpp(|player| {
                    update(player.pet_lifecycle_state_mut_like_cpp())
                })
                .is_some();
        }
        #[cfg(test)]
        {
            let mut state = self
                .player_pet_lifecycle_state_snapshot_like_cpp()
                .unwrap_or_default();
            update(&mut state);
            self.represented_pet_stable_like_cpp = state.stable;
            self.represented_character_pet_rows_empty_authority_complete_like_cpp =
                state.character_rows_empty_authority_complete;
            self.represented_temporary_unsummoned_pet_number_like_cpp =
                state.temporary_unsummoned_pet_number;
            self.represented_old_pet_spell_like_cpp = state.old_pet_spell;
            self.temporary_mount_pet_react_state_like_cpp = state.temporary_mount_react_state;
            true
        }
        #[cfg(not(test))]
        {
            let _ = update;
            false
        }
    }
    pub(in crate::session) fn invalidate_represented_character_pet_empty_authority_like_cpp(
        &mut self,
    ) {
        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.character_rows_empty_authority_complete = false;
        });
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
}
