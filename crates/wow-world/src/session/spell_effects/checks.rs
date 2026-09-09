//! Pre-execution checks the effect path performs.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Check if any hostile creature should aggro the player based on proximity.
    /// Called from movement handlers (CMSG_MOVE_*).
    pub(crate) async fn check_creature_aggro(&mut self) {
        use wow_packet::ServerPacket;
        use wow_packet::packets::combat::AttackStart;

        if self.resolved_in_combat_like_cpp() != Some(false) {
            return;
        }

        let player_pos = match self.player_position_like_cpp() {
            Some(p) => p,
            None => return,
        };
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let guids = self.world_creature_guids();
        let mut aggro_guid: Option<wow_core::ObjectGuid> = None;
        let player_combat_reach = self.canonical_player_combat_reach_snapshot_like_cpp();
        let player_level = self.player_level_like_cpp();
        let Some(player_detected_range_aura_mod) = self
            .resolved_total_represented_aura_modifier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModDetectedRange,
            )
        else {
            return;
        };
        let player_detected_range_aura_mod = player_detected_range_aura_mod as f32;
        let aggro_config = self.legacy_creature_aggro_config_like_cpp.clone();

        for guid in guids {
            let aggroed = self
                .mutate_world_creature(guid, |creature| {
                    if !creature.is_alive() || creature.creature.ai_ownership().aggro_radius <= 0.0
                    {
                        return false;
                    }
                    let effective_aggro_range =
                        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                            aggro_rate: aggro_config.creature_aggro_rate,
                            creature_combat_reach: creature.creature.unit().world().combat_reach(),
                            expansion_max_level: max_level_for_expansion_like_cpp(
                                creature.creature.lifecycle_metadata().required_expansion,
                            ),
                            max_player_level_config: aggro_config.max_player_level_config,
                            player_level_for_target: player_level,
                            creature_level_for_target: creature.level(),
                            creature_detect_range_aura_mod: creature
                                .creature
                                .unit()
                                .total_aura_modifier_like_cpp(
                                    wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE,
                                )
                                as f32,
                            player_detected_range_aura_mod,
                        }) + creature.creature.combat_distance();

                    creature
                        .creature
                        .try_ai_aggro_with_effective_range_like_cpp(
                            player_guid,
                            &player_pos,
                            player_combat_reach,
                            effective_aggro_range,
                        )
                })
                .unwrap_or(false);
            if aggroed {
                aggro_guid = Some(guid);
                break;
            }
        }

        if let Some(guid) = aggro_guid {
            let start = AttackStart {
                attacker: guid,
                victim: player_guid,
            };
            let _ = self.send_tx().send(start.to_bytes());
            self.set_combat_target_like_cpp(Some(guid));
            self.set_in_combat_like_cpp(true);
        }
    }
    /// C++ `Spell::CheckCast` gates for battle-pet unit-target effects.
    pub(in crate::session) fn check_represented_battle_pet_spell_like_cpp(
        &self,
        spell_info: &wow_data::SpellInfo,
        metadata: SpellCastMetadata,
    ) -> Option<SpellCastResult> {
        for effect in spell_info.effects() {
            let is_battle_pet_effect = effect.effect
                == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY
                || effect.effect
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL
                || effect.effect
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE;
            if !is_battle_pet_effect {
                continue;
            }

            let Some(companion_guid) = metadata.unit_target_battle_pet_companion_guid else {
                return Some(SpellCastResult::BadTargets);
            };

            if !self.has_represented_battle_pet_journal_lock_like_cpp() {
                return Some(SpellCastResult::CantDoThatRightNow);
            }

            let summoned_guid = self.represented_summoned_battle_pet_guid_like_cpp();
            if summoned_guid.is_none() || companion_guid.is_empty() {
                return Some(SpellCastResult::NoPet);
            }

            if summoned_guid != Some(companion_guid) {
                return Some(SpellCastResult::BadTargets);
            }

            let Some(pet) = self.represented_battle_pet_like_cpp(companion_guid) else {
                continue;
            };

            if let Some(species) = self.battle_pet_species_entry_like_cpp(pet.species) {
                let battle_pet_type = effect.effect_misc_value_1 as u32;
                if battle_pet_type != 0 {
                    let type_mask = 1u32
                        .checked_shl(u32::from(species.pet_type_enum))
                        .unwrap_or(0);
                    if battle_pet_type & type_mask == 0 {
                        return Some(SpellCastResult::WrongBattlePetType);
                    }
                }

                if (effect.effect
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL
                    || effect.effect
                        == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE)
                    && pet.level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP
                {
                    return Some(SpellCastResult::GrantPetLevelFail);
                }

                if species.has_flag_like_cpp(wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP)
                {
                    return Some(SpellCastResult::BadTargets);
                }
            }
        }

        None
    }
    /// C++ `SpellInfo::CheckLocation` aura limitation plus
    /// `Spell::CheckCast` per-effect water gate for represented mount spells.
    pub(in crate::session) fn check_represented_mount_spell_like_cpp(
        &self,
        spell_info: &wow_data::SpellInfo,
    ) -> Option<RepresentedMountSpellCheckOutcomeLikeCpp> {
        for effect in spell_info.effects() {
            if effect.is_mod_shapeshift_aura_like_cpp() {
                if let Some(form) = self
                    .spell_catalogs
                    .spell_shapeshift_form_store
                    .as_ref()
                    .and_then(|store| {
                        u32::try_from(effect.effect_misc_value_1)
                            .ok()
                            .and_then(|form_id| store.get(form_id))
                    })
                {
                    let (is_submerged, is_in_water) =
                        self.represented_player_mount_liquid_state_like_cpp()?;
                    let riding_skill =
                        self.resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?;
                    if form.mount_type_id != 0
                        && let Err(reject_reason) = self
                            .represented_mount_capability_selection_for_type_like_cpp(
                                form.mount_type_id,
                                u32::from(riding_skill),
                                None,
                                is_submerged,
                                is_in_water,
                            )
                    {
                        info!(
                            account = self.account_id,
                            spell_id = spell_info.spell_id,
                            form_id = effect.effect_misc_value_1,
                            mount_type_id = form.mount_type_id,
                            riding_skill,
                            map_id = self.player_map_id_like_cpp(),
                            ?reject_reason,
                            "Rejecting represented shapeshift mount form cast: no mount capability"
                        );
                        return Some(RepresentedMountSpellCheckOutcomeLikeCpp::CastFailed(
                            SpellCastResult::NotHere,
                        ));
                    }
                }
            }

            if !effect.is_mounted_aura_like_cpp() {
                continue;
            }

            let (_, is_in_water) = self.represented_player_mount_liquid_state_like_cpp()?;
            if is_in_water
                && spell_info.has_aura_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
                )
            {
                info!(
                    account = self.account_id,
                    spell_id = spell_info.spell_id,
                    "Rejecting represented flying mount cast while in water"
                );
                return Some(RepresentedMountSpellCheckOutcomeLikeCpp::CastFailed(
                    SpellCastResult::OnlyAbovewater,
                ));
            }

            if let Some(disallowed_source_id) = self.represented_disallowed_mount_form_like_cpp()? {
                self.send_packet(&MountResult {
                    result: MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP,
                });
                info!(
                    account = self.account_id,
                    spell_id = spell_info.spell_id,
                    disallowed_source_id,
                    "Rejecting represented mount cast: player is in a disallowed shapeshift form"
                );
                return Some(RepresentedMountSpellCheckOutcomeLikeCpp::DontReport);
            }

            let mut mount_type_id = u16::try_from(effect.effect_misc_value_2).unwrap_or_default();
            if let Some(mount_entry) = self.mount_store.as_ref().and_then(|store| {
                u32::try_from(spell_info.spell_id)
                    .ok()
                    .and_then(|spell_id| store.get_by_source_spell_id_like_cpp(spell_id))
            }) {
                mount_type_id = mount_entry.mount_type_id;
            }

            if mount_type_id != 0 {
                let current_area_id = self.player_zone_area_like_cpp()?.1;
                let (is_submerged, is_in_water) =
                    self.represented_player_mount_liquid_state_like_cpp()?;
                let riding_skill =
                    self.resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?;
                if let Err(reject_reason) = self
                    .represented_mount_capability_selection_for_type_like_cpp(
                        mount_type_id,
                        u32::from(riding_skill),
                        None,
                        is_submerged,
                        is_in_water,
                    )
                {
                    info!(
                        account = self.account_id,
                        spell_id = spell_info.spell_id,
                        mount_type_id,
                        riding_skill,
                        map_id = self.player_map_id_like_cpp(),
                        area_id = current_area_id,
                        is_submerged,
                        is_in_water,
                        ?reject_reason,
                        "Rejecting represented mount cast: no mount capability"
                    );
                    return Some(RepresentedMountSpellCheckOutcomeLikeCpp::CastFailed(
                        SpellCastResult::NotHere,
                    ));
                }
                info!(
                    account = self.account_id,
                    spell_id = spell_info.spell_id,
                    mount_type_id,
                    riding_skill,
                    map_id = self.player_map_id_like_cpp(),
                    area_id = current_area_id,
                    is_submerged,
                    is_in_water,
                    "Represented mount cast passed C++ mount capability check"
                );
            }
        }

        None
    }
}
