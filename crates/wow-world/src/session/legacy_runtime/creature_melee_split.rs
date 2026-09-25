//! Secondary-target side of creature white-swing split damage.

use crate::entity_update_bridge::unit_values_update_to_update_object;
use crate::map_manager::{RecipientRule, RuntimeEvent};
use crate::session::{CreatureMeleeVictimSyncIdentityLikeCpp, CreatureMeleeVictimSyncStateLikeCpp};
use wow_core::ObjectGuid;
use wow_packet::ServerPacket;

#[derive(Default)]
pub(super) struct MeleeSplitDamageOutcomeLikeCpp {
    pub absorbed: u32,
    pub damage: u32,
    pub mutation_events: Vec<RuntimeEvent>,
    pub combat_log_packets: Vec<Vec<u8>>,
    pub creature_syncs: Vec<(ObjectGuid, CreatureMeleeVictimSyncStateLikeCpp)>,
}

/// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_SPLIT_DAMAGE_PCT` tail
/// (`Unit.cpp:1958-2015`) for a represented creature white swing.
/// Active effects are snapshotted before secondary mutation; each valid caster
/// takes a percentage of the current remainder, while canonical Map health
/// remains the sole Player/Creature authority.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_melee_split_damage_like_cpp(
    canonical_manager: &mut wow_map::MapManager,
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
    let mut result = MeleeSplitDamageOutcomeLikeCpp {
        damage,
        ..Default::default()
    };
    if damage == 0 || attacker_guid == victim_guid {
        return result;
    }

    let Some(managed) = canonical_manager.find_map(u32::from(map_id), instance_id) else {
        return result;
    };
    let split_auras = if victim_guid.is_player() {
        managed
            .map()
            .get_typed_player(victim_guid)
            .map(|victim| {
                let auras = &victim.unit().subsystems().auras;
                crate::session_rules::player_aura_effects_full_by_spell_aura_type_like_cpp(
                    auras.runtime_applications_like_cpp(),
                    spell_store,
                    wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT,
                )
                .into_iter()
                .map(|effect| {
                    let provenance = auras.aura_cast_provenance_like_cpp(effect.slot);
                    (
                        effect.spell_id,
                        effect.caster_guid,
                        effect.misc_value as u32,
                        effect.amount,
                        provenance.cast_id,
                        provenance.spell_visual_id,
                    )
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        managed
            .map()
            .with_creature_like_cpp(victim_guid, |victim| {
                let auras = &victim.unit().subsystems().auras;
                crate::session_rules::creature_aura_effects_like_cpp(
                    &auras.applied_auras,
                    spell_store,
                    difficulty_id,
                    difficulty_store,
                )
                .into_iter()
                .filter(|effect| {
                    effect.aura_type == wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT
                })
                .map(|effect| {
                    let provenance = auras.aura_cast_provenance_like_cpp(effect.slot);
                    (
                        effect.spell_id,
                        effect.caster_guid,
                        effect.misc_value as u32,
                        effect.amount,
                        provenance.cast_id,
                        provenance.spell_visual_id,
                    )
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let primary_should_fake_damage = !victim_guid.is_player()
        && managed
            .map()
            .with_creature_like_cpp(victim_guid, |victim| {
                victim.should_fake_damage_from_like_cpp(true, attacker_is_player_controlled)
            })
            .unwrap_or(false);

    for (spell_id, caster_guid, aura_school_mask, amount, aura_cast_id, aura_spell_visual_id) in
        split_auras
    {
        if result.damage == 0 || aura_school_mask & school_mask == 0 || caster_guid == victim_guid {
            continue;
        }

        // `CalculatePct` followed by `RoundToInterval(0, damage)`. Use a
        // signed wide intermediate so malformed negative or >100 data cannot
        // wrap before C++'s clamp.
        let split_damage = (i64::from(result.damage) * i64::from(amount) / 100)
            .clamp(0, i64::from(result.damage)) as u32;
        if split_damage == 0 {
            continue;
        }
        let Some(secondary) = apply_secondary_split_damage_like_cpp(
            canonical_manager,
            map_id,
            instance_id,
            attacker_guid,
            victim_guid,
            caster_guid,
            spell_id,
            aura_cast_id,
            aura_spell_visual_id,
            split_damage,
            school_mask,
            attacker_is_player_controlled,
            spell_store,
            spell_misc_store,
            spell_threat_store,
            spell_chain_store,
            difficulty_id,
            difficulty_store,
        ) else {
            continue;
        };
        result.damage -= split_damage;
        result.absorbed += split_damage;
        if primary_should_fake_damage && secondary.reached_damage_delivery {
            // C++ performs this primary-victim sparring adjustment between
            // `DealDamageMods` for the secondary and its `DealDamage` call
            // (`Unit.cpp:2000-2003`). It changes the primary wire damage but
            // does not add the discarded remainder to the absorb counter.
            result.damage = 0;
        }
        result.mutation_events.extend(secondary.mutation_events);
        result.combat_log_packets.push(secondary.combat_log_packet);
        if let Some(sync) = secondary.creature_sync {
            result.creature_syncs.push((caster_guid, sync));
        }
    }

    result
}

pub(super) struct SecondarySplitDamageOutcomeLikeCpp {
    pub mutation_events: Vec<RuntimeEvent>,
    pub combat_log_packet: Vec<u8>,
    pub creature_sync: Option<CreatureMeleeVictimSyncStateLikeCpp>,
    pub reached_damage_delivery: bool,
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
) -> Option<SecondarySplitDamageOutcomeLikeCpp> {
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
                            crate::session_rules::player_aura_effects_all_like_cpp(
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
                        crate::session_rules::creature_aura_effects_like_cpp(
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
        return Some(SecondarySplitDamageOutcomeLikeCpp {
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
            creature_sync: None,
            reached_damage_delivery: false,
        });
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
    let mut creature_sync = None;
    if caster_guid.is_player() {
        let caster = canonical_manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_player_mut(caster_guid)?;
        pre_hit_health = caster.unit().data().health;
        if secondary_damage > 0 {
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
        let revision_before = caster.unit().health_state_revision_like_cpp();
        let identity_authority = caster.loot_authority_like_cpp().clone();
        let health_authority = caster.unit().health_state_revision_authority_like_cpp();
        let spawn_id = caster.spawn_id();
        let loot_revision_before = caster.loot_lifecycle_revision_like_cpp();
        let death_state_before = caster.unit().death_state();
        let ai_state_before = caster.ai_ownership().state;
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
                u64::from(crate::session::game_time_ms_like_cpp()),
                wow_entities::game_time_secs_like_cpp(),
            );
            if killed {
                caster.set_death_state_runtime(
                    wow_constants::DeathState::JustDied,
                    wow_entities::game_time_secs_like_cpp(),
                );
                caster.unit_mut().set_health(0);
            }
            let threat = if killed {
                None
            } else {
                super::creature_melee_threat::apply_creature_damage_threat_on_map_like_cpp(
                    canonical_manager
                        .find_map_mut(u32::from(map_id), instance_id)?
                        .map_mut(),
                    caster_guid,
                    attacker_guid,
                    applied_damage,
                    threat_plan,
                )
            };
            let caster = canonical_manager
                .find_map_mut(u32::from(map_id), instance_id)?
                .map_mut()
                .get_typed_creature_mut(caster_guid)?;
            let health_after = caster.unit().data().health;
            let revision_after = caster.unit().health_state_revision_like_cpp();
            creature_sync = Some(CreatureMeleeVictimSyncStateLikeCpp {
                applied_damage,
                threat,
                victim_health_before: pre_hit_health,
                victim_health_after: health_after,
                victim_health_state_revision_before: revision_before,
                victim_health_state_revision_after: revision_after,
                identity: CreatureMeleeVictimSyncIdentityLikeCpp {
                    authority: identity_authority,
                    health_state_revision_authority: health_authority,
                    spawn_id,
                    loot_lifecycle_revision_before: loot_revision_before,
                    loot_lifecycle_revision_after: caster.loot_lifecycle_revision_like_cpp(),
                    death_state_before,
                    death_state_after: caster.unit().death_state(),
                    ai_state_before,
                    ai_state_after: caster.ai_ownership().state,
                },
            });
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

    Some(SecondarySplitDamageOutcomeLikeCpp {
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
        creature_sync,
        reached_damage_delivery: true,
    })
}
