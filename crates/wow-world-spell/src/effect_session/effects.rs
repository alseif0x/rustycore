// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Remaining represented effect operations (#1363 slice 9c), from the World
//! `session/spell_effects/effects.rs`.

use super::{
    SpellEffectHostLikeCpp, add_honor_xp_to_current_player_like_cpp,
    apply_damage_with_generator_like_cpp, canonical_threatened_by_me_owner_guids_like_cpp,
    record_spell_execute_log_extra_attacks_like_cpp,
    scale_canonical_owner_threat_to_target_zero_like_cpp,
    stop_represented_player_pve_combat_like_cpp,
};
use std::sync::Arc;
use wow_constants::UnitState;
use wow_core::ObjectGuid;
use wow_core::Position;
use wow_entities::SpellCastMetadata;
use wow_packet::packets::spell::SpellTargetData;
use wow_world_core::entity_update_bridge::player_values_update_to_update_object;

const SPELL_SHAPESHIFT_FORM_FLAG_STANCE_LIKE_CPP: i32 = 0x0000_0001;
const CREATURE_MODEL_DATA_FLAG_CAN_MOUNT_WHILE_TRANSFORMED_AS_THIS_LIKE_CPP: u32 = 0x0000_0080;
const CHR_RACES_FLAG_CAN_MOUNT_LIKE_CPP: i32 = 0x0000_0004;

pub fn apply_play_movie_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    target_guid: ObjectGuid,
    movie_id: i32,
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if target_guid != player_guid {
        return;
    }

    let Ok(movie_id) = u32::try_from(movie_id) else {
        return;
    };
    let Some(movie_store) = host.hub_ref().catalogs.movie_store.as_ref() else {
        return;
    };
    if movie_store.get(movie_id).is_none() {
        return;
    }

    if host
        .hub_mut()
        .with_player_cinematic_state_like_cpp(|state| state.set_movie_like_cpp(Some(movie_id)))
        .is_none()
    {
        return;
    }
    host.send_packet(&wow_packet::packets::misc::TriggerMovie { movie_id });
}

/// C++ `Spell::EffectUpgradeHeirloom`.
pub fn apply_upgrade_heirloom_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    metadata: SpellCastMetadata,
) {
    let Ok(item_id) = u32::try_from(metadata.misc[0]) else {
        return;
    };
    let Some(cast_item_entry) = metadata.cast_item_entry else {
        return;
    };
    let Some(update) = host.upgrade_account_heirloom_like_cpp(item_id, cast_item_entry as i32)
    else {
        return;
    };
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if let Some(packet) = player_values_update_to_update_object(
        player_guid,
        host.core().player_map_id_like_cpp(),
        &update,
    ) {
        host.send_packet(&packet);
    }
}

pub fn represented_disallowed_mount_form_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &H,
) -> Option<Option<u32>> {
    if represented_transform_spell_allows_mount_like_cpp(host)? {
        return Some(None);
    }
    let visible_auras = host.hub_ref().resolved_player_visible_auras_like_cpp()?;

    if let (Some(spell_store), Some(shapeshift_form_store)) = (
        host.hub_ref().catalogs.spell_store(),
        host.hub_ref()
            .catalogs
            .spell_catalogs
            .spell_shapeshift_form_store
            .as_ref(),
    ) {
        for aura in visible_auras.values() {
            let Some(spell_info) = spell_store.get(aura.spell_id) else {
                continue;
            };

            for effect in spell_info
                .effects()
                .iter()
                .filter(|effect| effect.is_mod_shapeshift_aura_like_cpp())
            {
                let Ok(form_id) = u32::try_from(effect.effect_misc_value_1) else {
                    continue;
                };
                let Some(form) = shapeshift_form_store.get(form_id) else {
                    return Some(Some(form_id));
                };
                if form.flags & SPELL_SHAPESHIFT_FORM_FLAG_STANCE_LIKE_CPP == 0 {
                    return Some(Some(form_id));
                }
            }
        }
    }

    let (display_id, native_display_id) = host.core().canonical_player_display_ids_like_cpp()?;
    if display_id == 0 || display_id == native_display_id {
        return Some(None);
    }

    let Some(display_store) = host
        .hub_ref()
        .catalogs
        .creatures
        .display_info_store
        .as_ref()
    else {
        return Some(None);
    };
    let Some(display_extra_store) = host
        .hub_ref()
        .catalogs
        .creatures
        .display_info_extra_store
        .as_ref()
    else {
        return Some(None);
    };
    let disallowed = disallowed_mount_form_from_catalogs(
        0,
        display_id,
        native_display_id,
        None,
        Some(display_store),
        Some(display_extra_store),
        host.hub_ref()
            .catalogs
            .creatures
            .model_data_store
            .as_deref(),
        host.hub_ref().catalogs.chr.races_store.as_deref(),
    )
    .unwrap_or(false);
    Some(disallowed.then_some(display_id))
}

