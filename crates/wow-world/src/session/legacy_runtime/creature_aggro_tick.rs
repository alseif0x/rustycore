//! Legacy creature aggro, threat and victim selection tick.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

pub(in crate::session) fn legacy_creature_aggro_candidate_is_targetable_for_attack_like_cpp(
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
) -> bool {
    let player_flags = UnitFlags::from_bits_truncate(candidate.player_unit_flags);
    let player_state = UnitState::from_bits_truncate(candidate.player_unit_state);

    if candidate.player_is_game_master {
        return false;
    }

    // C++ anchors:
    // - `Unit::isTargetableForAttack(false)` rejects dead, `UNIT_STATE_UNATTACKABLE`,
    //   non-attackable, uninteractible and GM players. In Trinity 3.3.5
    //   `UNIT_STATE_UNATTACKABLE` is currently `UNIT_STATE_IN_FLIGHT`.
    // - `Creature::_IsTargetAcceptable` rejects `UNIT_STATE_DIED` unless the
    //   creature can detect feign death; that override is not represented in
    //   the transitional global aggro scan yet.
    // - `WorldObject::IsValidAttackTarget` rejects untargetable/taxi targets
    //   and `UNIT_FLAG_IMMUNE_TO_NPC` for creature-vs-player attacks.
    if player_state.intersects(UnitState::DIED | UnitState::IN_FLIGHT) {
        return false;
    }

    !player_flags.intersects(
        UnitFlags::NON_ATTACKABLE
            | UnitFlags::UNINTERACTIBLE
            | UnitFlags::NON_ATTACKABLE_2
            | UnitFlags::ON_TAXI
            | UnitFlags::NOT_ATTACKABLE_1
            | UnitFlags::IMMUNE_TO_NPC,
    )
}
fn legacy_creature_aggro_candidate_unit_snapshot_like_cpp(
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
) -> Unit {
    let mut unit = Unit::new(true);
    unit.world_mut().object_mut().create(candidate.player_guid);
    let _ = unit
        .world_mut()
        .set_map(u32::from(candidate.map_id), candidate.instance_id);
    unit.world_mut().relocate(candidate.position);
    *unit.world_mut().phase_shift_mut() = candidate.player_phase_shift.clone();
    unit.set_level(candidate.player_level);
    unit.set_combat_reach(candidate.player_combat_reach);
    unit.set_unit_flags_like_cpp(UnitFlags::from_bits_truncate(candidate.player_unit_flags));
    unit.set_unit_flags2_like_cpp(UnitFlags2::from_bits_truncate(candidate.player_unit_flags2));
    unit.add_unit_state(candidate.player_unit_state);
    unit.replace_visibility_detection_like_cpp(candidate.player_visibility_detection.clone());
    let _ = unit.add_to_world_like_cpp();
    unit
}
fn legacy_creature_aggro_creature_unit_snapshot_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    map_id: u16,
    instance_id: u32,
) -> Unit {
    let source = creature.creature.unit();
    let mut unit = Unit::new(true);
    unit.world_mut()
        .object_mut()
        .create(source.world().object().guid());
    let _ = unit.world_mut().set_map(u32::from(map_id), instance_id);
    unit.world_mut().relocate(creature.position());
    *unit.world_mut().phase_shift_mut() = source.world().phase_shift().clone();
    unit.set_level(creature.level());
    unit.set_combat_reach(source.data().combat_reach);
    unit.set_unit_flags_like_cpp(source.unit_flags_like_cpp());
    unit.set_unit_flags2_like_cpp(source.unit_flags2_like_cpp());
    unit.add_unit_state(source.unit_state());
    unit.replace_visibility_detection_like_cpp(source.visibility_detection_like_cpp().clone());
    let _ = unit.add_to_world_like_cpp();
    unit
}
pub(in crate::session) fn legacy_creature_aggro_candidate_visibility_decision_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    map_id: u16,
    instance_id: u32,
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    check_alert: bool,
) -> LegacyCreatureAggroVisibilityDecisionLikeCpp {
    if !candidate.player_visibility_represented {
        return LegacyCreatureAggroVisibilityDecisionLikeCpp::Unrepresented;
    }

    let seer = legacy_creature_aggro_creature_unit_snapshot_like_cpp(creature, map_id, instance_id);
    let target = legacy_creature_aggro_candidate_unit_snapshot_like_cpp(candidate);
    if seer.can_see_or_detect_unit_like_cpp(&target, false, false, check_alert) {
        LegacyCreatureAggroVisibilityDecisionLikeCpp::Allowed
    } else {
        LegacyCreatureAggroVisibilityDecisionLikeCpp::Rejected
    }
}
pub(in crate::session) fn legacy_creature_aggro_candidate_has_stealth_aura_like_cpp(
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
) -> bool {
    legacy_creature_aggro_candidate_unit_snapshot_like_cpp(candidate).has_stealth_aura_like_cpp()
}
fn legacy_creature_aggro_candidate_has_reputation_state_like_cpp(
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    faction_id: u32,
) -> bool {
    candidate
        .player_reputation_state_flags
        .iter()
        .any(|(candidate_faction_id, _)| *candidate_faction_id == faction_id)
}
fn legacy_creature_aggro_candidate_is_at_war_like_cpp(
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    faction_id: u32,
) -> bool {
    candidate
        .player_reputation_state_flags
        .iter()
        .find_map(|(candidate_faction_id, flags)| {
            (*candidate_faction_id == faction_id).then_some(*flags)
        })
        .is_some_and(|flags| flags & wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP != 0)
}
fn legacy_creature_aggro_candidate_reputation_standing_like_cpp(
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    faction_id: u32,
) -> i32 {
    candidate
        .player_reputation_standings
        .iter()
        .find_map(|(candidate_faction_id, standing)| {
            (*candidate_faction_id == faction_id).then_some(*standing)
        })
        .unwrap_or(0)
}
fn legacy_creature_aggro_candidate_has_forced_reputation_rank_like_cpp(
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    faction_id: u32,
) -> Option<wow_data::reputation::ReputationRankLikeCpp> {
    candidate
        .player_forced_reputation_ranks
        .iter()
        .find_map(|(candidate_faction_id, rank)| {
            (*candidate_faction_id == faction_id).then_some(*rank)
        })
}
pub(in crate::session) fn legacy_creature_aggro_candidate_is_hostile_to_creature_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<bool> {
    let faction_template_store = config.faction_template_store.as_ref()?;
    let creature_faction_template_id =
        creature.creature.unit().data().faction_template.max(0) as u32;
    if creature_faction_template_id == 0 || candidate.player_faction_template_id == 0 {
        return Some(false);
    }

    let creature_faction_template = faction_template_store.get(creature_faction_template_id)?;
    let player_faction_template =
        faction_template_store.get(candidate.player_faction_template_id)?;

    if creature_faction_template.is_contested_guard_faction_like_cpp()
        && candidate.player_is_contested_pvp
    {
        return Some(true);
    }

    let creature_faction_id = u32::from(creature_faction_template.faction);
    if creature_faction_id != 0 {
        if let Some(forced_rank) =
            legacy_creature_aggro_candidate_has_forced_reputation_rank_like_cpp(
                candidate,
                creature_faction_id,
            )
        {
            return Some(forced_rank <= wow_data::reputation::ReputationRankLikeCpp::Hostile);
        }
        if candidate
            .player_forced_reputation_faction_ids
            .contains(&creature_faction_id)
        {
            return None;
        }

        let player_flags2 = UnitFlags2::from_bits_truncate(candidate.player_unit_flags2);
        if !player_flags2.contains(UnitFlags2::IGNORE_REPUTATION)
            && let Some(faction_store) = config.faction_store.as_ref()
            && let Some(faction_entry) = faction_store.get(creature_faction_id)
            && faction_entry.can_have_reputation_like_cpp()
            && legacy_creature_aggro_candidate_has_reputation_state_like_cpp(
                candidate,
                creature_faction_id,
            )
        {
            if !legacy_creature_aggro_candidate_is_at_war_like_cpp(candidate, creature_faction_id) {
                return Some(false);
            }

            let rank = wow_data::reputation::reputation_rank_from_standing_like_cpp(
                legacy_creature_aggro_candidate_reputation_standing_like_cpp(
                    candidate,
                    creature_faction_id,
                ),
            );
            // C++ `GetFactionReactionTo` caps an at-war player reaction to at
            // most neutral; `Creature::_IsTargetAcceptable` still requires an
            // actually hostile reaction to start aggro.
            return Some(rank <= wow_data::reputation::ReputationRankLikeCpp::Hostile);
        }
    }

    if creature_faction_template.is_hostile_to_like_cpp(player_faction_template) {
        return Some(true);
    }
    if creature_faction_template.is_friendly_to_like_cpp(player_faction_template) {
        return Some(false);
    }
    if player_faction_template.is_friendly_to_like_cpp(creature_faction_template) {
        return Some(false);
    }
    if creature_faction_template.is_hostile_by_default_like_cpp() {
        return Some(true);
    }
    Some(false)
}
pub(in crate::session) fn legacy_creature_aggro_candidate_is_accessible_for_creature_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
) -> bool {
    let victim_is_in_water = candidate.player_liquid_status_like_cpp
        & (LIQUID_MAP_IN_WATER_LIKE_CPP | LIQUID_MAP_UNDER_WATER_LIKE_CPP)
        != 0;

    // C++ `Unit::isInAccessiblePlaceFor(Creature const*)`:
    // water victims require `Creature::CanEnterWater`; non-water victims
    // require `Creature::CanWalk() || Creature::CanFly()`.
    if victim_is_in_water {
        creature.creature.can_enter_water_like_cpp()
    } else {
        creature.creature.can_walk_like_cpp() || creature.creature.can_fly_like_cpp()
    }
}
/// Runs one global legacy creature aggro scan without spawning a loop.
///
/// C++ contrast: `CreatureAI::MoveInLineOfSight` checks aggressive creatures
/// through `Creature::CanStartAttack` and then engages a target. This
/// transitional Rust slice uses the existing represented `WorldCreature`
/// `try_aggro` radius model plus the represented C++ targetability,
/// faction/reputation, accessibility, vertical distance, template `CanFly`
/// exemption, home leash/NoGrayAggro, and represented `CanSeeOrDetect` gates;
/// real VMAP-backed LOS remains a later AI fidelity task.
/// The map owner computes aggro once and returns victim-session `AttackStart`
/// commands for delivery outside map locks.
pub fn run_legacy_creature_aggro_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    candidates: &[LegacyCreatureAggroCandidateLikeCpp],
) -> LegacyCreatureAggroTickOutcomeLikeCpp {
    run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        legacy_map_manager,
        candidates,
        LegacyCreatureAggroConfigLikeCpp::default(),
    )
}
pub fn run_legacy_creature_aggro_tick_once_with_config_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    candidates: &[LegacyCreatureAggroCandidateLikeCpp],
    config: LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureAggroTickOutcomeLikeCpp {
    use crate::map_manager::RuntimeTickOwner;

    let mut outcome = LegacyCreatureAggroTickOutcomeLikeCpp::default();
    let mut manager = legacy_map_manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let terrain = manager.terrain();
    if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
        outcome.skipped_owner_not_global = true;
        return outcome;
    }

    let map_keys = manager.active_map_keys();
    outcome.maps_seen = map_keys.len();
    let owner_snapshots: HashMap<_, _> = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.player_guid,
                LegacyCreatureAggroOwnerSnapshotLikeCpp {
                    map_id: candidate.map_id,
                    instance_id: candidate.instance_id,
                    position: candidate.position,
                    phase_shift: candidate.player_phase_shift.clone(),
                    combat_reach: candidate.player_combat_reach,
                    alive: !UnitState::from_bits_truncate(candidate.player_unit_state)
                        .contains(UnitState::DIED),
                    in_water: candidate.player_liquid_status_like_cpp
                        & (LIQUID_MAP_IN_WATER_LIKE_CPP | LIQUID_MAP_UNDER_WATER_LIKE_CPP)
                        != 0,
                    in_evade_mode: false,
                    unit_flags: UnitFlags::from_bits_truncate(candidate.player_unit_flags),
                    faction_template_id: Some(candidate.player_faction_template_id),
                    school_immunity_mask: candidate.player_school_immunity_mask,
                    damage_immunity_mask: candidate.player_damage_immunity_mask,
                    has_confuse_aura: candidate.player_has_confuse_aura,
                    has_breakable_stun_aura: candidate.player_has_breakable_stun_aura,
                },
            )
        })
        .collect();
    for (map_id, instance_id) in map_keys {
        let map_candidates: Vec<_> = candidates
            .iter()
            .filter(|candidate| candidate.map_id == map_id && candidate.instance_id == instance_id)
            .collect();
        outcome.candidates_seen += map_candidates.len();

        let guids = manager.creature_guids(map_id, instance_id);
        let mut assistance_calls = Vec::new();
        let mut owner_snapshots = owner_snapshots.clone();
        let mut creature_factions = HashMap::new();
        for owner_guid in &guids {
            if let Some(owner) = manager.find_creature(map_id, instance_id, *owner_guid) {
                owner_snapshots.insert(
                    *owner_guid,
                    LegacyCreatureAggroOwnerSnapshotLikeCpp {
                        map_id,
                        instance_id,
                        position: owner.position(),
                        phase_shift: owner.phase_shift().clone(),
                        combat_reach: owner.creature.unit().world().combat_reach(),
                        alive: owner.is_alive(),
                        in_water: owner
                            .creature
                            .movement_flags_like_cpp()
                            .contains(wow_constants::movement::MovementFlag::SWIMMING),
                        in_evade_mode: owner.creature.is_in_evade_mode_like_cpp(),
                        unit_flags: owner.creature.unit().unit_flags_like_cpp(),
                        faction_template_id: u32::try_from(
                            owner.creature.unit().data().faction_template,
                        )
                        .ok(),
                        school_immunity_mask: owner
                            .creature
                            .unit()
                            .subsystems()
                            .auras
                            .aura_school_mask_like_cpp(
                                wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
                            ),
                        damage_immunity_mask: owner
                            .creature
                            .unit()
                            .subsystems()
                            .auras
                            .aura_school_mask_like_cpp(
                                wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
                            ),
                        has_confuse_aura: owner
                            .creature
                            .unit()
                            .subsystems()
                            .auras
                            .has_aura_type_like_cpp(
                                wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE,
                            ),
                        has_breakable_stun_aura: owner
                            .creature
                            .unit()
                            .subsystems()
                            .auras
                            .has_breakable_by_damage_aura_type_like_cpp(
                                wow_data::spell::aura_types::SPELL_AURA_MOD_STUN,
                            ),
                    },
                );
                creature_factions
                    .insert(*owner_guid, owner.creature.unit().data().faction_template);
            }
        }
        for guid in guids.iter().copied() {
            outcome.creatures_seen += 1;
            let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) else {
                continue;
            };
            let expired_taunt_slots = creature.expire_taunt_auras_if_due_like_cpp();
            if !expired_taunt_slots.is_empty() {
                use crate::map_manager::{RecipientRule, RuntimeEvent};
                use wow_packet::ServerPacket;

                outcome.plan.events.push(RuntimeEvent {
                    source_guid: guid,
                    recipients: RecipientRule::NearbyVisibleDurable {
                        source_guid: guid,
                        map_id,
                        instance_id,
                        source_position: creature.position(),
                        range: creature.visibility_range_like_cpp(),
                        required_3d: false,
                    },
                    packet_bytes: wow_packet::packets::misc::AuraUpdate {
                        unit_guid: guid,
                        update_all: false,
                        auras: expired_taunt_slots
                            .into_iter()
                            .map(|slot| wow_packet::packets::misc::AuraInfoLikeCpp {
                                slot,
                                aura_data: None,
                            })
                            .collect(),
                    }
                    .to_bytes(),
                });
            }
            let due_assistance = creature.take_due_assistance_like_cpp();
            let _ = creature;
            for (victim_guid, assistant_guids) in due_assistance {
                let victim = map_candidates
                    .iter()
                    .find(|candidate| candidate.player_guid == victim_guid)
                    .copied();
                let victim_snapshot = owner_snapshots.get(&victim_guid);
                if victim.is_none() && victim_snapshot.is_none() {
                    continue;
                }
                for assistant_guid in assistant_guids {
                    let Some(assistant) =
                        manager.find_creature_mut(map_id, instance_id, assistant_guid)
                    else {
                        continue;
                    };
                    let flags = assistant.creature.unit().unit_flags_like_cpp();
                    if assistant.is_alive()
                        && !assistant.creature.is_in_combat()
                        && !assistant.creature.is_in_evade_mode_like_cpp()
                        && !assistant.creature.unit().has_unit_state(
                            (UnitState::STUNNED | UnitState::CONFUSED | UnitState::FLEEING).bits(),
                        )
                        && assistant
                            .creature
                            .has_react_state(wow_entities::ReactState::Aggressive)
                        && !assistant.creature.is_civilian_like_cpp()
                        && assistant
                            .creature
                            .unit()
                            .subsystems()
                            .control
                            .charmer_or_owner_guid()
                            .is_none()
                        && !flags.intersects(
                            UnitFlags::NON_ATTACKABLE
                                | UnitFlags::IMMUNE_TO_NPC
                                | UnitFlags::UNINTERACTIBLE,
                        )
                        && creature_factions.get(&guid)
                            == Some(&assistant.creature.unit().data().faction_template)
                        && (victim.is_some_and(|victim| {
                            legacy_creature_aggro_candidate_is_targetable_for_attack_like_cpp(
                                victim,
                            ) && legacy_creature_aggro_candidate_is_hostile_to_creature_like_cpp(
                                assistant, victim, &config,
                            )
                            .unwrap_or(false)
                        }) || victim_snapshot.is_some_and(|victim| {
                            victim.alive
                                && !victim.in_evade_mode
                                && legacy_creature_snapshot_is_hostile_to_creature_like_cpp(
                                    assistant, victim, &config,
                                )
                                .unwrap_or(false)
                        }))
                    {
                        // C++ `AssistDelayEvent` calls `SetNoCallAssistance(true)`
                        // only after the delayed `CanAssistTo` revalidation and
                        // immediately before `EngageWithTarget`.
                        assistant.set_no_call_assistance_like_cpp();
                        assistant.enter_combat(victim_guid);
                        assistant
                            .creature
                            .unit_mut()
                            .subsystems_mut()
                            .combat
                            .add_threat(victim_guid, 0.0);
                        outcome.assistance_starts += 1;
                        outcome.aggro_starts += 1;
                        use crate::map_manager::{RecipientRule, RuntimeEvent};
                        use wow_packet::ServerPacket;

                        outcome.plan.events.push(RuntimeEvent {
                            source_guid: assistant_guid,
                            recipients: RecipientRule::NearbyVisibleDurable {
                                source_guid: assistant_guid,
                                map_id,
                                instance_id,
                                source_position: assistant.position(),
                                range: assistant.visibility_range_like_cpp(),
                                required_3d: false,
                            },
                            packet_bytes: wow_packet::packets::combat::AttackStart {
                                attacker: assistant_guid,
                                victim: victim_guid,
                            }
                            .to_bytes(),
                        });
                        outcome.commands.push(
                            crate::session::mailbox::CreatureAttackStartLikeCppCommand {
                                attacker_guid: assistant_guid,
                                victim_guid,
                                previous_victim_guid: None,
                                map_id,
                                instance_id,
                                packet_already_broadcast: true,
                            },
                        );
                    }
                }
            }
            let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) else {
                continue;
            };
            match legacy_creature_update_threat_victim_like_cpp(
                creature,
                &map_candidates,
                &config,
                &owner_snapshots,
            ) {
                LegacyCreatureThreatUpdateLikeCpp::Unchanged => {}
                LegacyCreatureThreatUpdateLikeCpp::Switched { previous_victim } => {
                    outcome.victim_switches += 1;
                    if let Some(victim_guid) = creature.creature.ai_ownership().combat_target {
                        use crate::map_manager::{RecipientRule, RuntimeEvent};
                        use wow_packet::ServerPacket;
                        outcome.plan.events.push(RuntimeEvent {
                            source_guid: guid,
                            recipients: RecipientRule::NearbyVisibleDurable {
                                source_guid: guid,
                                map_id,
                                instance_id,
                                source_position: creature.position(),
                                range: creature.visibility_range_like_cpp(),
                                required_3d: false,
                            },
                            packet_bytes: wow_packet::packets::combat::AttackStart {
                                attacker: guid,
                                victim: victim_guid,
                            }
                            .to_bytes(),
                        });
                        outcome.commands.push(
                            crate::session::mailbox::CreatureAttackStartLikeCppCommand {
                                attacker_guid: guid,
                                victim_guid,
                                previous_victim_guid: (!previous_victim.is_empty())
                                    .then_some(previous_victim),
                                map_id,
                                instance_id,
                                packet_already_broadcast: true,
                            },
                        );
                    }
                }
                LegacyCreatureThreatUpdateLikeCpp::Evade {
                    previous_victim,
                    participant_guids,
                    removed_taunt_slots,
                } => {
                    use crate::map_manager::{RecipientRule, RuntimeEvent};
                    use wow_packet::ServerPacket;

                    if let Some(previous_victim) = previous_victim {
                        use wow_packet::packets::combat::SAttackStop;

                        outcome.plan.events.push(RuntimeEvent {
                            source_guid: guid,
                            recipients: RecipientRule::NearbyVisibleDurable {
                                source_guid: guid,
                                map_id,
                                instance_id,
                                source_position: creature.position(),
                                range: creature.visibility_range_like_cpp(),
                                required_3d: false,
                            },
                            packet_bytes: SAttackStop {
                                attacker: guid,
                                victim: previous_victim,
                                now_dead: false,
                            }
                            .to_bytes(),
                        });
                    }
                    for participant_guid in participant_guids {
                        outcome.stop_commands.push(
                            crate::session::mailbox::CreatureAttackStopLikeCppCommand {
                                attacker_guid: guid,
                                victim_guid: participant_guid,
                                map_id,
                                instance_id,
                            },
                        );
                    }
                    if !removed_taunt_slots.is_empty() {
                        outcome.plan.events.push(RuntimeEvent {
                            source_guid: guid,
                            recipients: RecipientRule::NearbyVisibleDurable {
                                source_guid: guid,
                                map_id,
                                instance_id,
                                source_position: creature.position(),
                                range: creature.visibility_range_like_cpp(),
                                required_3d: false,
                            },
                            packet_bytes: wow_packet::packets::misc::AuraUpdate {
                                unit_guid: guid,
                                update_all: false,
                                auras: removed_taunt_slots
                                    .into_iter()
                                    .map(|slot| wow_packet::packets::misc::AuraInfoLikeCpp {
                                        slot,
                                        aura_data: None,
                                    })
                                    .collect(),
                            }
                            .to_bytes(),
                        });
                    }
                    outcome.evades_started += 1;
                    continue;
                }
            }
            if let Some(victim_guid) = creature.take_assistance_call_like_cpp() {
                assistance_calls.push((
                    guid,
                    victim_guid,
                    creature.creature.unit().world().clone(),
                    creature.creature.unit().data().faction_template,
                ));
            }
            if creature
                .creature
                .unit()
                .has_unit_state(UnitState::SIGHTLESS.bits())
            {
                outcome.sightless_creatures_skipped += 1;
                continue;
            }
            let ai_kind = match legacy_creature_ai_selection_decision_like_cpp(creature, &config) {
                LegacyCreatureAiSelectionDecisionLikeCpp::Selected(ai_kind) => ai_kind,
                LegacyCreatureAiSelectionDecisionLikeCpp::ScriptRegistryUnrepresented => {
                    outcome.ai_selection_unrepresented += 1;
                    continue;
                }
            };
            if !creature_ai_uses_base_move_in_line_of_sight_like_cpp(&ai_kind) {
                outcome.ai_los_suppressed += 1;
                continue;
            }
            for candidate in &map_candidates {
                if !legacy_creature_aggro_candidate_is_targetable_for_attack_like_cpp(candidate) {
                    outcome.targetability_rejections += 1;
                    continue;
                }
                match legacy_creature_aggro_candidate_visibility_decision_like_cpp(
                    creature,
                    map_id,
                    instance_id,
                    candidate,
                    false,
                ) {
                    LegacyCreatureAggroVisibilityDecisionLikeCpp::Allowed => {}
                    LegacyCreatureAggroVisibilityDecisionLikeCpp::Rejected => {
                        if matches!(
                            legacy_creature_aggro_candidate_visibility_decision_like_cpp(
                                creature,
                                map_id,
                                instance_id,
                                candidate,
                                true,
                            ),
                            LegacyCreatureAggroVisibilityDecisionLikeCpp::Allowed
                        ) {
                            let alert_packet = legacy_creature_try_trigger_alert_like_cpp(
                                creature, candidate, &config,
                            );
                            if let Some(packet_bytes) = alert_packet {
                                use crate::map_manager::{RecipientRule, RuntimeEvent};

                                outcome.alert_triggers += 1;
                                outcome.plan.events.push(RuntimeEvent {
                                    source_guid: guid,
                                    recipients: RecipientRule::MapBroadcastVisible {
                                        map_id,
                                        instance_id,
                                    },
                                    packet_bytes,
                                });
                            } else {
                                outcome.alert_rejections += 1;
                            }
                        } else {
                            outcome.alert_rejections += 1;
                        }
                        outcome.visibility_rejections += 1;
                        continue;
                    }
                    LegacyCreatureAggroVisibilityDecisionLikeCpp::Unrepresented => {
                        outcome.visibility_unrepresented += 1;
                        continue;
                    }
                }
                match legacy_creature_aggro_candidate_is_hostile_to_creature_like_cpp(
                    creature, candidate, &config,
                ) {
                    Some(true) => {}
                    Some(false) => {
                        outcome.hostility_rejections += 1;
                        continue;
                    }
                    None => {
                        outcome.hostility_unrepresented += 1;
                        continue;
                    }
                }
                if !legacy_creature_aggro_candidate_is_accessible_for_creature_like_cpp(
                    creature, candidate,
                ) {
                    outcome.accessibility_rejections += 1;
                    continue;
                }
                if creature.creature.is_in_evade_mode_like_cpp() {
                    outcome.attacker_evade_rejections += 1;
                    continue;
                }
                match legacy_creature_ai_can_attack_decision_like_cpp(
                    &ai_kind, creature, candidate, &config,
                ) {
                    LegacyCreatureAiCanAttackDecisionLikeCpp::Allowed => {}
                    LegacyCreatureAiCanAttackDecisionLikeCpp::Rejected => {
                        outcome.ai_can_attack_rejections += 1;
                        continue;
                    }
                    LegacyCreatureAiCanAttackDecisionLikeCpp::Unrepresented => {
                        outcome.ai_can_attack_unrepresented += 1;
                        continue;
                    }
                }
                match legacy_creature_can_attack_leash_decision_like_cpp(
                    creature,
                    candidate,
                    &config,
                    &owner_snapshots,
                ) {
                    LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed => {}
                    LegacyCreatureCanAttackLeashDecisionLikeCpp::OwnerPositionUnrepresented => {
                        outcome.owner_position_unrepresented += 1;
                        continue;
                    }
                    LegacyCreatureCanAttackLeashDecisionLikeCpp::HomeRangeRejected => {
                        outcome.home_range_rejections += 1;
                        continue;
                    }
                }
                if check_no_gray_aggro_config_like_cpp(
                    &config,
                    candidate.player_level,
                    candidate.player_gray_level,
                    creature.level(),
                ) {
                    outcome.gray_aggro_rejections += 1;
                    continue;
                }
                let effective_aggro_range =
                    creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                        aggro_rate: config.creature_aggro_rate,
                        creature_combat_reach: creature.creature.unit().world().combat_reach(),
                        expansion_max_level: max_level_for_expansion_like_cpp(
                            creature.creature.lifecycle_metadata().required_expansion,
                        ),
                        max_player_level_config: config.max_player_level_config,
                        player_level_for_target: candidate.player_level,
                        creature_level_for_target: creature.level(),
                        creature_detect_range_aura_mod: creature
                            .creature
                            .unit()
                            .total_aura_modifier_like_cpp(
                                wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE,
                            ) as f32,
                        player_detected_range_aura_mod: candidate.player_detected_range_aura_mod,
                    }) + creature.creature.combat_distance();

                if creature
                    .creature
                    .try_ai_aggro_with_effective_range_like_cpp(
                        candidate.player_guid,
                        &candidate.position,
                        candidate.player_combat_reach,
                        effective_aggro_range,
                    )
                {
                    use wow_packet::ServerPacket;
                    use wow_packet::packets::movement::MonsterMoveStop;

                    creature
                        .creature
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .add_threat(candidate.player_guid, 0.0);
                    creature.sync_runtime_motion_master_like_cpp();
                    if creature.runtime_motion_master_current_kind_like_cpp()
                        == Some(wow_movement::MovementGeneratorType::Chase)
                        && let Some(stop) = creature.stop_move_spline_like_cpp()
                    {
                        use crate::map_manager::{RecipientRule, RuntimeEvent};

                        outcome.movement_interrupts += 1;
                        outcome.plan.events.push(RuntimeEvent {
                            source_guid: guid,
                            recipients: RecipientRule::NearbyVisible {
                                source_guid: guid,
                                map_id,
                                instance_id,
                                source_position: stop.position,
                                range: creature.visibility_range_like_cpp(),
                                required_3d: false,
                            },
                            packet_bytes: MonsterMoveStop {
                                mover_guid: guid,
                                current_pos: stop.position,
                                spline_id: stop.spline_id,
                            }
                            .to_bytes(),
                        });
                    }
                    outcome.aggro_starts += 1;
                    use crate::map_manager::{RecipientRule, RuntimeEvent};
                    outcome.plan.events.push(RuntimeEvent {
                        source_guid: guid,
                        recipients: RecipientRule::NearbyVisibleDurable {
                            source_guid: guid,
                            map_id,
                            instance_id,
                            source_position: creature.position(),
                            range: creature.visibility_range_like_cpp(),
                            required_3d: false,
                        },
                        packet_bytes: wow_packet::packets::combat::AttackStart {
                            attacker: guid,
                            victim: candidate.player_guid,
                        }
                        .to_bytes(),
                    });
                    outcome.commands.push(
                        crate::session::mailbox::CreatureAttackStartLikeCppCommand {
                            attacker_guid: guid,
                            victim_guid: candidate.player_guid,
                            previous_victim_guid: None,
                            map_id,
                            instance_id,
                            packet_already_broadcast: true,
                        },
                    );
                    if let Some(victim_guid) = creature.take_assistance_call_like_cpp() {
                        assistance_calls.push((
                            guid,
                            victim_guid,
                            creature.creature.unit().world().clone(),
                            creature.creature.unit().data().faction_template,
                        ));
                    }
                    break;
                }
            }
        }

        if config.family_assistance_radius > 0.0 {
            for (caller_guid, victim_guid, caller_world, caller_faction) in assistance_calls {
                let victim = map_candidates
                    .iter()
                    .find(|candidate| candidate.player_guid == victim_guid)
                    .copied();
                let victim_snapshot = owner_snapshots.get(&victim_guid);
                if victim.is_none() && victim_snapshot.is_none() {
                    continue;
                }
                let mut assistant_guids = Vec::new();
                for assistant_guid in guids.iter().copied().filter(|guid| *guid != caller_guid) {
                    let Some(assistant) =
                        manager.find_creature_mut(map_id, instance_id, assistant_guid)
                    else {
                        continue;
                    };
                    let flags = assistant.creature.unit().unit_flags_like_cpp();
                    if !assistant.is_alive()
                        || assistant.creature.is_in_combat()
                        || assistant.creature.is_in_evade_mode_like_cpp()
                        || assistant.creature.unit().has_unit_state(
                            (UnitState::STUNNED | UnitState::CONFUSED | UnitState::FLEEING).bits(),
                        )
                        || !assistant
                            .creature
                            .has_react_state(wow_entities::ReactState::Aggressive)
                        || assistant.creature.is_civilian_like_cpp()
                        || assistant
                            .creature
                            .unit()
                            .subsystems()
                            .control
                            .charmer_or_owner_guid()
                            .is_some()
                        || flags.intersects(
                            UnitFlags::NON_ATTACKABLE
                                | UnitFlags::IMMUNE_TO_NPC
                                | UnitFlags::UNINTERACTIBLE,
                        )
                        || assistant.creature.unit().data().faction_template != caller_faction
                        || !assistant.position().is_within_dist(
                            &caller_world.position(),
                            config.family_assistance_radius,
                        )
                        || terrain.as_ref().is_some_and(|terrain| {
                            !terrain.is_within_los_like_cpp(
                                u32::from(map_id),
                                &caller_world,
                                assistant.creature.unit().world(),
                            )
                        })
                        || !(victim.is_some_and(|victim| {
                            legacy_creature_aggro_candidate_is_hostile_to_creature_like_cpp(
                                assistant, victim, &config,
                            )
                            .unwrap_or(false)
                        }) || victim_snapshot.is_some_and(|victim| {
                            !victim.in_evade_mode
                                && legacy_creature_snapshot_is_hostile_to_creature_like_cpp(
                                    assistant, victim, &config,
                                )
                                .unwrap_or(false)
                        }))
                    {
                        continue;
                    }
                    assistant_guids.push(assistant_guid);
                }
                if !assistant_guids.is_empty() {
                    outcome.assistance_scheduled += assistant_guids.len();
                    if let Some(caller) =
                        manager.find_creature_mut(map_id, instance_id, caller_guid)
                    {
                        caller.schedule_assistance_like_cpp(
                            victim_guid,
                            assistant_guids,
                            config.family_assistance_delay_ms,
                        );
                    }
                }
            }
        }
    }

    outcome
}
