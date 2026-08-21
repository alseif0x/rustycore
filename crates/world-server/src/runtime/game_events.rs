//! Canonical game-event map updates and player fanout.

use super::*;

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventPoolUnspawnSummaryLikeCpp {
    pub(crate) event_pool_ids_seen: usize,
    pub(crate) missing_pool_templates: usize,
    pub(crate) invalid_template_map_ids: usize,
    pub(crate) pools_without_loaded_canonical_maps: usize,
    pub(crate) maps_matched: usize,
    pub(crate) pool_objects_removed: usize,
    pub(crate) pool_respawn_timers_removed: usize,
    pub(crate) pool_respawn_timers_missing: usize,
    pub(crate) pool_stale_index_entries: usize,
    pub(crate) pool_remove_errors: usize,
    pub(crate) pool_unsupported_action_kind: usize,
    pub(crate) blocked_pool_plan_errors: Vec<wow_map::PoolMgrPlanErrorLikeCpp>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventPoolEventUnspawnSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) missing_event_pool_ids: bool,
    pub(crate) pool_summary: GameEventPoolUnspawnSummaryLikeCpp,
}

impl GameEventPoolUnspawnSummaryLikeCpp {
    pub(crate) fn accumulate_despawn_summary_like_cpp(
        &mut self,
        summary: &wow_map::map::ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
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

pub(crate) fn game_event_unspawn_pools_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_pool_ids: &[u32],
) -> GameEventPoolUnspawnSummaryLikeCpp {
    let pool_mgr = canonical_spawn_metadata.pool_mgr_like_cpp();
    let mut summary = GameEventPoolUnspawnSummaryLikeCpp::default();

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
                .despawn_pool_safe_map_actions_like_cpp(pool_mgr, pool_id, true)
            {
                Ok(map_summary) => summary.accumulate_despawn_summary_like_cpp(&map_summary),
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

pub(crate) fn game_event_unspawn_pools_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: i16,
) -> GameEventPoolEventUnspawnSummaryLikeCpp {
    let Some(event_pool_ids) = canonical_spawn_metadata.game_event_pool_ids_like_cpp(event_id)
    else {
        return GameEventPoolEventUnspawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: true,
            pool_summary: GameEventPoolUnspawnSummaryLikeCpp::default(),
        };
    };

    GameEventPoolEventUnspawnSummaryLikeCpp {
        event_id,
        missing_event_pool_ids: false,
        pool_summary: game_event_unspawn_pools_like_cpp(
            manager,
            canonical_spawn_metadata,
            event_pool_ids,
        ),
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GameEventObjectUnspawnBucketSummaryLikeCpp {
    pub(crate) guids_seen: usize,
    pub(crate) skipped_active_in_other_event: usize,
    pub(crate) missing_spawn_metadata: usize,
    pub(crate) represented_object_mgr_grid_removals: usize,
    pub(crate) maps_matched: usize,
    pub(crate) without_loaded_canonical_maps: usize,
    pub(crate) respawn_timers_removed: usize,
    pub(crate) respawn_timers_missing: usize,
    pub(crate) live_objects_queued: usize,
    pub(crate) duplicate_queue_attempts: usize,
    pub(crate) stale_index_entries: usize,
    pub(crate) remove_errors: usize,
    pub(crate) unsupported_live_despawn_type: usize,
}

impl GameEventObjectUnspawnBucketSummaryLikeCpp {
    pub(crate) fn accumulate_despawn_outcome_like_cpp(
        &mut self,
        outcome: wow_map::map::DespawnAllBySpawnIdOutcomeLikeCpp,
    ) {
        self.live_objects_queued += outcome.queued;
        self.duplicate_queue_attempts += outcome.duplicates;
        self.stale_index_entries += outcome.stale_index_entries;
        self.remove_errors += outcome.remove_errors;
        self.unsupported_live_despawn_type += outcome.unsupported_live_despawn_type;
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) missing_event_creature_guids: bool,
    pub(crate) missing_event_gameobject_guids: bool,
    pub(crate) creature: GameEventObjectUnspawnBucketSummaryLikeCpp,
    pub(crate) gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventUnspawnForEventSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) non_pool: GameEventCreatureGameObjectUnspawnSummaryLikeCpp,
    pub(crate) pool_skipped_due_to_non_pool_bucket: bool,
    pub(crate) pool: GameEventPoolEventUnspawnSummaryLikeCpp,
}

pub(crate) fn game_event_guid_is_active_in_other_event_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    if event_id <= 0 {
        return false;
    }

    active_event_ids.iter().copied().any(|active_event_id| {
        if active_event_id == event_id as u16 {
            return false;
        }
        let Ok(active_event_id) = i16::try_from(active_event_id) else {
            return false;
        };
        let active_guids = match object_type {
            wow_map::SpawnObjectType::Creature => {
                canonical_spawn_metadata.game_event_creature_guids_like_cpp(active_event_id)
            }
            wow_map::SpawnObjectType::GameObject => {
                canonical_spawn_metadata.game_event_gameobject_guids_like_cpp(active_event_id)
            }
            wow_map::SpawnObjectType::AreaTrigger => None,
        };
        active_guids.is_some_and(|guids| guids.contains(&spawn_id))
    })
}

pub(crate) fn game_event_unspawn_object_guid_list_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
    object_type: wow_map::SpawnObjectType,
    spawn_ids: &[wow_map::SpawnId],
) -> GameEventObjectUnspawnBucketSummaryLikeCpp {
    let mut summary = GameEventObjectUnspawnBucketSummaryLikeCpp::default();

    for &spawn_id in spawn_ids {
        summary.guids_seen += 1;
        if game_event_guid_is_active_in_other_event_like_cpp(
            canonical_spawn_metadata,
            active_event_ids,
            event_id,
            object_type,
            spawn_id,
        ) {
            summary.skipped_active_in_other_event += 1;
            continue;
        }

        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(object_type, spawn_id)
        else {
            summary.missing_spawn_metadata += 1;
            continue;
        };

        // C++ anchor: GameEventMgr.cpp:1246-1327 removes ObjectMgr grid metadata
        // before walking loaded maps. RustyCore has no safe ObjectMgr mutation here,
        // so this is represented as a count only and SpawnStore remains immutable.
        summary.represented_object_mgr_grid_removals += 1;

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
            let despawn = map.despawn_all_by_spawn_id_like_cpp(object_type, spawn_id);
            summary.accumulate_despawn_outcome_like_cpp(despawn);
        });

        summary.maps_matched += maps_matched_for_spawn;
        if maps_matched_for_spawn == 0 {
            summary.without_loaded_canonical_maps += 1;
        }
    }

    summary
}

pub(crate) fn game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
) -> GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
    let Some(creature_guids) =
        canonical_spawn_metadata.game_event_creature_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: true,
            missing_event_gameobject_guids: false,
            creature: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
            gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
        };
    };

    let creature = game_event_unspawn_object_guid_list_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
        wow_map::SpawnObjectType::Creature,
        creature_guids,
    );

    let Some(gameobject_guids) =
        canonical_spawn_metadata.game_event_gameobject_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: false,
            missing_event_gameobject_guids: true,
            creature,
            gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
        };
    };

    let gameobject = game_event_unspawn_object_guid_list_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
        wow_map::SpawnObjectType::GameObject,
        gameobject_guids,
    );

    GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
        event_id,
        missing_event_creature_guids: false,
        missing_event_gameobject_guids: false,
        creature,
        gameobject,
    }
}

pub(crate) fn game_event_unspawn_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
) -> GameEventUnspawnForEventSummaryLikeCpp {
    let non_pool = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
    );
    let pool_skipped_due_to_non_pool_bucket =
        non_pool.missing_event_creature_guids || non_pool.missing_event_gameobject_guids;
    let pool = if pool_skipped_due_to_non_pool_bucket {
        GameEventPoolEventUnspawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: false,
            pool_summary: GameEventPoolUnspawnSummaryLikeCpp::default(),
        }
    } else {
        game_event_unspawn_pools_for_event_like_cpp(manager, canonical_spawn_metadata, event_id)
    };

    GameEventUnspawnForEventSummaryLikeCpp {
        event_id,
        non_pool,
        pool_skipped_due_to_non_pool_bucket,
        pool,
    }
}

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

