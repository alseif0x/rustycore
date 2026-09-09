//! Represented pet summoning, dismissal and stabling.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn unsummon_represented_pet_for_same_map_teleport_if_out_of_range_like_cpp(
        &mut self,
        destination: wow_core::Position,
        options: TeleportToOptionsLikeCpp,
    ) {
        if options & TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP != 0 {
            return;
        }
        let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() else {
            return;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        let should_unsummon = {
            let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
                return;
            };
            let Ok(manager) = manager.lock() else {
                return;
            };
            let Some(managed) = manager.find_map(map_id, instance_id) else {
                return;
            };
            let visibility_range = managed.map().visibility_range();
            managed
                .map()
                .get_pet(pet_guid)
                .is_some_and(|pet| pet.distance_to_position(destination) > visibility_range)
        };

        if should_unsummon {
            self.unsummon_represented_pet_temporary_if_any_like_cpp();
        }
    }
    pub(crate) fn set_represented_pet_stable_like_cpp(&mut self, stable: PetStable) {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| state.stable = stable);
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_stable_current_index_like_cpp(&self) -> Option<u32> {
        self.player_pet_lifecycle_state_snapshot_like_cpp()
            .and_then(|state| state.stable.current_pet_index)
    }
    pub(crate) fn load_represented_pet_stable_rows_like_cpp(
        &mut self,
        summoned_pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetStableRowLikeCpp>,
    ) -> usize {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let mut stable = PetStable::default();
        let mut loaded = 0usize;
        let mut query_had_rows = false;

        for row in rows {
            query_had_rows = true;
            let slot = row.slot;
            let pet_info = PetStableInfo {
                name: row.name,
                action_bar: row.action_bar,
                pet_number: row.pet_number,
                creature_id: row.creature_id,
                display_id: row.display_id,
                experience: row.experience,
                health: row.health,
                mana: row.mana,
                last_save_time: row.last_save_time,
                created_by_spell_id: row.created_by_spell_id,
                specialization_id: row.specialization_id,
                level: row.level,
                react_state: react_state_from_db_like_cpp(row.react_state),
                pet_type: pet_type_from_db_like_cpp(row.pet_type),
                was_renamed: row.was_renamed,
            };

            if PetSaveMode::is_active_slot(slot) {
                let index = slot as usize;
                if stable.active_pets.len() <= index {
                    stable.active_pets.resize_with(index + 1, || None);
                }
                stable.active_pets[index] = Some(pet_info);
            } else if PetSaveMode::is_stabled_slot(slot) {
                let index = (slot - PetSaveMode::stable_slot(0)) as usize;
                if stable.stabled_pets.len() <= index {
                    stable.stabled_pets.resize_with(index + 1, || None);
                }
                stable.stabled_pets[index] = Some(pet_info);
            } else if slot == PetSaveMode::NotInSlot as i16 {
                stable.unslotted_pets.push(pet_info);
            } else {
                continue;
            }

            loaded = loaded.saturating_add(1);
        }

        let selected_summoned_pet =
            Pet::get_load_pet_info(&stable, 0, summoned_pet_number, None).is_some();
        if selected_summoned_pet {
            stable.current_pet_index = stable
                .active_pets
                .iter()
                .position(|pet| {
                    pet.as_ref()
                        .is_some_and(|pet| pet.pet_number == summoned_pet_number)
                })
                .map(|index| index as u32);
        }

        if !self.update_player_pet_lifecycle_state_like_cpp(|state| {
            // C++ remembers the selected controlled pet on the Player even
            // while the live Pet object is absent from Map storage.
            if selected_summoned_pet {
                state.temporary_unsummoned_pet_number = summoned_pet_number;
            }
            state.stable = stable;
            state.character_rows_empty_authority_complete = !query_had_rows;
        }) {
            return 0;
        }
        loaded
    }
    #[cfg(test)]
    pub(crate) fn represented_temporary_unsummoned_pet_number_like_cpp(&self) -> u32 {
        self.player_pet_lifecycle_state_snapshot_like_cpp()
            .map_or(0, |state| state.temporary_unsummoned_pet_number)
    }
    pub(in crate::session) fn unsummon_represented_pet_temporary_if_any_like_cpp(&mut self) {
        let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() else {
            return;
        };
        let Some(pet_lifecycle) = self.player_pet_lifecycle_state_snapshot_like_cpp() else {
            return;
        };

        self.request_temporary_pet_unsummon_like_cpp();

        if let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone)
            && let Ok(mut manager) = manager.lock()
        {
            let mut removed = false;
            let mut temporary_pet_number = None;
            let mut temporary_pet_created_by_spell = 0;
            manager.do_for_all_maps_mut(|managed| {
                if removed {
                    return;
                }
                let Some(pet) = managed.map().get_typed_pet(pet_guid) else {
                    return;
                };
                if pet_lifecycle.temporary_unsummoned_pet_number == 0
                    && pet.is_controlled()
                    && !pet.is_temporary_summoned()
                {
                    temporary_pet_number = pet
                        .creature()
                        .unit()
                        .subsystems()
                        .control
                        .charm_info
                        .as_ref()
                        .map(|charm_info| charm_info.pet_number);
                    temporary_pet_created_by_spell = pet.created_by_spell_id_like_cpp();
                }
                match managed.map_mut().remove_from_map_like_cpp(pet_guid, false) {
                    Ok(_) => removed = true,
                    Err(wow_map::RemoveFromMapError::ObjectNotFound { .. }) => {}
                    Err(_) => {}
                }
            });
            if let Some(pet_number) = temporary_pet_number.filter(|pet_number| *pet_number != 0) {
                let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
                    state.temporary_unsummoned_pet_number = pet_number;
                    state.old_pet_spell = temporary_pet_created_by_spell;
                });
            }
        }

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
    }
    fn represented_pet_stable_info_by_number_like_cpp(
        &self,
        pet_number: u32,
    ) -> Option<PetStableInfo> {
        let pet_lifecycle = self.player_pet_lifecycle_state_snapshot_like_cpp()?;
        pet_lifecycle
            .stable
            .active_pets
            .iter()
            .chain(pet_lifecycle.stable.stabled_pets.iter())
            .flatten()
            .find(|pet| pet.pet_number == pet_number)
            .cloned()
            .or_else(|| {
                pet_lifecycle
                    .stable
                    .unslotted_pets
                    .iter()
                    .find(|pet| pet.pet_number == pet_number)
                    .cloned()
            })
    }
    fn is_pet_need_be_temporary_unsummoned_like_cpp(&self) -> bool {
        !self.player_is_in_world_for_registry_like_cpp()
            || self.resolved_player_is_alive_like_cpp() != Some(true)
            || self
                .resolved_player_movement_flags_like_cpp()
                .is_none_or(|flags| flags.contains(MovementFlag::FLYING))
    }
    pub(crate) fn resummon_pet_temporary_unsummoned_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.temporary_pet_resummon_requests_like_cpp = self
                .temporary_pet_resummon_requests_like_cpp
                .saturating_add(1);
        }

        let Some(pet_lifecycle) = self.player_pet_lifecycle_state_snapshot_like_cpp() else {
            return;
        };
        let pet_number = pet_lifecycle.temporary_unsummoned_pet_number;
        if pet_number == 0
            || self.is_pet_need_be_temporary_unsummoned_like_cpp()
            || self.player_pet_guid_state_like_cpp().flatten().is_some()
        {
            return;
        }

        self.invalidate_represented_character_pet_empty_authority_like_cpp();

        let load_info = Pet::get_load_pet_info(&pet_lifecycle.stable, 0, pet_number, None);
        let stable_info = load_info
            .filter(|info| info.pet_number == pet_number)
            .and_then(|_| self.represented_pet_stable_info_by_number_like_cpp(pet_number));
        let inserted_guid = stable_info.and_then(|info| {
            let owner_guid = self.player_guid()?;
            let map_id = u32::from(self.player_map_id_like_cpp());
            let instance_id = self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)?;
            let position = self.player_position_like_cpp()?;
            let creature_id = info.creature_id;
            let pet_guid = ObjectGuid::create_world_object(
                HighGuid::Pet,
                0,
                1,
                self.player_map_id_like_cpp(),
                instance_id,
                creature_id,
                i64::from(pet_number),
            );

            let mut pet = Pet::new(owner_guid, info.pet_type);
            pet.set_created_by_spell_id_like_cpp(info.created_by_spell_id);
            pet.set_specialization(info.specialization_id);
            pet.creature_mut()
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(pet_guid);
            pet.creature_mut()
                .unit_mut()
                .world_mut()
                .object_mut()
                .set_entry(creature_id);
            let _ = pet
                .creature_mut()
                .unit_mut()
                .world_mut()
                .set_map(map_id, instance_id);
            pet.creature_mut().unit_mut().world_mut().relocate(position);
            if !info.name.is_empty() {
                pet.creature_mut()
                    .unit_mut()
                    .world_mut()
                    .set_name(info.name);
            }
            if info.display_id != 0 {
                pet.creature_mut()
                    .set_display_id(info.display_id, true, None);
            }
            if info.level != 0 {
                pet.creature_mut().unit_mut().set_level(info.level);
            }
            pet.set_pet_experience(info.experience);
            if let Some(next_level_experience) = self
                .resolved_player_xp_for_level_like_cpp(info.level)
                .map(Pet::pet_next_level_xp_for_owner_level)
            {
                pet.set_pet_next_level_experience(next_level_experience);
            }
            if info.health == 0 && info.pet_type == PetType::Hunter {
                pet.creature_mut()
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::JustDied);
                pet.creature_mut().unit_mut().set_health(0);
            } else if info.health != 0 {
                pet.creature_mut()
                    .unit_mut()
                    .set_max_health(u64::from(info.health));
                pet.creature_mut()
                    .unit_mut()
                    .set_health(u64::from(info.health));
            }
            if info.mana != 0 {
                pet.creature_mut()
                    .unit_mut()
                    .set_power(PowerType::Mana, info.mana as i32);
            }
            pet.creature_mut().set_react_state(info.react_state);
            let charm_info = pet
                .creature_mut()
                .unit_mut()
                .subsystems_mut()
                .control
                .init_charm_info();
            charm_info.pet_number = pet_number;
            charm_info.command_state = wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP;
            charm_info.load_pet_action_bar_like_cpp(&info.action_bar);
            self.validate_represented_pet_action_bar_like_cpp(charm_info);
            if let Some(spells) = self
                .pet_load_query_holder_rows_like_cpp
                .spells
                .get(&pet_number)
            {
                for spell in spells {
                    pet.add_spell(
                        spell.spell_id,
                        active_state_from_db_like_cpp(spell.active),
                        PetSpellState::Unchanged,
                        PetSpellType::Normal,
                    );
                }
            }
            let spell_history = &mut pet
                .creature_mut()
                .unit_mut()
                .subsystems_mut()
                .spells
                .history;
            if let Some(cooldowns) = self
                .pet_load_query_holder_rows_like_cpp
                .spell_cooldowns
                .get(&pet_number)
            {
                for cooldown in cooldowns {
                    spell_history.add_cooldown(
                        cooldown.spell_id,
                        0,
                        unix_secs_to_ms_like_cpp(cooldown.cooldown_end_unix_secs),
                        cooldown.category_id,
                        unix_secs_to_ms_like_cpp(cooldown.category_end_unix_secs),
                        false,
                    );
                }
            }
            if let Some(charges) = self
                .pet_load_query_holder_rows_like_cpp
                .spell_charges
                .get(&pet_number)
            {
                for charge in charges {
                    spell_history.add_charge_state_like_cpp(
                        charge.category_id,
                        unix_secs_to_ms_like_cpp(charge.recharge_start_unix_secs),
                        unix_secs_to_ms_like_cpp(charge.recharge_end_unix_secs),
                    );
                }
            }
            let pet_aura_effects = self
                .pet_load_query_holder_rows_like_cpp
                .aura_effects
                .get(&pet_number)
                .cloned()
                .unwrap_or_default();
            if let Some(auras) = self
                .pet_load_query_holder_rows_like_cpp
                .auras
                .get(&pet_number)
            {
                let aura_subsystem = &mut pet.creature_mut().unit_mut().subsystems_mut().auras;
                for (index, aura) in auras.iter().enumerate() {
                    let Some(slot) = represented_pet_aura_slot_like_cpp(index) else {
                        break;
                    };
                    let caster_guid = if aura.caster_guid.is_empty() {
                        pet_guid
                    } else {
                        aura.caster_guid
                    };
                    let applied = wow_entities::AppliedAuraRef::new(
                        aura.spell_id,
                        caster_guid,
                        slot,
                        aura.effect_mask,
                    );
                    let aura_ref = wow_entities::AuraRef::new(aura.spell_id, caster_guid);
                    aura_subsystem.add_owned(wow_entities::OwnedAuraRef::new(
                        aura.spell_id,
                        caster_guid,
                        None,
                    ));
                    aura_subsystem.add_applied(applied);
                    aura_subsystem.set_loaded_aura_state_like_cpp(
                        aura_ref,
                        wow_entities::LoadedAuraStateLikeCpp::new(
                            aura.max_duration_ms,
                            aura.remain_time_ms,
                            aura.remain_charges,
                            aura.stack_count,
                            aura.recalculate_mask,
                        ),
                    );
                    aura_subsystem.visible_auras.insert(slot, aura_ref);

                    let effect_amounts: Vec<_> = pet_aura_effects
                        .iter()
                        .filter(|effect| {
                            let effect_caster_guid = if effect.caster_guid.is_empty() {
                                pet_guid
                            } else {
                                effect.caster_guid
                            };
                            effect_caster_guid == caster_guid
                                && effect.spell_id == aura.spell_id
                                && effect.effect_mask == aura.effect_mask
                        })
                        .map(|effect| {
                            let effect_ref = wow_entities::AppliedAuraRef::new(
                                aura.spell_id,
                                caster_guid,
                                slot,
                                1u32 << u32::from(effect.effect_index),
                            );
                            aura_subsystem
                                .applied_aura_amounts
                                .insert(effect_ref, effect.amount);
                            wow_entities::VisibleAuraEffectAmountLikeCpp {
                                effect_index: effect.effect_index,
                                amount: effect.amount,
                            }
                        })
                        .collect();
                    aura_subsystem.visible_aura_applications_like_cpp.insert(
                        slot,
                        wow_entities::VisibleAuraApplicationLikeCpp::new(
                            aura.effect_mask,
                            effect_amounts,
                        ),
                    );
                }
            }
            if info.pet_type == PetType::Hunter
                && let Some(declined_names) = self
                    .pet_load_query_holder_rows_like_cpp
                    .declined_names
                    .get(&pet_number)
            {
                pet.set_declined_names(Some(PetDeclinedNamesLikeCpp {
                    names: declined_names.names.clone(),
                }));
            }

            let manager = self.canonical_map_manager.as_ref().map(Arc::clone)?;
            let mut manager = manager.lock().ok()?;
            if manager.find_map_mut(map_id, instance_id).is_none() {
                manager.create_world_map(map_id, instance_id);
            }
            let managed = manager.find_map_mut(map_id, instance_id)?;
            managed
                .map_mut()
                .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).ok()?)
                .ok()?;
            Some(pet_guid)
        });

        if !self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.temporary_unsummoned_pet_number = 0;
        }) {
            return;
        }
        if let Some(pet_guid) = inserted_guid {
            let _ = self.set_player_pet_guid_like_cpp(Some(pet_guid));
            #[cfg(test)]
            if let Some(info) = self.represented_pet_stable_info_by_number_like_cpp(pet_number) {
                self.represented_pet_created_by_spell_like_cpp = info.created_by_spell_id;
                self.represented_pet_react_state_like_cpp = info.react_state as u8;
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn resummon_pet_temporary_unsummoned_if_any_like_cpp(&mut self) {
        self.resummon_pet_temporary_unsummoned_like_cpp();
    }
}
