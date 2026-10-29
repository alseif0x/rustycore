// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player::setDeathState(JUST_DIED), driven by its admitted native operation.

use rand::Rng;
use wow_constants::{DeathState, PowerType};
use wow_entities::{PlayerDeathOperationContext, PlayerDeathOperationId};
use wow_world_entities::{
    CombatSessionHostLikeCpp, CorpseHostLikeCpp, ResurrectionOperationCatalogs,
};
use wow_world_inventory::ItemSessionHostLikeCpp;
use wow_world_quest::QuestItemStorageHostLikeCpp;
use wow_world_spell::PlayerCastHostLikeCpp;

/// Admit lethal damage while native health is still nonzero, execute its
/// required death continuation, and only then release its save/retirement
/// fence. The caller owns the damage log and action-specific fall penalty.
pub async fn apply_player_damage_with_native_death_like_cpp<H>(
    host: &mut H,
    requested_damage: u32,
    context: PlayerDeathOperationContext,
    creature_spawns: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> Result<wow_world_core::session::PlayerDamageResultLikeCpp, &'static str>
where
    H: PlayerCastHostLikeCpp
        + QuestItemStorageHostLikeCpp
        + ItemSessionHostLikeCpp
        + CorpseHostLikeCpp
        + CombatSessionHostLikeCpp
        + crate::TrainerSessionHostLikeCpp,
{
    use wow_world_core::session::PlayerDamageResultLikeCpp;
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.core().player_handle_like_cpp.is_none() && host.test_fixtures_ref_like_cpp().is_some() {
        let (before, after, max, applied, killed) = host
            .hub_mut()
            .apply_owned_player_damage_like_cpp(requested_damage, DeathState::JustDied)
            .ok_or("Fixture damage owner is unavailable")?;
        if applied != 0
            && !killed
            && let Some(guid) = host.player_guid()
        {
            host.core()
                .send_player_health_update_like_cpp(guid, u64::from(after));
        }
        return Ok(PlayerDamageResultLikeCpp {
            health_before: u64::from(before),
            health_after: u64::from(after),
            max_health: u64::from(max),
            applied_damage: applied,
            killed,
        });
    }
    let handle = host
        .core()
        .player_handle_like_cpp
        .ok_or("Damage Player handle is unavailable")?;
    let key = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .ok_or("Damage Player Map is unavailable")?;
    let manager = host
        .core()
        .canonical_map_manager
        .as_ref()
        .ok_or("Damage Map manager is unavailable")?
        .clone();
    let (mut result, operation, pvp_death) = {
        let mut manager = manager
            .lock()
            .map_err(|_| "Damage Map lock is unavailable")?;
        let map = manager
            .find_map(key.map_id, key.instance_id)
            .ok_or("Damage Map disappeared")?
            .map();
        let attacker = context.attacker_guid();
        let unit = map
            .get_typed_player(attacker)
            .map(|player| player.unit())
            .or_else(|| {
                map.get_typed_creature(attacker)
                    .map(|creature| creature.unit())
            })
            .or_else(|| map.get_typed_pet(attacker).map(|pet| pet.creature().unit()));
        let pvp_death = unit.is_some_and(|unit| {
            let control = &unit.subsystems().control;
            match control
                .charmer_guid
                .or(control.owner_guid)
                .filter(|owner| owner.is_player())
            {
                Some(owner) => map.get_typed_player(owner).is_some(),
                None => map.get_typed_player(attacker).is_some(),
            }
        });
        let admission = manager
            .with_player_mut_like_cpp(handle, |player| {
                if player.has_unresolved_native_death_operation() {
                    return Err("Player already owns unresolved native death");
                }
                let before = player.unit().data().health;
                let max = player.unit().data().max_health;
                let applied = if player.unit().is_alive() {
                    u64::from(requested_damage).min(before) as u32
                } else {
                    0
                };
                let after = before - u64::from(applied);
                let killed = applied != 0 && after == 0;
                let operation = if killed {
                    Some(
                        player
                            .begin_native_death_operation(context)
                            .ok_or("Lethal damage could not admit native death")?,
                    )
                } else {
                    player.unit_mut().set_health(after);
                    None
                };
                Ok((
                    PlayerDamageResultLikeCpp {
                        health_before: before,
                        health_after: after,
                        max_health: max,
                        applied_damage: applied,
                        killed,
                    },
                    operation,
                ))
            })
            .ok_or("Damage Player incarnation is unavailable")??;
        (admission.0, admission.1, pvp_death)
    };
    let Some(operation) = operation else {
        // Unit.cpp:1015–1022,8115–8154: only nonlethal DealDamage reaches
        // ModifyHealth's immediate HealthUpdate. Lethal SetHealth dirties
        // UnitData::Health for the Map's existing update-field publication.
        if result.applied_damage != 0 {
            host.core()
                .send_player_health_update_like_cpp(handle.guid(), result.health_after);
        }
        return Ok(result);
    };
    let catalogs = ResurrectionOperationCatalogs {
        item_guid_generator,
        creature_spawns,
        graveyards: None,
    };
    result.health_after =
        finish_native_player_death_like_cpp(host, operation, pvp_death, catalogs).await?;
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.retire_completed_native_death_operation(operation)
        })
        .filter(|completed| *completed)
        .ok_or("Native damage death acknowledgement failed")?;
    Ok(result)
}