pub(crate) fn mirror_loaded_grid_creature_to_legacy_like_cpp(
    legacy_manager: Option<&SharedMapManager>,
    waypoint_paths: &spawn_store_loader::WaypointPathStoreLikeCpp,
    creature: wow_entities::Creature,
) -> bool {
    let Some(legacy_manager) = legacy_manager else {
        return false;
    };
    let Ok(map_id) = u16::try_from(creature.unit().world().map_id()) else {
        warn!(
            guid = ?creature.guid(),
            map_id = creature.unit().world().map_id(),
            "C++ AddToMap legacy mirror skipped: map id does not fit legacy MapManager key"
        );
        return false;
    };
    let instance_id = creature.unit().world().instance_id();
    let guid = creature.guid();
    let entry = creature.entry();
    let canonical_level = creature.level();
    let canonical_health = creature.current_health();
    let canonical_max_health = creature.max_health();
    let metadata = creature.lifecycle_metadata();
    let spawn_id = metadata.spawn_id;
    let selected_level = metadata.selected_level;
    let selected_display_id = metadata.selected_display_id;
    let position = creature.position();
    let (grid_x, grid_y) = wow_world::map_manager::world_to_grid_coords(position.x, position.y);
    let world_creature = wow_world::map_manager::WorldCreature::from_loaded_grid_canonical_like_cpp(
        creature,
        |path_id| waypoint_paths.get(path_id).cloned(),
    );
    if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
        info!(
            ?guid,
            entry,
            spawn_id,
            selected_level,
            selected_display_id,
            canonical_level,
            canonical_health,
            canonical_max_health,
            legacy_level = world_creature.level(),
            legacy_health = world_creature.current_hp(),
            legacy_max_health = world_creature.max_hp(),
            create_level = world_creature.create_data.level,
            create_health = world_creature.create_data.health,
            create_max_health = world_creature.create_data.max_health,
            "RUST_CREATURE_MIRROR loaded_grid_canonical_to_legacy"
        );
    }

    let mut guard = legacy_manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.find_creature(map_id, instance_id, guid).is_some() {
        return false;
    }
    guard.add_creature(map_id, instance_id, grid_x, grid_y, world_creature)
}

pub(crate) fn mirror_loaded_grid_primary_records_to_legacy_like_cpp(
    legacy_manager: Option<&SharedMapManager>,
    waypoint_paths: &spawn_store_loader::WaypointPathStoreLikeCpp,
    records: &[wow_entities::MapObjectRecord],
) -> usize {
    records
        .iter()
        .filter_map(|record| record.creature().cloned())
        .filter(|creature| {
            mirror_loaded_grid_creature_to_legacy_like_cpp(
                legacy_manager,
                waypoint_paths,
                creature.clone(),
            )
        })
        .count()
}

pub(crate) fn player_visible_grid_coords_like_cpp(
    position: Position,
    visibility_range: f32,
) -> Vec<wow_map::GridCoord> {
    let center_cell = wow_map::cell_from_world(position.x, position.y);
    let center_grid = wow_map::GridCoord::new(center_cell.grid_x(), center_cell.grid_y());
    let visible_area =
        wow_map::calculate_cell_area_like_cpp(position.x, position.y, visibility_range);
    let mut seen = BTreeSet::new();
    let mut grids = Vec::new();

    let mut push_grid = |grid: wow_map::GridCoord| {
        if seen.insert((grid.x_coord, grid.y_coord)) {
            grids.push(grid);
        }
    };

    // C++ visits the standing cell first (`CellImpl.h:105-107`). Keep the
    // player's own NGrid first, then cover any adjacent NGrids touched by the
    // visible cell area.
    push_grid(center_grid);
    for cell_x in visible_area.low_bound.x_coord..=visible_area.high_bound.x_coord {
        for cell_y in visible_area.low_bound.y_coord..=visible_area.high_bound.y_coord {
            let (grid, _, _) = wow_map::cell_to_grid_local(wow_map::CellCoord::new(cell_x, cell_y));
            push_grid(grid);
        }
    }

    grids
}

pub(crate) fn materialize_loaded_player_grid_records_like_cpp(
    map: &mut wow_map::Map,
    legacy_manager: &SharedMapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    map_id: u32,
    grid: wow_map::GridCoord,
    outcome: &mut wow_world::session::PlayerGridLoadOutcomeLikeCpp,
) {
    let spawn_mode = map.spawn_mode();
    let mut creature_spawn_ids = BTreeSet::new();
    let mut gameobject_spawn_ids = BTreeSet::new();
    let mut area_trigger_spawn_ids = BTreeSet::new();
    if let Some(ngrid) = map.get_ngrid(grid) {
        ngrid.visit_all_grids(|local_cell| {
            let Some(cell_guids) = canonical_spawn_metadata.spawn_store().cell_object_guids(
                map_id,
                spawn_mode,
                local_cell.cell_coord().get_id(),
            ) else {
                return;
            };
            creature_spawn_ids.extend(cell_guids.creatures.iter().copied());
            gameobject_spawn_ids.extend(cell_guids.gameobjects.iter().copied());
            area_trigger_spawn_ids.extend(cell_guids.area_triggers.iter().copied());
        });
    }

    for (object_type, spawn_id) in creature_spawn_ids
        .into_iter()
        .map(|spawn_id| (wow_map::SpawnObjectType::Creature, spawn_id))
        .chain(
            gameobject_spawn_ids
                .into_iter()
                .map(|spawn_id| (wow_map::SpawnObjectType::GameObject, spawn_id)),
        )
        .chain(
            area_trigger_spawn_ids
                .into_iter()
                .map(|spawn_id| (wow_map::SpawnObjectType::AreaTrigger, spawn_id)),
        )
    {
        let already_loaded_creature = match object_type {
            wow_map::SpawnObjectType::Creature => {
                map.get_creature_by_spawn_id_like_cpp(spawn_id).cloned()
            }
            wow_map::SpawnObjectType::GameObject | wow_map::SpawnObjectType::AreaTrigger => None,
        };
        let already_loaded = match object_type {
            wow_map::SpawnObjectType::Creature => already_loaded_creature.is_some(),
            wow_map::SpawnObjectType::GameObject => {
                map.get_gameobject_by_spawn_id_like_cpp(spawn_id).is_some()
            }
            wow_map::SpawnObjectType::AreaTrigger => map
                .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
                .is_some(),
        };
        if already_loaded {
            outcome.skipped_already_loaded += 1;
            if let Some(creature) = already_loaded_creature
                && mirror_loaded_grid_creature_to_legacy_like_cpp(
                    Some(legacy_manager),
                    canonical_spawn_metadata.waypoint_paths_like_cpp(),
                    creature,
                )
            {
                outcome.legacy_creature_mirrors += 1;
            }
            continue;
        }

        let should_spawn = map
            .spawn_grid_load_state_like_cpp(canonical_spawn_metadata.spawn_store())
            .should_be_spawned_on_grid_load(object_type, spawn_id);
        if !should_spawn {
            outcome.skipped_should_not_spawn += 1;
            continue;
        }

        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(object_type, spawn_id)
        else {
            outcome.stale_index_entries += 1;
            continue;
        };
        if spawn_data.map_id != map_id {
            outcome.stale_index_entries += 1;
            continue;
        }
        if !spawn_data.spawn_difficulties.contains(&spawn_mode) {
            outcome.skipped_difficulty_mismatch += 1;
            continue;
        }

        outcome.metadata_entries += 1;
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
            wow_map::SpawnObjectType::AreaTrigger => {
                build_loaded_grid_area_trigger_record_like_cpp(
                    map,
                    object_type,
                    spawn_id,
                    canonical_spawn_metadata,
                    area_trigger_template_store,
                )
            }
        }) else {
            outcome.load_record_missing += 1;
            match object_type {
                wow_map::SpawnObjectType::Creature => {
                    outcome.creature_load_record_missing += 1;
                }
                wow_map::SpawnObjectType::GameObject => {
                    outcome.gameobject_load_record_missing += 1;
                }
                wow_map::SpawnObjectType::AreaTrigger => {
                    outcome.area_trigger_load_record_missing += 1;
                }
            }
            continue;
        };

        for pre_add_record in records.pre_add_records {
            if map
                .add_map_object_record_to_map_like_cpp(pre_add_record)
                .is_ok()
            {
                outcome.pre_add_records_added += 1;
            } else {
                outcome.add_to_map_errors += 1;
            }
        }

        let primary_record = records.primary_record;
        let legacy_creature = primary_record.creature().cloned();
        match map.add_map_object_record_to_map_like_cpp(primary_record) {
            Ok(_add) => match object_type {
                wow_map::SpawnObjectType::Creature => {
                    outcome.creature_records_added += 1;
                    if let Some(creature) = legacy_creature
                        && mirror_loaded_grid_creature_to_legacy_like_cpp(
                            Some(legacy_manager),
                            canonical_spawn_metadata.waypoint_paths_like_cpp(),
                            creature,
                        )
                    {
                        outcome.legacy_creature_mirrors += 1;
                    }
                }
                wow_map::SpawnObjectType::GameObject => {
                    outcome.gameobject_records_added += 1;
                }
                wow_map::SpawnObjectType::AreaTrigger => {
                    outcome.area_trigger_records_added += 1;
                }
            },
            Err(_error) => {
                outcome.add_to_map_errors += 1;
            }
        }
    }
}

pub(crate) fn can_create_missing_login_grid_as_world_map_like_cpp(
    entry: wow_data::MapEntry,
) -> bool {
    // A missing non-split common world map is safe to materialize here.
    // Faction-split and instanceable maps (including battlegrounds, scenarios,
    // and garrisons) must already have been selected by the authoritative
    // CreateMap path, which owns team routing, access checks, difficulty,
    // locks, reset schedules, and instance IDs.
    entry.is_world_map() && !entry.is_garrison() && !entry.is_split_by_faction()
}

