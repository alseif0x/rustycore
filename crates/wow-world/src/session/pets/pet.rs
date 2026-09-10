//! Represented pet state at the Session boundary.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn player_pet_guid_state_like_cpp(&self) -> Option<Option<ObjectGuid>> {
        let canonical = self.with_owned_player_like_cpp(|player| player.gameplay_state().pet_guid);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
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
                player.gameplay_state_mut().pet_guid = pet_guid;
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
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
    fn with_canonical_pet_like_cpp<R>(
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
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.represented_pet_react_state_like_cpp,
                self.represented_pet_command_state_like_cpp,
            ));
        }
        canonical
    }
    fn update_canonical_pet_mode_state_like_cpp(
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
        self.pet_load_query_holder_rows_like_cpp.spells.clear();
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
    pub(in crate::session) fn validate_represented_pet_action_bar_like_cpp(
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
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_sign_petition_like_cpp(
        &mut self,
        petition_guid: ObjectGuid,
        choice: u8,
    ) {
        #[cfg(test)]
        self.represented_sign_petitions_like_cpp
            .push(RepresentedSignPetitionLikeCpp {
                petition_guid,
                choice,
            });
    }
    #[cfg(test)]
    pub(crate) fn represented_sign_petitions_like_cpp(&self) -> &[RepresentedSignPetitionLikeCpp] {
        &self.represented_sign_petitions_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_decline_petition_like_cpp(
        &mut self,
        petition_guid: ObjectGuid,
    ) {
        #[cfg(test)]
        self.represented_decline_petitions_like_cpp
            .push(RepresentedDeclinePetitionLikeCpp { petition_guid });
    }
    #[cfg(test)]
    pub(crate) fn represented_decline_petitions_like_cpp(
        &self,
    ) -> &[RepresentedDeclinePetitionLikeCpp] {
        &self.represented_decline_petitions_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_query_petition_like_cpp(
        &mut self,
        petition_id: u32,
        item_guid: ObjectGuid,
    ) {
        #[cfg(test)]
        self.represented_query_petitions_like_cpp
            .push(RepresentedQueryPetitionLikeCpp {
                petition_id,
                item_guid,
            });
    }
    #[cfg(test)]
    pub(crate) fn represented_query_petitions_like_cpp(
        &self,
    ) -> &[RepresentedQueryPetitionLikeCpp] {
        &self.represented_query_petitions_like_cpp
    }
    #[cfg(test)]
    pub(in crate::session) fn record_represented_tapper_pet_killed_unit_hooks_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) {
        let (Some(player_guid), Some(pet_guid)) = (
            self.player_guid(),
            self.player_pet_guid_state_like_cpp().flatten(),
        ) else {
            return;
        };
        let tapper_has_current_player = self
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.tap_list().contains(&player_guid)
            })
            .unwrap_or(true);
        if !tapper_has_current_player {
            return;
        }
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi {
                tapper_guid: player_guid,
                pet_guid,
                victim_guid: creature_guid,
            },
        );
    }
    fn represented_pet_position_like_cpp(&self, pet_guid: ObjectGuid) -> Option<Position> {
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
    fn send_represented_pet_spline_speed_like_cpp(
        &self,
        pet_guid: ObjectGuid,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let Some(opcode) =
            crate::session_rules::creature_movement_spline_speed_opcode_like_cpp(move_type)
        else {
            return;
        };
        let packet_bytes = wow_packet::packets::movement::MoveSplineSetSpeed {
            opcode,
            mover_guid: pet_guid,
            speed: PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()] * rate,
        }
        .to_bytes();
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        if self.client_visible_guids_like_cpp.contains(&pet_guid)
            && self.send_tx().send(packet_bytes.clone()).is_err()
        {
            warn!("Send channel closed for account {}", self.account_id);
        }

        let (Some(player_guid), Some(registry)) = (self.player_guid(), self.player_registry())
        else {
            return;
        };
        let Some(source_position) = self
            .represented_pet_position_like_cpp(pet_guid)
            .or_else(|| self.player_position_like_cpp())
        else {
            return;
        };
        for registration in registry.movement_recipients_within_range(
            player_guid,
            map_id,
            instance_id,
            source_position,
            crate::map_manager::VISIBILITY_RADIUS,
        ) {
            let _ = registry.try_send_current_command(
                registration,
                SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                    queued_at: Instant::now(),
                    source_guid: pet_guid,
                    map_id,
                    instance_id,
                    packet_bytes: packet_bytes.clone(),
                }),
            );
        }
    }
    pub(in crate::session) fn propagate_represented_player_speed_to_pet_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() else {
            return;
        };
        if self.resolved_in_combat_like_cpp() != Some(false) {
            return;
        }

        let rate = rate.max(0.01);
        let index = move_type.index();
        let canonical_changed = self.with_canonical_pet_mut_like_cpp(pet_guid, |pet| {
            let unit = pet.creature_mut().unit_mut();
            if unit.speed_rate_at_like_cpp(index) == Some(rate) {
                return false;
            }
            unit.set_speed_rate_at_like_cpp(index, rate)
        });
        let changed = match canonical_changed {
            Some(changed) => changed,
            None => {
                #[cfg(test)]
                {
                    if self.player_handle_like_cpp.is_none() {
                        if self.represented_pet_movement_speed_rates_like_cpp[index] == rate {
                            return;
                        }
                        self.represented_pet_movement_speed_rates_like_cpp[index] = rate;
                        true
                    } else {
                        false
                    }
                }
                #[cfg(not(test))]
                {
                    false
                }
            }
        };
        if !changed {
            return;
        }
        #[cfg(test)]
        {
            self.represented_pet_speed_propagations_like_cpp = self
                .represented_pet_speed_propagations_like_cpp
                .saturating_add(1);
        }
        self.send_represented_pet_spline_speed_like_cpp(pet_guid, move_type, rate);
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_movement_speed_rate_like_cpp(
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
        canonical.unwrap_or(self.represented_pet_movement_speed_rates_like_cpp[move_type.index()])
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_speed_propagations_like_cpp(&self) -> u32 {
        self.represented_pet_speed_propagations_like_cpp
    }
}
