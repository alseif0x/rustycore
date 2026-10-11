//! Creature movement phase and movement stepping.
//!
//! Moved out of the Session root under #619; run by the admitted canonical
//! executor over the canonical map (#1263 F6-8D3a-2, F6-8E).

use super::*;
use crate::creature_movement::CreatureMovementQueriesLikeCpp as _;

/// The creature movement step — `Unit::Update(p_time)`'s clock,
/// `UpdateSplineMovement` and `MotionMaster::Update` with the home, random,
/// waypoint and chase generators — written once over a borrowed canonical
/// creature (#1263 F6-8D3a-2).
///
/// This is a pure function of `creature`, `guid`, and the two mmap resources;
/// it does NOT touch any `WorldSession` state.
#[allow(clippy::too_many_arguments)]
pub(in crate::session) fn step_creature_movement_on_view_like_cpp(
    creature: &mut crate::creature_movement::CreatureMovementLikeCpp<'_>,
    guid: wow_core::ObjectGuid,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&crate::pathfinder::WorldMMapPathfinderWorkerLikeCpp>,
    terrain: Option<&crate::terrain::LiveTerrainHeights>,
    chase_target: Option<crate::creature_movement::ChaseTargetSnapshotLikeCpp>,
    diff_ms: u32,
) -> Option<Vec<u8>> {
    use wow_packet::ServerPacket;
    use wow_packet::packets::movement::{MonsterMove, MonsterMoveStop, MovementMonsterSpline};

    // C++ `Unit::Update(p_time)` gives `UpdateSplineMovement` and
    // `MotionMaster::Update` the same map-owned diff. Advance the creature's
    // logical deadline clock exactly once at that shared boundary; no wall
    // clock may independently move either side of the movement state machine.
    creature.advance_runtime_clock_like_cpp(diff_ms);

    if creature.is_alive() {
        // C++ `Unit::Update` advances `movespline` before calling
        // `i_motionMaster->Update(diff)`. The selected concrete generator below
        // therefore observes the spline's state from this same frame.
        let _ = creature.update_move_spline_like_cpp();
    }
    let current_generator = creature.tick_runtime_motion_master_like_cpp(diff_ms);

    if !creature.is_alive() {
        // Respawn ownership belongs to the global lifecycle tick. Reviving here
        // skips corpse removal, persisted timers, and destroy/create visibility,
        // leaving the client with a dead model that continues to move.
        return None;
    }

    if creature.state() == wow_entities::CreatureAiState::Returning {
        // C++ `HomeMovementGenerator<Creature>::SetTargetLocation` launches
        // `init.MoveTo(GetHomePosition())` with `generatePath = true`, so an
        // evading creature walks a navmesh route home instead of snapping there
        // (`HomeMovementGenerator.cpp:53-82`).
        let owner_ignores_pathfinding = creature
            .creature
            .unit()
            .has_unit_state(UnitState::IGNORE_PATHFINDING.bits());
        let source_map_id = creature.map_id();
        let source_instance_id = creature.instance_id();
        let phase_shift = creature.phase_shift().clone();
        let filter_context = creature.path_query_filter_context_like_cpp();
        let owner_capabilities = creature.detour_owner_capabilities_like_cpp();
        let should_try_pathfinding =
            mmap_config.should_try_pathfinding_like_cpp(source_map_id, owner_ignores_pathfinding);

        let outcome = creature.update_runtime_home_movement_like_cpp(
            should_try_pathfinding,
            terrain,
            |query| {
                resolve_creature_detour_path_like_cpp(
                    mmap_pathfinder,
                    guid,
                    creature_path_request_like_cpp(
                        query,
                        source_map_id,
                        source_instance_id,
                        &phase_shift,
                    ),
                )
            },
        );

        return match outcome {
            crate::creature_movement::ChaseTickOutcomeLikeCpp::Idle => None,
            crate::creature_movement::ChaseTickOutcomeLikeCpp::Stopped(stop) => Some(
                MonsterMoveStop {
                    mover_guid: guid,
                    current_pos: stop.position,
                    spline_id: stop.spline_id,
                }
                .to_bytes(),
            ),
            crate::creature_movement::ChaseTickOutcomeLikeCpp::Launched(from, move_spline) => {
                let packet_spline = MovementMonsterSpline::from_move_spline(&move_spline);
                let pkt = MonsterMove {
                    mover_guid: guid,
                    current_pos: from,
                    spline: packet_spline.clone(),
                };
                let bytes = pkt.to_bytes();
                trace_monster_move_packet_like_cpp(
                    "home",
                    guid,
                    creature.creature,
                    &move_spline,
                    &packet_spline,
                    &bytes,
                );
                Some(bytes)
            }
        };
    }

    if creature.state() == wow_entities::CreatureAiState::WalkingRandom
        && current_generator != Some(wow_movement::MovementGeneratorType::Random)
        && creature.movement_finished()
    {
        // Keep the legacy AI-state compatibility cleanup bounded even when a
        // stale WalkingRandom state no longer corresponds to the selected
        // MotionMaster default.
        creature.finish_move();
        creature
            .creature
            .set_ai_state(wow_entities::CreatureAiState::Idle);
        return None;
    }

    match current_generator {
        Some(wow_movement::MovementGeneratorType::Random)
            if matches!(
                creature.state(),
                wow_entities::CreatureAiState::Idle | wow_entities::CreatureAiState::WalkingRandom
            ) =>
        {
            let owner_ignores_pathfinding = creature
                .creature
                .unit()
                .has_unit_state(UnitState::IGNORE_PATHFINDING.bits());
            let source_map_id = creature.map_id();
            let source_instance_id = creature.instance_id();
            let phase_shift = creature.phase_shift().clone();
            // C++ builds the Detour filter from the owner in
            // `PathGenerator::CreateFilter`, so it must be sampled from this
            // creature rather than assumed.
            let filter_context = creature.path_query_filter_context_like_cpp();
            let owner_capabilities = creature.detour_owner_capabilities_like_cpp();
            // C++ `RandomMovementGenerator` keeps one `PathGenerator` for the
            // generator's lifetime, so its corridor is available to the next
            // query (`RandomMovementGenerator.cpp:140-143`).
            let previous_poly_refs = creature.active_random_path_poly_refs_like_cpp().to_vec();
            let should_try_pathfinding = mmap_config
                .should_try_pathfinding_like_cpp(source_map_id, owner_ignores_pathfinding);
            let movement = creature.update_default_random_movement_after_spline_like_cpp(
                diff_ms,
                should_try_pathfinding,
                terrain,
                |query| {
                    resolve_creature_detour_path_like_cpp(
                        mmap_pathfinder,
                        guid,
                        creature_path_request_like_cpp(
                            query,
                            source_map_id,
                            source_instance_id,
                            &phase_shift,
                        ),
                    )
                },
            );
            if let Some((from, move_spline)) = movement {
                let packet_spline = MovementMonsterSpline::from_move_spline(&move_spline);
                let pkt = MonsterMove {
                    mover_guid: guid,
                    current_pos: from,
                    spline: packet_spline.clone(),
                };
                let bytes = pkt.to_bytes();
                trace_monster_move_packet_like_cpp(
                    "random",
                    guid,
                    creature.creature,
                    &move_spline,
                    &packet_spline,
                    &bytes,
                );
                return Some(bytes);
            }
        }
        Some(wow_movement::MovementGeneratorType::Waypoint)
            if creature.state() == wow_entities::CreatureAiState::WalkingWaypoint =>
        {
            // C++ `WaypointMovementGenerator<Creature>::DoUpdate` advances the
            // generator from the map-owned creature update and `StartMove`
            // launches `MoveSplineInit::MoveTo(..., _generatePath)`, which
            // resolves `PathGenerator` before falling back to a direct segment.
            let owner_ignores_pathfinding = creature
                .creature
                .unit()
                .has_unit_state(UnitState::IGNORE_PATHFINDING.bits());
            let source_map_id = creature.map_id();
            let source_instance_id = creature.instance_id();
            let phase_shift = creature.phase_shift().clone();
            // Same owner-derived filter as the random generator: C++ constructs
            // one `PathGenerator` per query and always runs `CreateFilter`.
            let filter_context = creature.path_query_filter_context_like_cpp();
            let owner_capabilities = creature.detour_owner_capabilities_like_cpp();
            let should_try_pathfinding = mmap_config
                .should_try_pathfinding_like_cpp(source_map_id, owner_ignores_pathfinding);
            let (_action, launched_spline) = creature
                .update_default_waypoint_movement_after_spline_like_cpp(
                    diff_ms,
                    should_try_pathfinding,
                    terrain,
                    |query| {
                        resolve_creature_detour_path_like_cpp(
                            mmap_pathfinder,
                            guid,
                            creature_path_request_like_cpp(
                                query,
                                source_map_id,
                                source_instance_id,
                                &phase_shift,
                            ),
                        )
                    },
                );
            if let Some((from, move_spline)) = launched_spline {
                let packet_spline = MovementMonsterSpline::from_move_spline(&move_spline);
                let pkt = MonsterMove {
                    mover_guid: guid,
                    current_pos: from,
                    spline: packet_spline.clone(),
                };
                let bytes = pkt.to_bytes();
                trace_monster_move_packet_like_cpp(
                    "waypoint",
                    guid,
                    creature.creature,
                    &move_spline,
                    &packet_spline,
                    &bytes,
                );
                return Some(bytes);
            }
        }
        Some(wow_movement::MovementGeneratorType::Chase) => {
            // `MoveChase` replaces the default random/waypoint generator at the
            // top of C++ MotionMaster. With a live victim snapshot the generator
            // paths to it exactly as `ChaseMovementGenerator::Update` does;
            // without one (no accessor for this target) the superseded wander
            // spline is still stopped so neither server nor clients keep running
            // the lower-priority movement.
            let Some(target) = chase_target else {
                // The combat target vanished this tick (e.g. a player victim
                // died and dropped out of the world snapshot, or its GUID no
                // longer resolves). C++ chase `Update` returns false on
                // `!target || !target->IsInWorld()` and `MotionMaster` finalizes
                // the generator, so the runtime chase must be retired here too —
                // not just its spline stopped — or `UNIT_STATE_CHASE_MOVE` and
                // the generator would persist and re-drive toward the gone
                // target every tick.
                if let Some(stop) = creature.finalize_runtime_chase_movement_like_cpp() {
                    return Some(
                        MonsterMoveStop {
                            mover_guid: guid,
                            current_pos: stop.position,
                            spline_id: stop.spline_id,
                        }
                        .to_bytes(),
                    );
                }
                return None;
            };

            let owner_ignores_pathfinding = creature
                .creature
                .unit()
                .has_unit_state(UnitState::IGNORE_PATHFINDING.bits());
            let source_map_id = creature.map_id();
            let source_instance_id = creature.instance_id();
            let phase_shift = creature.phase_shift().clone();
            let filter_context = creature.path_query_filter_context_like_cpp();
            let owner_capabilities = creature.detour_owner_capabilities_like_cpp();
            // C++ chase keeps its `PathGenerator` between updates, so its
            // corridor is reusable (`ChaseMovementGenerator.cpp:174-175`).
            let previous_poly_refs = creature.active_chase_path_poly_refs_like_cpp().to_vec();
            let should_try_pathfinding = mmap_config
                .should_try_pathfinding_like_cpp(source_map_id, owner_ignores_pathfinding);

            let outcome = creature.update_runtime_chase_movement_like_cpp(
                diff_ms,
                target,
                should_try_pathfinding,
                terrain,
                |query| {
                    resolve_creature_detour_path_like_cpp(
                        mmap_pathfinder,
                        guid,
                        creature_path_request_like_cpp(
                            query,
                            source_map_id,
                            source_instance_id,
                            &phase_shift,
                        ),
                    )
                },
            );

            match outcome {
                crate::creature_movement::ChaseTickOutcomeLikeCpp::Idle => {}
                crate::creature_movement::ChaseTickOutcomeLikeCpp::Stopped(stop) => {
                    return Some(
                        MonsterMoveStop {
                            mover_guid: guid,
                            current_pos: stop.position,
                            spline_id: stop.spline_id,
                        }
                        .to_bytes(),
                    );
                }
                crate::creature_movement::ChaseTickOutcomeLikeCpp::Launched(from, move_spline) => {
                    let packet_spline = MovementMonsterSpline::from_move_spline(&move_spline);
                    let pkt = MonsterMove {
                        mover_guid: guid,
                        current_pos: from,
                        spline: packet_spline.clone(),
                    };
                    let bytes = pkt.to_bytes();
                    trace_monster_move_packet_like_cpp(
                        "chase",
                        guid,
                        creature.creature,
                        &move_spline,
                        &packet_spline,
                        &bytes,
                    );
                    return Some(bytes);
                }
            }
        }
        _ => {}
    }
    None
}
/// The creature movement phase over the admitted canonical store: for every
/// creature, the chase victim snapshot, the movement step and its publication
/// (the reached-home values update first, then the movement packet).
///
/// C++ `ChaseMovementGenerator` dereferences a live `Unit*`. The creature step
/// has no object accessor, so the victim's facts are snapshotted first:
/// players come from the caller's snapshot, creature victims from the store's
/// map. Both lookups are immutable and finish before the mutable borrow.
pub(in crate::session) fn run_creature_movement_phase_on_store_like_cpp(
    store: &mut AdmittedCanonicalCreatureStoreLikeCpp<'_>,
    terrain: Option<&crate::terrain::LiveTerrainHeights>,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&crate::pathfinder::WorldMMapPathfinderWorkerLikeCpp>,
    chase_targets: &HashMap<
        (u16, u32, ObjectGuid),
        crate::creature_movement::ChaseTargetSnapshotLikeCpp,
    >,
    diff_ms: u32,
    outcome: &mut CreatureMovementTickOutcomeLikeCpp,
) {
    use crate::{runtime_plan::RecipientRule, runtime_plan::RuntimeEvent};
    use wow_packet::ServerPacket;

    let map_keys = store.phase_map_keys_like_cpp();
    outcome.maps_seen = map_keys.len();
    for (map_id, instance_id) in map_keys {
        for guid in store.phase_creature_guids_like_cpp(map_id, instance_id) {
            let chase_target = store
                .phase_creature_like_cpp(map_id, instance_id, guid)
                .and_then(|creature| creature.ai_ownership().combat_target)
                .and_then(|target_guid| {
                    chase_targets
                        .get(&(map_id, instance_id, target_guid))
                        .copied()
                        .or_else(|| {
                            store
                                .phase_map_creature_like_cpp(map_id, instance_id, target_guid)
                                .map(|target| {
                                    crate::creature_movement::ChaseTargetSnapshotLikeCpp {
                                        guid: target_guid,
                                        position: target.position(),
                                        combat_reach: target.unit().data().combat_reach.max(0.0),
                                        in_world: target.is_alive(),
                                        // Creature entities carry no liquid state;
                                        // unknown, not "dry".
                                        in_water: None,
                                    }
                                })
                        })
                });

            let Some(mut creature) =
                store.phase_creature_movement_like_cpp(map_id, instance_id, guid)
            else {
                continue;
            };
            outcome.creatures_seen += 1;
            let packet_bytes = step_creature_movement_on_view_like_cpp(
                &mut creature,
                guid,
                mmap_config,
                mmap_pathfinder,
                terrain,
                chase_target,
                diff_ms,
            );
            let source_position = creature.position();
            let home_health_restore = if creature
                .runtime_like_cpp_mut()
                .take_home_health_restored_pending_like_cpp()
            {
                unit_values_update_to_update_object(guid, map_id, &creature.unit().values_update())
                    .map(|update| RuntimeEvent {
                        source_guid: guid,
                        recipients: RecipientRule::NearbyVisibleDurable {
                            source_guid: guid,
                            map_id,
                            instance_id,
                            source_position,
                            range: creature.visibility_range_like_cpp(),
                            required_3d: false,
                        },
                        packet_bytes: update.to_bytes(),
                    })
            } else {
                None
            };
            let visibility_range = creature.visibility_range_like_cpp();
            if let Some(event) = home_health_restore {
                outcome.plan.events.push(event);
            }
            if let Some(packet_bytes) = packet_bytes {
                outcome.movement_packets += 1;
                outcome.plan.events.push(RuntimeEvent {
                    source_guid: guid,
                    recipients: RecipientRule::NearbyVisible {
                        source_guid: guid,
                        map_id,
                        instance_id,
                        source_position,
                        range: visibility_range,
                        required_3d: false,
                    },
                    packet_bytes,
                });
            }
        }
    }
}
#[cfg(test)]
pub(in crate::session) fn creature_melee_spell_miss_threshold_3_3_5_like_cpp() -> u32 {
    wow_world_entities::creature_spell_publication::creature_melee_spell_miss_threshold_3_3_5_like_cpp()
}
pub(in crate::session) fn is_creature_melee_los_clear_like_cpp(
    attacker: &wow_entities::WorldObject,
    victim: &wow_entities::WorldObject,
    environment: &impl wow_entities::WorldObjectEnvironment,
) -> bool {
    attacker.is_within_los_in_map(
        victim,
        environment,
        wow_entities::LineOfSightOptions::default(),
    )
}
pub(in crate::session) fn apply_creature_melee_damage_to_canonical_player_on_map_like_cpp(
    canonical_map_manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    attacker_position: Position,
    attacker_combat_reach: f32,
    attacker_can_state_update: bool,
    victim_guid: ObjectGuid,
    damage: Option<u32>,
    wire_health_before: Option<u64>,
) -> CreatureMeleeApplyResultLikeCpp {
    let Some(managed) = canonical_map_manager.find_map_mut(map_id, instance_id) else {
        return CreatureMeleeApplyResultLikeCpp::MissingVictim;
    };

    let (health_before, target_level) = {
        let map = managed.map();
        let Some(victim) = map.get_typed_player(victim_guid) else {
            return CreatureMeleeApplyResultLikeCpp::MissingVictim;
        };

        let victim_position = victim.unit().world().position();
        let victim_data = victim.unit().data();
        let victim_combat_reach = victim_data.combat_reach;
        if !is_within_melee_range_like_cpp(
            attacker_position,
            attacker_combat_reach,
            victim_position,
            victim_combat_reach,
        ) {
            return CreatureMeleeApplyResultLikeCpp::OutOfRange;
        }
        if !is_within_target_boundary_radius_like_cpp(
            attacker_position,
            attacker_combat_reach,
            victim_position,
            victim_combat_reach,
            victim_data.bounding_radius,
        ) && !is_unit_facing_target_for_melee_like_cpp(attacker_position, victim_position)
        {
            return CreatureMeleeApplyResultLikeCpp::BadFacing;
        }
        if (!victim.unit().is_alive() || victim.unit().data().health == 0)
            && wire_health_before.is_none()
        {
            return CreatureMeleeApplyResultLikeCpp::VictimNotAlive;
        }
        if !attacker_can_state_update {
            return CreatureMeleeApplyResultLikeCpp::AttackerStateRejected;
        }
        // C++ `Unit::AttackerStateUpdate` requires a real attacker and checks
        // LOS before removing attacking-interrupt auras or calculating damage.
        let Some(attacker_validation) = map.with_creature_like_cpp(attacker_guid, |attacker| {
            if !attacker.is_alive() {
                return Err(CreatureMeleeApplyResultLikeCpp::AttackerUnavailable);
            }
            if !is_creature_melee_los_clear_like_cpp(
                attacker.unit().world(),
                victim.unit().world(),
                map,
            ) {
                return Err(CreatureMeleeApplyResultLikeCpp::LosRejected);
            }
            Ok(())
        }) else {
            return CreatureMeleeApplyResultLikeCpp::AttackerUnavailable;
        };
        if let Err(result) = attacker_validation {
            return result;
        }

        (
            victim.unit().data().health,
            victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8,
        )
    };

    let Some(damage) = damage else {
        return CreatureMeleeApplyResultLikeCpp::Ready;
    };

    let Some(victim) = managed.map_mut().get_typed_player_mut(victim_guid) else {
        return CreatureMeleeApplyResultLikeCpp::MissingVictim;
    };
    let health_after = health_before.saturating_sub(u64::from(damage));
    victim.unit_mut().set_health(health_after);
    if health_after == 0 {
        // The map owner commits the authoritative lethal transition. Session
        // delivery is presentation-only and may legitimately be skipped after
        // logout or a map transfer.
        victim
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
        victim.unit_mut().set_health(0);
    }
    let health_state_revision_after = victim.unit().health_state_revision_like_cpp();
    let wire_health_before = wire_health_before.unwrap_or(health_before);
    let over_damage = if u64::from(damage) >= wire_health_before {
        u64::from(damage)
            .saturating_sub(wire_health_before)
            .min(i32::MAX as u64) as i32
    } else {
        -1
    };
    CreatureMeleeApplyResultLikeCpp::Hit {
        victim_health_after: health_after,
        victim_health_state_revision_after: health_state_revision_after,
        over_damage,
        target_level,
        events: Vec::new(),
    }
}
pub(in crate::session) fn apply_creature_melee_damage_to_canonical_creature_on_map_like_cpp(
    canonical_map_manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    attacker_position: Position,
    attacker_combat_reach: f32,
    attacker_can_state_update: bool,
    victim_guid: ObjectGuid,
    damage: Option<u32>,
    // C++ `CalcDamageInfo`'s `(HitInfo, TargetState, Blocked)` when the caller
    // already rolled the attack table; `None` keeps this bridge's normal-hit
    // presentation.
    outcome_presentation: Option<(u32, u8, i32)>,
    absorbed: u32,
    wire_health_before: Option<u64>,
    represented_damage_done: Option<u32>,
    threat_plan: super::creature_melee_threat::CreatureDamageThreatPlanLikeCpp,
) -> CreatureMeleeApplyResultLikeCpp {
    use wow_packet::ServerPacket;
    use wow_packet::packets::combat::{
        AttackerStateUpdate, HIT_INFO_AFFECTS_VICTIM, HIT_INFO_FAKE_DAMAGE, VICTIM_STATE_HIT,
    };

    let Some(managed) = canonical_map_manager.find_map_mut(map_id, instance_id) else {
        return CreatureMeleeApplyResultLikeCpp::MissingVictim;
    };

    let (health_before, target_level, damage, damage_done, applied_damage, hit_info) = match {
        let map = managed.map();
        map.with_creature_like_cpp(victim_guid, |victim| {
            let victim_position = victim.unit().world().position();
            let victim_data = victim.unit().data();
            let victim_combat_reach = victim_data.combat_reach;
            if !is_within_melee_range_like_cpp(
                attacker_position,
                attacker_combat_reach,
                victim_position,
                victim_combat_reach,
            ) {
                return Err(CreatureMeleeApplyResultLikeCpp::OutOfRange);
            }
            if !is_within_target_boundary_radius_like_cpp(
                attacker_position,
                attacker_combat_reach,
                victim_position,
                victim_combat_reach,
                victim_data.bounding_radius,
            ) && !is_unit_facing_target_for_melee_like_cpp(attacker_position, victim_position)
            {
                return Err(CreatureMeleeApplyResultLikeCpp::BadFacing);
            }
            if !victim.is_alive() && wire_health_before.is_none() {
                return Err(CreatureMeleeApplyResultLikeCpp::VictimNotAlive);
            }
            if !attacker_can_state_update {
                return Err(CreatureMeleeApplyResultLikeCpp::AttackerStateRejected);
            }
            let Some(attacker_validation) = map.with_creature_like_cpp(attacker_guid, |attacker| {
                if !attacker.is_alive() {
                    return Err(CreatureMeleeApplyResultLikeCpp::AttackerUnavailable);
                }
                if !is_creature_melee_los_clear_like_cpp(
                    attacker.unit().world(),
                    victim.unit().world(),
                    map,
                ) {
                    return Err(CreatureMeleeApplyResultLikeCpp::LosRejected);
                }
                Ok(attacker.is_charmed_owned_by_player_or_player_like_cpp())
            }) else {
                return Err(CreatureMeleeApplyResultLikeCpp::AttackerUnavailable);
            };
            let attacker_is_player_controlled = attacker_validation?;

            let Some(damage) = damage else {
                return Err(CreatureMeleeApplyResultLikeCpp::Ready);
            };
            let damage_done = represented_damage_done.unwrap_or_else(|| {
                victim.calculate_damage_for_sparring_like_cpp(
                    true,
                    attacker_is_player_controlled,
                    damage,
                )
            });
            // `AttackerStateUpdate` already carries the raw damage. C++ applies
            // the unkillable Creature clamp later inside `DealDamage`, after
            // sparring and the share loop, so only the health transition uses
            // this reduced amount.
            let applied_damage = victim
                .damage_after_unkillable_gate_like_cpp(attacker_guid == victim_guid, damage_done);
            let mut hit_info =
                outcome_presentation.map_or(HIT_INFO_AFFECTS_VICTIM, |(info, _, _)| info);
            if victim.should_fake_damage_from_like_cpp(true, attacker_is_player_controlled) {
                hit_info |= HIT_INFO_FAKE_DAMAGE;
            }

            Ok((
                victim.unit().data().health,
                victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8,
                damage,
                damage_done,
                applied_damage,
                hit_info,
            ))
        })
    } {
        Some(Ok(prepared)) => prepared,
        Some(Err(result)) => return result,
        None => return CreatureMeleeApplyResultLikeCpp::MissingVictim,
    };

    let Some(victim) = managed.map_mut().get_typed_creature_mut(victim_guid) else {
        return CreatureMeleeApplyResultLikeCpp::MissingVictim;
    };
    let killed = victim.apply_ai_damage_before_death_state_at_game_time_like_cpp(
        applied_damage,
        u64::from(crate::session::game_time_ms_like_cpp()),
        wow_entities::game_time_secs_like_cpp(),
    );
    if killed {
        victim.set_death_state_runtime(
            wow_constants::DeathState::JustDied,
            wow_entities::game_time_secs_like_cpp(),
        );
        victim.unit_mut().set_health(0);
    }
    let health_after = victim.unit().data().health;
    let health_state_revision_after = victim.unit().health_state_revision_like_cpp();
    // C++ returns before `AddThreat` when sparring reduced `damageDone` to
    // zero. `UNKILLABLE` clamps only the later `damageTaken`, so that path
    // still reaches `AddThreat(0)` and establishes combat references.
    if !killed && damage_done != 0 {
        let _ = super::creature_melee_threat::apply_creature_damage_threat_on_map_like_cpp(
            managed.map_mut(),
            victim_guid,
            attacker_guid,
            applied_damage,
            threat_plan,
        );
    }
    // C++ serializes AttackerStateUpdate before DealMeleeDamage applies the
    // creature sparring clamp. Its overkill field therefore uses the raw wire
    // damage against pre-hit health, not the post-sparring applied damage.
    let wire_health_before = wire_health_before.unwrap_or(health_before);
    let over_damage = if u64::from(damage) >= wire_health_before {
        u64::from(damage)
            .saturating_sub(wire_health_before)
            .min(i32::MAX as u64) as i32
    } else {
        -1
    };
    let values_update = managed
        .map()
        .with_creature_like_cpp(victim_guid, |victim| victim.unit().values_update())
        .expect("the just-mutated creature remains in the canonical map");

    let mut events = Vec::new();
    events.push(RuntimeEvent {
        source_guid: attacker_guid,
        recipients: RecipientRule::MapBroadcastVisible {
            map_id: map_id as u16,
            instance_id,
        },
        packet_bytes: AttackerStateUpdate {
            attacker: attacker_guid,
            victim: victim_guid,
            hit_info,
            damage: damage.min(i32::MAX as u32) as i32,
            original_damage: damage.min(i32::MAX as u32) as i32,
            over_damage,
            blocked: outcome_presentation.map_or(0, |(_, _, blocked)| blocked.max(0)),
            absorbed: absorbed.min(i32::MAX as u32) as i32,
            victim_state: outcome_presentation.map_or(VICTIM_STATE_HIT, |(_, state, _)| state),
            school_mask: 1,
            target_level,
            expansion: 2,
        }
        .to_bytes(),
    });
    if let Some(update) =
        unit_values_update_to_update_object(victim_guid, map_id as u16, &values_update)
    {
        events.push(RuntimeEvent {
            source_guid: victim_guid,
            recipients: RecipientRule::MapBroadcastVisible {
                map_id: map_id as u16,
                instance_id,
            },
            packet_bytes: update.to_bytes(),
        });
    }

    CreatureMeleeApplyResultLikeCpp::Hit {
        victim_health_after: health_after,
        victim_health_state_revision_after: health_state_revision_after,
        over_damage,
        target_level,
        events,
    }
}
