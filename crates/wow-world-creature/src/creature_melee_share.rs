//! Secondary-target side of creature white-swing shared damage.

use wow_core::ObjectGuid;
use wow_packet::ServerPacket;
use wow_world_core::entity_update_bridge::unit_values_update_to_update_object;
use wow_world_core::runtime_plan::{RecipientRule, RuntimeEvent};

#[derive(Clone, Copy)]
pub(super) enum ShareAuraIdentityLikeCpp {
    Player {
        slot: u8,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect_index: u32,
    },
    Creature {
        applied: wow_entities::AppliedAuraRef,
        effect_index: u32,
    },
}

#[derive(Clone, Copy)]
struct ShareAuraFactsLikeCpp {
    caster_guid: ObjectGuid,
    school_mask: u32,
    amount: i32,
    spell_id: i32,
}

#[derive(Default)]
pub(super) struct MeleeShareDamageOutcomeLikeCpp {
    pub mutation_events: Vec<RuntimeEvent>,
    pub primary_was_share_target: bool,
    pub primary_player_share_health_updates: Vec<u64>,
}

pub(super) struct MeleeShareDamageEntryOutcomeLikeCpp {
    pub mutation_events: Vec<RuntimeEvent>,
    pub player_health_after: Option<u64>,
    pub pending_death: Option<super::PendingMapPlayerDeath>,
    pub primary_target: bool,
}

