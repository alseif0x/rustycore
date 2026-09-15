use super::super::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GameEventObjectSpawnBucketSummaryLikeCpp {
    pub(crate) guids_seen: usize,
    pub(crate) missing_spawn_metadata: usize,
    pub(crate) represented_object_mgr_grid_additions: usize,
    pub(crate) maps_matched: usize,
    pub(crate) without_loaded_canonical_maps: usize,
    pub(crate) respawn_timers_removed: usize,
    pub(crate) respawn_timers_missing: usize,
    pub(crate) unloaded_grid_skips: usize,
    pub(crate) load_attempts: usize,
    pub(crate) loader_blocked_or_missing: usize,
    pub(crate) successful_loaded_grid_spawns: usize,
    pub(crate) legacy_creature_mirrors: usize,
    pub(crate) add_to_map_failures: usize,
    pub(crate) gameobject_not_spawned_by_default_skips: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct GameEventCreatureGameObjectSpawnSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) missing_event_creature_guids: bool,
    pub(crate) missing_event_gameobject_guids: bool,
    pub(crate) creature: GameEventObjectSpawnBucketSummaryLikeCpp,
    pub(crate) gameobject: GameEventObjectSpawnBucketSummaryLikeCpp,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventSpawnForEventSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) non_pool: GameEventCreatureGameObjectSpawnSummaryLikeCpp,
    pub(crate) pool_skipped_due_to_non_pool_bucket: bool,
    pub(crate) pool: GameEventPoolEventSpawnSummaryLikeCpp,
}