/// C++ `Unit::IsDisallowedMountForm` for the selected DB2 form/display
/// inputs. `None` means a required catalog was not available to resolve the
/// decision; an absent row in an available catalog follows C++ lookup rules.
pub fn disallowed_mount_form_from_catalogs(
    form_id: u32,
    display_id: u32,
    native_display_id: u32,
    form_store: Option<&wow_data::SpellShapeshiftFormStore>,
    display_store: Option<&wow_data::CreatureDisplayInfoStore>,
    display_extra_store: Option<&wow_data::CreatureDisplayInfoExtraStore>,
    model_store: Option<&wow_data::CreatureModelDataStore>,
    races_store: Option<&wow_data::character_progression::ChrRacesStore>,
) -> Option<bool> {
    if form_id != 0 {
        let form = form_store?.get(form_id);
        if form.is_none_or(|form| form.flags & SPELL_SHAPESHIFT_FORM_FLAG_STANCE_LIKE_CPP == 0) {
            return Some(true);
        }
    }
    if display_id == native_display_id {
        return Some(false);
    }
    let display = display_store?.get(display_id);
    let Some(display) = display else {
        return Some(true);
    };
    let Ok(display_extra_id) = u32::try_from(display.extended_display_info_id) else {
        return Some(true);
    };
    let display_extra = display_extra_store?.get(display_extra_id);
    let Some(display_extra) = display_extra else {
        return Some(true);
    };
    let model_store = model_store?;
    let races_store = races_store?;
    let model_cannot_mount = model_store
        .get(u32::from(display.model_id))
        .is_some_and(|model| {
            model.flags & CREATURE_MODEL_DATA_FLAG_CAN_MOUNT_WHILE_TRANSFORMED_AS_THIS_LIKE_CPP == 0
        });
    let race_cannot_mount = u32::try_from(display_extra.display_race_id)
        .ok()
        .and_then(|race_id| races_store.get(race_id))
        .is_some_and(|race| race.flags & CHR_RACES_FLAG_CAN_MOUNT_LIKE_CPP == 0);
    Some(model_cannot_mount && race_cannot_mount)
}

fn represented_transform_spell_allows_mount_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &H,
) -> Option<bool> {
    // C++ `Unit::IsInDisallowedMountForm` reads
    // `IsDisallowedMountForm(GetTransformSpell(), ...)`
    // (`Unit.cpp:8813-8820`), so the canonical `m_transformSpell` owner is
    // the single authority instead of a visible-aura scan.
    let transform_spell = host
        .hub_ref()
        .player_aura_subsystem_snapshot_like_cpp()?
        .transform_spell_like_cpp();
    if transform_spell == 0 {
        return Some(false);
    }
    let spell_store = host.hub_ref().catalogs.spell_store()?;
    Some(spell_store.has_attribute0_like_cpp(
        transform_spell,
        wow_data::spell::attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED,
    ))
}

pub fn represented_object_target_position_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &H,
    target_data: &SpellTargetData,
) -> Option<Position> {
    let target_guid = target_data.unit;
    if target_guid.is_empty() {
        return None;
    }
    let player_map_key = host.core().current_canonical_player_map_key_like_cpp()?;
    let manager = host.core().canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
    if let Some(player) = map.map().get_typed_player(target_guid) {
        return Some(player.unit().world().position());
    }
    if let Some(creature) = map
        .map()
        .creature_transform_vitals_snapshot_like_cpp(target_guid)
    {
        return Some(creature.position);
    }
    if let Some(gameobject) = map.map().get_typed_game_object(target_guid) {
        return Some(gameobject.world().position());
    }
    map.map()
        .get_typed_dynamic_object(target_guid)
        .map(|dynamic_object| dynamic_object.world().position())
}

pub fn implicit_target_conditions_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &H,
    spell_id: i32,
    effect_index: u32,
) -> Option<Arc<wow_data::ConditionContainer>> {
    host.hub_ref()
        .catalogs
        .spell_catalogs
        .spell_store
        .as_deref()
        .and_then(|store| store.implicit_target_conditions_like_cpp(spell_id, effect_index))
        .and_then(|conditions| conditions.upgrade())
}

pub fn has_implicit_target_conditions_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &H,
    spell_id: i32,
    effect_index: u32,
) -> bool {
    implicit_target_conditions_like_cpp(host, spell_id, effect_index)
        .is_some_and(|conditions| !conditions.is_empty())
}

