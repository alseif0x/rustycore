// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical map tick orchestration.
//!
//! C++ `MapManager::Update` (`Maps/MapManager.cpp:287-318`) advances one shared
//! timer, decides `CanUnload` before updating, joins the map updates and then
//! runs `DelayedUpdate`. #787 splits that sequence where C++ drives each map's
//! world sessions (`Maps/Map.cpp:669-680`), so the coordinator can release
//! every synchronous guard for the session phase and resume the same tick.
//!
//! Separated from `runtime/map.rs` under #787, following that file's recorded
//! split direction: canonical tick orchestration apart from respawn
//! persistence projections and recipient delivery.

use super::map::{
    build_loaded_grid_creature_respawn_record_like_cpp,
    build_loaded_grid_creature_spawn_group_spawn_record_like_cpp,
    build_loaded_grid_gameobject_respawn_record_like_cpp,
};
use super::tick_summary::CanonicalSpawnGroupConditionTickSummaryLikeCpp;
use super::*;

/// One admitted map of a split canonical tick and the sessions C++ would drive
/// inside its `Map::Update` (#787).
#[derive(Debug)]
pub(crate) struct CanonicalMapSessionPassPlanLikeCpp {
    pub(crate) plan: wow_map::MapTickPlanLikeCpp,
    /// Per admitted map, its in-world players in `m_mapRefManager` order.
    pub(crate) participants: Vec<CanonicalMapSessionPassMapLikeCpp>,
}

/// One admitted map and the sessions its tick drives, with the identities
/// frozen under the guard that is released immediately afterwards (#787).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalMapSessionPassMapLikeCpp {
    pub(crate) key: wow_map::MapKey,
    /// The incarnation of the map this tick admitted; a map recreated under the
    /// same key during the pass is a different map.
    pub(crate) incarnation: u64,
    pub(crate) participants: Vec<wow_map::MapSessionPassParticipantLikeCpp>,
}

/// A process-unique identity for one canonical map coordinator.
///
/// Two coordinators would produce the same epoch sequence, so the epoch alone
/// cannot tell a session whose request it is holding. C++ has one
/// `MapManager::Update` caller and needs no such identity.
pub(crate) fn canonical_map_coordinator_id_like_cpp() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// C++ `MapManager::Update` up to the point where each map drives its world
/// sessions. The caller must release every synchronous guard before delivering
/// the session requests, and then resume the same tick with
/// [`canonical_map_tick_resume_like_cpp`].
pub(crate) fn canonical_map_tick_begin_like_cpp(
    manager: &mut wow_map::MapManager,
    diff_ms: u32,
) -> Option<CanonicalMapSessionPassPlanLikeCpp> {
    let plan = manager.begin_tick_like_cpp(diff_ms).into_started()?;
    let participants = plan
        .updated_maps_like_cpp()
        .iter()
        .map(|participant| CanonicalMapSessionPassMapLikeCpp {
            key: participant.key,
            incarnation: participant.incarnation,
            participants: manager.map_session_pass_participants_like_cpp(participant.key),
        })
        .collect();
    Some(CanonicalMapSessionPassPlanLikeCpp { plan, participants })
}

/// The rest of the same canonical tick, resumed with the diff saved at the
/// split. Runs exactly once per plan.
pub(crate) fn canonical_map_tick_resume_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    plan: wow_map::MapTickPlanLikeCpp,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
    let effective_diff_ms = plan.effective_diff_ms();
    let resumed = manager.resume_tick_with_pool_update_loaded_grid_records_context(
        plan,
        canonical_spawn_metadata.spawn_store(),
        canonical_spawn_metadata.pool_mgr_like_cpp(),
        |map, object_type, spawn_id| match object_type {
            wow_map::SpawnObjectType::GameObject => {
                build_loaded_grid_gameobject_respawn_record_like_cpp(
                    map,
                    object_type,
                    spawn_id,
                    canonical_spawn_metadata,
                    loaded_grid_creature_respawn_caches,
                )
            }
            wow_map::SpawnObjectType::Creature | wow_map::SpawnObjectType::AreaTrigger => None,
        },
    );
    if resumed != wow_map::MapTickResumeLikeCpp::Resumed {
        // The manager is not holding this tick any more: nothing was mutated and
        // no tail may run, or the summary would report phases that never ran.
        return None;
    }
    canonical_map_tick_tail_like_cpp(
        manager,
        legacy_manager,
        effective_diff_ms,
        scheduler,
        canonical_spawn_metadata,
        condition_store,
        map_store,
        loaded_grid_creature_respawn_caches,
    )
}

