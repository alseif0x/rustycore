// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared represented preparation checks, repeated before launch for late
//! failure (#1363 slice 9e), from the World `session/player_cast/checks.rs`
//! and `session/spell_effects/checks.rs`: C++ `Spell::CheckCast` for the
//! spell focus, the battle-pet effects and the represented mount spells.
//! None is an already-published rejection; Some(None) is an admitted cast
//! requiring no focus object. This does not expand the retained effect engine.

use tracing::{debug, info};
use wow_constants::SpellCastResult;
use wow_core::ObjectGuid;
use wow_data::skill::SKILL_RIDING_LIKE_CPP;
use wow_entities::{RepresentedAuraEffectLikeCpp, SpellCastMetadata};
use wow_packet::packets::misc::{MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP, MountResult};
use wow_world_entities::spell_click_values::spell_effect_is_represented_summon_object_slot_like_cpp;
use wow_world_inventory::MAX_BATTLE_PET_LEVEL_LIKE_CPP;

use super::SpellExecutionHostLikeCpp;
use crate::RepresentedSpellFocusObjectLikeCpp;
use crate::effect_session::represented_disallowed_mount_form_like_cpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepresentedMountSpellCheckOutcomeLikeCpp {
    CastFailed(SpellCastResult),
    DontReport,
}

pub fn check_represented_cast_preparation_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    spell_info: &wow_data::SpellInfo,
    cast_id: ObjectGuid,
    spell_visual: &wow_packet::packets::spell::SpellCastVisual,
    metadata: SpellCastMetadata,
) -> Result<Option<Option<RepresentedSpellFocusObjectLikeCpp>>, &'static str> {
    let spell_id = spell_info.spell_id;
    let has_represented_gameobject_summon_effect = spell_info.effects().iter().any(|effect| {
        effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD
            || spell_effect_is_represented_summon_object_slot_like_cpp(effect.effect)
    });
    let represented_spell_focus_aura_satisfies_check_cast =
        if spell_info.requires_spell_focus_like_cpp() {
            let (state, hub) = host.split_spell_state_ref();
            state
                .resolved_has_represented_aura_effect_with_misc_value_like_cpp(
                    hub,
                    RepresentedAuraEffectLikeCpp::ProvideSpellFocus,
                    i32::try_from(spell_info.requires_spell_focus).unwrap_or(i32::MAX),
                )
                .ok_or("Canonical Player aura owner unavailable")?
        } else {
            false
        };
    let represented_focus_object = if spell_info.requires_spell_focus_like_cpp()
        && has_represented_gameobject_summon_effect
        && !represented_spell_focus_aura_satisfies_check_cast
    {
        let (state, hub) = host.split_spell_state_ref();
        state.search_spell_focus_like_cpp(hub, spell_info.requires_spell_focus)
    } else {
        None
    };
    if spell_info.requires_spell_focus_like_cpp()
        && has_represented_gameobject_summon_effect
        && !represented_spell_focus_aura_satisfies_check_cast
        && represented_focus_object.is_none()
    {
        // C++ `Spell::CheckCast` fails before `SMSG_SPELL_GO` when no
        // matching `SPELL_AURA_PROVIDE_SPELL_FOCUS` aura or focus object
        // exists. `Spell::SendCastResult` carries the SpellFocusObject id
        // in FailedArg1 for this failure reason.
        host.send_packet(&wow_packet::packets::spell::CastFailed {
            cast_id,
            spell_id,
            visual: spell_visual.clone(),
            reason: SpellCastResult::RequiresSpellFocus as i32,
            fail_arg1: i32::try_from(spell_info.requires_spell_focus).unwrap_or(i32::MAX),
            fail_arg2: 0,
        });
        debug!(
            account = host.core().account_id,
            spell_id = spell_id,
            requires_spell_focus = spell_info.requires_spell_focus,
            "Failing live GameObject summon because C++ SearchSpellFocus found no represented focusObject"
        );
        return Ok(None);
    }

    if let Some(reason) = check_represented_battle_pet_spell_like_cpp(host, &spell_info, metadata) {
        host.send_packet(&wow_packet::packets::spell::CastFailed {
            cast_id,
            spell_id,
            visual: spell_visual.clone(),
            reason: reason as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
        debug!(
            account = host.core().account_id,
            spell_id = spell_id,
            reason = reason as i32,
            "Failing represented battle-pet spell because C++ Spell::CheckCast rejected it"
        );
        return Ok(None);
    }

    if let Some(outcome) = check_represented_mount_spell_like_cpp(host, &spell_info) {
        match outcome {
            RepresentedMountSpellCheckOutcomeLikeCpp::CastFailed(reason) => {
                host.send_packet(&wow_packet::packets::spell::CastFailed {
                    cast_id,
                    spell_id,
                    visual: spell_visual.clone(),
                    reason: reason as i32,
                    fail_arg1: 0,
                    fail_arg2: 0,
                });
                debug!(
                    account = host.core().account_id,
                    spell_id = spell_id,
                    reason = reason as i32,
                    "Failing represented mount spell because C++ Spell::CheckCast rejected it"
                );
            }
            RepresentedMountSpellCheckOutcomeLikeCpp::DontReport => {
                debug!(
                    account = host.core().account_id,
                    spell_id = spell_id,
                    "Failing represented mount spell with C++ SPELL_FAILED_DONT_REPORT"
                );
            }
        }
        return Ok(None);
    }

    Ok(Some(represented_focus_object))
}

fn check_represented_battle_pet_spell_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &H,
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

        if !host.has_represented_battle_pet_journal_lock_like_cpp() {
            return Some(SpellCastResult::CantDoThatRightNow);
        }

        let summoned_guid = host
            .hub_ref()
            .represented_summoned_battle_pet_guid_like_cpp();
        if summoned_guid.is_none() || companion_guid.is_empty() {
            return Some(SpellCastResult::NoPet);
        }

        if summoned_guid != Some(companion_guid) {
            return Some(SpellCastResult::BadTargets);
        }

        let Some(pet) = host.represented_battle_pet_like_cpp(companion_guid) else {
            continue;
        };

        if let Some(species) =
            wow_world_inventory::battle_pet_species_entry_like_cpp(host, pet.species)
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

            if species.has_flag_like_cpp(wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP) {
                return Some(SpellCastResult::BadTargets);
            }
        }
    }

    None
}

