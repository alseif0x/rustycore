//! Legacy creature lifecycle tick, respawn and despawn.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// Runs one global legacy creature lifecycle tick without spawning a loop.
///
/// This is Slice 4A.3c.3 dormant infrastructure. It covers only the parts of
/// the legacy session creature tick that change creature existence:
///
/// - corpse removal after `corpse_despawn_at`;
/// - pushing the map-owned respawn queue;
/// - draining ready respawns and re-adding map/canonical creature state.
///
/// Packet delivery remains outside this function. Callers must fan out
/// `RefreshVisibleWorldCreaturesLikeCpp` for each `refresh_map_keys` entry so
/// every session runs its own C++-style `Player::UpdateVisibilityOf` seam.
pub fn run_legacy_creature_lifecycle_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    map_store: &wow_data::MapStore,
    now: Instant,
) -> LegacyCreatureLifecycleTickOutcomeLikeCpp {
    use crate::map_manager::{
        RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp,
        respawn_time_from_instant_like_cpp, world_creature_from_pending_respawn_like_cpp,
        world_to_grid_coords,
    };
    use std::collections::BTreeSet;

    let mut outcome = LegacyCreatureLifecycleTickOutcomeLikeCpp::default();
    let mut affected_maps = BTreeSet::new();
    let mut canonical_respawn_despawns: Vec<(u32, u32, ObjectGuid, wow_map::RespawnInfoLikeCpp)> =
        Vec::new();
    let mut canonical_plain_despawns: Vec<(u32, u32, ObjectGuid)> = Vec::new();
    let mut canonical_respawn_removes: Vec<(u32, u32, wow_map::SpawnObjectType, wow_map::SpawnId)> =
        Vec::new();
    let mut canonical_inserts: Vec<(u32, u32, wow_entities::Creature)> = Vec::new();
    // `now` is the scheduler's tick deadline and may predate a blocking-worker
    // stall. Keep it for due checks, but pair conversions to Unix game time
    // with a fresh monotonic snapshot from the same execution point.
    let conversion_now = Instant::now();
    let conversion_now_secs = unix_now();
    let legacy_map_keys = legacy_map_manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .active_map_keys();
    let persistent_world_map_keys: BTreeSet<(u16, u32)> = legacy_map_keys
        .iter()
        .copied()
        .filter(|(map_id, _)| {
            map_store
                .get(u32::from(*map_id))
                .is_some_and(|entry| !entry.is_instanceable_like_cpp())
        })
        .collect();

    {
        let mut manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
            outcome.skipped_owner_not_global = true;
            return outcome;
        }

        let map_keys = legacy_map_keys;
        outcome.maps_seen = map_keys.len();
        for (map_id, instance_id) in map_keys {
            let guids = manager.creature_guids(map_id, instance_id);
            outcome.creatures_seen += guids.len();

            // C++ saves the respawn time from JUST_DIED, not when the corpse
            // is eventually removed. Only persistent world-map spawns belong
            // in the global characters.respawn table.
            if persistent_world_map_keys.contains(&(map_id, instance_id)) {
                for guid in &guids {
                    let pending =
                        manager
                            .find_creature(map_id, instance_id, *guid)
                            .and_then(|creature| {
                                (!creature.is_alive()
                                    && creature.creature.spawn_id() != 0
                                    && creature.creature.runtime_state().save_respawn_requested)
                                    .then(|| {
                                        pending_respawn_from_world_creature_like_cpp(
                                            creature,
                                            creature.respawn_at_from_death_at_game_time_like_cpp(
                                                conversion_now,
                                                conversion_now_secs,
                                            ),
                                            map_id,
                                        )
                                    })
                            });
                    if let Some(pending) = pending {
                        if let Some(stmt) = manager.save_pending_respawn_time_like_cpp(
                            map_id,
                            instance_id,
                            &pending,
                            conversion_now,
                            conversion_now_secs,
                        ) {
                            outcome.respawn_db_mutations.push(stmt);
                        }
                        if let Some(creature) =
                            manager.find_creature_mut(map_id, instance_id, *guid)
                        {
                            creature.creature.runtime_state_mut().save_respawn_requested = false;
                        }
                    }
                }
            }

            let despawn_guids: Vec<ObjectGuid> = guids
                .iter()
                .filter(|guid| {
                    manager
                        .find_creature(map_id, instance_id, **guid)
                        .is_some_and(|creature| {
                            !creature.is_alive()
                                && creature.creature.unit().death_state()
                                    == wow_constants::DeathState::Corpse
                                && creature.corpse_despawn_due_like_cpp()
                        })
                })
                .copied()
                .collect();

            for guid in despawn_guids {
                if let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) {
                    creature.creature.clear_loot_like_cpp();
                }
                let Some(creature) = manager.remove_creature_any(map_id, instance_id, guid) else {
                    continue;
                };
                let respawn_at = creature.respawn_at_from_death_at_game_time_like_cpp(
                    conversion_now,
                    conversion_now_secs,
                );
                let pending =
                    pending_respawn_from_world_creature_like_cpp(&creature, respawn_at, map_id);
                let grid = wow_map::compute_grid_coord(pending.home_pos.x, pending.home_pos.y);
                let pending_respawn_secs = respawn_time_from_instant_like_cpp(
                    respawn_at,
                    conversion_now,
                    conversion_now_secs,
                );
                let canonical_respawn_info = wow_map::RespawnInfoLikeCpp {
                    object_type: wow_map::SpawnObjectType::Creature,
                    spawn_id: pending.spawn_id,
                    entry: pending.create_data.entry,
                    respawn_time: pending_respawn_secs,
                    grid_id: grid.get_id(),
                };
                if persistent_world_map_keys.contains(&(map_id, instance_id))
                    && creature.creature.spawn_id() != 0
                    && manager
                        .persisted_respawn_time_like_cpp(
                            map_id,
                            instance_id,
                            wow_map::SpawnObjectType::Creature,
                            pending.spawn_id,
                        )
                        .is_none_or(|stored| stored < pending_respawn_secs)
                {
                    // REP_RESPAWN is an upsert, so replace the in-memory row
                    // directly; no intermediate DEL is needed for the DB row.
                    let _ = manager.remove_persisted_respawn_time_like_cpp(
                        map_id,
                        instance_id,
                        wow_map::SpawnObjectType::Creature,
                        pending.spawn_id,
                    );
                    if let Some(stmt) = manager.save_pending_respawn_time_like_cpp(
                        map_id,
                        instance_id,
                        &pending,
                        conversion_now,
                        conversion_now_secs,
                    ) {
                        outcome.respawn_db_mutations.push(stmt);
                    }
                }
                manager.push_respawn(map_id, instance_id, pending);
                if creature.creature.spawn_id() != 0 {
                    canonical_respawn_despawns.push((
                        u32::from(map_id),
                        instance_id,
                        guid,
                        canonical_respawn_info,
                    ));
                } else {
                    canonical_plain_despawns.push((u32::from(map_id), instance_id, guid));
                }
                affected_maps.insert((map_id, instance_id));
                outcome.corpses_despawned += 1;
            }

            let ready_respawns = manager.drain_ready_respawns(map_id, instance_id, now);
            for respawn in ready_respawns {
                let guid = respawn.create_data.guid;
                if manager.find_creature(map_id, instance_id, guid).is_some()
                    || (respawn.persistent_spawn
                        && manager
                            .find_creature_guid_by_spawn_id_like_cpp(
                                map_id,
                                instance_id,
                                respawn.spawn_id,
                            )
                            .is_some())
                {
                    if respawn.persistent_spawn {
                        if let Some(stmt) = manager.remove_persisted_respawn_time_like_cpp(
                            map_id,
                            instance_id,
                            wow_map::SpawnObjectType::Creature,
                            respawn.spawn_id,
                        ) {
                            outcome.respawn_db_mutations.push(stmt);
                        }
                        canonical_respawn_removes.push((
                            u32::from(map_id),
                            instance_id,
                            wow_map::SpawnObjectType::Creature,
                            respawn.spawn_id,
                        ));
                    }
                    affected_maps.insert((map_id, instance_id));
                    continue;
                }
                let position =
                    crate::map_manager::pending_respawn_create_position_like_cpp(&respawn);
                let mut world_creature =
                    world_creature_from_pending_respawn_like_cpp(&respawn, instance_id);
                // C++ Creature::Respawn ground-snaps via UpdateAllowedPositionZ
                // (Creature.cpp:461). No-op when terrain is not wired.
                if let Some(terrain) = manager.terrain() {
                    crate::map_manager::snap_respawn_creature_to_ground_like_cpp(
                        &mut world_creature,
                        map_id,
                        &terrain,
                    );
                }
                let canonical_creature = world_creature.creature.clone();
                let (grid_x, grid_y) = world_to_grid_coords(position.x, position.y);
                if manager.add_creature(map_id, instance_id, grid_x, grid_y, world_creature) {
                    if respawn.persistent_spawn {
                        if let Some(stmt) = manager.remove_persisted_respawn_time_like_cpp(
                            map_id,
                            instance_id,
                            wow_map::SpawnObjectType::Creature,
                            respawn.spawn_id,
                        ) {
                            outcome.respawn_db_mutations.push(stmt);
                        }
                        canonical_respawn_removes.push((
                            u32::from(map_id),
                            instance_id,
                            wow_map::SpawnObjectType::Creature,
                            respawn.spawn_id,
                        ));
                    }
                    canonical_inserts.push((u32::from(map_id), instance_id, canonical_creature));
                    affected_maps.insert((map_id, instance_id));
                    outcome.respawns_processed += 1;
                }
            }
        }
    }

    if let Some(canonical_map_manager) = canonical_map_manager {
        for (map_id, instance_id, guid) in canonical_plain_despawns {
            remove_canonical_creature_map_object_on_map_like_cpp(
                canonical_map_manager,
                map_id,
                instance_id,
                guid,
            );
            outcome.canonical_removes += 1;
        }
        for (map_id, instance_id, guid, info) in canonical_respawn_despawns {
            let (respawn_added, object_removed) =
                add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp(
                    canonical_map_manager,
                    map_id,
                    instance_id,
                    guid,
                    info,
                );
            if respawn_added {
                outcome.canonical_respawn_adds += 1;
            }
            if object_removed {
                outcome.canonical_removes += 1;
            }
        }
        for (map_id, instance_id, object_type, spawn_id) in canonical_respawn_removes {
            if remove_canonical_respawn_time_on_map_like_cpp(
                canonical_map_manager,
                map_id,
                instance_id,
                object_type,
                spawn_id,
            ) {
                outcome.canonical_respawn_removes += 1;
            }
        }
        for (map_id, instance_id, creature) in canonical_inserts {
            let guid = creature.guid();
            let expected_legacy_authority = creature.loot_authority_like_cpp().clone();
            let expected_legacy_stamp = expected_legacy_authority.stamp_like_cpp();
            let authority = insert_canonical_creature_map_object_on_map_like_cpp(
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
            outcome.canonical_inserts += 1;
        }
    }

    outcome.refresh_map_keys = affected_maps.into_iter().collect();
    outcome
}