/// Finish the admitted operation through Unit::Kill's post-death penalty.
/// Keep Complete owned until the admitting combat future acknowledges it.
pub async fn finish_native_player_death_like_cpp<H>(
    host: &mut H,
    operation: PlayerDeathOperationId,
    pvp_death: bool,
    catalogs: ResurrectionOperationCatalogs<'_>,
) -> Result<u64, &'static str>
where
    H: PlayerCastHostLikeCpp
        + QuestItemStorageHostLikeCpp
        + ItemSessionHostLikeCpp
        + CorpseHostLikeCpp
        + CombatSessionHostLikeCpp
        + crate::TrainerSessionHostLikeCpp,
{
    let handle = host
        .core()
        .player_handle_like_cpp
        .ok_or("Native death handle is unavailable")?;
    let context = set_player_just_died_like_cpp(host, operation, catalogs).await?;
    if host.core().player_handle_like_cpp != Some(handle) {
        return Err("Damage Player incarnation changed during death");
    }
    let in_battleground = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            player.set_pvp_death_like_cpp(pvp_death);
            player.battleground_instance_id_like_cpp().is_some()
        })
        .ok_or("Damage Player disappeared after native death")?;
    // Unit.cpp:10632–10650: capture controlling-player identity before the
    // callbacks, set PvP death after setDeathState, then apply native wear.
    if (context.durability_loss() && !pvp_death && !in_battleground)
        || (pvp_death
            && host
                .hub_ref()
                .config
                .death_policy_like_cpp
                .durability_loss_in_pvp)
    {
        let percent =
            wow_world_inventory::apply_represented_durability_loss_on_death_like_cpp(host);
        host.core()
            .send_packet_realm(&wow_packet::packets::misc::DurabilityDamageDeath {
                percent: percent as i32,
            });
    }
    let health = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            player
                .complete_native_death_operation(operation)
                .then_some(player.unit().data().health)
        })
        .flatten()
        .ok_or("Native damage death did not complete")?;
    Ok(health)
}

/// C++ `Player::UpdateObjectVisibility(true)` and `UpdateVisibilityForPlayer`
/// for the current canonical Player, with the nearby-session refresh queued
/// before its own forced visibility diff.
pub async fn force_player_visibility_with_catalogs_like_cpp<H>(
    host: &mut H,
    creature_spawns: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
) -> bool
where
    H: PlayerCastHostLikeCpp + wow_world_entities::VisibilitySessionHostLikeCpp + Send,
{
    let Some(handle) = host.core().player_handle_like_cpp else {
        return false;
    };
    if host.player_guid() != Some(handle.guid())
        || !host
            .core()
            .with_owned_player_like_cpp(|player| {
                player.guid() == handle.guid() && player.unit().world().object().is_in_world()
            })
            .unwrap_or(false)
    {
        return false;
    }

    wow_world_spell::notify_other_players_visibility_changed_like_cpp(host);
    wow_world_entities::force_update_visibility_with_catalogs_like_cpp(host, creature_spawns).await;
    true
}

