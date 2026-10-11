use super::super::*;
/// Initialize one loaded-grid creature's default waypoint movement on its
/// canonical incarnation (C++ `Creature::AddToWorld` → `MotionMaster::Initialize`
/// → `WaypointMovementGenerator::DoInitialize` with the spawn's `PathId`).
///
/// #1263 F6-8D3b-2: this used to happen only on the legacy compatibility copy
/// the canonical→legacy mirror built for every loaded-grid creature; the mirror
/// is gone and the canonical incarnation is the creature's only
/// representation. Random movement keeps its lazy initialization on the first
/// movement update; idle needs none. Returns whether a waypoint path was
/// initialized.
pub(crate) fn initialize_loaded_grid_creature_default_motion_like_cpp(
    creature: &mut wow_entities::Creature,
    waypoint_paths: &spawn_store_loader::WaypointPathStoreLikeCpp,
) -> bool {
    if creature.default_movement_type() != wow_entities::MovementGeneratorType::Waypoint {
        return false;
    }
    wow_world::creature_movement::CreatureMovementLikeCpp::canonical_like_cpp(creature)
        .initialize_default_waypoint_movement_with_path_resolver_like_cpp(|path_id| {
            waypoint_paths.get(path_id).cloned()
        });
    true
}

/// [`initialize_loaded_grid_creature_default_motion_like_cpp`] for the
/// creatures a map-owned respawn/spawn pass already added to `map`.
pub(crate) fn initialize_loaded_grid_primary_records_default_motion_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    waypoint_paths: &spawn_store_loader::WaypointPathStoreLikeCpp,
    records: &[wow_entities::MapObjectRecord],
) -> usize {
    records
        .iter()
        .filter_map(|record| record.creature().map(wow_entities::Creature::guid))
        .filter(|guid| {
            map.get_typed_creature_mut(*guid).is_some_and(|creature| {
                initialize_loaded_grid_creature_default_motion_like_cpp(creature, waypoint_paths)
            })
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
        let already_loaded = match object_type {
            wow_map::SpawnObjectType::Creature => {
                map.get_creature_by_spawn_id_like_cpp(spawn_id).is_some()
            }
            wow_map::SpawnObjectType::GameObject => {
                map.get_gameobject_by_spawn_id_like_cpp(spawn_id).is_some()
            }
            wow_map::SpawnObjectType::AreaTrigger => map
                .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
                .is_some(),
        };
        if already_loaded {
            outcome.skipped_already_loaded += 1;
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

        let mut primary_record = records.primary_record;
        if let Some(creature) = primary_record.creature_mut() {
            initialize_loaded_grid_creature_default_motion_like_cpp(
                creature,
                canonical_spawn_metadata.waypoint_paths_like_cpp(),
            );
        }
        match map.add_map_object_record_to_map_like_cpp(primary_record) {
            Ok(_add) => match object_type {
                wow_map::SpawnObjectType::Creature => {
                    outcome.creature_records_added += 1;
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
        player_visible_grid_coords_like_cpp(position, wow_entities::VISIBILITY_RADIUS_LIKE_CPP);

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
            || outcome.load_record_missing != 0)
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
            "RUST_CREATURE_VIS login_grid_load"
        );
    }

    outcome
}