pub(crate) fn existing_login_grid_map_matches_map_entry_like_cpp(
    entry: wow_data::MapEntry,
    kind: wow_map::ManagedMapKind,
    instance_id: u32,
    authoritative_instance_selected: bool,
) -> bool {
    if entry.is_garrison() {
        return authoritative_instance_selected && matches!(kind, wow_map::ManagedMapKind::World);
    }
    if entry.is_world_map() && !entry.is_garrison() {
        if entry.is_split_by_faction() && !authoritative_instance_selected {
            return false;
        }
        return matches!(kind, wow_map::ManagedMapKind::World);
    }
    if !authoritative_instance_selected || instance_id == 0 {
        return false;
    }
    if entry.is_dungeon() {
        return matches!(kind, wow_map::ManagedMapKind::Dungeon { .. });
    }
    if entry.is_battleground_or_arena() {
        return matches!(kind, wow_map::ManagedMapKind::Battleground);
    }
    false
}

pub(crate) fn ensure_login_player_grid_loaded_like_cpp(
    canonical_map_manager: &SharedCanonicalMapManager,
    legacy_manager: &SharedMapManager,
    canonical_spawn_metadata: &SharedCanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    map_store: Option<&wow_data::MapStore>,
    map_id: u16,
    authoritative_instance_id: Option<u32>,
    position: Position,
) -> wow_world::session::PlayerGridLoadOutcomeLikeCpp {
    let mut outcome = wow_world::session::PlayerGridLoadOutcomeLikeCpp::default();
    let map_id_u32 = u32::from(map_id);
    let instance_id = authoritative_instance_id.unwrap_or(0);
    let cell = wow_map::cell_from_world(position.x, position.y);
    let grid = wow_map::GridCoord::new(cell.grid_x(), cell.grid_y());
    let visible_grids =
        player_visible_grid_coords_like_cpp(position, wow_world::map_manager::VISIBILITY_RADIUS);

    let Ok(metadata) = canonical_spawn_metadata.lock() else {
        outcome.map_unavailable = true;
        warn!(
            map_id = map_id_u32,
            instance_id, "C++ login grid load skipped: canonical spawn metadata lock poisoned"
        );
        return outcome;
    };
    let Ok(mut manager) = canonical_map_manager.lock() else {
        outcome.map_unavailable = true;
        warn!(
            map_id = map_id_u32,
            instance_id, "C++ login grid load skipped: canonical map manager lock poisoned"
        );
        return outcome;
    };

    let Some(map_entry) = map_store.and_then(|store| store.get(map_id_u32).copied()) else {
        outcome.map_unavailable = true;
        warn!(
            map_id = map_id_u32,
            instance_id, "C++ login grid load rejected: Map.db2 entry unavailable"
        );
        return outcome;
    };

    let managed_map = if manager.find_map(map_id_u32, instance_id).is_some() {
        let existing_kind = manager
            .find_map(map_id_u32, instance_id)
            .expect("canonical map checked above")
            .kind();
        if !existing_login_grid_map_matches_map_entry_like_cpp(
            map_entry,
            existing_kind,
            instance_id,
            authoritative_instance_id.is_some(),
        ) {
            outcome.map_unavailable = true;
            warn!(
                map_id = map_id_u32,
                instance_id,
                instance_type = map_entry.instance_type,
                kind = ?existing_kind,
                "C++ login grid load rejected: canonical map kind or instance ID is incompatible"
            );
            return outcome;
        }
        manager
            .find_map_mut(map_id_u32, instance_id)
            .expect("canonical map checked above")
    } else {
        if !can_create_missing_login_grid_as_world_map_like_cpp(map_entry) {
            outcome.map_unavailable = true;
            warn!(
                map_id = map_id_u32,
                instance_id,
                instance_type = map_entry.instance_type,
                "C++ login grid load rejected: authoritative instanceable map is unavailable"
            );
            return outcome;
        }
        outcome.map_created = true;
        manager.create_world_map(map_id_u32, instance_id)
    };
    let map = managed_map.map_mut();

    // C++ Map::AddPlayerToMap -> EnsureGridLoadedForActiveObject(cell, player)
    // loads the player's grid before SendInitSelf. Rusty's NoopGridLifecycle does
    // not own ObjectMgr DB state, so this bridge materializes loaded-grid records
    // immediately after marking the grid loaded/active.
    outcome.grid_loaded_now =
        map.ensure_grid_loaded_for_active_object(&cell, wow_map::ActiveObjectKind::Player);

    for visible_grid in visible_grids {
        if visible_grid != grid {
            outcome.grid_loaded_now |=
                map.ensure_grid_loaded(&wow_map::cell_from_grid_center(visible_grid));
        }
        materialize_loaded_player_grid_records_like_cpp(
            map,
            legacy_manager,
            &metadata,
            loaded_grid_creature_respawn_caches,
            area_trigger_template_store,
            map_id_u32,
            visible_grid,
            &mut outcome,
        );
    }

    if std::env::var_os("RUSTYCORE_CREATURE_VIS_TRACE").is_some()
        && (outcome.map_created
            || outcome.grid_loaded_now
            || outcome.metadata_entries != 0
            || outcome.skipped_already_loaded != 0
            || outcome.skipped_should_not_spawn != 0
            || outcome.skipped_difficulty_mismatch != 0
            || outcome.stale_index_entries != 0
            || outcome.creature_records_added != 0
            || outcome.gameobject_records_added != 0
            || outcome.area_trigger_records_added != 0
            || outcome.pre_add_records_added != 0
            || outcome.add_to_map_errors != 0
            || outcome.load_record_missing != 0
            || outcome.legacy_creature_mirrors != 0)
    {
        info!(
            map_id = map_id_u32,
            instance_id,
            x = position.x,
            y = position.y,
            z = position.z,
            cell_x = cell.cell_x(),
            cell_y = cell.cell_y(),
            grid_x = grid.x_coord,
            grid_y = grid.y_coord,
            map_created = outcome.map_created,
            grid_loaded_now = outcome.grid_loaded_now,
            metadata_entries = outcome.metadata_entries,
            skipped_already_loaded = outcome.skipped_already_loaded,
            skipped_should_not_spawn = outcome.skipped_should_not_spawn,
            skipped_difficulty_mismatch = outcome.skipped_difficulty_mismatch,
            stale_index_entries = outcome.stale_index_entries,
            creature_records_added = outcome.creature_records_added,
            gameobject_records_added = outcome.gameobject_records_added,
            area_trigger_records_added = outcome.area_trigger_records_added,
            pre_add_records_added = outcome.pre_add_records_added,
            add_to_map_errors = outcome.add_to_map_errors,
            load_record_missing = outcome.load_record_missing,
            creature_load_record_missing = outcome.creature_load_record_missing,
            gameobject_load_record_missing = outcome.gameobject_load_record_missing,
            area_trigger_load_record_missing = outcome.area_trigger_load_record_missing,
            legacy_creature_mirrors = outcome.legacy_creature_mirrors,
            "RUST_CREATURE_VIS login_grid_load"
        );
    }

    outcome
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

pub(crate) fn create_canonical_map_manager(configs: &WorldConfigSet) -> wow_map::MapManager {
    let grid_cleanup_delay_ms =
        world_config_u32(configs, "CONFIG_INTERVAL_GRIDCLEAN", 5 * 60 * 1000)
            .max(wow_map::MIN_GRID_DELAY_MS);
    let map_update_interval_ms = world_config_u32(configs, "CONFIG_INTERVAL_MAPUPDATE", 10)
        .max(wow_map::MIN_MAP_UPDATE_DELAY_MS);
    let map_update_threads = world_config_u32(configs, "CONFIG_NUMTHREADS", 1);

    let mut manager = wow_map::MapManager::new(grid_cleanup_delay_ms, map_update_interval_ms);
    if map_update_threads > 0 {
        manager
            .map_updater_mut()
            .activate(map_update_threads as usize);
    }

    info!(
        "Canonical MapManager initialized: grid_cleanup_delay_ms={}, map_update_interval_ms={}, map_update_threads={}",
        grid_cleanup_delay_ms, map_update_interval_ms, map_update_threads,
    );

    manager
}

pub(crate) fn map_db2_entries_from_stores(
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    map_id: u32,
    difficulty_id: u8,
) -> Option<MapDb2Entries> {
    MapDb2Entries::from_stores_like_cpp(map_store, map_difficulty_store, map_id, difficulty_id)
}

pub(crate) fn register_loaded_instance_ids(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: &Mutex<wow_map::MapManager>,
    instance_ids: &[u32],
) {
    let Some(max_instance_id) = instance_ids.iter().copied().max() else {
        return;
    };

    match legacy_map_manager.write() {
        Ok(mut manager) => {
            manager.init_instance_ids_from_max(max_instance_id);
            for &instance_id in instance_ids {
                manager.register_instance_id(instance_id);
            }
        }
        Err(_) => warn!("Legacy MapManager lock poisoned; persisted instance ids not registered"),
    }

    match canonical_map_manager.lock() {
        Ok(mut manager) => {
            manager.init_instance_ids(u64::from(max_instance_id));
            for &instance_id in instance_ids {
                manager.register_instance_id(instance_id);
            }
        }
        Err(_) => {
            warn!("Canonical MapManager lock poisoned; persisted instance ids not registered")
        }
    }

    info!(
        "Registered {} persisted instance ids with MapManager, max_instance_id={}",
        instance_ids.len(),
        max_instance_id
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CanonicalGameEventSchedulerLikeCpp {
    pub(crate) timer_ms: u32,
    pub(crate) interval_ms: u32,
}

impl CanonicalGameEventSchedulerLikeCpp {
    pub(crate) fn start_system(next_delay_ms: u64) -> Self {
        let interval_ms = clamp_game_event_delay_ms_like_cpp(next_delay_ms).max(1);
        Self {
            timer_ms: interval_ms,
            interval_ms,
        }
    }

    pub(crate) fn update(&mut self, diff_ms: u32) -> bool {
        if self.timer_ms <= diff_ms {
            self.timer_ms = self.interval_ms;
            true
        } else {
            self.timer_ms -= diff_ms;
            false
        }
    }

    pub(crate) fn set_interval_and_reset(&mut self, next_delay_ms: u64) {
        self.interval_ms = clamp_game_event_delay_ms_like_cpp(next_delay_ms).max(1);
        self.timer_ms = self.interval_ms;
    }

    #[cfg(test)]
    pub(crate) const fn timer_ms(&self) -> u32 {
        self.timer_ms
    }

    #[cfg(test)]
    pub(crate) const fn interval_ms(&self) -> u32 {
        self.interval_ms
    }
}

pub(crate) fn clamp_game_event_delay_ms_like_cpp(delay_ms: u64) -> u32 {
    u32::try_from(delay_ms).unwrap_or(u32::MAX)
}

pub(crate) fn current_unix_time_secs_like_cpp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub(crate) fn game_event_quest_complete_response_from_summary_like_cpp(
    quest_id: u32,
    summary: &GameEventQuestCompleteDbBridgeSummaryLikeCpp,
) -> GameEventQuestCompleteResponseLikeCpp {
    GameEventQuestCompleteResponseLikeCpp {
        quest_id,
        condition_save_updates_queued: summary.condition_save_updates_queued,
        condition_save_updates_executed: summary.condition_save_updates_executed,
        condition_save_updates_failed: summary.condition_save_updates_failed,
        condition_save_updates_skipped_non_progress: summary
            .condition_save_updates_skipped_non_progress,
        save_world_event_state_requested: summary.save_world_event_state_requested,
        world_event_state_save_requested: summary.world_event_state_save_requested,
        world_event_state_saves_queued: summary.world_event_state_summary.saves_queued,
        world_event_state_saves_executed: summary.world_event_state_summary.saves_executed,
        world_event_state_saves_failed: summary.world_event_state_summary.saves_failed,
        world_event_state_saves_skipped_event_id_out_of_range: summary
            .world_event_state_summary
            .saves_skipped_event_id_out_of_range,
        world_event_state_saves_skipped_missing_event: summary
            .world_event_state_summary
            .saves_skipped_missing_event,
        force_game_event_update_requested: summary.force_game_event_update_requested_flag,
        force_game_event_update_requests: summary.force_game_event_update_requested,
        processor_failed: false,
    }
}

pub(crate) fn game_event_quest_complete_processor_failed_response_like_cpp(
    quest_id: u32,
) -> GameEventQuestCompleteResponseLikeCpp {
    GameEventQuestCompleteResponseLikeCpp {
        quest_id,
        processor_failed: true,
        ..GameEventQuestCompleteResponseLikeCpp::default()
    }
}

pub(crate) async fn run_game_event_quest_complete_processor_like_cpp(
    command_rx: flume::Receiver<GameEventQuestCompleteCommandLikeCpp>,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    character_db: Arc<CharacterDatabase>,
) {
    while let Ok(command) = command_rx.recv_async().await {
        let quest_id = command.quest_id;
        let maybe_summary = {
            let Ok(mut metadata) = canonical_spawn_metadata.lock() else {
                tracing::error!(
                    quest_id,
                    "CanonicalSpawnMetadataLikeCpp mutex poisoned during C++ GameEventMgr::HandleQuestComplete bridge"
                );
                let _ = command.response_tx.try_send(
                    game_event_quest_complete_processor_failed_response_like_cpp(quest_id),
                );
                continue;
            };
            let outcome = metadata.represented_handle_game_event_quest_complete_like_cpp(
                quest_id,
                current_unix_time_secs_like_cpp(),
            );
            materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata)
        };

        let mut summary = maybe_summary;
        execute_game_event_quest_complete_condition_save_db_bridge_like_cpp(
            character_db.as_ref(),
            &mut summary,
        )
        .await;
        execute_game_event_world_event_state_db_bridge_like_cpp(
            character_db.as_ref(),
            &mut summary.world_event_state_summary,
        )
        .await;

        let response = game_event_quest_complete_response_from_summary_like_cpp(quest_id, &summary);
        let _ = command.response_tx.try_send(response);
    }
}

pub(crate) fn represented_game_event_world_conditions_met_like_cpp(_event_id: u16) -> bool {
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GameEventLiveUpdateActionLikeCpp {
    Spawn(i16),
    Unspawn(i16),
    AnnounceEvent {
        event_id: u16,
        description: String,
        description_len: usize,
        announce: u8,
        config_event_announce: bool,
    },
    ChangeEquipOrModel {
        event_id: u16,
        activate: bool,
    },
    RunSmartAIScripts {
        event_id: u16,
        activate: bool,
    },
    ResetEventSeasonalQuests {
        event_id: u16,
        event_start_time: u64,
    },
    UpdateEventQuests {
        event_id: u16,
        activate: bool,
    },
    UpdateWorldStates {
        event_id: u16,
        activate: bool,
    },
    UpdateNpcFlags {
        event_id: u16,
    },
    UpdateNpcVendor {
        event_id: u16,
        activate: bool,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct GameEventSeasonalQuestDbDeleteLikeCpp {
    pub(crate) event_id: u16,
    pub(crate) event_start_time: i64,
    pub(crate) statement: PreparedStatement,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct GameEventLiveUpdateSideEffectSummaryLikeCpp {
    pub(crate) actions: Vec<GameEventLiveUpdateActionLikeCpp>,
    pub(crate) spawn_actions: usize,
    pub(crate) unspawn_actions: usize,
    pub(crate) announce_event_actions: usize,
    pub(crate) announce_event_description_len_total: usize,
    pub(crate) announce_event_world_text_represented: usize,
    pub(crate) announce_event_lines: usize,
    pub(crate) announce_event_registry_missing: usize,
    pub(crate) announce_event_send_attempted: usize,
    pub(crate) announce_event_send_queued: usize,
    pub(crate) announce_event_send_failed: usize,
    pub(crate) announce_event_localization_unrepresented: usize,
    pub(crate) announce_event_in_world_filter_unrepresented: usize,
    pub(crate) announce_event_not_in_world_skipped: usize,
    pub(crate) announce_event_world_text_unimplemented: usize,
    pub(crate) announce_event_session_fanout_unimplemented: usize,
    pub(crate) change_equip_or_model_actions: usize,
    pub(crate) change_equip_or_model_records_seen: usize,
    pub(crate) change_equip_or_model_records_applied: usize,
    pub(crate) change_equip_or_model_missing_event_buckets: usize,
    pub(crate) change_equip_or_model_missing_spawn_metadata: usize,
    pub(crate) change_equip_or_model_missing_runtime_rows: usize,
    pub(crate) change_equip_or_model_maps_matched: usize,
    pub(crate) change_equip_or_model_live_creatures_mutated: usize,
    pub(crate) change_equip_or_model_stale_index_or_wrong_kind: usize,
    pub(crate) change_equip_or_model_model_validation_unavailable: usize,
    pub(crate) run_smart_ai_actions: usize,
    pub(crate) run_smart_ai_maps_visited: usize,
    pub(crate) run_smart_ai_creature_candidates: usize,
    pub(crate) run_smart_ai_gameobject_candidates: usize,
    pub(crate) run_smart_ai_creature_ai_enabled_unrepresented: usize,
    pub(crate) run_smart_ai_script_dispatch_unrepresented: usize,
    pub(crate) reset_event_seasonal_quests_actions: usize,
    pub(crate) reset_event_seasonal_quests_event_start_time_zero: usize,
    pub(crate) reset_event_seasonal_quests_event_start_time_nonzero: usize,
    pub(crate) reset_event_seasonal_quests_player_session_runtime_unimplemented: usize,
    pub(crate) reset_event_seasonal_quests_player_session_registry_missing: usize,
    pub(crate) reset_event_seasonal_quests_player_session_send_attempted: usize,
    pub(crate) reset_event_seasonal_quests_player_session_send_queued: usize,
    pub(crate) reset_event_seasonal_quests_player_session_send_failed: usize,
    pub(crate) reset_event_seasonal_quests_character_db_statement_unimplemented: usize,
    pub(crate) reset_event_seasonal_quests_character_db_delete_queued: usize,
    pub(crate) reset_event_seasonal_quests_character_db_delete_executed: usize,
    pub(crate) reset_event_seasonal_quests_character_db_delete_failed: usize,
    pub(crate) reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range:
        usize,
    pub(crate) reset_event_seasonal_quest_db_deletes: Vec<GameEventSeasonalQuestDbDeleteLikeCpp>,
    pub(crate) update_event_quests_actions: usize,
    pub(crate) update_event_quests_creature_records_seen: usize,
    pub(crate) update_event_quests_gameobject_records_seen: usize,
    pub(crate) update_event_quests_creature_inserted: usize,
    pub(crate) update_event_quests_gameobject_inserted: usize,
    pub(crate) update_event_quests_creature_removed: usize,
    pub(crate) update_event_quests_gameobject_removed: usize,
    pub(crate) update_event_quests_creature_remove_misses: usize,
    pub(crate) update_event_quests_gameobject_remove_misses: usize,
    pub(crate) update_event_quests_creature_no_match: usize,
    pub(crate) update_event_quests_gameobject_no_match: usize,
    pub(crate) update_event_quests_creature_missing_event_buckets: usize,
    pub(crate) update_event_quests_gameobject_missing_event_buckets: usize,
    pub(crate) update_event_quests_creature_skipped_active_other_event: usize,
    pub(crate) update_event_quests_gameobject_skipped_active_other_event: usize,
    pub(crate) update_world_states_actions: usize,
    pub(crate) update_world_states_no_holiday: usize,
    pub(crate) update_world_states_missing_event: usize,
    pub(crate) update_world_states_store_missing: usize,
    pub(crate) update_world_states_holiday_not_weekend_battleground: usize,
    pub(crate) update_world_states_battlemaster_list_missing: usize,
    pub(crate) update_world_states_holiday_world_state_zero: usize,
    pub(crate) update_world_states_holiday_lookup_unrepresented: usize,
    pub(crate) update_world_states_set_value_represented: usize,
    pub(crate) update_world_states_set_value_attempts: usize,
    pub(crate) update_world_states_realm_changed_or_inserted: usize,
    pub(crate) update_world_states_realm_unchanged_noop: usize,
    pub(crate) update_world_states_map_specific_no_map_unsupported: usize,
    pub(crate) update_world_states_global_message_represented: usize,
    pub(crate) update_world_states_global_message_registry_missing: usize,
    pub(crate) update_world_states_global_message_send_attempted: usize,
    pub(crate) update_world_states_global_message_send_queued: usize,
    pub(crate) update_world_states_global_message_send_failed: usize,
    pub(crate) update_world_states_global_message_not_in_world_skipped: usize,
    pub(crate) update_world_states_last_world_state_id: Option<i16>,
    pub(crate) update_world_states_last_world_state_value: Option<i32>,
    pub(crate) update_npc_flags_actions: usize,
    pub(crate) update_npc_flags_records_seen: usize,
    pub(crate) update_npc_flags_missing_event_buckets: usize,
    pub(crate) update_npc_flags_missing_spawn_metadata: usize,
    pub(crate) update_npc_flags_template_npcflag_missing: usize,
    pub(crate) update_npc_flags_maps_matched: usize,
    pub(crate) update_npc_flags_indexed_guids: usize,
    pub(crate) update_npc_flags_live_creatures_mutated: usize,
    pub(crate) update_npc_flags_stale_index_or_wrong_kind: usize,
    pub(crate) update_npc_flags_low_applied: usize,
    pub(crate) update_npc_flags2_applied: usize,
    pub(crate) update_npc_flags_values_updates_built: usize,
    pub(crate) update_npc_flags_values_update_empty: usize,
    pub(crate) update_npc_flags_values_update_map_id_out_of_range: usize,
    pub(crate) update_npc_flags_values_update_registry_missing: usize,
    pub(crate) update_npc_flags_values_update_not_in_world_skipped: usize,
    pub(crate) update_npc_flags_values_update_wrong_map_skipped: usize,
    pub(crate) update_npc_flags_values_update_send_attempted: usize,
    pub(crate) update_npc_flags_values_update_send_queued: usize,
    pub(crate) update_npc_flags_values_update_send_failed: usize,
    pub(crate) update_npc_vendor_actions: usize,
    pub(crate) update_npc_vendor_records_seen: usize,
    pub(crate) update_npc_vendor_items_added: usize,
    pub(crate) update_npc_vendor_items_removed: usize,
    pub(crate) update_npc_vendor_missing_event_buckets: usize,
    pub(crate) update_npc_vendor_remove_misses: usize,
    pub(crate) update_npc_vendor_no_match: usize,
}

pub(crate) fn game_event_signed_id_like_cpp(event_id: u16) -> i16 {
    i16::try_from(event_id).unwrap_or(i16::MAX)
}

pub(crate) fn should_announce_game_event_like_cpp(
    announce: u8,
    config_event_announce: bool,
) -> bool {
    announce == 1 || (announce == 2 && config_event_announce)
}

pub(crate) fn game_event_announcement_lines_like_cpp(description: &str) -> Vec<String> {
    // C++ WorldWorldTextBuilder formats LANG_EVENTMESSAGE first and then
    // ChatHandler::LineFromMessage tokenizes the resulting buffer with strtok("\n"),
    // so empty newline runs are skipped. Rust does not have ObjectMgr TrinityString
    // locale storage yet; represent the known enUS fallback format explicitly.
    let formatted = format!("|cffff0000[Event Message]: {description}|r");
    formatted
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn fanout_game_event_announcement_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    description: &str,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    summary.announce_event_world_text_represented += 1;
    summary.announce_event_localization_unrepresented += 1;

    let lines = game_event_announcement_lines_like_cpp(description);
    summary.announce_event_lines += lines.len();
    if lines.is_empty() {
        return;
    }

    let Some(player_registry) = player_registry else {
        summary.announce_event_registry_missing += 1;
        return;
    };

    let packet_bytes: Vec<Vec<u8>> = lines
        .into_iter()
        .map(|text| {
            ChatPkt {
                msg_type: ChatMsg::System,
                language: 0,
                sender_guid: ObjectGuid::EMPTY,
                sender_name: String::new(),
                target_guid: ObjectGuid::EMPTY,
                target_name: String::new(),
                prefix: String::new(),
                channel: String::new(),
                text,
                virtual_realm: 0,
            }
            .to_bytes()
        })
        .collect();

    for recipient in player_registry.runtime_recipients() {
        if !recipient.is_in_world {
            summary.announce_event_not_in_world_skipped += 1;
            continue;
        }

        for bytes in &packet_bytes {
            summary.announce_event_send_attempted += 1;
            match player_registry.try_send_current_packet(recipient.registration, bytes.clone()) {
                Ok(()) => summary.announce_event_send_queued += 1,
                Err(_) => summary.announce_event_send_failed += 1,
            }
        }
    }
}

pub(crate) fn game_event_seasonal_quest_db_delete_like_cpp(
    event_id: u16,
    event_start_time: u64,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Ok(event_start_time_i64) = i64::try_from(event_start_time) else {
        summary.reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range += 1;
        return;
    };

    let mut statement = PreparedStatement::new(
        CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT.sql(),
    );
    statement.set_u16(0, event_id);
    statement.set_i64(1, event_start_time_i64);

    summary.reset_event_seasonal_quests_character_db_delete_queued += 1;
    summary
        .reset_event_seasonal_quest_db_deletes
        .push(GameEventSeasonalQuestDbDeleteLikeCpp {
            event_id,
            event_start_time: event_start_time_i64,
            statement,
        });
}

pub(crate) fn fanout_reset_event_seasonal_quests_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    event_start_time: u64,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Some(player_registry) = player_registry else {
        summary.reset_event_seasonal_quests_player_session_registry_missing += 1;
        return;
    };

    for recipient in player_registry.runtime_recipients() {
        summary.reset_event_seasonal_quests_player_session_send_attempted += 1;
        let command = SessionCommand::ResetSeasonalQuestStatus(ResetSeasonalQuestStatusCommand {
            event_id,
            event_start_time,
        });
        match player_registry.try_send_current_command(recipient.registration, command) {
            Ok(()) => summary.reset_event_seasonal_quests_player_session_send_queued += 1,
            Err(_) => summary.reset_event_seasonal_quests_player_session_send_failed += 1,
        }
    }
}

pub(crate) fn fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let reset_actions: Vec<(u16, u64)> = summary
        .actions
        .iter()
        .filter_map(|action| match action {
            GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                event_id,
                event_start_time,
            } => Some((*event_id, *event_start_time)),
            _ => None,
        })
        .collect();

    for (event_id, event_start_time) in reset_actions {
        fanout_reset_event_seasonal_quests_to_player_sessions_like_cpp(
            player_registry,
            event_id,
            event_start_time,
            summary,
        );
    }
}

pub(crate) async fn execute_game_event_seasonal_quest_db_deletes_like_cpp(
    character_db: &CharacterDatabase,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let db_delete_total = summary.reset_event_seasonal_quest_db_deletes.len();
    for (db_delete_index, db_delete) in summary
        .reset_event_seasonal_quest_db_deletes
        .drain(..)
        .enumerate()
    {
        match character_db.execute(&db_delete.statement).await {
            Ok(_) => {
                summary.reset_event_seasonal_quests_character_db_delete_executed += 1;
            }
            Err(error) => {
                summary.reset_event_seasonal_quests_character_db_delete_failed += 1;
                tracing::error!(
                    error = %error,
                    db_delete_index = db_delete_index + 1,
                    db_delete_total,
                    event_id = db_delete.event_id,
                    event_start_time = db_delete.event_start_time,
                    "Failed to execute C++ World::ResetEventSeasonalQuests character DB delete; continuing live update loop"
                );
            }
        }
    }
}

pub(crate) fn game_event_live_update_actions_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    config_event_announce: bool,
) -> Vec<GameEventLiveUpdateActionLikeCpp> {
    let mut actions = Vec::new();
    for &event_id in &outcome.negative_spawn_event_ids {
        actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(event_id));
    }
    for start_outcome in &outcome.start_outcomes {
        if let spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(summary) = start_outcome {
            if summary.apply_new_event_requested {
                let event_id = game_event_signed_id_like_cpp(summary.event_id);
                if let Some(event) = canonical_spawn_metadata.game_event_like_cpp(summary.event_id)
                {
                    if should_announce_game_event_like_cpp(event.announce, config_event_announce) {
                        actions.push(GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
                            event_id: summary.event_id,
                            description: event.description.clone(),
                            description_len: event.description.len(),
                            announce: event.announce,
                            config_event_announce,
                        });
                    }
                }
                actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::Unspawn(-event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags {
                    event_id: summary.event_id,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                    event_id: summary.event_id,
                    event_start_time: canonical_spawn_metadata.game_event_last_start_time_like_cpp(
                        summary.event_id,
                        outcome.current_time_secs,
                    ),
                });
            }
        }
    }
    for stop_outcome in &outcome.stop_outcomes {
        if let spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(summary) = stop_outcome {
            if summary.unapply_event_requested {
                let event_id = game_event_signed_id_like_cpp(summary.event_id);
                actions.push(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::Unspawn(event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(-event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags {
                    event_id: summary.event_id,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                    event_id: summary.event_id,
                    activate: false,
                });
            }
        }
    }
    actions
}

pub(crate) fn game_event_change_equip_or_model_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let records = canonical_spawn_metadata
        .game_event_model_equip_like_cpp(event_id)
        .map_or_else(Vec::new, <[_]>::to_vec);

    for record in &records {
        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(wow_map::SpawnObjectType::Creature, record.spawn_id)
        else {
            summary.change_equip_or_model_missing_spawn_metadata += 1;
            continue;
        };

        let (equipment_id, model_id) = if activate {
            (record.equipment_id, record.model_id)
        } else {
            (record.equipment_id_prev, record.model_id_prev)
        };
        let mut maps_matched_for_record = 0usize;
        manager.do_for_all_maps_mut(|map| {
            if map.map_id() == spawn_data.map_id {
                maps_matched_for_record += 1;
                let outcome = map
                    .map_mut()
                    .change_game_event_equip_or_model_by_spawn_id_like_cpp(
                        record.spawn_id,
                        equipment_id,
                        model_id,
                        false,
                    );
                summary.change_equip_or_model_live_creatures_mutated +=
                    outcome.live_creatures_mutated;
                summary.change_equip_or_model_stale_index_or_wrong_kind +=
                    outcome.stale_index_or_wrong_kind;
                summary.change_equip_or_model_model_validation_unavailable +=
                    outcome.model_validation_unavailable;
            }
        });
        summary.change_equip_or_model_maps_matched += maps_matched_for_record;
    }

    let baseline_summary = canonical_spawn_metadata
        .change_game_event_model_equip_baseline_like_cpp(event_id, activate);
    summary.change_equip_or_model_records_seen += baseline_summary.records_seen;
    summary.change_equip_or_model_records_applied += baseline_summary.records_applied;
    if baseline_summary.missing_event_bucket {
        summary.change_equip_or_model_missing_event_buckets += 1;
    }
    summary.change_equip_or_model_missing_spawn_metadata += baseline_summary.missing_spawn_metadata;
    summary.change_equip_or_model_missing_runtime_rows +=
        baseline_summary.missing_creature_runtime_rows;
    summary
}

pub(crate) fn fanout_game_event_npc_flag_values_update_to_visible_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    values_update: &wow_map::GameEventNpcFlagValuesUpdateLikeCpp,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Ok(map_id) = u16::try_from(values_update.map_id) else {
        summary.update_npc_flags_values_update_map_id_out_of_range += 1;
        return;
    };
    let Some(packet_update) = unit_values_update_to_packet(&values_update.values_update) else {
        summary.update_npc_flags_values_update_empty += 1;
        return;
    };
    let update = wow_packet::packets::update::UpdateObject::unit_values_update(
        values_update.guid,
        map_id,
        packet_update.clone(),
    );
    summary.update_npc_flags_values_updates_built += 1;

    let Some(player_registry) = player_registry else {
        summary.update_npc_flags_values_update_registry_missing += 1;
        return;
    };

    let packet_bytes = update.to_bytes();
    for recipient in player_registry.runtime_recipients() {
        if !recipient.is_in_world {
            summary.update_npc_flags_values_update_not_in_world_skipped += 1;
            continue;
        }
        if recipient.map_id != map_id {
            summary.update_npc_flags_values_update_wrong_map_skipped += 1;
            continue;
        }

        summary.update_npc_flags_values_update_send_attempted += 1;
        let command =
            SessionCommand::SendVisibleObjectValuesUpdate(SendVisibleObjectValuesUpdateCommand {
                object_guid: values_update.guid,
                map_id,
                packet_bytes: packet_bytes.clone(),
                unit_values_update: Some(packet_update.clone()),
            });
        match player_registry.try_send_current_command(recipient.registration, command) {
            Ok(()) => summary.update_npc_flags_values_update_send_queued += 1,
            Err(_) => summary.update_npc_flags_values_update_send_failed += 1,
        }
    }
}

