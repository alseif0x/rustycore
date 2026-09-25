//! Load persisted pet state during character login.

use super::*;

impl WorldSession {
    pub(super) async fn load_represented_login_pet_state_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
        summoned_pet_number: u32,
    ) {
        const AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP: u16 = 0x010;
        if self
            .resolved_represented_at_login_flags_like_cpp()
            .is_some_and(|flags| (flags & AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP) != 0)
        {
            let outcome = player_lifecycle_port
                .reset_login_pet_talents_like_cpp(guid.counter() as u64)
                .await;
            if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason: error }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason: error } =
                outcome.spell_delete
            {
                warn!(
                    player_guid = guid.counter(),
                    %error,
                    "failed to apply represented AT_LOGIN_RESET_PET_TALENTS pet_spell delete like C++"
                );
            }
            if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason: error }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason: error } =
                outcome.specialization_reset
            {
                warn!(
                    player_guid = guid.counter(),
                    %error,
                    "failed to apply represented AT_LOGIN_RESET_PET_TALENTS pet specialization reset like C++"
                );
            }
        }

        self.begin_represented_character_pet_authority_load_like_cpp();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetStable(rows),
            ) => {
                let rows = rows.into_iter().map(|row| CharacterPetStableRowLikeCpp {
                    pet_number: row.pet_number,
                    creature_id: row.creature_id,
                    display_id: row.display_id,
                    level: row.level,
                    experience: row.experience,
                    react_state: row.react_state,
                    slot: row.slot,
                    name: row.name,
                    was_renamed: row.was_renamed,
                    health: row.health,
                    mana: row.mana,
                    action_bar: row.action_bar,
                    last_save_time: row.last_save_time,
                    created_by_spell_id: row.created_by_spell_id,
                    pet_type: row.pet_type,
                    specialization_id: row.specialization_id,
                });
                let loaded =
                    self.load_represented_pet_stable_rows_like_cpp(summoned_pet_number, rows);
                trace!(
                    player_guid = guid.counter(),
                    summoned_pet_number,
                    loaded,
                    "loaded represented character_pet stable rows like C++"
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => warn!(
                player_guid = guid.counter(),
                error = %reason,
                "failed to load represented character_pet rows"
            ),
            _ => unreachable!("pet stable request returned a different row family"),
        }
        if summoned_pet_number != 0 {
            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetAuras(rows),
                ) => {
                    let rows = rows.into_iter().map(|row| CharacterPetAuraRowLikeCpp {
                        caster_guid: object_guid_from_db_binary_like_cpp(row.caster_guid_binary),
                        spell_id: row.spell_id,
                        effect_mask: row.effect_mask,
                        recalculate_mask: row.recalculate_mask,
                        difficulty: row.difficulty,
                        stack_count: row.stack_count,
                        max_duration_ms: row.max_duration_ms,
                        remain_time_ms: row.remain_time_ms,
                        remain_charges: row.remain_charges,
                    });
                    let loaded =
                        self.load_represented_pet_aura_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number, loaded, "loaded represented pet_aura rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_aura rows")
                }
                _ => unreachable!("pet aura request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetAuraEffects(rows),
                ) => {
                    let rows = rows
                        .into_iter()
                        .map(|row| CharacterPetAuraEffectRowLikeCpp {
                            caster_guid: object_guid_from_db_binary_like_cpp(
                                row.caster_guid_binary,
                            ),
                            spell_id: row.spell_id,
                            effect_mask: row.effect_mask,
                            effect_index: row.effect_index,
                            amount: row.amount,
                            base_amount: row.base_amount,
                        });
                    let loaded = self
                        .load_represented_pet_aura_effect_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented pet_aura_effect rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_aura_effect rows")
                }
                _ => unreachable!("pet aura-effect request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(rows),
                ) => {
                    let rows = rows.into_iter().map(|row| CharacterPetSpellRowLikeCpp {
                        spell_id: row.spell_id,
                        active: row.active,
                    });
                    let loaded =
                        self.load_represented_pet_spell_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number, loaded, "loaded represented pet_spell rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_spell rows")
                }
                _ => unreachable!("pet spell request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCooldowns(rows),
                ) => {
                    let rows = rows
                        .into_iter()
                        .map(|row| CharacterPetSpellCooldownRowLikeCpp {
                            spell_id: row.spell_id,
                            cooldown_end_unix_secs: row.cooldown_end_unix_secs,
                            category_id: row.category_id,
                            category_end_unix_secs: row.category_end_unix_secs,
                        });
                    let loaded = self.load_represented_pet_spell_cooldown_rows_like_cpp(
                        summoned_pet_number,
                        rows,
                    );
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented pet_spell_cooldown rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_spell_cooldown rows")
                }
                _ => unreachable!("pet cooldown request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCharges(rows),
                ) => {
                    let rows = rows
                        .into_iter()
                        .map(|row| CharacterPetSpellChargeRowLikeCpp {
                            category_id: row.category_id,
                            recharge_start_unix_secs: row.recharge_start_unix_secs,
                            recharge_end_unix_secs: row.recharge_end_unix_secs,
                        });
                    let loaded = self
                        .load_represented_pet_spell_charge_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented pet_spell_charges rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_spell_charges rows")
                }
                _ => unreachable!("pet charge request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames {
                        player_guid: guid.counter() as u64,
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetDeclinedNames(rows),
                ) => {
                    let row = rows
                        .into_iter()
                        .next()
                        .map(|row| CharacterPetDeclinedNamesRowLikeCpp { names: row.names });
                    let loaded =
                        self.load_represented_pet_declined_names_like_cpp(summoned_pet_number, row);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented character_pet_declinedname row like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented character_pet_declinedname row")
                }
                _ => unreachable!("pet declined-name request returned a different row family"),
            }
        }
        if self
            .resolved_represented_at_login_flags_like_cpp()
            .is_some_and(|flags| (flags & AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP) != 0)
        {
            self.apply_represented_login_pet_talent_reset_like_cpp();
        }
    }
}