/// C++ `Spell::EffectAddExtraAttacks`.
///
/// Represented boundary: current canonical player target only. This stores
/// C++ `Unit::AddExtraAttacks` state on the target unit; consuming the
/// queued extra swings during melee update and combat-log emission remains
/// outside this bounded slice.
pub fn apply_add_extra_attacks_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    effect: u32,
    damage: i32,
    target_guid: ObjectGuid,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    if target_guid != player_guid
        || host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true)
    {
        return false;
    }
    let count = damage as u32;
    let queued = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .add_extra_attacks_like_cpp(count)
                .is_some()
        })
        .unwrap_or(false);
    if queued {
        // C++ `ExecuteLogEffectExtraAttacks` (`Spell.cpp:5088-5095`).
        record_spell_execute_log_extra_attacks_like_cpp(
            host,
            i32::try_from(effect).unwrap_or(0),
            target_guid,
            count,
        );
    }
    queued
}

/// C++ `Spell::EffectInebriate`.
///
/// Represented boundary: current canonical player target only. The owning
/// application computes the C++ clamped delta and applies the complete native
/// `SetDrunkValue` transition. Its boolean reports only whether the drunken
/// threshold changed, not whether the stored value changed. Drunken Vomit
/// (`67468`) remains unimplemented.
pub fn apply_inebriate_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    damage: i32,
    target_guid: ObjectGuid,
    cast_item_guid: Option<ObjectGuid>,
    cast_item_entry: Option<u32>,
) -> Result<bool, &'static str> {
    let Some(player_guid) = host.player_guid() else {
        return Ok(false);
    };
    if target_guid != player_guid {
        return Ok(false);
    }

    host.set_player_drunk_value_like_cpp(target_guid, damage, cast_item_guid, cast_item_entry)
}

/// C++ `Spell::EffectDismissPet`.
///
/// Represented boundary: current represented pet target only. Full C++
/// `Unit::IsPet` / `Pet::Remove(PET_SAVE_NOT_IN_SLOT)` requires the live
/// pet runtime; this slice preserves the hit-target no-op semantics and
/// clears the represented active pet state when the target is that pet.
pub fn apply_dismiss_pet_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    target_guid: ObjectGuid,
) -> bool {
    if host.hub_ref().player_pet_guid_state_like_cpp().flatten() != Some(target_guid) {
        return false;
    }

    host.hub_mut()
        .invalidate_represented_character_pet_empty_authority_like_cpp();
    let _ = host.hub_mut().set_player_pet_guid_like_cpp(None);
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.pets.represented_pet_react_state_like_cpp =
            wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP;
        fixtures.pets.represented_pet_command_state_like_cpp =
            wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP;
    }
    true
}

pub fn apply_dual_wield_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    target_guid: ObjectGuid,
) -> Result<(), &'static str> {
    let player_guid = host.player_guid().ok_or("No player GUID")?;
    if target_guid != player_guid {
        return Ok(());
    }

    let _ = host.core().mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
    });

    Ok(())
}

pub fn apply_titan_grip_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    penalty_spell_id: i32,
) -> Result<(), &'static str> {
    let _ = host.player_guid().ok_or("No player GUID")?;

    let penalty_spell_id = u32::try_from(penalty_spell_id).unwrap_or(0);
    let _ = host.core().mutate_canonical_player_like_cpp(|player| {
        player.set_can_titan_grip(true, penalty_spell_id);
    });

    Ok(())
}

pub fn apply_parry_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
) -> Result<(), &'static str> {
    let _ = host.player_guid().ok_or("No player GUID")?;

    let _ = host.core().mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_parry_like_cpp(true);
    });

    Ok(())
}

pub fn apply_block_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
) -> Result<(), &'static str> {
    let _ = host.player_guid().ok_or("No player GUID")?;

    let _ = host.core().mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_block_like_cpp(true);
    });

    Ok(())
}

pub fn apply_give_honor_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    damage: i32,
    target_guid: ObjectGuid,
) -> Result<(), &'static str> {
    let player_guid = host.player_guid().ok_or("No player GUID")?;
    if target_guid != player_guid {
        return Ok(());
    }

    let honor_update = add_honor_xp_to_current_player_like_cpp(host, damage);

    // C++ `Spell::EffectGiveHonor` sets only `Honor` and `OriginalHonor`
    // before `SendDirectMessage(packet.Write())`; `Target` and `Rank`
    // remain default-initialized.
    host.send_packet(&wow_packet::packets::combat::PvpCredit {
        original_honor: damage,
        honor: damage,
        target: ObjectGuid::EMPTY,
        rank: 0,
    });
    if let Some(update) = honor_update
        && let Some(packet) = player_values_update_to_update_object(
            player_guid,
            host.core().player_map_id_like_cpp(),
            &update,
        )
    {
        host.send_packet(&packet);
    }
    Ok(())
}