pub(crate) fn game_event_update_npc_flags_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    creature_template_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    active_event_ids: &[u16],
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let Some(records) = canonical_spawn_metadata.game_event_npc_flags_like_cpp(event_id) else {
        summary.update_npc_flags_missing_event_buckets += 1;
        return summary;
    };
    summary.update_npc_flags_records_seen = records.len();

    for record in records {
        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(wow_map::SpawnObjectType::Creature, record.spawn_id)
        else {
            summary.update_npc_flags_missing_spawn_metadata += 1;
            continue;
        };
        let template_npc_flags = creature_template_store
            .get(spawn_data.id)
            .map(|template| template.npc_flags)
            .unwrap_or_else(|| {
                summary.update_npc_flags_template_npcflag_missing += 1;
                0
            });
        let overlay = canonical_spawn_metadata
            .game_event_npc_flag_mask_like_cpp(record.spawn_id, active_event_ids);
        let npcflag_mask_with_template = overlay | template_npc_flags;

        let mut maps_matched_for_record = 0usize;
        manager.do_for_all_maps_mut(|map| {
            if map.map_id() == spawn_data.map_id {
                maps_matched_for_record += 1;
                let outcome = map
                    .map_mut()
                    .update_game_event_npc_flags_by_spawn_id_like_cpp(
                        record.spawn_id,
                        npcflag_mask_with_template,
                    );
                summary.update_npc_flags_indexed_guids += outcome.indexed_guids;
                summary.update_npc_flags_live_creatures_mutated += outcome.live_creatures_mutated;
                summary.update_npc_flags_stale_index_or_wrong_kind +=
                    outcome.stale_index_or_wrong_kind;
                summary.update_npc_flags_low_applied += outcome.npc_flags_low_applied;
                summary.update_npc_flags2_applied += outcome.npc_flags2_applied;
                for values_update in &outcome.values_updates {
                    fanout_game_event_npc_flag_values_update_to_visible_sessions_like_cpp(
                        player_registry,
                        values_update,
                        &mut summary,
                    );
                }
            }
        });
        summary.update_npc_flags_maps_matched += maps_matched_for_record;
    }

    summary
}

