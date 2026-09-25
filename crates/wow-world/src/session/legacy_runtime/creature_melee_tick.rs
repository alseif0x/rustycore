//! Legacy creature melee tick and its damage application.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::creature_melee_sync::{
    CreatureVictimCompatibilitySyncLikeCpp, PendingCreatureSwingLikeCpp,
};
use super::*;

mod absorption;
mod creature_victim;
mod player_victim;
mod secondary_targets;
use creature_victim::creature_victim_damage_like_cpp;
use player_victim::player_victim_damage_like_cpp;
use secondary_targets::{
    MeleeSecondaryTargetsOutcomeLikeCpp, apply_secondary_targets_damage_like_cpp,
};

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
/// The per-swing presentation `run_legacy_creature_melee_tick_once_like_cpp`
/// resolves before `DealMeleeDamage` and publishes at delivery.
///
/// C++ `Unit::CalculateMeleeDamage` fills these terms before the health write.
/// The player-victim phase in `player_victim` writes them and this tick's
/// delivery owns publishing them, so they travel between the two as one value.
/// The split/share locals live in `secondary_targets`, the phase that owns them.
struct MeleeSwingStateLikeCpp {
    pub(super) hit_info: u32,
    pub(super) victim_state: u8,
    pub(super) original_damage: u32,
    pub(super) avoided_outcome: Option<crate::session_rules::RepresentedMeleeOutcomeLikeCpp>,
    // The creature-victim branch publishes through the compatibility
    // bridge, so it carries its own presentation and avoid flag.
    pub(super) creature_victim_presentation: Option<(u32, u8, i32)>,
    pub(super) creature_victim_avoided: bool,
    pub(super) outcome_represented: bool,
    // C++ `CalcAbsorbResist`'s result for this swing: the absorbed amount
    // the packet publishes and every shield it spent. The victim session
    // owns the absorb-log publication and the aura transition, so it
    // receives the consumption list at delivery.
    pub(super) absorbed_damage: u32,
    pub(super) mana_spent: u32,
    pub(super) absorb_consumptions:
        Vec<crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp>,
    pub(super) creature_victim_absorb_events: Vec<RuntimeEvent>,
}

impl MeleeSwingStateLikeCpp {
    /// The pre-table presentation defaults C++ starts one swing's damage with.
    fn new_like_cpp(damage: u32) -> Self {
        Self {
            hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
            victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
            original_damage: damage,
            avoided_outcome: None,
            creature_victim_presentation: None,
            creature_victim_avoided: false,
            outcome_represented: false,
            absorbed_damage: 0,
            mana_spent: 0,
            absorb_consumptions: Vec::new(),
            creature_victim_absorb_events: Vec::new(),
        }
    }
}