/// Copy only current application/effect identities. C++ rereads each copied
/// effect's current amount and application after callbacks during iteration.
#[allow(clippy::too_many_arguments)]
pub(super) fn share_aura_identities_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    victim_guid: ObjectGuid,
    damage_done: u32,
    spell_store: &wow_data::SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Vec<ShareAuraIdentityLikeCpp> {
    if damage_done == 0 {
        return Vec::new();
    }
    let Some(managed) = manager.find_map(u32::from(map_id), instance_id) else {
        return Vec::new();
    };
    if victim_guid.is_player() {
        let Some(victim) = managed.map().get_typed_player(victim_guid) else {
            return Vec::new();
        };
        let runtime = victim
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp();
        let mut slots = runtime.keys().copied().collect::<Vec<_>>();
        slots.sort_unstable();
        return slots
            .into_iter()
            .flat_map(|slot| {
                let aura = &runtime[&slot];
                spell_store
                    .get(aura.spell_id)
                    .into_iter()
                    .flat_map(move |spell| {
                        spell.effects().iter().filter_map(move |effect| {
                            let bit = 1_u32.checked_shl(effect.effect_index)?;
                            (effect.effect_aura
                                == wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT
                                && aura.effect_mask & bit != 0)
                                .then_some(ShareAuraIdentityLikeCpp::Player {
                                    slot,
                                    spell_id: aura.spell_id,
                                    caster_guid: aura.caster_guid,
                                    effect_index: effect.effect_index,
                                })
                        })
                    })
            })
            .collect();
    }
    managed
        .map()
        .with_creature_like_cpp(victim_guid, |victim| {
            victim
                .unit()
                .subsystems()
                .auras
                .applied_auras
                .iter()
                .flat_map(|applied| {
                    let spell_id = i32::try_from(applied.spell_id).unwrap_or(0);
                    spell_store
                        .effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
                        .into_iter()
                        .flat_map(move |effects| {
                            effects.iter().filter_map(move |effect| {
                                let bit = 1_u32.checked_shl(effect.effect_index)?;
                                (effect.effect_aura
                                    == wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT
                                    && applied.effect_mask & bit != 0)
                                    .then_some(ShareAuraIdentityLikeCpp::Creature {
                                        applied: *applied,
                                        effect_index: effect.effect_index,
                                    })
                            })
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn current_share_aura_facts_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    victim_guid: ObjectGuid,
    identity: ShareAuraIdentityLikeCpp,
    spell_store: &wow_data::SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Option<ShareAuraFactsLikeCpp> {
    let managed = manager.find_map(u32::from(map_id), instance_id)?;
    match identity {
        ShareAuraIdentityLikeCpp::Player {
            slot,
            spell_id,
            caster_guid,
            effect_index,
        } => {
            let victim = managed.map().get_typed_player(victim_guid)?;
            let aura = victim
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .get(&slot)?;
            if aura.spell_id != spell_id
                || aura.caster_guid != caster_guid
                || !1_u32
                    .checked_shl(effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
            {
                return None;
            }
            let spell = spell_store.get(spell_id)?;
            let effect = spell.effects().iter().find(|effect| {
                effect.effect_index == effect_index
                    && effect.effect_aura
                        == wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT
            })?;
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|row| u8::try_from(effect_index).ok() == Some(row.effect_index))
                .map(|row| row.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            Some(ShareAuraFactsLikeCpp {
                caster_guid,
                school_mask: effect.effect_misc_value_1 as u32,
                amount,
                spell_id,
            })
        }
        ShareAuraIdentityLikeCpp::Creature {
            applied,
            effect_index,
        } => managed
            .map()
            .with_creature_like_cpp(victim_guid, |victim| {
                if !victim
                    .unit()
                    .subsystems()
                    .auras
                    .applied_auras
                    .contains(&applied)
                    || !1_u32
                        .checked_shl(effect_index)
                        .is_some_and(|bit| applied.effect_mask & bit != 0)
                {
                    return None;
                }
                let spell_id = i32::try_from(applied.spell_id).unwrap_or(0);
                let effect = spell_store
                    .effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)?
                    .iter()
                    .find(|effect| {
                        effect.effect_index == effect_index
                            && effect.effect_aura
                                == wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT
                    })?;
                Some(ShareAuraFactsLikeCpp {
                    caster_guid: applied.caster_guid,
                    school_mask: effect.effect_misc_value_1 as u32,
                    amount: effect.calc_value_no_caster_like_cpp(),
                    spell_id,
                })
            })
            .flatten(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_melee_share_entry_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    identity: ShareAuraIdentityLikeCpp,
    damage_done: u32,
    damage_school_mask: u32,
    attacker_is_player_controlled: bool,
    spell_store: &wow_data::SpellStore,
    spell_misc_store: Option<&wow_data::SpellMiscStore>,
    spell_threat_store: Option<&wow_data::SpellThreatStoreLikeCpp>,
    spell_chain_store: Option<&wow_data::SpellChainStoreLikeCpp>,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    native_death: bool,
) -> Result<Option<MeleeShareDamageEntryOutcomeLikeCpp>, &'static str> {
    let Some(aura) = current_share_aura_facts_like_cpp(
        manager,
        map_id,
        instance_id,
        victim_guid,
        identity,
        spell_store,
        difficulty_id,
        difficulty_store,
    ) else {
        return Ok(None);
    };
    if aura.school_mask & damage_school_mask == 0 || aura.amount <= 0 {
        return Ok(None);
    }
    // C++ `CalculatePct(uint32, int32)` multiplies in float and truncates.
    let share_damage = (damage_done as f32 * aura.amount as f32 / 100.0) as u32;
    if share_damage == 0 {
        return Ok(None);
    }
    let Some(secondary) = apply_secondary_share_damage_like_cpp(
        manager,
        map_id,
        instance_id,
        attacker_guid,
        aura.caster_guid,
        share_damage,
        attacker_is_player_controlled,
        aura.spell_id,
        spell_store,
        spell_misc_store,
        spell_threat_store,
        spell_chain_store,
        difficulty_id,
        difficulty_store,
        native_death,
    )
    .transpose()?
    else {
        return Ok(None);
    };
    Ok(Some(MeleeShareDamageEntryOutcomeLikeCpp {
        mutation_events: secondary.mutation_events,
        player_health_after: secondary.player_health_after,
        pending_death: secondary.pending_death,
        primary_target: aura.caster_guid == victim_guid,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_melee_share_damage_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    damage_done: u32,
    damage_school_mask: u32,
    attacker_is_player_controlled: bool,
    spell_store: &wow_data::SpellStore,
    spell_misc_store: Option<&wow_data::SpellMiscStore>,
    spell_threat_store: Option<&wow_data::SpellThreatStoreLikeCpp>,
    spell_chain_store: Option<&wow_data::SpellChainStoreLikeCpp>,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> MeleeShareDamageOutcomeLikeCpp {
    let identities = share_aura_identities_like_cpp(
        manager,
        map_id,
        instance_id,
        victim_guid,
        damage_done,
        spell_store,
        difficulty_id,
        difficulty_store,
    );
    let mut result = MeleeShareDamageOutcomeLikeCpp::default();
    for identity in identities {
        let Some(entry) = apply_melee_share_entry_like_cpp(
            manager,
            map_id,
            instance_id,
            attacker_guid,
            victim_guid,
            identity,
            damage_done,
            damage_school_mask,
            attacker_is_player_controlled,
            spell_store,
            spell_misc_store,
            spell_threat_store,
            spell_chain_store,
            difficulty_id,
            difficulty_store,
            false,
        )
        .expect("synchronous share target application cannot admit native death") else {
            continue;
        };
        result.primary_was_share_target |= entry.primary_target;
        if entry.primary_target && victim_guid.is_player() {
            if let Some(health) = entry.player_health_after {
                result.primary_player_share_health_updates.push(health);
            }
        } else {
            result.mutation_events.extend(entry.mutation_events);
        }
    }
    result
}

struct SecondaryShareDamageOutcomeLikeCpp {
    mutation_events: Vec<RuntimeEvent>,
    player_health_after: Option<u64>,
    pending_death: Option<super::PendingMapPlayerDeath>,
}

/// Apply the already-sized `NODAMAGE` recursive call after its represented
/// `DealDamageMods` target gates. Damage immunity is deliberately absent: the
/// target 3.4.3 share loop does not call `IsImmunedToDamage` here.
#[allow(clippy::too_many_arguments)]
fn apply_secondary_share_damage_like_cpp(
    canonical_manager: &mut wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    caster_guid: ObjectGuid,
    share_damage: u32,
    attacker_is_player_controlled: bool,
    spell_id: i32,
    spell_store: &wow_data::SpellStore,
    spell_misc_store: Option<&wow_data::SpellMiscStore>,
    spell_threat_store: Option<&wow_data::SpellThreatStoreLikeCpp>,
    spell_chain_store: Option<&wow_data::SpellChainStoreLikeCpp>,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    defer_lethal_player: bool,
) -> Option<Result<SecondaryShareDamageOutcomeLikeCpp, &'static str>> {
    let managed = canonical_manager.find_map(u32::from(map_id), instance_id)?;
    let rejected_by_damage_mods = if caster_guid.is_player() {
        managed.map().get_typed_player(caster_guid).map(|caster| {
            !caster.unit().world().object().is_in_world()
                || !caster.unit().is_alive()
                || caster.unit().unit_state() & wow_constants::unit::UnitState::IN_FLIGHT.bits()
                    != 0
        })
    } else {
        managed.map().with_creature_like_cpp(caster_guid, |caster| {
            !caster.unit().world().object().is_in_world()
                || !caster.is_alive()
                || caster.unit().unit_state() & wow_constants::unit::UnitState::IN_FLIGHT.bits()
                    != 0
                || caster.is_evading_attacks_like_cpp()
        })
    }?;
    if rejected_by_damage_mods {
        return Some(Ok(SecondaryShareDamageOutcomeLikeCpp {
            mutation_events: Vec::new(),
            player_health_after: None,
            pending_death: None,
        }));
    }

    let threat_plan = super::creature_melee_threat::plan_creature_damage_threat_like_cpp(
        managed.map(),
        attacker_guid,
        Some(spell_id),
        Some(spell_store),
        spell_misc_store,
        spell_threat_store,
        spell_chain_store,
        difficulty_id,
        difficulty_store,
    );
    let mut mutation_events = Vec::new();
    let mut player_health_after = None;
    let mut pending_death = None;
    if caster_guid.is_player() {
        let health_before = canonical_manager
            .find_map(u32::from(map_id), instance_id)?
            .map()
            .get_typed_player(caster_guid)?
            .unit()
            .data()
            .health;
        if defer_lethal_player && share_damage > 0 && u64::from(share_damage) >= health_before {
            match super::native_player_death::admit_map_player_death(
                canonical_manager,
                wow_map::MapKey::new(u32::from(map_id), instance_id),
                caster_guid,
                attacker_guid,
                false,
            ) {
                Ok(pending) => pending_death = Some(pending),
                Err(error) => return Some(Err(error)),
            }
        }
        if pending_death.is_some() {
            return Some(Ok(SecondaryShareDamageOutcomeLikeCpp {
                mutation_events,
                player_health_after: None,
                pending_death,
            }));
        }
        let caster = canonical_manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_player_mut(caster_guid)?;
        let health_before = caster.unit().data().health;
        let health_after = health_before.saturating_sub(u64::from(share_damage));
        caster.unit_mut().set_health(health_after);
        if health_after == 0 {
            caster
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
            caster.unit_mut().set_health(0);
        }
        mutation_events.push(RuntimeEvent {
            source_guid: caster_guid,
            recipients: RecipientRule::ExplicitPlayer(caster_guid),
            packet_bytes: wow_packet::packets::combat::HealthUpdate {
                guid: caster_guid,
                health: health_after.min(i64::MAX as u64) as i64,
            }
            .to_bytes(),
        });
        player_health_after = Some(health_after);
    } else {
        let caster = canonical_manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_creature_mut(caster_guid)?;
        let applied_damage = caster.calculate_damage_for_sparring_like_cpp(
            true,
            attacker_is_player_controlled,
            share_damage,
        );
        let applied_damage = caster
            .damage_after_unkillable_gate_like_cpp(attacker_guid == caster_guid, applied_damage);
        let killed = caster.apply_ai_damage_before_death_state_at_game_time_like_cpp(
            applied_damage,
            u64::from(wow_world_core::session::game_time_ms_like_cpp()),
            wow_entities::game_time_secs_like_cpp(),
        );
        if killed {
            caster.set_kill_death_state_runtime_like_cpp(wow_entities::game_time_secs_like_cpp());
            caster.unit_mut().set_health(0);
        }
        if !killed {
            let _ = super::creature_melee_threat::apply_creature_damage_threat_on_map_like_cpp(
                canonical_manager
                    .find_map_mut(u32::from(map_id), instance_id)?
                    .map_mut(),
                caster_guid,
                attacker_guid,
                applied_damage,
                threat_plan,
            );
        }
        let caster = canonical_manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_creature_mut(caster_guid)?;
        if let Some(update) =
            unit_values_update_to_update_object(caster_guid, map_id, &caster.unit().values_update())
        {
            mutation_events.push(RuntimeEvent {
                source_guid: caster_guid,
                recipients: RecipientRule::MapBroadcastVisible {
                    map_id,
                    instance_id,
                },
                packet_bytes: update.to_bytes(),
            });
        }
    }

    Some(Ok(SecondaryShareDamageOutcomeLikeCpp {
        mutation_events,
        player_health_after,
        pending_death,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use wow_world_test_support::canonical_creatures::shared_canonical_map_manager;
    use wow_world_test_support::map_objects::add_canonical_test_player_on_map;

    const SPELL_ID: i32 = 96_491;
    const SLOT: u8 = 13;

    fn share_spell_store() -> wow_data::SpellStore {
        let mut store = wow_data::SpellStore::new();
        store.insert(
            SPELL_ID,
            wow_data::SpellInfo {
                spell_id: SPELL_ID,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 49,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
                    effect_base_points: 49,
                    effect_misc_value_1: 1,
                    ..Default::default()
                }],
            },
        );
        store
    }

    fn install_share_aura(player: &mut wow_entities::Player, caster_guid: ObjectGuid, amount: i32) {
        let snapshot = wow_entities::AuraThreatSnapshotLikeCpp::new(
            [0; 2],
            vec![(
                1,
                wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
                amount,
                1,
            )],
        );
        player.apply_player_threat_aura_like_cpp(
            SPELL_ID as u32,
            caster_guid,
            SLOT,
            snapshot.clone(),
        );
        player.install_player_threat_aura_like_cpp(
            SLOT,
            snapshot,
            wow_entities::AuraApplicationLikeCpp {
                spell_id: SPELL_ID,
                difficulty_id: 0,
                caster_guid,
                slot: SLOT,
                duration_total: 60_000,
                duration_remaining: 60_000,
                stack_count: 1,
                aura_flags: 1,
                effect_mask: 1,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: amount,
                represented_effect_amounts: vec![
                    wow_entities::RepresentedAuraEffectAmountLikeCpp {
                        effect_index: 0,
                        amount,
                    },
                ],
                represented_misc_value: Some(1),
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            },
        );
    }

    #[test]
    fn nonlethal_share_still_applies_to_its_caster_like_cpp() {
        let canonical = shared_canonical_map_manager();
        let primary = ObjectGuid::create_player(1, 96_492);
        let caster = ObjectGuid::create_player(1, 96_493);
        add_canonical_test_player_on_map(
            &canonical,
            primary,
            wow_core::Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
        );
        add_canonical_test_player_on_map(
            &canonical,
            caster,
            wow_core::Position::new(11.0, 10.0, 0.0, 0.0),
            0,
            0,
        );
        {
            let mut manager = canonical.lock().unwrap();
            let map = manager.find_map_mut(0, 0).unwrap().map_mut();
            let primary_player = map.get_typed_player_mut(primary).unwrap();
            primary_player.unit_mut().set_max_health(1_000);
            primary_player.unit_mut().set_health(1_000);
            install_share_aura(primary_player, caster, 50);
            let caster_player = map.get_typed_player_mut(caster).unwrap();
            caster_player.unit_mut().set_max_health(100);
            caster_player.unit_mut().set_health(100);
        }
        let spell_store = share_spell_store();
        let result = {
            let mut manager = canonical.lock().unwrap();
            apply_melee_share_damage_like_cpp(
                &mut manager,
                0,
                0,
                ObjectGuid::create_world_object(
                    wow_core::guid::HighGuid::Creature,
                    0,
                    1,
                    0,
                    0,
                    1,
                    96_494,
                ),
                primary,
                100,
                1,
                false,
                &spell_store,
                None,
                None,
                None,
                0,
                None,
            )
        };
        assert!(!result.primary_was_share_target);
        assert_eq!(result.mutation_events.len(), 1);
        let manager = canonical.lock().unwrap();
        assert_eq!(
            manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_player(caster)
                .unwrap()
                .unit()
                .data()
                .health,
            50
        );
    }

    #[test]
    fn lethal_self_share_admits_native_death_before_health_zero_like_cpp() {
        let canonical = shared_canonical_map_manager();
        let primary = ObjectGuid::create_player(1, 96_495);
        add_canonical_test_player_on_map(
            &canonical,
            primary,
            wow_core::Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
        );
        {
            let mut manager = canonical.lock().unwrap();
            manager
                .adopt_active_player_like_cpp(primary)
                .expect("native-death fixture adopts the active Player owner");
            let player = manager
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .get_typed_player_mut(primary)
                .unwrap();
            player.unit_mut().set_max_health(1_000);
            player.unit_mut().set_health(1);
            install_share_aura(player, primary, 100);
        }
        let spell_store = share_spell_store();
        let (entry, health, alive) = {
            let mut manager = canonical.lock().unwrap();
            let identity =
                share_aura_identities_like_cpp(&manager, 0, 0, primary, 100, &spell_store, 0, None)
                    .into_iter()
                    .next()
                    .unwrap();
            let entry = apply_melee_share_entry_like_cpp(
                &mut manager,
                0,
                0,
                ObjectGuid::create_world_object(
                    wow_core::guid::HighGuid::Creature,
                    0,
                    1,
                    0,
                    0,
                    1,
                    96_496,
                ),
                primary,
                identity,
                100,
                1,
                false,
                &spell_store,
                None,
                None,
                None,
                0,
                None,
                true,
            )
            .unwrap()
            .unwrap();
            let player = manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_player(primary)
                .unwrap();
            (entry, player.unit().data().health, player.unit().is_alive())
        };
        assert_eq!(health, 1);
        assert!(alive);
        assert!(entry.pending_death.is_some());
    }
}