pub(crate) fn canonical_map_update_tick_set_inactive_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    diff_ms: u32,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
    let plan = canonical_map_tick_begin_like_cpp(manager, diff_ms)?;
    canonical_map_tick_resume_like_cpp(
        manager,
        legacy_manager,
        plan.plan,
        scheduler,
        canonical_spawn_metadata,
        condition_store,
        map_store,
        loaded_grid_creature_respawn_caches,
    )
}

#[allow(clippy::too_many_arguments)]
fn canonical_map_tick_tail_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    effective_diff_ms: u32,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
    let mut summary = CanonicalSpawnGroupConditionTickSummaryLikeCpp {
        player_visibility_refresh_intents: manager
            .take_player_visibility_refresh_intents_like_cpp(),
        ..Default::default()
    };
    manager.do_for_all_maps_mut(|managed_map| {
        summary.expired_pvp_combat_refs.extend(
            managed_map
                .last_expired_pvp_combat_refs_like_cpp()
                .iter()
                .map(|(owner, target)| {
                    (
                        managed_map.map_id(),
                        managed_map.instance_id(),
                        *owner,
                        *target,
                    )
                }),
        );
        let map_kind = managed_map.kind();
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let map_is_instanceable = map_store
            .get(map_id)
            .is_some_and(|entry| entry.is_instanceable_like_cpp());
        for info in managed_map
            .last_game_objects_update_summary()
            .respawn_db_saves
        {
            match queue_respawn_db_save_like_cpp(
                map_kind,
                map_is_instanceable,
                map_id,
                instance_id,
                info,
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
    });
    if !scheduler.update(effective_diff_ms) {
        return (!summary.respawn_db_saves.is_empty()
            || !summary.expired_pvp_combat_refs.is_empty()
            || !summary.player_visibility_refresh_intents.is_empty())
        .then_some(summary);
    }

    // C++ `Map::Update` runs `ProcessRespawns()` immediately before
    // `UpdateSpawnGroupConditions()` when `_respawnCheckTimer` expires.
    // This tick executes the safe in-memory ProcessRespawns side effects produced
    // by represented composite CheckRespawn guards: zero-delete for inactive
    // spawn-group/live-object blockers, linked-respawn future reschedules, pooled
    // timer UpdatePool plans, and the safe `DoRespawn` unloaded-grid early-return
    // branch after timer removal. DB delete/save effects are queued for async
    // execution after releasing the MapManager lock. Loaded-grid Creature
    // DB-backed loading is wired through the map-owned seam for supported
    // fixed-level and variable-level cases, including DB-backed FormationInfo
    // propagation into the bounded SearchFormation/AddCreatureToGroup seam;
    // AddToWorld ObjectAccessor/fanout, scripts/AI, vehicle runtime beyond local
    // evidence, zonescript, formation movement/combat/full CreatureGroup runtime,
    // dynamic-tree, full GameObject physical-removal lifecycle, AreaTrigger
    // runtime and full PoolMgr runtime remain gaps.
    // RustyCore does not yet expose CONFIG_RESPAWN_DYNAMIC_ESCORTNPC
    // or Creature::IsEscorted ownership here, so the bridge passes false/false.
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        });
    manager.do_for_all_maps_mut(|managed_map| {
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

    Some(summary)
}