/// Execute the ordered native body without holding a Map guard over a
/// callback. On error the exact operation remains Ready/InFlight and fences
/// save, replacement and retirement. Completion does not imply that the
/// caller's remaining Unit::Kill participants or packet delivery succeeded.
pub async fn set_player_just_died_like_cpp<H>(
    host: &mut H,
    operation: PlayerDeathOperationId,
    catalogs: ResurrectionOperationCatalogs<'_>,
) -> Result<PlayerDeathOperationContext, &'static str>
where
    H: PlayerCastHostLikeCpp
        + QuestItemStorageHostLikeCpp
        + ItemSessionHostLikeCpp
        + CorpseHostLikeCpp
        + CombatSessionHostLikeCpp
        + crate::TrainerSessionHostLikeCpp,
{
    let handle = host
        .core()
        .player_handle_like_cpp
        .ok_or("Native death owner is unavailable")?;
    let validate_owner = |host: &H| {
        if host.core().player_handle_like_cpp != Some(handle)
            || host.player_guid() != Some(handle.guid())
            || !host
                .core()
                .with_owned_player_like_cpp(|player| {
                    player
                        .native_death_operation_snapshot()
                        .is_some_and(|current| current.id() == operation)
                })
                .unwrap_or(false)
        {
            return Err("Native death Player incarnation or operation changed during callback");
        }
        Ok(())
    };
    // These nonempty participants still require their own native cleanup
    // owners. Refuse before changing the admitted Player's native fields.
    host.core()
        .with_owned_player_like_cpp(|player| {
            let subsystems = player.unit().subsystems();
            let pet = player.gameplay_state().pet_guid;
            if subsystems.vehicle.vehicle_guid.is_some()
                || subsystems.vehicle.kit.is_some()
                || subsystems.control.charmed_guid.is_some()
                || subsystems
                    .control
                    .controlled_guids
                    .iter()
                    .any(|guid| Some(*guid) != pet)
                || subsystems
                    .control
                    .summon_slots
                    .iter()
                    .any(|guid| !guid.is_empty() && Some(*guid) != pet)
            {
                return Err(
                    "Native death vehicle, totem or controlled-unit cleanup is unavailable",
                );
            }
            let auras = &subsystems.auras;
            let mut spell_counts = std::collections::BTreeMap::<i32, usize>::new();
            for aura in auras.runtime_applications_like_cpp().values() {
                *spell_counts.entry(aura.spell_id).or_default() += 1;
            }
            for (spell_id, count) in spell_counts {
                if count > 1 {
                    auras.ordered_slots_for_spell_removal(spell_id)?;
                }
            }
            if [328, 396, 468].iter().any(|kind| {
                auras
                    .applied_aura_types
                    .get(kind)
                    .is_some_and(|effects| !effects.is_empty())
            }) {
                return Err("Native death health/power-change Aura callback is unavailable");
            }
            if auras.applied_auras.iter().any(|aura| {
                !auras
                    .runtime_applications_like_cpp()
                    .values()
                    .any(|runtime| {
                        runtime.spell_id as u32 == aura.spell_id
                            && runtime.caster_guid == aura.caster_guid
                            && runtime.slot == aura.slot
                            && runtime.effect_mask == aura.effect_mask
                    })
            }) || auras.owned_auras.iter().any(|aura| {
                !auras
                    .runtime_applications_like_cpp()
                    .values()
                    .any(|runtime| {
                        runtime.spell_id as u32 == aura.spell_id
                            && runtime.caster_guid == aura.caster_guid
                    })
            }) {
                return Err("Native death Aura has no application cleanup owner");
            }
            Ok(())
        })
        .ok_or("Native death Player is unavailable")??;
    let context = host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.claim_native_death_operation(operation))
        .flatten()
        .ok_or("Native death operation cannot be claimed")?;

    // Player.cpp:1168–1185: cancel, sober up, combo, resurrection request,
    // pet removal, SelfRes and live quest failure, in that order.
    {
        let mut fields = host.split_trainer_mut();
        crate::cancel_pending_spell_cast_request_like_cpp(&mut fields.hub, fields.spell_state);
    }
    crate::set_player_drunk_value_like_cpp(
        host,
        handle.guid(),
        crate::PlayerDrunkValueInput::Set(0),
        None,
        None,
    )?;
    clear_power(host, PowerType::ComboPoints)?;
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.clear_resurrection_request_like_cpp();
        })
        .ok_or("Native death Player disappeared before pet removal")?;
    crate::remove_pet_not_in_slot_like_cpp(host, catalogs).await?;
    validate_owner(host)?;
    if host.core().with_owned_player_like_cpp(|player| {
        !player
            .unit()
            .subsystems()
            .control
            .controlled_guids
            .is_empty()
    }) != Some(false)
    {
        return Err("Native death controlled-unit cleanup remains unresolved after pet removal");
    }
    {
        let hub = host.hub_ref();
        let spells = hub
            .catalogs
            .spell_store()
            .ok_or("Native death SpellInfo catalog is unavailable")?;
        let server = hub
            .catalogs
            .spell_catalogs
            .serverside_spell_store
            .as_deref()
            .ok_or("Native death server spell catalog is unavailable")?;
        let difficulty = hub.core.current_map_difficulty_id_like_cpp();
        hub.core
            .with_owned_player_mut_like_cpp(|player| {
                wow_world_spell::initialize_self_resurrection_spells(
                    player,
                    difficulty,
                    spells,
                    server,
                    hub.catalogs
                        .spell_class_options_store()
                        .map(|store| store.as_ref()),
                    hub.catalogs.difficulty_store().map(|store| store.as_ref()),
                    |chance| rand::thread_rng().gen_range(0..100_u32) < chance,
                )
            })
            .ok_or("Native death SelfRes owner is unavailable")?
            .map_err(|_| "Native death SelfRes initialization is unavailable")?;
    }
    // QUEST_FLAGS_COMPLETION_NO_DEATH, QuestDef.h. Application awaits every
    // complete FailQuest before it reads the next current quest-log slot.
    crate::quest_failure::fail_quests_with_flag_like_cpp(host, catalogs, 0x0000_0001).await?;
    validate_owner(host)?;

    // Unit.cpp:8529–8550: mark dead before CombatStop or Aura removal.
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_death_state(DeathState::JustDied)
        })
        .ok_or("Native death Unit owner is unavailable")?;
    finish_player_non_alive_state_like_cpp(host)?;
    let power = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.subsystems_mut().auras.clear_all_reactives_like_cpp();
            unit.subsystems_mut().auras.clear_diminishings();
            if unit.subsystems_mut().motion.stop_on_death() {
                unit.subsystems_mut().motion.finalize_spline();
            }
            // Unit.cpp:8565–8570: remove-health modifiers may have left 1 HP.
            unit.set_health(0);
            wow_entities::represented_power_type_from_u8_like_cpp(unit.data().display_power)
        })
        .ok_or("Native death Unit disappeared before power cleanup")?;
    if let Some(power) = power {
        clear_power(host, power)?;
    }
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.reset_death_presentation_like_cpp();
        })
        .ok_or("Native death Unit disappeared before completion")?;
    wow_world_entities::PlayerMovementHostLikeCpp::remove_auras_with_interrupt_flags_like_cpp(
        host,
        wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP,
        0,
    );
    host.core()
        .send_packet_realm(&wow_packet::packets::misc::StandStateUpdate {
            anim_kit_id: 0,
            stand_state: 0,
        });
    Ok(context)
}