pub(crate) fn fanout_realm_update_world_state_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    world_state_id: i32,
    value: i32,
    hidden: bool,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Some(player_registry) = player_registry else {
        summary.update_world_states_global_message_registry_missing += 1;
        return;
    };

    // C++ assigns signed `int32 worldStateId` into packet `uint32 VariableID`;
    // Rust's `as u32` preserves the same two's-complement wrapping semantics.
    let packet = wow_packet::packets::misc::UpdateWorldState {
        variable_id: world_state_id as u32,
        value,
        hidden,
    };
    let bytes = packet.to_bytes();

    for recipient in player_registry.runtime_recipients() {
        if !recipient.is_in_world {
            summary.update_world_states_global_message_not_in_world_skipped += 1;
            continue;
        }

        summary.update_world_states_global_message_send_attempted += 1;
        match player_registry.try_send_current_packet(recipient.registration, bytes.clone()) {
            Ok(()) => summary.update_world_states_global_message_send_queued += 1,
            Err(_) => summary.update_world_states_global_message_send_failed += 1,
        }
    }
}

pub(crate) fn game_event_update_npc_vendor_like_cpp(
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let vendor_summary =
        canonical_spawn_metadata.update_game_event_npc_vendor_cache_like_cpp(event_id, activate);
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    summary.update_npc_vendor_records_seen = vendor_summary.records_seen;
    summary.update_npc_vendor_items_added = vendor_summary.items_added;
    summary.update_npc_vendor_items_removed = vendor_summary.items_removed;
    summary.update_npc_vendor_remove_misses = vendor_summary.remove_misses;
    summary.update_npc_vendor_no_match = vendor_summary.no_match;
    if vendor_summary.missing_event_bucket {
        summary.update_npc_vendor_missing_event_buckets = 1;
    }
    summary
}

