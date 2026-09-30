//! Legacy creature movement tick and application movement publication.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::super::creature_movement_adapter::movement_monster_spline_from_move_spline_like_cpp;
use super::*;

/// Advances domain movement, then serializes and traces the resulting packet.
/// Policy configuration and the pathfinder worker remain application-owned.
#[allow(clippy::too_many_arguments)]
pub(crate) fn step_creature_movement_like_cpp(
    creature: &mut crate::map_manager::WorldCreature,
    guid: wow_core::ObjectGuid,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&crate::map_manager::WorldMMapPathfinderWorkerLikeCpp>,
    terrain: Option<&crate::map_manager::LiveTerrainHeights>,
    chase_target: Option<crate::map_manager::ChaseTargetSnapshotLikeCpp>,
    diff_ms: u32,
) -> Option<Vec<u8>> {
    use wow_map::{CreatureMovementSource, CreatureMovementStep};
    use wow_packet::ServerPacket;
    use wow_packet::packets::movement::{MonsterMove, MonsterMoveStop};

    let movement = creature.step_movement(
        diff_ms,
        chase_target,
        terrain,
        |map_id, ignore| mmap_config.should_try_pathfinding_like_cpp(map_id, ignore),
        |query, map_id, instance_id, phase_shift| {
            resolve_creature_detour_path_like_cpp(
                mmap_pathfinder,
                guid,
                creature_path_request_like_cpp(query, map_id, instance_id, phase_shift),
            )
        },
    )?;
    match movement {
        CreatureMovementStep::Stop(stop) => Some(
            MonsterMoveStop {
                mover_guid: guid,
                current_pos: stop.position,
                spline_id: stop.spline_id,
            }
            .to_bytes(),
        ),
        CreatureMovementStep::Launch {
            source,
            from,
            spline: move_spline,
        } => {
            let source = match source {
                CreatureMovementSource::Home => "home",
                CreatureMovementSource::Random => "random",
                CreatureMovementSource::Waypoint => "waypoint",
                CreatureMovementSource::Chase => "chase",
            };
            let packet_spline = movement_monster_spline_from_move_spline_like_cpp(&move_spline);
            let pkt = MonsterMove {
                mover_guid: guid,
                current_pos: from,
                spline: packet_spline.clone(),
            };
            let bytes = pkt.to_bytes();
            trace_monster_move_packet_like_cpp(
                source,
                guid,
                creature,
                &move_spline,
                &packet_spline,
                &bytes,
            );
            Some(bytes)
        }
    }
}
/// Runs one global legacy creature-movement tick without spawning a loop.
///
/// This is the Slice 4A.3b single-shot driver body. It is gated by
/// `RuntimeTickOwner::GlobalLegacy`; with the default `Session` owner it is a
/// no-op. The lock order is explicit:
///
/// 1. take the legacy map write lock, mutate creatures, collect packet events
///    and canonical sync snapshots;
/// 2. release the legacy lock;
/// 3. sync canonical map state under its mutex;
/// 4. return a `RuntimePlan` for a caller to deliver outside all map locks.
///
/// There is no async work, no packet delivery, and no production loop here.
pub fn run_legacy_creature_movement_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&crate::map_manager::WorldMMapPathfinderWorkerLikeCpp>,
    chase_targets: &HashMap<(u16, u32, ObjectGuid), crate::map_manager::ChaseTargetSnapshotLikeCpp>,
    diff_ms: u32,
) -> LegacyCreatureMovementTickOutcomeLikeCpp {
    use crate::map_manager::{RecipientRule, RuntimeEvent, RuntimePlan, RuntimeTickOwner};
    use wow_packet::ServerPacket;

    let mut outcome = LegacyCreatureMovementTickOutcomeLikeCpp {
        skipped_owner_not_global: false,
        maps_seen: 0,
        creatures_seen: 0,
        movement_packets: 0,
        canonical_syncs: 0,
        plan: RuntimePlan { events: Vec::new() },
    };
    let mut canonical_syncs: Vec<(u32, u32, wow_core::ObjectGuid, wow_entities::Creature)> =
        Vec::new();

    {
        let mut manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
            outcome.skipped_owner_not_global = true;
            return outcome;
        }

        let map_keys = manager.active_map_keys();
        let live_terrain = manager.terrain();
        outcome.maps_seen = map_keys.len();
        for (map_id, instance_id) in map_keys {
            let guids = manager.creature_guids(map_id, instance_id);
            for guid in guids {
                // C++ `ChaseMovementGenerator` dereferences a live `Unit*`. This
                // runtime has no object accessor inside the creature step, so the
                // victim's facts are snapshotted first: players come from the
                // caller's registry snapshot, creature victims from this manager.
                // Both lookups are immutable and finish before the mutable borrow.
                let chase_target = manager
                    .find_creature(map_id, instance_id, guid)
                    .and_then(|creature| creature.creature.ai_ownership().combat_target)
                    .and_then(|target_guid| {
                        chase_targets
                            .get(&(map_id, instance_id, target_guid))
                            .copied()
                            .or_else(|| {
                                manager.find_creature(map_id, instance_id, target_guid).map(
                                    |target| crate::map_manager::ChaseTargetSnapshotLikeCpp {
                                        guid: target_guid,
                                        position: target.position(),
                                        combat_reach: target
                                            .creature
                                            .unit()
                                            .data()
                                            .combat_reach
                                            .max(0.0),
                                        in_world: target.creature.is_alive(),
                                        // Creature entities carry no liquid state;
                                        // unknown, not "dry".
                                        in_water: None,
                                    },
                                )
                            })
                    });

                let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) else {
                    continue;
                };
                outcome.creatures_seen += 1;
                let packet_bytes = step_creature_movement_like_cpp(
                    creature,
                    guid,
                    mmap_config,
                    mmap_pathfinder,
                    live_terrain.as_deref(),
                    chase_target,
                    diff_ms,
                );
                let source_position = creature.position();
                if creature.take_home_health_restored_pending_like_cpp()
                    && let Some(update) = unit_values_update_to_update_object(
                        guid,
                        map_id,
                        &creature.creature.unit().values_update(),
                    )
                {
                    outcome.plan.events.push(RuntimeEvent {
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
                    });
                }
                canonical_syncs.push((
                    u32::from(map_id),
                    instance_id,
                    guid,
                    creature.creature.clone(),
                ));
                if let Some(packet_bytes) = packet_bytes {
                    outcome.movement_packets += 1;
                    let visibility_range = creature.visibility_range_like_cpp();
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

    if let Some(canonical_map_manager) = canonical_map_manager {
        for (map_id, instance_id, guid, creature) in canonical_syncs {
            let expected_legacy_authority = creature.loot_authority_like_cpp().clone();
            let expected_legacy_stamp = expected_legacy_authority.stamp_like_cpp();
            let authority = sync_canonical_creature_entity_on_map_like_cpp(
                canonical_map_manager,
                map_id,
                instance_id,
                creature,
            );
            if let Some(authority) = authority {
                let mut legacy = legacy_map_manager
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if let Some(world_creature) =
                    legacy.find_creature_mut(map_id as u16, instance_id, guid)
                {
                    let _ = world_creature
                        .creature
                        .rebind_loot_authority_if_current_like_cpp(
                            &expected_legacy_authority,
                            expected_legacy_stamp,
                            authority,
                        );
                }
            }
            outcome.canonical_syncs += 1;
        }
    }

    outcome
}
pub(in crate::session) fn creature_melee_spell_miss_threshold_3_3_5_like_cpp() -> u32 {
    // `Unit::MeleeSpellHitResult` delegates its miss bucket to
    // `MeleeSpellMissChance`, whose victim miss chance is the constant 5.0%
    // returned by `Unit::GetUnitMissChance`. Weapon-skill and level deltas are
    // not applied by this legacy melee-spell path.
    500
}
pub(in crate::session) fn is_creature_melee_los_clear_like_cpp(
    attacker: &wow_entities::WorldObject,
    victim: &wow_entities::WorldObject,
    environment: &impl wow_entities::WorldObjectEnvironment,
) -> bool {
    wow_map::map::is_creature_melee_los_clear_like_cpp(attacker, victim, environment)
}
/// Mirror one already-committed canonical creature health/death transition
/// into the legacy owner under that owner's write lock.
///
/// Every identity and before-state comparison happens before mutation. The
/// caller therefore gets an optimistic atomic CAS at the legacy boundary: a
/// respawn/replacement, loot-authority handoff, damage/heal ABA, or prior
/// replay rejects without touching the current creature.
pub(in crate::session) fn apply_creature_melee_victim_sync_to_legacy_like_cpp(
    victim: &mut crate::map_manager::WorldCreature,
    sync: &CreatureMeleeVictimSyncStateLikeCpp,
    game_time_secs: i64,
) -> bool {
    let unit = victim.creature.unit();
    let identity = &sync.identity;
    if unit.data().health != sync.victim_health_before
        || unit.death_state() != identity.death_state_before
        || unit.health_state_revision_like_cpp() != sync.victim_health_state_revision_before
        || victim.creature.spawn_id() != identity.spawn_id
        || victim.creature.loot_lifecycle_revision_like_cpp()
            != identity.loot_lifecycle_revision_before
        || victim.creature.ai_ownership().state != identity.ai_state_before
        || !victim
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&identity.authority)
        || !victim
            .creature
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &identity.health_state_revision_authority,
            )
        || victim
            .creature
            .loot_authority_like_cpp()
            .lifecycle_like_cpp()
            == OwnedLootAuthorityLifecycle::Detached
    {
        return false;
    }

    let killed = victim
        .take_damage_before_death_state_at_game_time_like_cpp(sync.applied_damage, game_time_secs);
    if !killed && let Some(threat) = &sync.threat {
        victim.enter_combat(threat.attacker_guid);
        let combat = &mut victim.creature.unit_mut().subsystems_mut().combat;
        combat.set_in_combat_with(threat.attacker_guid, false, false);
        combat.add_threat(threat.attacker_guid, threat.delta);
    }
    if killed {
        victim.complete_death_state_after_kill_hooks_at_game_time_like_cpp(game_time_secs);
        victim.creature.unit_mut().set_health(0);
    }

    let unit = victim.creature.unit();
    let state_matches = unit.data().health == sync.victim_health_after
        && unit.death_state() == identity.death_state_after
        && victim.creature.spawn_id() == identity.spawn_id
        && victim.creature.loot_lifecycle_revision_like_cpp()
            == identity.loot_lifecycle_revision_after
        && victim.creature.ai_ownership().state == identity.ai_state_after
        && victim
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&identity.authority)
        && victim
            .creature
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &identity.health_state_revision_authority,
            );
    if !state_matches {
        return false;
    }

    victim
        .creature
        .unit_mut()
        .adopt_committed_health_state_revision_for_mirror_like_cpp(
            sync.victim_health_state_revision_after,
        );
    true
}
