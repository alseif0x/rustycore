//! Represented pet summoning, dismissal and stabling.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {}

impl crate::session::PetsCx<'_> {
    pub(crate) fn resummon_pet_temporary_unsummoned_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.hub
                .fixtures
                .pets
                .temporary_pet_resummon_requests_like_cpp = self
                .hub
                .fixtures
                .pets
                .temporary_pet_resummon_requests_like_cpp
                .saturating_add(1);
        }

        let Some(pet_lifecycle) = self
            .hub
            .shared()
            .player_pet_lifecycle_state_snapshot_like_cpp()
        else {
            return;
        };
        let pet_number = pet_lifecycle.temporary_unsummoned_pet_number;
        if pet_number == 0
            || self
                .hub
                .shared()
                .is_pet_need_be_temporary_unsummoned_like_cpp()
            || self
                .hub
                .shared()
                .player_pet_guid_state_like_cpp()
                .flatten()
                .is_some()
        {
            return;
        }

        self.hub
            .invalidate_represented_character_pet_empty_authority_like_cpp();

        let load_info = Pet::get_load_pet_info(&pet_lifecycle.stable, 0, pet_number, None);
        let stable_info = load_info
            .filter(|info| info.pet_number == pet_number)
            .and_then(|_| {
                self.hub
                    .shared()
                    .represented_pet_stable_info_by_number_like_cpp(pet_number)
            });
        let inserted_guid = stable_info.and_then(|info| {
            let owner_guid = self.hub.core.player_guid()?;
            let map_id = u32::from(self.hub.core.player_map_id_like_cpp());
            let instance_id = self
                .hub
                .core
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)?;
            let position = self.hub.shared().player_position_like_cpp()?;
            let creature_id = info.creature_id;
            let pet_guid = ObjectGuid::create_world_object(
                HighGuid::Pet,
                0,
                1,
                self.hub.core.player_map_id_like_cpp(),
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
                .hub
                .shared()
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
            self.hub
                .catalogs
                .validate_represented_pet_action_bar_like_cpp(charm_info);
            if let Some(spells) = self
                .lifecycle
                .pet_load_spells_for_pet_number_like_cpp(pet_number)
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
                .lifecycle
                .pet_load_spell_cooldowns_for_pet_number_like_cpp(pet_number)
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
                .lifecycle
                .pet_load_spell_charges_for_pet_number_like_cpp(pet_number)
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
                .lifecycle
                .pet_load_aura_effects_for_pet_number_like_cpp(pet_number)
                .cloned()
                .unwrap_or_default();
            if let Some(auras) = self
                .lifecycle
                .pet_load_auras_for_pet_number_like_cpp(pet_number)
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
                    .lifecycle
                    .pet_load_declined_names_for_pet_number_like_cpp(pet_number)
            {
                pet.set_declined_names(Some(PetDeclinedNamesLikeCpp {
                    names: declined_names.names.clone(),
                }));
            }

            let manager = self
                .hub
                .core
                .canonical_map_manager
                .as_ref()
                .map(Arc::clone)?;
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

        if !self
            .hub
            .update_player_pet_lifecycle_state_like_cpp(|state| {
                state.temporary_unsummoned_pet_number = 0;
            })
        {
            return;
        }
        if let Some(pet_guid) = inserted_guid {
            let _ = self.hub.set_player_pet_guid_like_cpp(Some(pet_guid));
            #[cfg(test)]
            if let Some(info) = self
                .hub
                .shared()
                .represented_pet_stable_info_by_number_like_cpp(pet_number)
            {
                self.hub
                    .fixtures
                    .pets
                    .represented_pet_created_by_spell_like_cpp = info.created_by_spell_id;
                self.hub.fixtures.pets.represented_pet_react_state_like_cpp =
                    info.react_state as u8;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn resummon_pet_temporary_unsummoned_if_any_like_cpp(&mut self) {
        self.resummon_pet_temporary_unsummoned_like_cpp();
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/summoning/f3_shims.rs"]
mod f3_shims;
