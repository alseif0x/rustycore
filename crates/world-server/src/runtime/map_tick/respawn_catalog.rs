//! Existing Catalog respawn/spawn-group APP phase, unchanged.
use super::*;

pub(super) mod prepared;

/// Execute the respawn and spawn-group condition phase for the maps admitted
/// by one split tick.
///
/// TrinityCore places this phase before `resetMarkedCells` and the
/// `ObjectUpdater` visitors (`Map.cpp:682-693`).  The plan's incarnation list
/// is the authority here: sessions run while the manager guard is released,
/// so a map recreated under the same key must not receive the predecessor's
/// respawn work.
#[allow(clippy::too_many_arguments)]
pub(super) fn canonical_map_tick_respawn_phase_like_cpp(
    manager: &mut wow_map::MapManager,
    admitted_maps: &[wow_map::MapTickParticipantLikeCpp],
    legacy_manager: Option<&SharedMapManager>,
    effective_diff_ms: u32,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> CanonicalSpawnGroupConditionTickSummaryLikeCpp {
    let mut summary = CanonicalSpawnGroupConditionTickSummaryLikeCpp::default();
    if !scheduler.update(effective_diff_ms) {
        return summary;
    }

    // C++ `Map::Update` calls `ProcessRespawns()` immediately before
    // `UpdateSpawnGroupConditions()` when `_respawnCheckTimer` expires.  The
    // represented in-memory branches and their asynchronous DB statements
    // remain unchanged; only their position in the tick moves before the
    // object visitors.
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        });
    manager.for_admitted_maps_mut_like_cpp(admitted_maps, |managed_map| {
        summary.maps_evaluated += 1;
        let map_kind = managed_map.kind();
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let map_is_instanceable = map_store
            .get(map_id)
            .is_some_and(|entry| entry.is_instanceable_like_cpp());
        let before_respawn_keys = managed_map
            .map()
            .respawn_timer_keys_like_cpp()
            .collect::<BTreeSet<_>>();
        let respawn_summary = managed_map
            .map_mut()
            .process_due_respawns_composite_loaded_grid_respawns_like_cpp(
                now_secs,
                canonical_spawn_metadata.spawn_store(),
                canonical_spawn_metadata.linked_respawns_like_cpp(),
                canonical_spawn_metadata.pool_mgr_like_cpp(),
                5,
                false,
                |_, _| false,
                |_, _| 0.0,
                |_candidates, count| (0..count).collect(),
                true,
                |map, object_type, spawn_id| match object_type {
                    wow_map::SpawnObjectType::Creature => {
                        build_loaded_grid_creature_respawn_record_like_cpp(
                            map,
                            object_type,
                            spawn_id,
                            canonical_spawn_metadata,
                            loaded_grid_creature_respawn_caches,
                        )
                    }
                    wow_map::SpawnObjectType::GameObject => {
                        build_loaded_grid_gameobject_respawn_record_like_cpp(
                            map,
                            object_type,
                            spawn_id,
                            canonical_spawn_metadata,
                            loaded_grid_creature_respawn_caches,
                        )
                    }
                    wow_map::SpawnObjectType::AreaTrigger => None,
                },
            );
        summary.respawn_deleted_inactive_spawn_group +=
            respawn_summary.deleted_inactive_spawn_group;
        summary.respawn_deleted_live_object_blocker += respawn_summary.deleted_live_object_blocker;
        for rescheduled in respawn_summary.rescheduled_linked_respawns {
            match queue_respawn_db_save_like_cpp(
                map_kind,
                map_is_instanceable,
                map_id,
                instance_id,
                rescheduled,
            ) {
                RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) => {
                    summary.respawn_db_save_queued += 1;
                    summary.respawn_db_saves.push(save);
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap => {
                    summary.respawn_db_save_skipped_non_world_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInstanceableMap => {
                    summary.respawn_db_save_skipped_instanceable_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId => {
                    summary.respawn_db_save_skipped_invalid_map_id += 1;
                }
            }
        }
        summary.respawn_processed_pool_timers += respawn_summary.processed_pool_timers;
        summary.respawn_processed_unloaded_grid_respawns +=
            respawn_summary.processed_unloaded_grid_respawns;
        summary.respawn_executed_loaded_grid_respawns +=
            respawn_summary.executed_loaded_grid_respawns;
        summary.respawn_legacy_creature_mirrors +=
            mirror_loaded_grid_primary_records_to_legacy_like_cpp(
                legacy_manager,
                canonical_spawn_metadata.waypoint_paths_like_cpp(),
                &respawn_summary.loaded_grid_primary_records,
            );
        summary.respawn_blocked_loaded_grid_respawn_loads +=
            respawn_summary.blocked_loaded_grid_respawn_loads;
        summary.respawn_blocked_loaded_grid_respawn_add_to_map +=
            respawn_summary.blocked_loaded_grid_respawn_add_to_map;
        summary.respawn_pool_update_plans += respawn_summary.pool_update_plans.len();
        summary.respawn_blocked_pool_plan_errors += respawn_summary.blocked_pool_plan_errors.len();
        summary.respawn_blocked_missing_spawn_data += respawn_summary.blocked_missing_spawn_data;
        summary.respawn_blocked_pool_runtime += respawn_summary.blocked_pool_runtime;
        summary.respawn_blocked_do_respawn_runtime += respawn_summary.blocked_do_respawn_runtime;
        summary.respawn_blocked_linked_respawn_non_future +=
            respawn_summary.blocked_linked_respawn_non_future;
        summary.respawn_blocked_unsupported_spawn_type +=
            respawn_summary.blocked_unsupported_spawn_type;

        let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
            managed_map,
            canonical_spawn_metadata,
            condition_store,
            loaded_grid_creature_respawn_caches,
        );
        summary.outcomes += outcomes.len();
        summary.applied_set_inactive += outcomes
            .iter()
            .filter(|outcome| outcome.applied_change.is_some())
            .count();
        summary.planned_spawn += outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.action,
                    wow_map::map::SpawnGroupConditionActionLikeCpp::Spawn { .. }
                )
            })
            .count();
        summary.planned_despawn += outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.action,
                    wow_map::map::SpawnGroupConditionActionLikeCpp::Despawn { .. }
                )
            })
            .count();
        for spawn in outcomes
            .iter()
            .filter_map(|outcome| outcome.spawn_outcome.as_ref())
        {
            summary.condition_spawn_executed_loaded_grid_spawns +=
                spawn.executed_loaded_grid_spawns;
            summary.condition_spawn_legacy_creature_mirrors +=
                mirror_loaded_grid_primary_records_to_legacy_like_cpp(
                    legacy_manager,
                    canonical_spawn_metadata.waypoint_paths_like_cpp(),
                    &spawn.loaded_grid_primary_records,
                );
            summary.condition_spawn_blocked_loaded_grid_spawn_loads +=
                spawn.blocked_loaded_grid_spawn_loads;
            summary.condition_spawn_blocked_loaded_grid_creature_loads +=
                spawn.blocked_loaded_grid_creature_loads;
            summary.condition_spawn_blocked_loaded_grid_gameobject_loads +=
                spawn.blocked_loaded_grid_gameobject_loads;
            summary.condition_spawn_blocked_loaded_grid_spawn_add_to_map +=
                spawn.blocked_loaded_grid_spawn_add_to_map;
            summary.condition_spawn_load_plan_count += spawn.load_plans.len();
            summary.condition_spawn_unsupported_spawn_types += spawn.unsupported_spawn_types;
            summary.condition_spawn_skipped_respawn_timer_active +=
                spawn.skipped_respawn_timer_active;
            summary.condition_spawn_skipped_live_object_active += spawn.skipped_live_object_active;
            summary.condition_spawn_skipped_unloaded_grid += spawn.skipped_unloaded_grid;
            summary.condition_spawn_skipped_difficulty_mismatch +=
                spawn.skipped_difficulty_mismatch;
        }
        for despawn in outcomes
            .iter()
            .filter_map(|outcome| outcome.despawn_outcome)
        {
            if despawn.blocked_missing_group == 0 && despawn.blocked_system_group == 0 {
                summary.despawn_executed += 1;
            }
            summary.despawn_objects_removed += despawn.objects_removed;
            summary.despawn_respawn_timers_removed += despawn.respawn_timers_removed;
            summary.despawn_blocked_missing_group += despawn.blocked_missing_group;
            summary.despawn_blocked_system_group += despawn.blocked_system_group;
            summary.despawn_unsupported_live_types += despawn.unsupported_live_despawn_types;
            summary.despawn_respawn_timer_unsupported_types +=
                despawn.respawn_timer_unsupported_types;
            summary.despawn_stale_index_entries += despawn.stale_index_entries;
            summary.despawn_remove_errors += despawn.remove_errors;
        }
        let after_respawn_keys = managed_map
            .map()
            .respawn_timer_keys_like_cpp()
            .collect::<BTreeSet<_>>();
        for &(object_type, spawn_id) in before_respawn_keys.difference(&after_respawn_keys) {
            match queue_respawn_db_delete_like_cpp(
                map_kind,
                map_is_instanceable,
                map_id,
                instance_id,
                object_type,
                spawn_id,
            ) {
                RespawnDbDeleteQueueOutcomeLikeCpp::Queued(delete) => {
                    summary.respawn_db_delete_queued += 1;
                    summary.respawn_db_deletes.push(delete);
                }
                RespawnDbDeleteQueueOutcomeLikeCpp::SkippedNonWorldMap => {
                    summary.respawn_db_delete_skipped_non_world_map += 1;
                }
                RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInstanceableMap => {
                    summary.respawn_db_delete_skipped_instanceable_map += 1;
                }
                RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInvalidMapId => {
                    summary.respawn_db_delete_skipped_invalid_map_id += 1;
                }
            }
        }
    });

    summary
}