pub(crate) fn game_event_spawn_object_guid_list_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    object_type: wow_map::SpawnObjectType,
    spawn_ids: &[wow_map::SpawnId],
) -> GameEventObjectSpawnBucketSummaryLikeCpp {
    let mut summary = GameEventObjectSpawnBucketSummaryLikeCpp::default();

    for &spawn_id in spawn_ids {
        summary.guids_seen += 1;
        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(object_type, spawn_id)
        else {
            summary.missing_spawn_metadata += 1;
            continue;
        };

        // C++ anchor: GameEventMgr.cpp:1176-1180 and 1201-1204 add ObjectMgr
        // grid metadata before walking already-loaded maps. RustyCore has no
        // safe ObjectMgr grid-cell mutation in this world-server bridge, so the
        // immutable canonical SpawnStore evidence is represented by this count.
        summary.represented_object_mgr_grid_additions += 1;

        let mut maps_matched_for_spawn = 0usize;
        manager.do_for_all_maps_mut(|managed_map| {
            if managed_map.map_id() != spawn_data.map_id {
                return;
            }
            maps_matched_for_spawn += 1;
            let map = managed_map.map_mut();
            if map
                .remove_respawn_time_like_cpp(object_type, spawn_id)
                .is_some()
            {
                summary.respawn_timers_removed += 1;
            } else {
                summary.respawn_timers_missing += 1;
            }

            let cell = wow_map::cell_from_world(spawn_data.spawn_point.x, spawn_data.spawn_point.y);
            let grid = wow_map::GridCoord::new(cell.grid_x(), cell.grid_y());
            if !map.is_grid_loaded(grid) {
                summary.unloaded_grid_skips += 1;
                return;
            }

            summary.load_attempts += 1;
            let Some(records) = (match object_type {
                wow_map::SpawnObjectType::Creature => {
                    build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
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
            }) else {
                summary.loader_blocked_or_missing += 1;
                return;
            };

            if object_type == wow_map::SpawnObjectType::GameObject
                && !records
                    .primary_record
                    .game_object()
                    .is_some_and(wow_entities::GameObject::spawned_by_default)
            {
                summary.gameobject_not_spawned_by_default_skips += 1;
                return;
            }

            for pre_add_record in records.pre_add_records {
                let _ = map.add_map_object_record_to_map_like_cpp(pre_add_record);
            }
            let legacy_mirror_record = records.primary_record.creature().cloned();
            match map.add_map_object_record_to_map_like_cpp(records.primary_record) {
                Ok(_outcome) => {
                    summary.successful_loaded_grid_spawns += 1;
                    if let Some(creature) = legacy_mirror_record
                        && mirror_loaded_grid_creature_to_legacy_like_cpp(
                            legacy_manager,
                            canonical_spawn_metadata.waypoint_paths_like_cpp(),
                            creature,
                        )
                    {
                        summary.legacy_creature_mirrors += 1;
                    }
                }
                Err(_error) => {
                    summary.add_to_map_failures += 1;
                }
            }
        });
        summary.maps_matched += maps_matched_for_spawn;
        if maps_matched_for_spawn == 0 {
            summary.without_loaded_canonical_maps += 1;
        }
    }

    summary
}

pub(crate) fn game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    event_id: i16,
) -> GameEventCreatureGameObjectSpawnSummaryLikeCpp {
    let Some(creature_guids) =
        canonical_spawn_metadata.game_event_creature_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectSpawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: true,
            missing_event_gameobject_guids: false,
            creature: GameEventObjectSpawnBucketSummaryLikeCpp::default(),
            gameobject: GameEventObjectSpawnBucketSummaryLikeCpp::default(),
        };
    };

    let creature = game_event_spawn_object_guid_list_for_event_like_cpp(
        manager,
        legacy_manager,
        canonical_spawn_metadata,
        loaded_grid_creature_respawn_caches,
        wow_map::SpawnObjectType::Creature,
        creature_guids,
    );

    let Some(gameobject_guids) =
        canonical_spawn_metadata.game_event_gameobject_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectSpawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: false,
            missing_event_gameobject_guids: true,
            creature,
            gameobject: GameEventObjectSpawnBucketSummaryLikeCpp::default(),
        };
    };

    let gameobject = game_event_spawn_object_guid_list_for_event_like_cpp(
        manager,
        legacy_manager,
        canonical_spawn_metadata,
        loaded_grid_creature_respawn_caches,
        wow_map::SpawnObjectType::GameObject,
        gameobject_guids,
    );

    GameEventCreatureGameObjectSpawnSummaryLikeCpp {
        event_id,
        missing_event_creature_guids: false,
        missing_event_gameobject_guids: false,
        creature,
        gameobject,
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventPoolSpawnSummaryLikeCpp {
    pub(crate) event_pool_ids_seen: usize,
    pub(crate) missing_pool_templates: usize,
    pub(crate) invalid_template_map_ids: usize,
    pub(crate) pools_without_loaded_canonical_maps: usize,
    pub(crate) maps_matched: usize,
    pub(crate) executed_loaded_grid_respawns: usize,
    pub(crate) legacy_creature_mirrors: usize,
    pub(crate) blocked_loaded_grid_respawn_add_to_map: usize,
    pub(crate) pool_spawn_actions_skipped_unloaded_grid: usize,
    pub(crate) pool_spawn_actions_blocked_loaded_grid: usize,
    pub(crate) pool_spawn_action_load_plans: usize,
    pub(crate) pool_spawn_actions_missing_spawn_data: usize,
    pub(crate) pool_objects_removed: usize,
    pub(crate) pool_respawn_timers_removed: usize,
    pub(crate) pool_respawn_timers_missing: usize,
    pub(crate) pool_stale_index_entries: usize,
    pub(crate) pool_remove_errors: usize,
    pub(crate) pool_unsupported_action_kind: usize,
    pub(crate) blocked_pool_plan_errors: Vec<wow_map::PoolMgrPlanErrorLikeCpp>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventPoolEventSpawnSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) missing_event_pool_ids: bool,
    pub(crate) pool_summary: GameEventPoolSpawnSummaryLikeCpp,
}

