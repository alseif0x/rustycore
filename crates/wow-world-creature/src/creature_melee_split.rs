//! Secondary-target side of creature white-swing split damage.

use wow_core::ObjectGuid;
use wow_packet::ServerPacket;
use wow_world_core::entity_update_bridge::unit_values_update_to_update_object;
use wow_world_core::runtime_plan::{RecipientRule, RuntimeEvent};

#[derive(Default)]
pub(super) struct MeleeSplitDamageOutcomeLikeCpp {
    pub absorbed: u32,
    pub damage: u32,
    pub mutation_events: Vec<RuntimeEvent>,
    pub combat_log_packets: Vec<Vec<u8>>,
}

#[derive(Clone, Copy)]
pub(super) enum SplitAuraIdentityLikeCpp {
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
struct SplitAuraFactsLikeCpp {
    spell_id: i32,
    caster_guid: ObjectGuid,
    school_mask: u32,
    amount: i32,
    cast_id: ObjectGuid,
    visual_id: i32,
}

pub(super) struct MeleeSplitDamageEntryOutcomeLikeCpp {
    pub damage_after: u32,
    pub absorbed: u32,
    pub mutation_events: Vec<RuntimeEvent>,
    pub combat_log_source_guid: ObjectGuid,
    pub combat_log_packet: Vec<u8>,
    pub pending_death: Option<super::PendingMapPlayerDeath>,
}

/// Copy only effect identities, as `CopyAuraEffectList` does. The current
/// application and amount are resolved again when each entry is reached.
#[allow(clippy::too_many_arguments)]
pub(super) fn split_aura_identities_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    victim_guid: ObjectGuid,
    spell_store: &wow_data::SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Vec<SplitAuraIdentityLikeCpp> {
    let Some(managed) = manager.find_map(u32::from(map_id), instance_id) else {
        return Vec::new();
    };
    if victim_guid.is_player() {
        let Some(victim) = managed.map().get_typed_player(victim_guid) else {
            return Vec::new();
        };
        let auras = victim
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp();
        let mut slots = auras.keys().copied().collect::<Vec<_>>();
        slots.sort_unstable();
        return slots
            .into_iter()
            .flat_map(|slot| {
                let aura = &auras[&slot];
                spell_store
                    .get(aura.spell_id)
                    .into_iter()
                    .flat_map(move |spell| {
                        spell.effects().iter().filter_map(move |effect| {
                            let bit = 1_u32.checked_shl(effect.effect_index)?;
                            (effect.effect_aura
                                == wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT
                                && aura.effect_mask & bit != 0)
                                .then_some(SplitAuraIdentityLikeCpp::Player {
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
                                    == wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT
                                    && applied.effect_mask & bit != 0)
                                    .then_some(SplitAuraIdentityLikeCpp::Creature {
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

fn current_split_aura_facts_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    victim_guid: ObjectGuid,
    identity: SplitAuraIdentityLikeCpp,
    spell_store: &wow_data::SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Option<SplitAuraFactsLikeCpp> {
    let managed = manager.find_map(u32::from(map_id), instance_id)?;
    match identity {
        SplitAuraIdentityLikeCpp::Player {
            slot,
            spell_id,
            caster_guid,
            effect_index,
        } => {
            let victim = managed.map().get_typed_player(victim_guid)?;
            let auras = &victim.unit().subsystems().auras;
            let aura = auras.runtime_applications_like_cpp().get(&slot)?;
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
                        == wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT
            })?;
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|row| u8::try_from(effect_index).ok() == Some(row.effect_index))
                .map(|row| row.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            let provenance = auras.aura_cast_provenance_like_cpp(slot);
            Some(SplitAuraFactsLikeCpp {
                spell_id,
                caster_guid,
                school_mask: effect.effect_misc_value_1 as u32,
                amount,
                cast_id: provenance.cast_id,
                visual_id: provenance.spell_visual_id,
            })
        }
        SplitAuraIdentityLikeCpp::Creature {
            applied,
            effect_index,
        } => managed
            .map()
            .with_creature_like_cpp(victim_guid, |victim| {
                let auras = &victim.unit().subsystems().auras;
                if !auras.applied_auras.contains(&applied)
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
                                == wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT
                    })?;
                let provenance = auras.aura_cast_provenance_like_cpp(applied.slot);
                Some(SplitAuraFactsLikeCpp {
                    spell_id,
                    caster_guid: applied.caster_guid,
                    school_mask: effect.effect_misc_value_1 as u32,
                    amount: effect.calc_value_no_caster_like_cpp(),
                    cast_id: provenance.cast_id,
                    visual_id: provenance.spell_visual_id,
                })
            })
            .flatten(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_melee_split_entry_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    identity: SplitAuraIdentityLikeCpp,
    current_damage: u32,
    school_mask: u32,
    attacker_is_player_controlled: bool,
    spell_store: &wow_data::SpellStore,
    spell_misc_store: Option<&wow_data::SpellMiscStore>,
    spell_threat_store: Option<&wow_data::SpellThreatStoreLikeCpp>,
    spell_chain_store: Option<&wow_data::SpellChainStoreLikeCpp>,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    native_death: bool,
) -> Result<Option<MeleeSplitDamageEntryOutcomeLikeCpp>, &'static str> {
    if current_damage == 0 {
        return Ok(None);
    }
    let Some(aura) = current_split_aura_facts_like_cpp(
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
    if aura.school_mask & school_mask == 0 || aura.caster_guid == victim_guid {
        return Ok(None);
    }
    // `CalculatePct` followed by `RoundToInterval(0, damage)`.
    let split_damage = (i64::from(current_damage) * i64::from(aura.amount) / 100)
        .clamp(0, i64::from(current_damage)) as u32;
    if split_damage == 0 {
        return Ok(None);
    }
    let Some(secondary) = apply_secondary_split_damage_like_cpp(
        manager,
        map_id,
        instance_id,
        attacker_guid,
        victim_guid,
        aura.caster_guid,
        aura.spell_id,
        aura.cast_id,
        aura.visual_id,
        split_damage,
        school_mask,
        attacker_is_player_controlled,
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
    let mut damage_after = current_damage - split_damage;
    if secondary.reached_damage_delivery
        && !victim_guid.is_player()
        && manager
            .find_map(u32::from(map_id), instance_id)
            .and_then(|managed| {
                managed.map().with_creature_like_cpp(victim_guid, |victim| {
                    victim.should_fake_damage_from_like_cpp(true, attacker_is_player_controlled)
                })
            })
            .unwrap_or(false)
    {
        // C++ `Unit.cpp:2000-2003` changes only the primary wire damage.
        damage_after = 0;
    }
    Ok(Some(MeleeSplitDamageEntryOutcomeLikeCpp {
        damage_after,
        absorbed: split_damage,
        mutation_events: secondary.mutation_events,
        combat_log_source_guid: if secondary.reached_damage_delivery {
            aura.caster_guid
        } else {
            // Unit.cpp:1968-1991 emits the immunity SpellMissLog from the
            // primary victim; Unit.cpp:2010-2011 sends the damage log from
            // the secondary caster.
            victim_guid
        },
        combat_log_packet: secondary.combat_log_packet,
        pending_death: secondary.pending_death,
    }))
}

/// Synchronous compatibility/testing API. Production drives the same entry
/// helper in its admitted async future so a lethal Player can await native death.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_melee_split_damage_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    damage: u32,
    school_mask: u32,
    attacker_is_player_controlled: bool,
    spell_store: &wow_data::SpellStore,
    spell_misc_store: Option<&wow_data::SpellMiscStore>,
    spell_threat_store: Option<&wow_data::SpellThreatStoreLikeCpp>,
    spell_chain_store: Option<&wow_data::SpellChainStoreLikeCpp>,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> MeleeSplitDamageOutcomeLikeCpp {
    let identities = split_aura_identities_like_cpp(
        manager,
        map_id,
        instance_id,
        victim_guid,
        spell_store,
        difficulty_id,
        difficulty_store,
    );
    let mut result = MeleeSplitDamageOutcomeLikeCpp {
        damage,
        ..Default::default()
    };
    if damage == 0 || attacker_guid == victim_guid {
        return result;
    }
    for identity in identities {
        let Some(entry) = apply_melee_split_entry_like_cpp(
            manager,
            map_id,
            instance_id,
            attacker_guid,
            victim_guid,
            identity,
            result.damage,
            school_mask,
            attacker_is_player_controlled,
            spell_store,
            spell_misc_store,
            spell_threat_store,
            spell_chain_store,
            difficulty_id,
            difficulty_store,
            false,
        )
        .expect("synchronous split target application cannot admit native death") else {
            continue;
        };
        result.damage = entry.damage_after;
        result.absorbed = result.absorbed.saturating_add(entry.absorbed);
        result.mutation_events.extend(entry.mutation_events);
        result.combat_log_packets.push(entry.combat_log_packet);
    }
    result
}

pub(super) struct SecondarySplitDamageOutcomeLikeCpp {
    pub mutation_events: Vec<RuntimeEvent>,
    pub combat_log_packet: Vec<u8>,
    pub reached_damage_delivery: bool,
    pub pending_death: Option<super::PendingMapPlayerDeath>,
}

/// Validate and apply one already-sized split to its aura caster. `None`
/// means C++ would skip the aura before `DamageInfo::AbsorbDamage` because the
/// caster is missing, self, out of world or dead. Immunity is a successful
/// split: primary damage is absorbed and the returned packet is a miss log.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_secondary_split_damage_like_cpp(
    canonical_manager: &mut wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    primary_victim_guid: ObjectGuid,
    caster_guid: ObjectGuid,
    spell_id: i32,
    aura_cast_id: ObjectGuid,
    aura_spell_visual_id: i32,
    split_damage: u32,
    school_mask: u32,
    attacker_is_player_controlled: bool,
    spell_store: &wow_data::SpellStore,
    spell_misc_store: Option<&wow_data::SpellMiscStore>,
    spell_threat_store: Option<&wow_data::SpellThreatStoreLikeCpp>,
    spell_chain_store: Option<&wow_data::SpellChainStoreLikeCpp>,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    defer_lethal_player: bool,
) -> Option<Result<SecondarySplitDamageOutcomeLikeCpp, &'static str>> {
    if caster_guid == primary_victim_guid {
        return None;
    }
    let managed = canonical_manager.find_map(u32::from(map_id), instance_id)?;
    let (caster_effects, deal_damage_mods_absorb) = if caster_guid.is_player() {
        managed
            .map()
            .get_typed_player(caster_guid)
            .and_then(|caster| {
                (caster.unit().world().object().is_in_world() && caster.unit().is_alive()).then(
                    || {
                        (
                            wow_world_spell::aura_effects::player_aura_effects_all_like_cpp(
                                caster
                                    .unit()
                                    .subsystems()
                                    .auras
                                    .runtime_applications_like_cpp(),
                                spell_store,
                            )
                            .into_iter()
                            .map(|effect| effect.as_applied_like_cpp())
                            .collect::<Vec<_>>(),
                            caster.unit().unit_state()
                                & wow_constants::unit::UnitState::IN_FLIGHT.bits()
                                != 0,
                        )
                    },
                )
            })
    } else {
        managed
            .map()
            .with_creature_like_cpp(caster_guid, |caster| {
                (caster.unit().world().object().is_in_world() && caster.is_alive()).then(|| {
                    (
                        wow_world_spell::aura_effects::creature_aura_effects_like_cpp(
                            &caster.unit().subsystems().auras.applied_auras,
                            spell_store,
                            difficulty_id,
                            difficulty_store,
                        ),
                        caster.unit().unit_state()
                            & wow_constants::unit::UnitState::IN_FLIGHT.bits()
                            != 0
                            || caster.is_evading_attacks_like_cpp(),
                    )
                })
            })
            .flatten()
    }?;

    if caster_effects.iter().any(|effect| {
        matches!(
            effect.aura_type,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY
                | wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY
        ) && (effect.misc_value as u32) & school_mask != 0
    }) {
        return Some(Ok(SecondarySplitDamageOutcomeLikeCpp {
            mutation_events: Vec::new(),
            combat_log_packet: wow_packet::packets::combat::SpellMissLog {
                spell_id,
                caster: primary_victim_guid,
                entries: vec![wow_packet::packets::combat::SpellMissLogEntry {
                    victim: caster_guid,
                    miss_reason: 7,
                }],
            }
            .to_bytes(),
            reached_damage_delivery: false,
            pending_death: None,
        }));
    }

    // C++ `DealDamageMods` runs after the primary `DamageInfo` has absorbed
    // the full split. An in-flight or evading secondary target therefore
    // keeps the primary absorption, takes no health damage and reports the
    // split as absorbed in its non-melee log.
    let secondary_damage = if deal_damage_mods_absorb {
        0
    } else {
        split_damage
    };
    let secondary_absorbed = split_damage - secondary_damage;
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
    let pre_hit_health;
    let pending_death = if caster_guid.is_player() && secondary_damage > 0 && defer_lethal_player {
        let Some(health) = canonical_manager
            .find_map(u32::from(map_id), instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .get_typed_player(caster_guid)
                    .map(|caster| caster.unit().data().health)
            })
        else {
            return None;
        };
        if u64::from(secondary_damage) >= health {
            match super::native_player_death::admit_map_player_death(
                canonical_manager,
                wow_map::MapKey::new(u32::from(map_id), instance_id),
                caster_guid,
                attacker_guid,
                false,
            ) {
                Ok(pending) => Some(pending),
                Err(error) => return Some(Err(error)),
            }
        } else {
            None
        }
    } else {
        None
    };
    if caster_guid.is_player() {
        let caster = canonical_manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_player_mut(caster_guid)?;
        pre_hit_health = caster.unit().data().health;
        if secondary_damage > 0 && pending_death.is_none() {
            let health_after = pre_hit_health.saturating_sub(u64::from(secondary_damage));
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
        }
    } else {
        let caster = canonical_manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_creature_mut(caster_guid)?;
        pre_hit_health = caster.unit().data().health;
        let applied_damage = caster.calculate_damage_for_sparring_like_cpp(
            true,
            attacker_is_player_controlled,
            secondary_damage,
        );
        let applied_damage = caster
            .damage_after_unkillable_gate_like_cpp(attacker_guid == caster_guid, applied_damage);
        if secondary_damage > 0 {
            let killed = caster.apply_ai_damage_before_death_state_at_game_time_like_cpp(
                applied_damage,
                u64::from(wow_world_core::session::game_time_ms_like_cpp()),
                wow_entities::game_time_secs_like_cpp(),
            );
            if killed {
                caster
                    .set_kill_death_state_runtime_like_cpp(wow_entities::game_time_secs_like_cpp());
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
            if let Some(update) = unit_values_update_to_update_object(
                caster_guid,
                map_id,
                &caster.unit().values_update(),
            ) {
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
    }

    Some(Ok(SecondarySplitDamageOutcomeLikeCpp {
        mutation_events,
        combat_log_packet: wow_packet::packets::combat::SpellNonMeleeDamageLog {
            target: caster_guid,
            caster: attacker_guid,
            cast_id: aura_cast_id,
            spell_id,
            visual_id: aura_spell_visual_id,
            damage: secondary_damage.min(i32::MAX as u32) as i32,
            original_damage: secondary_damage.min(i32::MAX as u32) as i32,
            overkill: if u64::from(secondary_damage) > pre_hit_health {
                u64::from(secondary_damage)
                    .saturating_sub(pre_hit_health)
                    .min(i32::MAX as u64) as i32
            } else {
                -1
            },
            school_mask: school_mask.min(u32::from(u8::MAX)) as u8,
            absorbed: secondary_absorbed.min(i32::MAX as u32) as i32,
            resisted: 0,
            shield_block: 0,
            periodic: false,
            flags: 0,
        }
        .to_bytes(),
        reached_damage_delivery: true,
        pending_death,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use wow_world_test_support::canonical_creatures::shared_canonical_map_manager;
    use wow_world_test_support::map_objects::{
        add_canonical_test_player_on_map, test_creature_guid,
    };

    const SPELL_ID: i32 = 96_481;
    const SLOT: u8 = 12;

    fn split_spell_store() -> wow_data::SpellStore {
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
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT,
                    effect_base_points: 49,
                    effect_misc_value_1: 1,
                    ..Default::default()
                }],
            },
        );
        store
    }

    fn install_split_aura(player: &mut wow_entities::Player, caster_guid: ObjectGuid, amount: i32) {
        let snapshot = wow_entities::AuraThreatSnapshotLikeCpp::new(
            [0; 2],
            vec![(
                1,
                wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT,
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
    fn split_entry_rechecks_current_amount_and_preserves_nonlethal_damage_like_cpp() {
        let canonical = shared_canonical_map_manager();
        let primary = ObjectGuid::create_player(1, 96_482);
        let caster = ObjectGuid::create_player(1, 96_483);
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
            install_split_aura(primary_player, caster, 50);
            let caster_player = map.get_typed_player_mut(caster).unwrap();
            caster_player.unit_mut().set_max_health(100);
            caster_player.unit_mut().set_health(100);
        }
        let spell_store = split_spell_store();
        let identity = {
            let manager = canonical.lock().unwrap();
            split_aura_identities_like_cpp(&manager, 0, 0, primary, &spell_store, 0, None)
                .into_iter()
                .next()
                .expect("one copied split effect identity")
        };
        {
            let mut manager = canonical.lock().unwrap();
            manager
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .get_typed_player_mut(primary)
                .unwrap()
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(SLOT)
                .unwrap()
                .represented_effect_amounts[0]
                .amount = 20;
            let entry = apply_melee_split_entry_like_cpp(
                &mut manager,
                0,
                0,
                test_creature_guid(96_484),
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
                false,
            )
            .unwrap()
            .expect("the still-applied split effect receives its current amount");
            assert_eq!(entry.absorbed, 20);
            assert_eq!(entry.damage_after, 80);
            assert_eq!(entry.combat_log_source_guid, caster);
        }
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
            80,
            "the secondary receives the current nonlethal split amount"
        );
    }
}
