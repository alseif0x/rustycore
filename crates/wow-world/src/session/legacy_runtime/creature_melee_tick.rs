//! Legacy creature melee tick and its damage application.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// Apply one player's melee swings to a legacy creature.
///
/// Lifted out of `run_combat_tick` by #28. This is the write path that made
/// every logged-in session a writer of shared creature combat state; extracting
/// it is what lets the global loop become its sole owner. Damage arithmetic,
/// tap assignment, threat, the death branch and the swing record are unchanged.
pub(in crate::session) fn apply_player_melee_to_legacy_creature_like_cpp(
    creature: &mut crate::map_manager::WorldCreature,
    player_guid: ObjectGuid,
    tap_group_guids: &[ObjectGuid],
    canonical_swings: Option<&[crate::session::combat::RepresentedMeleeSwingLikeCpp]>,
) -> Option<PlayerMeleeCreatureHitLikeCpp> {
    if !creature.is_alive() {
        return None;
    }
    if creature.state() != wow_entities::CreatureAiState::InCombat {
        creature.enter_combat(player_guid);
    }
    let damages: Vec<crate::session::combat::RepresentedMeleeSwingLikeCpp> = match canonical_swings
    {
        Some(swings) => swings.to_vec(),
        None => {
            if !creature.can_swing() {
                return None;
            }
            vec![
                crate::session::combat::RepresentedMeleeSwingLikeCpp::hit_like_cpp(
                    creature.roll_damage()?.max(1),
                ),
            ]
        }
    };
    let entry = creature.entry();
    let level = creature.level();
    let mut swings = Vec::new();
    let mut swing_presentations = Vec::new();
    let mut died = false;
    let mut move_stop = None;
    for swing in damages {
        if !creature.is_alive() {
            break;
        }
        let damage = swing.damage;
        // C++ `DealMeleeDamage` applies nothing for a missed or avoided swing:
        // no damage, no tap and no threat.
        if damage == 0 {
            swings.push((0, false, -1));
            swing_presentations.push((
                swing.hit_info,
                swing.victim_state,
                swing.blocked,
                swing.original_damage,
            ));
            continue;
        }
        let health_before = creature.current_hp();
        creature
            .creature
            .set_tapped_by_player(player_guid, tap_group_guids);
        died = creature.take_damage_before_death_state_like_cpp(damage);
        let over_damage = if died {
            damage.saturating_sub(health_before) as i32
        } else {
            -1
        };
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .add_threat(player_guid, damage as f32);
        swings.push((damage, died, over_damage));
        swing_presentations.push((
            swing.hit_info,
            swing.victim_state,
            swing.blocked,
            swing.original_damage,
        ));
        if died {
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.clear_threat();
            combat.clear_attackers();
            move_stop = creature
                .stop_move_spline_like_cpp()
                .map(|stop| (stop.position, stop.spline_id));
            break;
        }
    }
    if canonical_swings.is_none() {
        creature.record_swing();
    }
    let values_update = creature.creature.unit().values_update();
    Some(PlayerMeleeCreatureHitLikeCpp {
        swings,
        swing_presentations,
        entry,
        level,
        died,
        move_stop,
        values_update,
    })
}
/// Runs one global legacy creature melee tick without spawning a loop.
///
/// This is dormant infrastructure for the next runtime slice after movement
/// and lifecycle. C++ contrast: `Creature::Update` calls
/// `DoMeleeAttackIfReady()` from the map object update phase. This function
/// preserves the pre-existing transitional damage bridge while the complete
/// C++ outcome/proc pipeline remains a later runtime slice. Spell-hit RNG
/// accreditation must not turn otherwise valid creature swings into no-ops.
pub fn run_legacy_creature_melee_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    config: &crate::session::LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureMeleeTickOutcomeLikeCpp {
    use crate::map_manager::RuntimeTickOwner;
    use wow_entities::CurrentSpellSlot;

    #[derive(Clone, Copy)]
    struct PendingCreatureSwingLikeCpp {
        map_id: u16,
        instance_id: u32,
        attacker_guid: ObjectGuid,
        attacker_position: Position,
        attacker_combat_reach: f32,
        attacker_can_state_update: bool,
        victim_guid: ObjectGuid,
    }

    struct CreatureVictimCompatibilitySyncLikeCpp {
        swing: PendingCreatureSwingLikeCpp,
        state: CreatureMeleeVictimSyncStateLikeCpp,
    }

    struct CreatureVictimCompatibilitySyncChainLikeCpp {
        swing: PendingCreatureSwingLikeCpp,
        states: Vec<CreatureMeleeVictimSyncStateLikeCpp>,
    }

    let mut outcome = LegacyCreatureMeleeTickOutcomeLikeCpp::default();
    let mut pending_swings = Vec::new();

    {
        let mut manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
            outcome.skipped_owner_not_global = true;
            return outcome;
        }

        let map_keys = manager.active_map_keys();
        outcome.maps_seen = map_keys.len();
        for (map_id, instance_id) in map_keys {
            let guids = manager.creature_guids(map_id, instance_id);
            for guid in guids {
                let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) else {
                    continue;
                };
                outcome.creatures_seen += 1;
                if !creature.can_swing() {
                    continue;
                }
                // C++ `TurretAI` calls `SetCanMelee(false)` in its
                // constructor. The transitional selector stores the explicit
                // DB AIName rather than a live AI object, so enforce that
                // constructor side effect at the global melee boundary.
                if creature.creature.lifecycle_metadata().ai_name == "TurretAI" {
                    outcome.melee_precondition_rejections += 1;
                    continue;
                }
                if !creature.creature.can_melee_like_cpp() {
                    outcome.melee_precondition_rejections += 1;
                    continue;
                }
                let unit = creature.creature.unit();
                if unit.has_unit_state(UnitState::CHARGING.bits())
                    || (unit.has_unit_state(UnitState::CASTING.bits())
                        && !unit
                            .current_spell(CurrentSpellSlot::Channeled)
                            .is_some_and(|spell| spell.allow_actions_during_channel))
                {
                    outcome.melee_precondition_rejections += 1;
                    continue;
                }
                let Some(victim_guid) = creature.creature.ai_ownership().combat_target else {
                    continue;
                };
                if !victim_guid.is_player() && !victim_guid.is_any_type_creature() {
                    continue;
                }
                pending_swings.push(PendingCreatureSwingLikeCpp {
                    map_id,
                    instance_id,
                    attacker_guid: guid,
                    attacker_position: creature.position(),
                    attacker_combat_reach: creature.creature.unit().world().combat_reach(),
                    attacker_can_state_update: creature
                        .creature
                        .unit()
                        .can_attacker_state_update_melee_like_cpp(false),
                    victim_guid,
                });
                outcome.swings_ready += 1;
            }
        }
    }

    let Some(canonical_map_manager) = canonical_map_manager else {
        return outcome;
    };

    let mut creature_victim_syncs = Vec::new();
    for mut swing in pending_swings {
        // One C++ map update owns attacker validation, RNG consumption, damage,
        // attacking-aura removal, and timer rearm as one serial operation. Hold
        // both transitional owners in the established canonical -> legacy order
        // so a target switch or same-GUID respawn cannot cross that commit.
        let Ok(mut canonical_manager) = canonical_map_manager.lock() else {
            outcome.melee_precondition_rejections += 1;
            continue;
        };
        let mut legacy_manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(attacker) =
            legacy_manager.find_creature_mut(swing.map_id, swing.instance_id, swing.attacker_guid)
        else {
            outcome.melee_precondition_rejections += 1;
            continue;
        };

        if !attacker.can_swing()
            || attacker.creature.ai_ownership().combat_target != Some(swing.victim_guid)
            || attacker.creature.lifecycle_metadata().ai_name == "TurretAI"
            || !attacker.creature.can_melee_like_cpp()
        {
            outcome.melee_precondition_rejections += 1;
            continue;
        }
        let unit = attacker.creature.unit();
        if unit.has_unit_state(UnitState::CHARGING.bits())
            || (unit.has_unit_state(UnitState::CASTING.bits())
                && !unit
                    .current_spell(CurrentSpellSlot::Channeled)
                    .is_some_and(|spell| spell.allow_actions_during_channel))
        {
            outcome.melee_precondition_rejections += 1;
            continue;
        }
        swing.attacker_position = attacker.position();
        swing.attacker_combat_reach = unit.world().combat_reach();
        swing.attacker_can_state_update = unit.can_attacker_state_update_melee_like_cpp(false);

        let canonical_attacker_is_same_incarnation = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_like_cpp(swing.attacker_guid, |canonical_attacker| {
                        canonical_attacker.spawn_id() == attacker.creature.spawn_id()
                            && canonical_attacker
                                .loot_authority_like_cpp()
                                .shares_storage_like_cpp(
                                    attacker.creature.loot_authority_like_cpp(),
                                )
                            && canonical_attacker
                                .unit()
                                .shares_health_state_revision_authority_like_cpp(
                                    &attacker
                                        .creature
                                        .unit()
                                        .health_state_revision_authority_like_cpp(),
                                )
                    })
            })
            .unwrap_or(false);
        if !canonical_attacker_is_same_incarnation {
            outcome.melee_precondition_rejections += 1;
            outcome.attacker_incarnation_rejections += 1;
            continue;
        }
        // Movement snapshots normally publish this position before melee, but
        // the two global ticks may overlap between their legacy read and
        // canonical write phases. With both owners locked and the incarnation
        // proven equal, align the canonical WorldObject used by the LOS check to
        // the same live position used for range and facing.
        if let Some(managed) =
            canonical_manager.find_map_mut(u32::from(swing.map_id), swing.instance_id)
        {
            let _ = managed
                .map_mut()
                .relocate_map_object_like_cpp(swing.attacker_guid, swing.attacker_position);
        }

        let apply = |canonical_manager: &mut wow_map::MapManager,
                     swing: &PendingCreatureSwingLikeCpp,
                     damage| {
            if swing.victim_guid.is_player() {
                apply_creature_melee_damage_to_canonical_player_on_map_like_cpp(
                    canonical_manager,
                    u32::from(swing.map_id),
                    swing.instance_id,
                    swing.attacker_guid,
                    swing.attacker_position,
                    swing.attacker_combat_reach,
                    swing.attacker_can_state_update,
                    swing.victim_guid,
                    damage,
                )
            } else {
                apply_creature_melee_damage_to_canonical_creature_on_map_like_cpp(
                    canonical_manager,
                    u32::from(swing.map_id),
                    swing.instance_id,
                    swing.attacker_guid,
                    swing.attacker_position,
                    swing.attacker_combat_reach,
                    swing.attacker_can_state_update,
                    swing.victim_guid,
                    damage,
                )
            }
        };

        match apply(&mut canonical_manager, &swing, None) {
            CreatureMeleeApplyResultLikeCpp::Ready => {}
            CreatureMeleeApplyResultLikeCpp::Hit { .. } => {
                unreachable!("melee precondition validation must not mutate canonical health")
            }
            CreatureMeleeApplyResultLikeCpp::OutOfRange => {
                outcome.melee_range_rejections += 1;
                attacker.record_failed_swing_retry_like_cpp();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::BadFacing => {
                outcome.melee_facing_rejections += 1;
                attacker.record_failed_swing_retry_like_cpp();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerStateRejected => {
                outcome.attacker_state_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::LosRejected => {
                outcome.melee_los_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::VictimNotAlive => {
                // `DoMeleeAttackIfReady` resets BASE_ATTACK after
                // `AttackerStateUpdate` returns early for a dead victim.
                outcome.melee_precondition_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerUnavailable => {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::MissingVictim => {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
        }

        if !swing.attacker_can_state_update {
            outcome.attacker_state_rejections += 1;
            attacker.record_swing();
            continue;
        }
        let Some(damage) = attacker.roll_damage() else {
            outcome.melee_precondition_rejections += 1;
            // `DoMeleeAttackIfReady` rearms BASE_ATTACK after
            // `AttackerStateUpdate` even if damage calculation cannot produce a
            // represented result.
            attacker.record_swing();
            continue;
        };
        // The compatibility bridge preserves the pre-existing damage and wire
        // behavior, but it does not model RollMeleeOutcomeAgainst or later
        // proc/daze draws. Keep gameplay running while preventing a later
        // creature spell from claiming an exact shared-RNG position.
        attacker.invalidate_runtime_rng_authority_like_cpp();
        let damage = damage.max(1);

        // C++ `CalculateMeleeDamage` rolls the attack table after mitigation and
        // before the outcome switch (`Unit.cpp:1341-1443`). A player victim can
        // resolve the miss band here; dodge/parry/block/crit and the
        // player-victim armour/taken terms remain the documented boundary of
        // this slice. Both aura sums need the spell store, so without one the
        // pre-table always-hit bridge is preserved.
        let mut hit_info = wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM;
        let mut victim_state = wow_packet::packets::combat::VICTIM_STATE_HIT;
        let mut original_damage = damage;
        let mut avoided_outcome = None;
        let damage = if swing.victim_guid.is_player() {
            match config.spell_store.as_deref() {
                Some(spell_store) => {
                    let map_difficulty_id = canonical_manager
                        .find_map(u32::from(swing.map_id), swing.instance_id)
                        .map(|managed| managed.difficulty())
                        .unwrap_or(0);
                    // C++ `MeleeSpellMissChance` (`Unit.cpp:11652-11685`): the
                    // creature's `m_modMeleeHitChance` is zero
                    // (`Unit.cpp:360`), so only its `SPELL_AURA_MOD_HIT_CHANCE`
                    // sum and the victim's `SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE`
                    // sum move the flat 5.0.
                    let attacker_hit_chance_aura_pct =
                        crate::session_rules::creature_aura_effects_like_cpp(
                            &attacker.creature.unit().subsystems().auras.applied_auras,
                            spell_store,
                            map_difficulty_id,
                            config.difficulty_store.as_deref(),
                        )
                        .into_iter()
                        .filter(|effect| {
                            effect.aura_type
                                == wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE
                        })
                        .map(|effect| effect.amount as f32)
                        .sum::<f32>();
                    let victim = canonical_manager
                        .find_map(u32::from(swing.map_id), swing.instance_id)
                        .and_then(|managed| managed.map().get_typed_player(swing.victim_guid))
                        .map(|player| {
                            let victim_hit_chance_aura_pct =
                                crate::session_rules::player_aura_effects_by_spell_aura_type_like_cpp(
                                    player.unit().subsystems().auras.runtime_applications_like_cpp(),
                                    spell_store,
                                    wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
                                )
                                .into_iter()
                                .map(|(_, amount)| amount as f32)
                                .sum::<f32>();
                            (player.level_like_cpp(), victim_hit_chance_aura_pct)
                        });
                    match victim {
                        Some((victim_level, victim_hit_chance_aura_pct)) => {
                            let attacker_facts =
                                crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp {
                                    level: attacker.creature.level(),
                                    melee_hit_chance_pct: 0.0,
                                    hit_chance_aura_pct: attacker_hit_chance_aura_pct,
                                    crit_damage_multiplier: 1.0,
                                    ..Default::default()
                                };
                            let victim_facts =
                                crate::session_rules::RepresentedMeleeVictimFactsLikeCpp {
                                    level: victim_level,
                                    is_player: true,
                                    attacker_melee_hit_chance_pct: victim_hit_chance_aura_pct,
                                    ..Default::default()
                                };
                            let inputs = crate::session_rules::melee_outcome_inputs_like_cpp(
                                &attacker_facts,
                                &victim_facts,
                            );
                            let rolled =
                                crate::session_rules::rolled_melee_outcome_like_cpp(&inputs[0]);
                            let (damage, _blocked, original) =
                                crate::session_rules::melee_outcome_damage_like_cpp(
                                    rolled,
                                    damage,
                                    attacker_facts.level,
                                    victim_facts.level,
                                    attacker_facts.crit_damage_multiplier,
                                );
                            let (info, state) =
                                crate::session_rules::melee_outcome_presentation_like_cpp(
                                    rolled, false,
                                );
                            hit_info = info;
                            victim_state = state;
                            original_damage = original;
                            if matches!(
                                rolled,
                                crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Evade
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Miss
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Dodge
                                    | crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Parry
                            ) {
                                avoided_outcome = Some(rolled);
                            }
                            damage
                        }
                        None => damage,
                    }
                }
                None => damage,
            }
        } else {
            damage
        };
        if avoided_outcome.is_none() {
            outcome.melee_outcomes_unrepresented += 1;
        }

        // C++ `CalculateMeleeDamage` returns before `DealMeleeDamage` for an
        // avoided swing (`Unit.cpp:1345-1355`, `1395-1407`): no health write, no
        // death check and no proc. The command carries the victim's unchanged
        // canonical tuple so the session's revision gate stays exact.
        if let Some(avoided) = avoided_outcome {
            let victim = canonical_manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .and_then(|managed| managed.map().get_typed_player(swing.victim_guid))
                .map(|victim| {
                    (
                        victim.unit().data().health,
                        victim.unit().health_state_revision_like_cpp(),
                        victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8,
                    )
                });
            let Some((victim_health_after, victim_health_state_revision_after, target_level)) =
                victim
            else {
                outcome.melee_precondition_rejections += 1;
                continue;
            };
            // C++ `Unit::AttackerStateUpdate` removes the attacking-interrupt
            // auras before `CalculateMeleeDamage`, so an avoided swing removes
            // them too (`Unit.cpp:2172-2173`).
            outcome.attacking_interrupt_auras_removed += attacker
                .creature
                .unit_mut()
                .remove_attacking_interrupt_auras_like_cpp();
            attacker.record_swing();
            outcome.commands.push(
                crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
                    attacker_guid: swing.attacker_guid,
                    victim_guid: swing.victim_guid,
                    map_id: swing.map_id,
                    instance_id: swing.instance_id,
                    damage: 0,
                    over_damage: -1,
                    target_level,
                    victim_health_after,
                    victim_health_state_revision_after,
                    hit_info,
                    victim_state,
                    original_damage,
                },
            );
            continue;
        }

        let (
            victim_applied_damage,
            victim_health_before,
            victim_health_after,
            victim_health_state_revision_before,
            victim_health_state_revision_after,
            victim_creature_sync_identity,
            over_damage,
            target_level,
            events,
        ) = match apply(&mut canonical_manager, &swing, Some(damage)) {
            CreatureMeleeApplyResultLikeCpp::Hit {
                victim_applied_damage,
                victim_health_before,
                victim_health_after,
                victim_health_state_revision_before,
                victim_health_state_revision_after,
                victim_creature_sync_identity,
                over_damage,
                target_level,
                events,
            } => (
                victim_applied_damage,
                victim_health_before,
                victim_health_after,
                victim_health_state_revision_before,
                victim_health_state_revision_after,
                victim_creature_sync_identity,
                over_damage,
                target_level,
                events,
            ),
            CreatureMeleeApplyResultLikeCpp::Ready => unreachable!(
                "melee apply with represented damage must not return validation readiness"
            ),
            CreatureMeleeApplyResultLikeCpp::OutOfRange => {
                outcome.melee_range_rejections += 1;
                attacker.record_failed_swing_retry_like_cpp();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::BadFacing => {
                outcome.melee_facing_rejections += 1;
                attacker.record_failed_swing_retry_like_cpp();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerStateRejected => {
                outcome.attacker_state_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::LosRejected => {
                outcome.melee_los_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::VictimNotAlive => {
                outcome.melee_precondition_rejections += 1;
                attacker.record_swing();
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerUnavailable => {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
            CreatureMeleeApplyResultLikeCpp::MissingVictim => {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
        };
        outcome.attacking_interrupt_auras_removed += attacker
            .creature
            .unit_mut()
            .remove_attacking_interrupt_auras_like_cpp();
        attacker.record_swing();
        outcome.canonical_hits += 1;
        if swing.victim_guid.is_player() {
            outcome.commands.push(
                crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
                    attacker_guid: swing.attacker_guid,
                    victim_guid: swing.victim_guid,
                    map_id: swing.map_id,
                    instance_id: swing.instance_id,
                    damage,
                    over_damage,
                    target_level,
                    victim_health_after,
                    victim_health_state_revision_after,
                    hit_info,
                    victim_state,
                    original_damage,
                },
            );
        } else {
            outcome.canonical_creature_hits += 1;
            outcome.plan.events.extend(events);
            if victim_health_state_revision_after != victim_health_state_revision_before {
                creature_victim_syncs.push(CreatureVictimCompatibilitySyncLikeCpp {
                    swing,
                    state: CreatureMeleeVictimSyncStateLikeCpp {
                        applied_damage: victim_applied_damage,
                        victim_health_before,
                        victim_health_after,
                        victim_health_state_revision_before,
                        victim_health_state_revision_after,
                        identity: victim_creature_sync_identity
                            .expect("creature victim commits carry incarnation authority"),
                    },
                });
            }
        }
    }

    // Multiple attackers can commit against one creature during a single
    // batch. Chain their contiguous health revisions so canonical authority is
    // checked once against the final desired tuple, then replay each committed
    // transition into the legacy mirror in FIFO order.
    let mut creature_victim_sync_chains: Vec<CreatureVictimCompatibilitySyncChainLikeCpp> =
        Vec::new();
    for sync in creature_victim_syncs {
        let contiguous = creature_victim_sync_chains.iter_mut().find(|chain| {
            let existing = chain
                .states
                .last()
                .expect("creature victim sync chains are never empty");
            chain.swing.map_id == sync.swing.map_id
                && chain.swing.instance_id == sync.swing.instance_id
                && chain.swing.victim_guid == sync.swing.victim_guid
                && existing.victim_health_after == sync.state.victim_health_before
                && existing.victim_health_state_revision_after
                    == sync.state.victim_health_state_revision_before
                && existing.identity.death_state_after == sync.state.identity.death_state_before
                && existing.identity.ai_state_after == sync.state.identity.ai_state_before
                && existing.identity.loot_lifecycle_revision_after
                    == sync.state.identity.loot_lifecycle_revision_before
                && existing.identity.spawn_id == sync.state.identity.spawn_id
                && existing
                    .identity
                    .authority
                    .shares_storage_like_cpp(&sync.state.identity.authority)
                && existing
                    .identity
                    .health_state_revision_authority
                    .shares_storage_like_cpp(&sync.state.identity.health_state_revision_authority)
        });
        if let Some(chain) = contiguous {
            chain.states.push(sync.state);
        } else {
            creature_victim_sync_chains.push(CreatureVictimCompatibilitySyncChainLikeCpp {
                swing: sync.swing,
                states: vec![sync.state],
            });
        }
    }

    // Lock order is canonical -> legacy, matching the existing spell-cast
    // validation bridge. Holding canonical authority through the legacy CAS
    // closes the final window where a heal/death/respawn could otherwise make
    // this mirror write stale.
    for chain in creature_victim_sync_chains {
        let desired = chain
            .states
            .last()
            .expect("creature victim sync chains are never empty");
        let Ok(mut canonical_manager) = canonical_map_manager.lock() else {
            outcome.legacy_creature_victim_sync_cas_rejections += 1;
            continue;
        };
        let canonical_is_desired = canonical_manager
            .find_map_mut(u32::from(chain.swing.map_id), chain.swing.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_like_cpp(chain.swing.victim_guid, |victim| {
                        let unit = victim.unit();
                        let identity = &desired.identity;
                        unit.data().health == desired.victim_health_after
                            && unit.death_state() == identity.death_state_after
                            && unit.health_state_revision_like_cpp()
                                == desired.victim_health_state_revision_after
                            && victim.spawn_id() == identity.spawn_id
                            && victim.loot_lifecycle_revision_like_cpp()
                                == identity.loot_lifecycle_revision_after
                            && victim.ai_ownership().state == identity.ai_state_after
                            && victim
                                .loot_authority_like_cpp()
                                .shares_storage_like_cpp(&identity.authority)
                            && victim
                                .unit()
                                .shares_health_state_revision_authority_like_cpp(
                                    &identity.health_state_revision_authority,
                                )
                            && victim.loot_authority_like_cpp().lifecycle_like_cpp()
                                != OwnedLootAuthorityLifecycle::Detached
                    })
            })
            .unwrap_or(false);
        if !canonical_is_desired {
            outcome.legacy_creature_victim_sync_cas_rejections += 1;
            continue;
        }

        let mut legacy_manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(victim) = legacy_manager.find_creature_mut(
            chain.swing.map_id,
            chain.swing.instance_id,
            chain.swing.victim_guid,
        ) else {
            outcome.legacy_creature_victim_sync_cas_rejections += 1;
            continue;
        };
        let game_time_secs = wow_entities::game_time_secs_like_cpp();
        for state in &chain.states {
            if apply_creature_melee_victim_sync_to_legacy_like_cpp(victim, state, game_time_secs) {
                outcome.legacy_creature_victim_syncs += 1;
            } else {
                outcome.legacy_creature_victim_sync_cas_rejections += 1;
                break;
            }
        }
    }

    outcome
}
