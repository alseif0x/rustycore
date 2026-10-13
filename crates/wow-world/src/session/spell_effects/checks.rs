//! Pre-execution checks the effect path performs.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
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

            let summoned_guid =
                crate::session::hub_ref(self).represented_summoned_battle_pet_guid_like_cpp();
            if summoned_guid.is_none() || companion_guid.is_empty() {
                return Some(SpellCastResult::NoPet);
            }

            if summoned_guid != Some(companion_guid) {
                return Some(SpellCastResult::BadTargets);
            }

            let Some(pet) = self.represented_battle_pet_like_cpp(companion_guid) else {
                continue;
            };

            if let Some(species) =
                wow_world_inventory::battle_pet_species_entry_like_cpp(self, pet.species)
            {
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
                    .catalogs
                    .spell_catalogs
                    .spell_shapeshift_form_store
                    .as_ref()
                    .and_then(|store| {
                        u32::try_from(effect.effect_misc_value_1)
                            .ok()
                            .and_then(|form_id| store.get(form_id))
                    })
                {
                    let (is_submerged, is_in_water) = crate::session::hub_ref(self)
                        .represented_player_mount_liquid_state_like_cpp()?;
                    let riding_skill = crate::session::hub_ref(self)
                        .resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?;
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
                            account = self.core.account_id,
                            spell_id = spell_info.spell_id,
                            form_id = effect.effect_misc_value_1,
                            mount_type_id = form.mount_type_id,
                            riding_skill,
                            map_id = self.core.player_map_id_like_cpp(),
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

            let (_, is_in_water) =
                crate::session::hub_ref(self).represented_player_mount_liquid_state_like_cpp()?;
            if is_in_water
                && spell_info.has_aura_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
                )
            {
                info!(
                    account = self.core.account_id,
                    spell_id = spell_info.spell_id,
                    "Rejecting represented flying mount cast while in water"
                );
                return Some(RepresentedMountSpellCheckOutcomeLikeCpp::CastFailed(
                    SpellCastResult::OnlyAbovewater,
                ));
            }

            if let Some(disallowed_source_id) = represented_disallowed_mount_form_like_cpp(self)? {
                self.send_packet(&MountResult {
                    result: MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP,
                });
                info!(
                    account = self.core.account_id,
                    spell_id = spell_info.spell_id,
                    disallowed_source_id,
                    "Rejecting represented mount cast: player is in a disallowed shapeshift form"
                );
                return Some(RepresentedMountSpellCheckOutcomeLikeCpp::DontReport);
            }

            let mut mount_type_id = u16::try_from(effect.effect_misc_value_2).unwrap_or_default();
            if let Some(mount_entry) = self.catalogs.mount_store.as_ref().and_then(|store| {
                u32::try_from(spell_info.spell_id)
                    .ok()
                    .and_then(|spell_id| store.get_by_source_spell_id_like_cpp(spell_id))
            }) {
                mount_type_id = mount_entry.mount_type_id;
            }

            if mount_type_id != 0 {
                let current_area_id = crate::session::hub_ref(self).player_zone_area_like_cpp()?.1;
                let (is_submerged, is_in_water) = crate::session::hub_ref(self)
                    .represented_player_mount_liquid_state_like_cpp()?;
                let riding_skill = crate::session::hub_ref(self)
                    .resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?;
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
                        account = self.core.account_id,
                        spell_id = spell_info.spell_id,
                        mount_type_id,
                        riding_skill,
                        map_id = self.core.player_map_id_like_cpp(),
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
                    account = self.core.account_id,
                    spell_id = spell_info.spell_id,
                    mount_type_id,
                    riding_skill,
                    map_id = self.core.player_map_id_like_cpp(),
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