pub(crate) fn game_event_update_world_states_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
    mut world_state_mgr: Option<&mut spawn_store_loader::WorldStateMgrLikeCpp>,
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let Some(event) = canonical_spawn_metadata.game_event_like_cpp(event_id) else {
        summary.update_world_states_missing_event = 1;
        return summary;
    };

    if event.holiday_id == 0 {
        summary.update_world_states_no_holiday = 1;
        return summary;
    }

    let Some(battlemaster_list_store) = battlemaster_list_store else {
        summary.update_world_states_store_missing = 1;
        summary.update_world_states_holiday_lookup_unrepresented = 1;
        return summary;
    };

    match battlemaster_list_store.holiday_world_state_for_weekend_holiday_like_cpp(event.holiday_id)
    {
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayNone => {
            summary.update_world_states_no_holiday = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayNotWeekendBattleground { .. } => {
            summary.update_world_states_holiday_not_weekend_battleground = 1;
            summary.update_world_states_holiday_lookup_unrepresented = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::BattlemasterListMissing { .. } => {
            summary.update_world_states_battlemaster_list_missing = 1;
            summary.update_world_states_holiday_lookup_unrepresented = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayWorldStateZero { .. } => {
            summary.update_world_states_holiday_world_state_zero = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::SetValueRepresented {
            world_state_id, ..
        } => {
            let value = if activate { 1 } else { 0 };
            summary.update_world_states_set_value_attempts = 1;
            summary.update_world_states_last_world_state_id = Some(world_state_id);
            summary.update_world_states_last_world_state_value = Some(value);
            let Some(world_state_mgr) = world_state_mgr.as_deref_mut() else {
                summary.update_world_states_set_value_represented = 1;
                return summary;
            };
            match world_state_mgr.set_value_realm_or_map_null_like_cpp(
                i32::from(world_state_id),
                value,
                false,
            ) {
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::RealmInsertedOrChanged {
                    world_state_id,
                    new_value,
                    hidden,
                    global_message_represented,
                    ..
                } => {
                    summary.update_world_states_realm_changed_or_inserted = 1;
                    if global_message_represented {
                        summary.update_world_states_global_message_represented = 1;
                        fanout_realm_update_world_state_to_player_sessions_like_cpp(
                            player_registry,
                            world_state_id,
                            new_value,
                            hidden,
                            &mut summary,
                        );
                    }
                }
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::RealmUnchanged { .. } => {
                    summary.update_world_states_realm_unchanged_noop = 1;
                }
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::MapSpecificNoMapUnsupported { .. } => {
                    summary.update_world_states_map_specific_no_map_unsupported = 1;
                }
            }
        }
    }

    summary
}

pub(crate) fn game_event_update_quests_like_cpp(
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let quest_summary = canonical_spawn_metadata
        .update_game_event_quest_relation_cache_like_cpp(event_id, activate);
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    summary.update_event_quests_creature_records_seen = quest_summary.creature_records_seen;
    summary.update_event_quests_gameobject_records_seen = quest_summary.gameobject_records_seen;
    summary.update_event_quests_creature_inserted = quest_summary.creature_inserted;
    summary.update_event_quests_gameobject_inserted = quest_summary.gameobject_inserted;
    summary.update_event_quests_creature_removed = quest_summary.creature_removed;
    summary.update_event_quests_gameobject_removed = quest_summary.gameobject_removed;
    summary.update_event_quests_creature_remove_misses = quest_summary.creature_remove_misses;
    summary.update_event_quests_gameobject_remove_misses = quest_summary.gameobject_remove_misses;
    summary.update_event_quests_creature_no_match = quest_summary.creature_no_match;
    summary.update_event_quests_gameobject_no_match = quest_summary.gameobject_no_match;
    summary.update_event_quests_creature_skipped_active_other_event =
        quest_summary.creature_skipped_active_other_event;
    summary.update_event_quests_gameobject_skipped_active_other_event =
        quest_summary.gameobject_skipped_active_other_event;
    if quest_summary.creature_missing_event_bucket {
        summary.update_event_quests_creature_missing_event_buckets = 1;
    }
    if quest_summary.gameobject_missing_event_bucket {
        summary.update_event_quests_gameobject_missing_event_buckets = 1;
    }
    summary
}

pub(crate) fn game_event_run_smart_ai_scripts_like_cpp(
    manager: &wow_map::MapManager,
    _event_id: u16,
    _activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    manager.do_for_all_maps(|managed_map| {
        let candidates = managed_map
            .map()
            .game_event_smart_ai_script_candidates_like_cpp();
        summary.run_smart_ai_maps_visited += candidates.maps_visited;
        summary.run_smart_ai_creature_candidates += candidates.in_world_creature_candidates;
        summary.run_smart_ai_gameobject_candidates += candidates.in_world_gameobject_candidates;
        summary.run_smart_ai_creature_ai_enabled_unrepresented +=
            candidates.creature_ai_enabled_unrepresented;
        summary.run_smart_ai_script_dispatch_unrepresented +=
            candidates.script_dispatch_unrepresented;
    });
    summary
}

pub(crate) fn consume_game_event_live_update_side_effects_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
    mut world_state_mgr: Option<&mut spawn_store_loader::WorldStateMgrLikeCpp>,
    player_registry: Option<&PlayerRegistry>,
    active_event_ids: &[u16],
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    config_event_announce: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let actions = game_event_live_update_actions_like_cpp(
        canonical_spawn_metadata,
        outcome,
        config_event_announce,
    );
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp {
        actions,
        ..GameEventLiveUpdateSideEffectSummaryLikeCpp::default()
    };
    for action in summary.actions.clone() {
        match action {
            GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
                event_id: _,
                description,
                description_len,
                announce: _,
                config_event_announce: _,
            } => {
                summary.announce_event_actions += 1;
                summary.announce_event_description_len_total += description_len;
                fanout_game_event_announcement_to_player_sessions_like_cpp(
                    player_registry,
                    &description,
                    &mut summary,
                );
            }
            GameEventLiveUpdateActionLikeCpp::Spawn(event_id) => {
                let _ = game_event_spawn_for_event_like_cpp(
                    manager,
                    legacy_manager,
                    canonical_spawn_metadata,
                    loaded_grid_creature_respawn_caches,
                    event_id,
                );
                summary.spawn_actions += 1;
            }
            GameEventLiveUpdateActionLikeCpp::Unspawn(event_id) => {
                let _ = game_event_unspawn_for_event_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    active_event_ids,
                    event_id,
                );
                summary.unspawn_actions += 1;
            }
            GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel { event_id, activate } => {
                let change_summary = game_event_change_equip_or_model_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    event_id,
                    activate,
                );
                summary.change_equip_or_model_actions += 1;
                summary.change_equip_or_model_records_seen +=
                    change_summary.change_equip_or_model_records_seen;
                summary.change_equip_or_model_records_applied +=
                    change_summary.change_equip_or_model_records_applied;
                summary.change_equip_or_model_missing_event_buckets +=
                    change_summary.change_equip_or_model_missing_event_buckets;
                summary.change_equip_or_model_missing_spawn_metadata +=
                    change_summary.change_equip_or_model_missing_spawn_metadata;
                summary.change_equip_or_model_missing_runtime_rows +=
                    change_summary.change_equip_or_model_missing_runtime_rows;
                summary.change_equip_or_model_maps_matched +=
                    change_summary.change_equip_or_model_maps_matched;
                summary.change_equip_or_model_live_creatures_mutated +=
                    change_summary.change_equip_or_model_live_creatures_mutated;
                summary.change_equip_or_model_stale_index_or_wrong_kind +=
                    change_summary.change_equip_or_model_stale_index_or_wrong_kind;
                summary.change_equip_or_model_model_validation_unavailable +=
                    change_summary.change_equip_or_model_model_validation_unavailable;
            }
            GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts { event_id, activate } => {
                let smart_ai_summary =
                    game_event_run_smart_ai_scripts_like_cpp(manager, event_id, activate);
                summary.run_smart_ai_actions += 1;
                summary.run_smart_ai_maps_visited += smart_ai_summary.run_smart_ai_maps_visited;
                summary.run_smart_ai_creature_candidates +=
                    smart_ai_summary.run_smart_ai_creature_candidates;
                summary.run_smart_ai_gameobject_candidates +=
                    smart_ai_summary.run_smart_ai_gameobject_candidates;
                summary.run_smart_ai_creature_ai_enabled_unrepresented +=
                    smart_ai_summary.run_smart_ai_creature_ai_enabled_unrepresented;
                summary.run_smart_ai_script_dispatch_unrepresented +=
                    smart_ai_summary.run_smart_ai_script_dispatch_unrepresented;
            }
            GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                event_id,
                event_start_time,
            } => {
                summary.reset_event_seasonal_quests_actions += 1;
                if event_start_time == 0 {
                    summary.reset_event_seasonal_quests_event_start_time_zero += 1;
                } else {
                    summary.reset_event_seasonal_quests_event_start_time_nonzero += 1;
                }
                game_event_seasonal_quest_db_delete_like_cpp(
                    event_id,
                    event_start_time,
                    &mut summary,
                );
            }
            GameEventLiveUpdateActionLikeCpp::UpdateEventQuests { event_id, activate } => {
                let quest_summary =
                    game_event_update_quests_like_cpp(canonical_spawn_metadata, event_id, activate);
                summary.update_event_quests_actions += 1;
                summary.update_event_quests_creature_records_seen +=
                    quest_summary.update_event_quests_creature_records_seen;
                summary.update_event_quests_gameobject_records_seen +=
                    quest_summary.update_event_quests_gameobject_records_seen;
                summary.update_event_quests_creature_inserted +=
                    quest_summary.update_event_quests_creature_inserted;
                summary.update_event_quests_gameobject_inserted +=
                    quest_summary.update_event_quests_gameobject_inserted;
                summary.update_event_quests_creature_removed +=
                    quest_summary.update_event_quests_creature_removed;
                summary.update_event_quests_gameobject_removed +=
                    quest_summary.update_event_quests_gameobject_removed;
                summary.update_event_quests_creature_remove_misses +=
                    quest_summary.update_event_quests_creature_remove_misses;
                summary.update_event_quests_gameobject_remove_misses +=
                    quest_summary.update_event_quests_gameobject_remove_misses;
                summary.update_event_quests_creature_no_match +=
                    quest_summary.update_event_quests_creature_no_match;
                summary.update_event_quests_gameobject_no_match +=
                    quest_summary.update_event_quests_gameobject_no_match;
                summary.update_event_quests_creature_missing_event_buckets +=
                    quest_summary.update_event_quests_creature_missing_event_buckets;
                summary.update_event_quests_gameobject_missing_event_buckets +=
                    quest_summary.update_event_quests_gameobject_missing_event_buckets;
                summary.update_event_quests_creature_skipped_active_other_event +=
                    quest_summary.update_event_quests_creature_skipped_active_other_event;
                summary.update_event_quests_gameobject_skipped_active_other_event +=
                    quest_summary.update_event_quests_gameobject_skipped_active_other_event;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateWorldStates { event_id, activate } => {
                let world_state_summary = game_event_update_world_states_like_cpp(
                    canonical_spawn_metadata,
                    battlemaster_list_store,
                    world_state_mgr.as_deref_mut(),
                    player_registry,
                    event_id,
                    activate,
                );
                summary.update_world_states_actions += 1;
                summary.update_world_states_no_holiday +=
                    world_state_summary.update_world_states_no_holiday;
                summary.update_world_states_missing_event +=
                    world_state_summary.update_world_states_missing_event;
                summary.update_world_states_store_missing +=
                    world_state_summary.update_world_states_store_missing;
                summary.update_world_states_holiday_not_weekend_battleground +=
                    world_state_summary.update_world_states_holiday_not_weekend_battleground;
                summary.update_world_states_battlemaster_list_missing +=
                    world_state_summary.update_world_states_battlemaster_list_missing;
                summary.update_world_states_holiday_world_state_zero +=
                    world_state_summary.update_world_states_holiday_world_state_zero;
                summary.update_world_states_holiday_lookup_unrepresented +=
                    world_state_summary.update_world_states_holiday_lookup_unrepresented;
                summary.update_world_states_set_value_represented +=
                    world_state_summary.update_world_states_set_value_represented;
                summary.update_world_states_set_value_attempts +=
                    world_state_summary.update_world_states_set_value_attempts;
                summary.update_world_states_realm_changed_or_inserted +=
                    world_state_summary.update_world_states_realm_changed_or_inserted;
                summary.update_world_states_realm_unchanged_noop +=
                    world_state_summary.update_world_states_realm_unchanged_noop;
                summary.update_world_states_map_specific_no_map_unsupported +=
                    world_state_summary.update_world_states_map_specific_no_map_unsupported;
                summary.update_world_states_global_message_represented +=
                    world_state_summary.update_world_states_global_message_represented;
                summary.update_world_states_global_message_registry_missing +=
                    world_state_summary.update_world_states_global_message_registry_missing;
                summary.update_world_states_global_message_send_attempted +=
                    world_state_summary.update_world_states_global_message_send_attempted;
                summary.update_world_states_global_message_send_queued +=
                    world_state_summary.update_world_states_global_message_send_queued;
                summary.update_world_states_global_message_send_failed +=
                    world_state_summary.update_world_states_global_message_send_failed;
                summary.update_world_states_global_message_not_in_world_skipped +=
                    world_state_summary.update_world_states_global_message_not_in_world_skipped;
                summary.update_world_states_last_world_state_id =
                    world_state_summary.update_world_states_last_world_state_id;
                summary.update_world_states_last_world_state_value =
                    world_state_summary.update_world_states_last_world_state_value;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id } => {
                let npc_flag_summary = game_event_update_npc_flags_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    loaded_grid_creature_respawn_caches.template_store.as_ref(),
                    player_registry,
                    event_id,
                    active_event_ids,
                );
                summary.update_npc_flags_actions += 1;
                summary.update_npc_flags_records_seen +=
                    npc_flag_summary.update_npc_flags_records_seen;
                summary.update_npc_flags_missing_event_buckets +=
                    npc_flag_summary.update_npc_flags_missing_event_buckets;
                summary.update_npc_flags_missing_spawn_metadata +=
                    npc_flag_summary.update_npc_flags_missing_spawn_metadata;
                summary.update_npc_flags_template_npcflag_missing +=
                    npc_flag_summary.update_npc_flags_template_npcflag_missing;
                summary.update_npc_flags_maps_matched +=
                    npc_flag_summary.update_npc_flags_maps_matched;
                summary.update_npc_flags_indexed_guids +=
                    npc_flag_summary.update_npc_flags_indexed_guids;
                summary.update_npc_flags_live_creatures_mutated +=
                    npc_flag_summary.update_npc_flags_live_creatures_mutated;
                summary.update_npc_flags_stale_index_or_wrong_kind +=
                    npc_flag_summary.update_npc_flags_stale_index_or_wrong_kind;
                summary.update_npc_flags_low_applied +=
                    npc_flag_summary.update_npc_flags_low_applied;
                summary.update_npc_flags2_applied += npc_flag_summary.update_npc_flags2_applied;
                summary.update_npc_flags_values_updates_built +=
                    npc_flag_summary.update_npc_flags_values_updates_built;
                summary.update_npc_flags_values_update_empty +=
                    npc_flag_summary.update_npc_flags_values_update_empty;
                summary.update_npc_flags_values_update_map_id_out_of_range +=
                    npc_flag_summary.update_npc_flags_values_update_map_id_out_of_range;
                summary.update_npc_flags_values_update_registry_missing +=
                    npc_flag_summary.update_npc_flags_values_update_registry_missing;
                summary.update_npc_flags_values_update_not_in_world_skipped +=
                    npc_flag_summary.update_npc_flags_values_update_not_in_world_skipped;
                summary.update_npc_flags_values_update_wrong_map_skipped +=
                    npc_flag_summary.update_npc_flags_values_update_wrong_map_skipped;
                summary.update_npc_flags_values_update_send_attempted +=
                    npc_flag_summary.update_npc_flags_values_update_send_attempted;
                summary.update_npc_flags_values_update_send_queued +=
                    npc_flag_summary.update_npc_flags_values_update_send_queued;
                summary.update_npc_flags_values_update_send_failed +=
                    npc_flag_summary.update_npc_flags_values_update_send_failed;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor { event_id, activate } => {
                let npc_vendor_summary = game_event_update_npc_vendor_like_cpp(
                    canonical_spawn_metadata,
                    event_id,
                    activate,
                );
                summary.update_npc_vendor_actions += 1;
                summary.update_npc_vendor_records_seen +=
                    npc_vendor_summary.update_npc_vendor_records_seen;
                summary.update_npc_vendor_items_added +=
                    npc_vendor_summary.update_npc_vendor_items_added;
                summary.update_npc_vendor_items_removed +=
                    npc_vendor_summary.update_npc_vendor_items_removed;
                summary.update_npc_vendor_missing_event_buckets +=
                    npc_vendor_summary.update_npc_vendor_missing_event_buckets;
                summary.update_npc_vendor_remove_misses +=
                    npc_vendor_summary.update_npc_vendor_remove_misses;
                summary.update_npc_vendor_no_match += npc_vendor_summary.update_npc_vendor_no_match;
            }
        }
    }
    summary
}