fn clear_power<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    power: PowerType,
) -> Result<(), &'static str> {
    use wow_packet::ServerPacket;
    let publish = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            if unit.get_power_index(power).is_none() {
                return false;
            }
            unit.set_power(power, 0);
            unit.world().object().is_in_world()
        })
        .ok_or("Native death power owner is unavailable")?;
    if publish {
        let packet = wow_packet::packets::combat::PowerUpdate {
            guid: host
                .player_guid()
                .ok_or("Native death power recipient is unavailable")?,
            powers: vec![(0, power as u8)],
        };
        host.core().send_packet_realm(&packet);
        host.broadcast_player_packet_to_visible_set_realm_like_cpp(packet.to_bytes());
    }
    Ok(())
}

/// Unit.cpp:8527–8550 repeats these effects for every nonalive transition.
/// JUST_DIED-only health, power, stand and quest effects remain in their owner.
pub fn finish_player_non_alive_state_like_cpp<H>(host: &mut H) -> Result<(), &'static str>
where
    H: PlayerCastHostLikeCpp + CombatSessionHostLikeCpp,
{
    let handle = host
        .core()
        .player_handle_like_cpp
        .ok_or("Nonalive Player owner is unavailable")?;
    let state = host
        .core()
        .with_owned_player_like_cpp(|player| player.unit().death_state())
        .ok_or("Nonalive Player incarnation is unavailable")?;
    if host.player_guid() != Some(handle.guid())
        || matches!(state, DeathState::Alive | DeathState::JustRespawned)
    {
        return Err("Nonalive Player state is not admitted");
    }
    wow_world_entities::combat_stop_like_cpp(host);
    crate::interrupt_non_melee_spell_cast_for_loot_like_cpp(host);
    if host.core().player_handle_like_cpp != Some(handle)
        || host.player_guid() != Some(handle.guid())
    {
        return Err("Nonalive Player owner changed before controlled-unit cleanup");
    }
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            if player.guid() != handle.guid() || player.unit().death_state() != state {
                return Err("Nonalive Player owner changed before controlled-unit cleanup");
            }
            if !player
                .unit()
                .subsystems()
                .control
                .controlled_guids
                .is_empty()
            {
                return Err("Nonalive Player controlled-unit cleanup remains unresolved");
            }
            // Unit.cpp:6394-6420 RemoveAllControlled clears this even for an empty set.
            player
                .unit_mut()
                .remove_unit_flags_like_cpp(wow_constants::UnitFlags::PET_IN_COMBAT);
            Ok(())
        })
        .ok_or("Nonalive Player disappeared before controlled-unit cleanup")??;
    remove_death_auras(host)?;
    if host.core().player_handle_like_cpp != Some(handle)
        || host.player_guid() != Some(handle.guid())
        || host
            .core()
            .with_owned_player_like_cpp(|player| player.unit().death_state())
            != Some(state)
    {
        return Err("Nonalive Player owner changed during cleanup");
    }
    Ok(())
}