impl GameEventPoolSpawnSummaryLikeCpp {
    pub(crate) fn accumulate_spawn_summary_like_cpp(
        &mut self,
        summary: &wow_map::map::ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.executed_loaded_grid_respawns += summary.executed_loaded_grid_respawns;
        self.blocked_loaded_grid_respawn_add_to_map +=
            summary.blocked_loaded_grid_respawn_add_to_map;
        self.pool_spawn_actions_skipped_unloaded_grid +=
            summary.pool_spawn_actions_skipped_unloaded_grid;
        self.pool_spawn_actions_blocked_loaded_grid +=
            summary.pool_spawn_actions_blocked_loaded_grid;
        self.pool_spawn_action_load_plans += summary.pool_spawn_action_load_plans.len();
        self.pool_spawn_actions_missing_spawn_data += summary.pool_spawn_actions_missing_spawn_data;
        self.pool_objects_removed += summary.pool_objects_removed;
        self.pool_respawn_timers_removed += summary.pool_respawn_timers_removed;
        self.pool_respawn_timers_missing += summary.pool_respawn_timers_missing;
        self.pool_stale_index_entries += summary.pool_stale_index_entries;
        self.pool_remove_errors += summary.pool_remove_errors;
        self.pool_unsupported_action_kind += summary.pool_unsupported_action_kind;
        self.blocked_pool_plan_errors
            .extend(summary.blocked_pool_plan_errors.iter().copied());
    }
}

pub(crate) fn game_event_spawn_pools_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    event_pool_ids: &[u32],
) -> GameEventPoolSpawnSummaryLikeCpp {
    let pool_mgr = canonical_spawn_metadata.pool_mgr_like_cpp();
    let mut summary = GameEventPoolSpawnSummaryLikeCpp::default();

    for &pool_id in event_pool_ids {
        summary.event_pool_ids_seen += 1;
        let Some(pool_template) = pool_mgr.pool_template_like_cpp(pool_id) else {
            summary.missing_pool_templates += 1;
            continue;
        };
        let Ok(map_id) = u32::try_from(pool_template.map_id) else {
            summary.invalid_template_map_ids += 1;
            continue;
        };

        let mut maps_matched_for_pool = 0usize;
        manager.do_for_all_maps_mut(|managed_map| {
            if managed_map.map_id() != map_id {
                return;
            }
            maps_matched_for_pool += 1;
            match managed_map
                .map_mut()
                .spawn_pool_loaded_grid_records_like_cpp(
                    pool_mgr,
                    pool_id,
                    canonical_spawn_metadata.spawn_store(),
                    |_kind, _pool_id| 0.0,
                    |_candidates, count| (0..count).collect(),
                    |map, object_type, spawn_id| match object_type {
                        wow_map::SpawnObjectType::Creature => {
                            build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
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
                ) {
                Ok(map_summary) => {
                    summary.legacy_creature_mirrors +=
                        mirror_loaded_grid_primary_records_to_legacy_like_cpp(
                            legacy_manager,
                            canonical_spawn_metadata.waypoint_paths_like_cpp(),
                            &map_summary.loaded_grid_primary_records,
                        );
                    summary.accumulate_spawn_summary_like_cpp(&map_summary);
                }
                Err(error) => summary.blocked_pool_plan_errors.push(error),
            }
        });
        summary.maps_matched += maps_matched_for_pool;
        if maps_matched_for_pool == 0 {
            summary.pools_without_loaded_canonical_maps += 1;
        }
    }

    summary
}

pub(crate) fn game_event_spawn_pools_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    event_id: i16,
) -> GameEventPoolEventSpawnSummaryLikeCpp {
    let Some(event_pool_ids) = canonical_spawn_metadata.game_event_pool_ids_like_cpp(event_id)
    else {
        return GameEventPoolEventSpawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: true,
            pool_summary: GameEventPoolSpawnSummaryLikeCpp::default(),
        };
    };

    GameEventPoolEventSpawnSummaryLikeCpp {
        event_id,
        missing_event_pool_ids: false,
        pool_summary: game_event_spawn_pools_like_cpp(
            manager,
            legacy_manager,
            canonical_spawn_metadata,
            loaded_grid_creature_respawn_caches,
            event_pool_ids,
        ),
    }
}