pub fn run_legacy_creature_melee_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    config: &crate::session::LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureMeleeTickOutcomeLikeCpp {
    use crate::map_manager::RuntimeTickOwner;
    use wow_entities::CurrentSpellSlot;

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
        let primary_threat_plan = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .map(|managed| {
                super::creature_melee_threat::plan_creature_damage_threat_like_cpp(
                    managed.map(),
                    swing.attacker_guid,
                    None,
                    config.spell_store.as_deref(),
                    config.spell_misc_store.as_deref(),
                    config.spell_threat_store.as_deref(),
                    config.spell_chain_store.as_deref(),
                    managed.difficulty(),
                    config.difficulty_store.as_deref(),
                )
            })
            .unwrap_or_default();

        let apply = |canonical_manager: &mut wow_map::MapManager,
                     swing: &PendingCreatureSwingLikeCpp,
                     damage,
                     presentation: Option<(u32, u8, i32)>,
                     absorbed: u32,
                     wire_health_before: Option<u64>,
                     represented_damage_done: Option<u32>| {
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
                    wire_health_before,
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
                    presentation,
                    absorbed,
                    wire_health_before,
                    represented_damage_done,
                    primary_threat_plan,
                )
            }
        };

        match apply(&mut canonical_manager, &swing, None, None, 0, None, None) {
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
        // before the outcome switch (`Unit.cpp:1341-1443`). A player victim
        // resolves miss/dodge/parry/crit here; the block band and the
        // player-victim armour/taken terms remain the documented boundary of
        // this slice. Every term needs the spell store, so without one the
        // pre-table always-hit bridge is preserved.
        let mut state = MeleeSwingStateLikeCpp::new_like_cpp(damage);
        let damage = if swing.victim_guid.is_player() {
            player_victim_damage_like_cpp(
                &mut canonical_manager,
                attacker,
                &swing,
                config,
                damage,
                &mut state,
            )
        } else {
            creature_victim_damage_like_cpp(
                &mut canonical_manager,
                attacker,
                &swing,
                config,
                damage,
                &mut state,
            )
        };
        let MeleeSecondaryTargetsOutcomeLikeCpp {
            damage,
            primary_wire_health_before,
            represented_damage_done,
            split_mutation_events,
            split_combat_log_packets,
            share_mutation_events,
            primary_was_share_target,
            primary_player_share_health_updates,
        } = apply_secondary_targets_damage_like_cpp(
            &mut canonical_manager,
            attacker,
            &swing,
            config,
            damage,
            &mut state,
            &mut creature_victim_syncs,
        );
        if !state.outcome_represented {
            outcome.melee_outcomes_unrepresented += 1;
        }

        // C++ `CalculateMeleeDamage` returns before `DealMeleeDamage` for an
        // avoided swing (`Unit.cpp:1345-1355`, `1395-1407`): no health write, no
        // death check and no proc. The command carries the victim's unchanged
        // canonical tuple so the session's revision gate stays exact.
        if let Some(avoided) = state.avoided_outcome {
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
                    hit_info: state.hit_info,
                    victim_state: state.victim_state,
                    original_damage: state.original_damage,
                    absorbed: 0,
                    mana_spent: 0,
                    absorb_consumptions: Vec::new(),
                    split_combat_log_packets: Vec::new(),
                    self_share_health_updates: Vec::new(),
                },
            );
            continue;
        }

        let (
            victim_applied_damage,
            victim_threat,
            victim_health_before,
            victim_health_after,
            victim_health_state_revision_before,
            victim_health_state_revision_after,
            victim_creature_sync_identity,
            over_damage,
            target_level,
            events,
        ) = match apply(
            &mut canonical_manager,
            &swing,
            Some(damage),
            state.creature_victim_presentation,
            state.absorbed_damage,
            primary_was_share_target
                .then_some(primary_wire_health_before)
                .flatten(),
            (!swing.victim_guid.is_player()).then_some(represented_damage_done),
        ) {
            CreatureMeleeApplyResultLikeCpp::Hit {
                victim_applied_damage,
                victim_threat,
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
                victim_threat,
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
                    hit_info: state.hit_info,
                    victim_state: state.victim_state,
                    original_damage: state.original_damage,
                    absorbed: state.absorbed_damage,
                    mana_spent: state.mana_spent,
                    absorb_consumptions: state.absorb_consumptions.clone(),
                    split_combat_log_packets: split_combat_log_packets.clone(),
                    self_share_health_updates: primary_player_share_health_updates,
                },
            );
            outcome.plan.events.extend(split_mutation_events);
            outcome.plan.events.extend(share_mutation_events);
        } else {
            if !state.creature_victim_avoided {
                outcome.canonical_creature_hits += 1;
            }
            outcome
                .plan
                .events
                .extend(state.creature_victim_absorb_events);
            outcome.plan.events.extend(split_mutation_events);
            outcome
                .plan
                .events
                .extend(
                    split_combat_log_packets
                        .into_iter()
                        .map(|packet_bytes| RuntimeEvent {
                            source_guid: swing.victim_guid,
                            recipients: RecipientRule::MapBroadcastVisible {
                                map_id: swing.map_id,
                                instance_id: swing.instance_id,
                            },
                            packet_bytes,
                        }),
                );
            let mut primary_events = events.into_iter();
            if let Some(attacker_state) = primary_events.next() {
                outcome.plan.events.push(attacker_state);
            }
            outcome.plan.events.extend(share_mutation_events);
            outcome.plan.events.extend(primary_events);
            if victim_health_state_revision_after != victim_health_state_revision_before
                || victim_threat.is_some()
            {
                creature_victim_syncs.push(CreatureVictimCompatibilitySyncLikeCpp {
                    swing,
                    state: CreatureMeleeVictimSyncStateLikeCpp {
                        applied_damage: victim_applied_damage,
                        threat: victim_threat,
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

    super::creature_melee_sync::replay_creature_victim_syncs_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        creature_victim_syncs,
        &mut outcome,
    );
    outcome
}