fn remove_death_auras<H: PlayerCastHostLikeCpp>(host: &mut H) -> Result<(), &'static str> {
    let mut removed_bases = Vec::new();
    loop {
        let candidate = {
            let hub = host.hub_ref();
            let spells = hub
                .catalogs
                .spell_store()
                .ok_or("Death Aura SpellInfo catalog is unavailable")?;
            let difficulties = hub.catalogs.difficulty_store().map(|store| store.as_ref());
            hub.core
                .with_owned_player_like_cpp(|player| {
                    let auras = &player.unit().subsystems().auras;
                    let mut applications = auras
                        .runtime_applications_like_cpp()
                        .values()
                        .collect::<Vec<_>>();
                    // m_appliedAuras is a multimap keyed by spell id. A refresh
                    // timestamp is not insertion provenance for equivalent keys.
                    let mut orders = std::collections::BTreeMap::new();
                    for aura in &applications {
                        if applications
                            .iter()
                            .filter(|other| other.spell_id == aura.spell_id)
                            .count()
                            > 1
                        {
                            orders
                                .entry(aura.spell_id)
                                .or_insert(auras.ordered_slots_for_spell_removal(aura.spell_id)?);
                        }
                    }
                    applications.sort_by_key(|aura| {
                        (
                            aura.spell_id,
                            orders
                                .get(&aura.spell_id)
                                .and_then(|slots| slots.iter().position(|slot| *slot == aura.slot))
                                .unwrap_or(0),
                        )
                    });
                    for aura in applications {
                        let key = wow_data::spell::resolve_spell_info_key_difficulty(
                            spells,
                            aura.spell_id as u32,
                            aura.difficulty_id,
                            difficulties,
                        )
                        .map_err(|_| "Death Aura SpellInfo difficulty is unavailable")?
                        .ok_or("Death Aura SpellInfo key is unavailable")?;
                        let attributes = spells
                            .misc_attribute_words_at_difficulty_like_cpp(
                                aura.spell_id,
                                key,
                                difficulties,
                            )
                            .map_err(|_| "Death Aura attributes are unavailable")?;
                        if attributes[0] & wow_data::spell::attributes::SPELL_ATTR0_PASSIVE == 0
                            && attributes[3] & 0x0010_0000 == 0
                        {
                            let applied = auras
                                .applied_auras
                                .iter()
                                .find(|applied| {
                                    applied.slot == aura.slot
                                        && applied.spell_id == aura.spell_id as u32
                                        && applied.caster_guid == aura.caster_guid
                                })
                                .copied();
                            return Ok(Some((
                                aura.slot,
                                wow_entities::AuraRef::new(aura.spell_id as u32, aura.caster_guid),
                                applied,
                            )));
                        }
                    }
                    Ok(None)
                })
                .ok_or("Death Aura Player is unavailable")??
        };
        let Some((slot, base, applied)) = candidate else {
            // Source's owned-Aura loop follows every applied-Aura callback.
            // The selected runtime owner has already run each supported effect
            // removal; retire only those exact Base identities here.
            host.core()
                .with_owned_player_mut_like_cpp(|player| {
                    let auras = &mut player.unit_mut().subsystems_mut().auras;
                    let mut owned = auras
                        .owned_auras
                        .iter()
                        .copied()
                        .filter(|aura| removed_bases.contains(&aura.aura_ref()))
                        .collect::<Vec<_>>();
                    owned.sort_by_key(|aura| aura.spell_id);
                    for aura in owned {
                        auras.remove_owned(aura);
                        auras.set_aura_death_policy_like_cpp(aura.aura_ref(), false, false);
                    }
                })
                .ok_or("Death Aura owner disappeared before owned cleanup")?;
            return Ok(());
        };
        if let Some(applied) = applied {
            // Unit.cpp:3449–3490: unlink native references before effect
            // callbacks. A failed callback retains the native operation fence.
            host.core()
                .with_owned_player_mut_like_cpp(|player| {
                    player
                        .unit_mut()
                        .subsystems_mut()
                        .auras
                        .unapply_aura(applied, 6)
                })
                .filter(|removed| *removed)
                .ok_or("Death Aura applied identity changed")?;
        }
        host.remove_aura(slot)?;
        removed_bases.push(base);
    }
}