/// C++ `SpellInfo::CheckLocation` aura limitation plus
/// `Spell::CheckCast` per-effect water gate for represented mount spells.
fn check_represented_mount_spell_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_info: &wow_data::SpellInfo,
) -> Option<RepresentedMountSpellCheckOutcomeLikeCpp> {
    for effect in spell_info.effects() {
        if effect.is_mod_shapeshift_aura_like_cpp() {
            if let Some(form) = host
                .hub_ref()
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
                let (is_submerged, is_in_water) = host
                    .hub_ref()
                    .represented_player_mount_liquid_state_like_cpp()?;
                let riding_skill = host
                    .hub_ref()
                    .resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?;
                if form.mount_type_id != 0
                    && let Err(reject_reason) =
                        crate::represented_mount_capability_selection_for_type_like_cpp(
                            host,
                            form.mount_type_id,
                            u32::from(riding_skill),
                            None,
                            is_submerged,
                            is_in_water,
                        )
                {
                    info!(
                        account = host.core().account_id,
                        spell_id = spell_info.spell_id,
                        form_id = effect.effect_misc_value_1,
                        mount_type_id = form.mount_type_id,
                        riding_skill,
                        map_id = host.core().player_map_id_like_cpp(),
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

        let (_, is_in_water) = host
            .hub_ref()
            .represented_player_mount_liquid_state_like_cpp()?;
        if is_in_water
            && spell_info.has_aura_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
            )
        {
            info!(
                account = host.core().account_id,
                spell_id = spell_info.spell_id,
                "Rejecting represented flying mount cast while in water"
            );
            return Some(RepresentedMountSpellCheckOutcomeLikeCpp::CastFailed(
                SpellCastResult::OnlyAbovewater,
            ));
        }

        if let Some(disallowed_source_id) = represented_disallowed_mount_form_like_cpp(host)? {
            host.send_packet(&MountResult {
                result: MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP,
            });
            info!(
                account = host.core().account_id,
                spell_id = spell_info.spell_id,
                disallowed_source_id,
                "Rejecting represented mount cast: player is in a disallowed shapeshift form"
            );
            return Some(RepresentedMountSpellCheckOutcomeLikeCpp::DontReport);
        }

        let mut mount_type_id = u16::try_from(effect.effect_misc_value_2).unwrap_or_default();
        if let Some(mount_entry) = host
            .hub_ref()
            .catalogs
            .mount_store
            .as_ref()
            .and_then(|store| {
                u32::try_from(spell_info.spell_id)
                    .ok()
                    .and_then(|spell_id| store.get_by_source_spell_id_like_cpp(spell_id))
            })
        {
            mount_type_id = mount_entry.mount_type_id;
        }

        if mount_type_id != 0 {
            let current_area_id = host.hub_ref().player_zone_area_like_cpp()?.1;
            let (is_submerged, is_in_water) = host
                .hub_ref()
                .represented_player_mount_liquid_state_like_cpp()?;
            let riding_skill = host
                .hub_ref()
                .resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?;
            if let Err(reject_reason) =
                crate::represented_mount_capability_selection_for_type_like_cpp(
                    host,
                    mount_type_id,
                    u32::from(riding_skill),
                    None,
                    is_submerged,
                    is_in_water,
                )
            {
                info!(
                    account = host.core().account_id,
                    spell_id = spell_info.spell_id,
                    mount_type_id,
                    riding_skill,
                    map_id = host.core().player_map_id_like_cpp(),
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
                account = host.core().account_id,
                spell_id = spell_info.spell_id,
                mount_type_id,
                riding_skill,
                map_id = host.core().player_map_id_like_cpp(),
                area_id = current_area_id,
                is_submerged,
                is_in_water,
                "Represented mount cast passed C++ mount capability check"
            );
        }
    }

    None
}