/// C++ `Spell::EffectModifyCooldown`.
///
/// Represented boundary: the current Rust runtime exposes `SpellHistory`
/// through the canonical player seam only. Other unit targets remain a
/// no-op until generic unit spell histories are live.
pub fn apply_modify_cooldown_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    damage: i32,
    effect_trigger_spell: i32,
    target_guid: ObjectGuid,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    if target_guid != player_guid {
        return false;
    }
    let Ok(trigger_spell_id) = u32::try_from(effect_trigger_spell) else {
        return false;
    };
    if trigger_spell_id == 0 {
        return false;
    }

    let now_ms = u64::from(wow_world_core::session::game_time_ms_like_cpp());
    host.core()
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .spells
                .history
                .modify_cooldown(trigger_spell_id, i64::from(damage), false, now_ms)
        })
        .unwrap_or(false)
}

pub fn current_map_is_dungeon_like_cpp<H: SpellEffectHostLikeCpp>(host: &H) -> bool {
    current_map_dungeon_state_like_cpp(host).unwrap_or(false)
}

/// Resolve the current map's C++ `MapEntry::IsDungeon` classification
/// without conflating missing Map.db2 metadata with an overworld map.
pub fn current_map_dungeon_state_like_cpp<H: SpellEffectHostLikeCpp>(host: &H) -> Option<bool> {
    host.hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()
        .and_then(|store| store.get(u32::from(host.core().player_map_id_like_cpp())))
        .map(|entry| entry.is_dungeon())
}

/// C++ `Spell::EffectDistract` / `SPELL_EFFECT_DISTRACT`.
pub fn apply_distract_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    damage: i32,
    target_guid: ObjectGuid,
    target_data: &SpellTargetData,
) -> Result<(), &'static str> {
    use wow_packet::ServerPacket;
    use wow_packet::packets::movement::{MonsterMove, MovementMonsterSpline};

    let Some(destination) = target_data.dst_location.map(|location| location.position) else {
        return Ok(());
    };
    let duration_ms = u32::try_from(damage.max(0))
        .unwrap_or(0)
        .saturating_mul(1_000);

    let packet = host
        .core_mut()
        .mutate_world_creature(target_guid, |creature| {
            if creature.ai_ownership().combat_target.is_some() {
                return None;
            }
            if creature.unit().has_unit_state(
                (UnitState::CONFUSED | UnitState::STUNNED | UnitState::FLEEING).bits(),
            ) {
                return None;
            }

            let orientation = creature.position().angle_to(&destination);
            let (_, from, spline) =
                wow_world_core::creature_movement::CreatureMovementLikeCpp::canonical_like_cpp(
                    creature,
                )
                .begin_distract_movement_like_cpp(duration_ms, orientation)?;
            Some(
                MonsterMove {
                    mover_guid: target_guid,
                    current_pos: from,
                    spline: MovementMonsterSpline::from_move_spline(&spline),
                }
                .to_bytes(),
            )
        })
        .flatten();

    if let Some(packet) = packet {
        let _ = host.core().send_tx().send(packet);
    }

    Ok(())
}

/// C++ `Spell::EffectSanctuary`.
pub fn apply_sanctuary_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    target_guid: ObjectGuid,
) -> Result<(), &'static str> {
    if target_guid.is_empty() {
        return Ok(());
    }

    if target_guid.is_player() && !current_map_is_dungeon_like_cpp(host) {
        stop_represented_player_pve_combat_like_cpp(host, target_guid);
        return Ok(());
    }

    let owner_guids = canonical_threatened_by_me_owner_guids_like_cpp(host, target_guid);
    for owner_guid in owner_guids {
        scale_canonical_owner_threat_to_target_zero_like_cpp(host, owner_guid, target_guid);
        let _ = host.core_mut().mutate_world_creature(owner_guid, |owner| {
            owner
                .unit_mut()
                .subsystems_mut()
                .combat
                .scale_threat(target_guid, 0.0);
        });
    }
    Ok(())
}

pub async fn apply_instakill_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    spell_id: i32,
    target_guid: ObjectGuid,
) -> Result<(), &'static str> {
    let caster = host.player_guid().ok_or("No player GUID")?;
    let Some(current_hp) = host
        .core_mut()
        .mutate_world_creature(target_guid, |creature| {
            creature.is_alive().then_some(creature.current_hp())
        })
        .flatten()
    else {
        return Ok(());
    };

    host.send_packet(&wow_packet::packets::combat::SpellInstakillLog {
        target: target_guid,
        caster,
        spell_id,
    });
    apply_damage_with_generator_like_cpp(
        host,
        item_guid_generator,
        Some(spell_id),
        target_guid,
        current_hp,
    )
    .await
}