pub(crate) fn game_event_spawn_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    event_id: i16,
) -> GameEventSpawnForEventSummaryLikeCpp {
    let non_pool = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
        manager,
        legacy_manager,
        canonical_spawn_metadata,
        loaded_grid_creature_respawn_caches,
        event_id,
    );
    let pool_skipped_due_to_non_pool_bucket =
        non_pool.missing_event_creature_guids || non_pool.missing_event_gameobject_guids;
    let pool = if pool_skipped_due_to_non_pool_bucket {
        GameEventPoolEventSpawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: false,
            pool_summary: GameEventPoolSpawnSummaryLikeCpp::default(),
        }
    } else {
        game_event_spawn_pools_for_event_like_cpp(
            manager,
            legacy_manager,
            canonical_spawn_metadata,
            loaded_grid_creature_respawn_caches,
            event_id,
        )
    };

    GameEventSpawnForEventSummaryLikeCpp {
        event_id,
        non_pool,
        pool_skipped_due_to_non_pool_bucket,
        pool,
    }
}

pub(crate) fn apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
    managed_map: &mut wow_map::ManagedMap,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Vec<wow_map::map::SpawnGroupConditionUpdateOutcomeLikeCpp> {
    let map_id = managed_map.map_id();
    let instance_id = managed_map.instance_id();
    let difficulty_id = u32::from(managed_map.map().spawn_mode());
    let groups = canonical_spawn_metadata.spawn_group_templates_for_map_like_cpp(map_id);
    if groups.is_empty() {
        debug!(
            map_id,
            instance_id,
            difficulty_id,
            "UpdateSpawnGroupConditions loaded-grid helper found no spawn groups for map"
        );
        return Vec::new();
    }

    let group_templates = groups
        .iter()
        .map(|(_group_id, template)| *template)
        .collect::<Vec<_>>();
    let groups_evaluated = group_templates.len();
    let map_ref = ConditionMapRef::new(map_id, instance_id);
    let map_state = ConditionMapStateSnapshot {
        active_event_ids: &[],
        world_states: &[],
        difficulty_id,
        instance_data: &[],
        instance_data64: &[],
        boss_states: &[],
        scenario_step_id: None,
    };
    let outcomes = managed_map
        .map_mut()
        .apply_update_spawn_group_conditions_loaded_grid_records_like_cpp(
            group_templates,
            canonical_spawn_metadata.spawn_store(),
            |group| {
                is_spawn_group_meeting_map_conditions_like_cpp(
                    condition_store,
                    group.group_id,
                    map_ref,
                    Some(map_state),
                    &[],
                )
            },
            |map, object_type, spawn_id, force| match object_type {
                wow_map::SpawnObjectType::Creature => {
                    let _ = force;
                    // C++ `UpdateSpawnGroupConditions -> SpawnGroupSpawn(spawnGroupId)`
                    // uses default `force=false`; `wow-map` has already filtered active
                    // respawn timers before calling this loaded-grid LoadFromDB seam.
                    build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
                        map,
                        object_type,
                        spawn_id,
                        canonical_spawn_metadata,
                        loaded_grid_creature_respawn_caches,
                    )
                }
                wow_map::SpawnObjectType::GameObject => {
                    let _ = force;
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
    let applied_set_inactive = outcomes
        .iter()
        .filter(|outcome| outcome.applied_change.is_some())
        .count();
    let planned_spawn = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.action,
                wow_map::map::SpawnGroupConditionActionLikeCpp::Spawn { .. }
            )
        })
        .count();
    let planned_despawn = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.action,
                wow_map::map::SpawnGroupConditionActionLikeCpp::Despawn { .. }
            )
        })
        .count();
    debug!(
        map_id,
        instance_id,
        difficulty_id,
        groups_evaluated,
        outcomes = outcomes.len(),
        applied_set_inactive,
        planned_spawn,
        planned_despawn,
        "Applied C++ UpdateSpawnGroupConditions loaded-grid SpawnGroupSpawn helper to canonical map"
    );

    outcomes
}
