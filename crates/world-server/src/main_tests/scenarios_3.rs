//! Scenarios for [`super`], part 3.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_spawn_non_pool_gameobject_not_spawned_by_default_is_not_added_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    let event_id = 1;
    let spawn_id = 535501;
    let entry = 9001;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::GameObject,
            spawn_id,
            571,
            entry,
            0.0,
            0.0,
            -30,
        ),
        |_| false,
    );
    let guids = push_game_event_guid_for_test_like_cpp(
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
        SpawnObjectType::GameObject,
        event_id,
        spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                spawn_id,
                rotation: [0.0, 0.0, 0.0, 1.0],
                anim_progress: 55,
                state: 1,
                string_id: "game-event-go-not-default".to_string(),
                spawn_time_secs: -30,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(42, entry);
    manager
        .find_map_mut(571, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            spawn_id,
            535000,
        ));

    let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        None,
        &metadata,
        &caches,
        event_id,
    );

    assert_eq!(summary.gameobject.guids_seen, 1);
    assert_eq!(summary.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.gameobject.load_attempts, 1);
    assert_eq!(
        summary.gameobject.gameobject_not_spawned_by_default_skips,
        1
    );
    assert_eq!(summary.gameobject.successful_loaded_grid_spawns, 0);
    let map = manager.find_map(571, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
        0
    );
    assert!(map.get_gameobject_by_spawn_id_like_cpp(spawn_id).is_none());
}
#[test]
fn game_event_pool_spawn_uses_canonical_event_pool_ids_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let event_id = 7;
    let pool_id = 5321;
    let spawn_id = 532101;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: SpawnGroupTemplateData {
                group_id: 5321,
                name: "game-event-canonical-spawn".to_string(),
                map_id: 1,
                flags: SpawnGroupFlags::NONE,
            },
            id: 99,
            spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(10))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        store,
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        game_event_pools,
    );
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary =
        game_event_spawn_pools_for_event_like_cpp(&mut manager, None, &metadata, &caches, event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.missing_event_pool_ids);
    assert_eq!(summary.pool_summary.event_pool_ids_seen, 1);
    assert_eq!(summary.pool_summary.maps_matched, 1);
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map 1")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
    assert!(
        !manager
            .find_map(2, 0)
            .expect("test map 2")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}
#[test]
fn game_event_pool_unspawn_uses_canonical_event_pool_ids_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let event_id = 8;
    let pool_id = 5322;
    let spawn_id = 532201;
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(10))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        SpawnStore::new(),
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        game_event_pools,
    );
    for map_id in [1, 2] {
        let map = manager
            .find_map_mut(map_id, 0)
            .expect("test canonical map")
            .map_mut();
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            532200,
        ));
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
            .expect("test spawned creature pool data");
    }

    let summary = game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.missing_event_pool_ids);
    assert_eq!(summary.pool_summary.event_pool_ids_seen, 1);
    assert_eq!(summary.pool_summary.maps_matched, 1);
    assert!(
        !manager
            .find_map(1, 0)
            .expect("test map 1")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
    assert!(
        manager
            .find_map(2, 0)
            .expect("test map 2")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}
#[test]
fn game_event_pool_missing_event_id_is_noop_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let pool_id = 5323;
    let spawn_id = 532301;
    let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        SpawnStore::new(),
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
    );
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
        .expect("test spawned creature pool data");
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let spawn_summary =
        game_event_spawn_pools_for_event_like_cpp(&mut manager, None, &metadata, &caches, 99);
    let unspawn_summary = game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, 99);

    assert!(spawn_summary.missing_event_pool_ids);
    assert_eq!(spawn_summary.pool_summary.event_pool_ids_seen, 0);
    assert!(unspawn_summary.missing_event_pool_ids);
    assert_eq!(unspawn_summary.pool_summary.event_pool_ids_seen, 0);
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}
#[test]
fn game_event_pool_empty_event_id_list_is_noop_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 1;
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        SpawnStore::new(),
        PoolMgrLikeCpp::new(),
        game_event_pools,
    );
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let spawn_summary =
        game_event_spawn_pools_for_event_like_cpp(&mut manager, None, &metadata, &caches, event_id);
    let unspawn_summary =
        game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, event_id);

    assert!(!spawn_summary.missing_event_pool_ids);
    assert_eq!(spawn_summary.pool_summary.event_pool_ids_seen, 0);
    assert!(!unspawn_summary.missing_event_pool_ids);
    assert_eq!(unspawn_summary.pool_summary.event_pool_ids_seen, 0);
    assert_eq!(spawn_summary.pool_summary.maps_matched, 0);
    assert_eq!(unspawn_summary.pool_summary.maps_matched, 0);
}
#[test]
fn game_event_pool_spawn_filters_by_pool_template_map_id_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let pool_id = 5301;
    let spawn_id = 530101;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: SpawnGroupTemplateData {
                group_id: 5301,
                name: "game-event-spawn".to_string(),
                map_id: 1,
                flags: SpawnGroupFlags::NONE,
            },
            id: 99,
            spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let metadata = canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(
        store,
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
    );
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary =
        game_event_spawn_pools_like_cpp(&mut manager, None, &metadata, &caches, &[pool_id]);

    assert_eq!(summary.event_pool_ids_seen, 1);
    assert_eq!(summary.missing_pool_templates, 0);
    assert_eq!(summary.maps_matched, 1);
    assert_eq!(summary.pools_without_loaded_canonical_maps, 0);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert!(summary.blocked_pool_plan_errors.is_empty());
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map 1")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
    assert!(
        !manager
            .find_map(2, 0)
            .expect("test map 2")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}
#[test]
fn game_event_pool_spawn_missing_pool_template_is_counted_noop_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(PoolMgrLikeCpp::new());
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary = game_event_spawn_pools_like_cpp(&mut manager, None, &metadata, &caches, &[5302]);

    assert_eq!(summary.event_pool_ids_seen, 1);
    assert_eq!(summary.missing_pool_templates, 1);
    assert_eq!(summary.maps_matched, 0);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
    assert!(summary.blocked_pool_plan_errors.is_empty());
    assert!(
        !manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(530201)
    );
}
#[test]
fn game_event_pool_spawn_loaded_grid_records_blocked_loader_and_unloaded_skips_loader_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let loaded_spawn_id = 530301;
    let unloaded_spawn_id = 530302;
    let mut store = SpawnStore::new();
    let group = SpawnGroupTemplateData {
        group_id: 5303,
        name: "game-event-spawn-loaded-grid".to_string(),
        map_id: 1,
        flags: SpawnGroupFlags::NONE,
    };
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id: loaded_spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: group.clone(),
            id: 99,
            spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id: 5303,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id: unloaded_spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: group,
            id: 99,
            spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id: 5303,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(5303, PoolTemplateDataLikeCpp::new(2, 1));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5303);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(loaded_spawn_id, 0.0), 2);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(unloaded_spawn_id, 0.0), 2);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5303, group)
        .expect("test creature pool group");
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .ensure_grid_loaded(&wow_map::cell_from_world(0.0, 0.0));
    let metadata = canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(store, pool_mgr);
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary = game_event_spawn_pools_like_cpp(&mut manager, None, &metadata, &caches, &[5303]);

    assert_eq!(summary.maps_matched, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(summary.pool_spawn_action_load_plans, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    let map = manager.find_map(1, 0).expect("test map").map();
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(loaded_spawn_id)
    );
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(unloaded_spawn_id)
    );
}
#[test]
fn game_event_pool_unspawn_filters_by_pool_template_map_id_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let pool_id = 5291;
    let spawn_id = 529101;
    let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
    );

    for map_id in [1, 2] {
        let map = manager
            .find_map_mut(map_id, 0)
            .expect("test canonical map")
            .map_mut();
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            200,
        ));
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
            .expect("test spawned creature pool data");
    }

    let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[pool_id]);

    assert_eq!(summary.event_pool_ids_seen, 1);
    assert_eq!(summary.missing_pool_templates, 0);
    assert_eq!(summary.maps_matched, 1);
    assert_eq!(summary.pools_without_loaded_canonical_maps, 0);
    assert_eq!(summary.pool_respawn_timers_removed, 0);
    assert_eq!(summary.pool_respawn_timers_missing, 0);
    assert!(summary.blocked_pool_plan_errors.is_empty());
    let map_1 = manager.find_map(1, 0).expect("test map 1").map();
    assert_eq!(
        map_1.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        200
    );
    assert!(
        !map_1
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
    let map_2 = manager.find_map(2, 0).expect("test map 2").map();
    assert_eq!(
        map_2.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        200
    );
    assert!(
        map_2
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}
#[test]
fn game_event_pool_unspawn_missing_pool_template_is_counted_noop_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let spawn_id = 529201;
    let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::Creature,
        spawn_id,
        300,
    ));
    let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(PoolMgrLikeCpp::new());

    let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[5292]);

    assert_eq!(summary.event_pool_ids_seen, 1);
    assert_eq!(summary.missing_pool_templates, 1);
    assert_eq!(summary.maps_matched, 0);
    assert_eq!(summary.pool_respawn_timers_removed, 0);
    assert!(summary.blocked_pool_plan_errors.is_empty());
    assert_eq!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        300
    );
}
#[test]
fn game_event_pool_unspawn_always_delete_removes_non_spawned_member_timer_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let pool_id = 5293;
    let spawn_id = 529301;
    let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
    );
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            400,
        ));

    let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[pool_id]);

    assert_eq!(summary.maps_matched, 1);
    assert_eq!(summary.pool_objects_removed, 0);
    assert_eq!(summary.pool_respawn_timers_removed, 1);
    assert_eq!(summary.pool_respawn_timers_missing, 0);
    assert_eq!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );
}
#[test]
fn worldserver_cli_defaults_match_cpp_startup_options() {
    let cli = WorldServerCliLikeCpp::parse_from(Vec::<String>::new());

    assert_eq!(cli.config_file, None);
    assert_eq!(cli.config_dir, PathBuf::from("worldserver.conf.d"));
    assert!(!cli.show_help);
    assert!(!cli.show_version);
}
#[test]
fn worldserver_cli_parses_short_and_long_options_like_cpp() {
    let cli = WorldServerCliLikeCpp::parse_from(
        [
            "--unknown",
            "--config",
            "/tmp/world.conf",
            "-cd",
            "/tmp/world.conf.d",
        ]
        .into_iter()
        .map(str::to_string),
    );

    assert_eq!(cli.config_file, Some(PathBuf::from("/tmp/world.conf")));
    assert_eq!(cli.config_dir, PathBuf::from("/tmp/world.conf.d"));

    let cli = WorldServerCliLikeCpp::parse_from(
        [
            "--config=/etc/rustycore/worldserver.conf",
            "--config-dir=/etc/rustycore/worldserver.conf.d",
            "--help",
            "--version",
        ]
        .into_iter()
        .map(str::to_string),
    );

    assert_eq!(
        cli.config_file,
        Some(PathBuf::from("/etc/rustycore/worldserver.conf"))
    );
    assert_eq!(
        cli.config_dir,
        PathBuf::from("/etc/rustycore/worldserver.conf.d")
    );
    assert!(cli.show_help);
    assert!(cli.show_version);
}
#[test]
fn worldserver_cli_help_and_version_match_cpp_surface() {
    let help = worldserver_cli_help_like_cpp();
    assert!(help.contains("--config"));
    assert!(help.contains("--config-dir"));
    assert!(!help.contains("--update-databases-only"));
    assert!(help.contains("--version"));
    assert!(help.contains("--help"));

    let version = worldserver_full_version_like_cpp();
    assert!(version.contains("RustyCore World Server"));
    assert!(version.contains(env!("CARGO_PKG_VERSION")));
    let revision = worldserver_revision_like_cpp();
    assert!(
        matches!(revision.len(), 40 | 64)
            && revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "world-server builds from a Git checkout must embed the exact source revision"
    );
    assert!(version.contains(revision));
}
#[test]
fn world_runtime_state_stop_and_counter_match_cpp_contract() {
    let world = WorldRuntimeStateLikeCpp::new();

    assert!(!world.is_stopped_like_cpp());
    assert_eq!(world.get_exit_code_like_cpp(), SHUTDOWN_EXIT_CODE_LIKE_CPP);
    assert_eq!(world.world_loop_counter_like_cpp(), 0);

    assert_eq!(world.increment_world_loop_counter_like_cpp(), 1);
    assert_eq!(world.increment_world_loop_counter_like_cpp(), 2);
    assert_eq!(world.world_loop_counter_like_cpp(), 2);

    world.stop_now_like_cpp(1);
    assert!(world.is_stopped_like_cpp());
    assert_eq!(world.get_exit_code_like_cpp(), 1);
    assert_eq!(
        process_exit_code_like_cpp(2),
        std::process::ExitCode::from(2)
    );
    assert_eq!(ERROR_EXIT_CODE_LIKE_CPP, 1);
    assert_eq!(RESTART_EXIT_CODE_LIKE_CPP, 2);
}
#[test]
fn freeze_detector_poll_matches_cpp_counter_contract() {
    let mut detector = FreezeDetectorLikeCpp::new(60_000, 1_000);

    assert_eq!(
        detector.poll_once_like_cpp(2_000, 1),
        FreezeDetectorPollOutcomeLikeCpp::Advanced
    );
    assert_eq!(
        detector.poll_once_like_cpp(61_000, 1),
        FreezeDetectorPollOutcomeLikeCpp::StillAlive
    );
    assert_eq!(
        detector.poll_once_like_cpp(62_001, 1),
        FreezeDetectorPollOutcomeLikeCpp::Abort { stuck_ms: 60_001 }
    );
    assert_eq!(
        detector.poll_once_like_cpp(63_000, 2),
        FreezeDetectorPollOutcomeLikeCpp::Advanced
    );
}
#[test]
fn world_update_loop_step_matches_cpp_timing_contract() {
    let world = WorldRuntimeStateLikeCpp::new();

    assert_eq!(
        half_max_core_stuck_time_like_cpp(0),
        u32::MAX,
        "C++ uses numeric_limits<uint32>::max() when halfMaxCoreStuckTime is zero"
    );

    let sleep = world_update_loop_step_like_cpp(&world, 1_000, 1_003, 10, 60_000);
    assert_eq!(
        sleep,
        WorldUpdateLoopStepOutcomeLikeCpp::Sleep {
            sleep_ms: 7,
            log_waiting_like_cpp: false
        }
    );
    assert_eq!(
        world.world_loop_counter_like_cpp(),
        1,
        "C++ increments m_worldLoopCounter before the sleep branch"
    );

    let long_sleep = world_update_loop_step_like_cpp(&world, 2_000, 2_000, 30_000, 60_000);
    assert_eq!(
        long_sleep,
        WorldUpdateLoopStepOutcomeLikeCpp::Sleep {
            sleep_ms: 30_000,
            log_waiting_like_cpp: true
        }
    );

    let update = world_update_loop_step_like_cpp(&world, 3_000, 3_025, 10, 60_000);
    assert_eq!(
        update,
        WorldUpdateLoopStepOutcomeLikeCpp::Update {
            diff_ms: 25,
            next_real_prev_time_ms: 3_025
        }
    );

    let wrap_update = world_update_loop_step_like_cpp(&world, u32::MAX - 4, 5, 1, 60_000);
    assert_eq!(
        wrap_update,
        WorldUpdateLoopStepOutcomeLikeCpp::Update {
            diff_ms: 10,
            next_real_prev_time_ms: 5
        }
    );
}
#[test]
fn world_update_loop_direct_configs_match_cpp_defaults_and_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    let root = unique_temp_dir("world_update_loop_direct_configs");
    let config = root.join("worldserver.conf");

    fs::write(&config, "").expect("write empty config failed");
    wow_config::load_config(config.to_str().expect("utf8 config path"))
        .expect("load empty config failed");

    assert_eq!(min_world_update_time_ms_like_cpp(), 1);
    assert_eq!(max_core_stuck_time_secs_like_cpp(), 60);
    assert_eq!(max_core_stuck_time_ms_like_cpp(), 60_000);

    fs::write(&config, "MinWorldUpdateTime = 7\nMaxCoreStuckTime = 0\n")
        .expect("write override config failed");
    wow_config::load_config(config.to_str().expect("utf8 config path"))
        .expect("load override config failed");

    assert_eq!(min_world_update_time_ms_like_cpp(), 7);
    assert_eq!(max_core_stuck_time_secs_like_cpp(), 0);
    assert_eq!(
        max_core_stuck_time_ms_like_cpp(),
        0,
        "C++ treats MaxCoreStuckTime=0 as disabled before constructing FreezeDetector"
    );
}
#[test]
fn world_config_resolution_prefers_lowercase_cpp_name() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    let root = unique_temp_dir("world_config_resolution");
    let lower = root.join("worldserver.conf");
    let legacy = root.join("WorldServer.conf");

    fs::write(&lower, "WorldServerPort = 8085\n").expect("write lower failed");
    fs::write(&legacy, "WorldServerPort = 9000\n").expect("write legacy failed");

    let report = load_world_config_from(
        &[
            lower.to_str().expect("utf8 path"),
            legacy.to_str().expect("utf8 path"),
        ],
        root.join("worldserver.conf.d").to_str().expect("utf8 path"),
    )
    .expect("config should load");

    assert_eq!(report.candidate_index, 0);
    assert_eq!(wow_config::get_value::<u16>("WorldServerPort"), Some(8085));

    fs::remove_dir_all(root).expect("cleanup failed");
}
#[test]
fn world_config_cli_config_uses_exact_file_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    let root = unique_temp_dir("world_config_cli_exact");
    let default_file = root.join("worldserver.conf");
    let override_file = root.join("custom-world.conf");
    let config_dir = root.join("custom-world.conf.d");

    fs::create_dir_all(&config_dir).expect("config dir failed");
    fs::write(&default_file, "WorldServerPort = 8085\n").expect("write default failed");
    fs::write(&override_file, "WorldServerPort = 9100\n").expect("write override failed");
    fs::write(
        config_dir.join("overlay.conf"),
        "InstanceServerPort = 9101\n",
    )
    .expect("write overlay failed");

    let override_path = override_file.to_string_lossy().into_owned();
    let config_dir_path = config_dir.to_string_lossy().into_owned();
    let report = load_world_config_from(&[override_path.as_str()], &config_dir_path)
        .expect("config should load");

    assert_eq!(report.initial_file, override_path);
    assert_eq!(report.candidate_index, 0);
    assert_eq!(wow_config::get_value::<u16>("WorldServerPort"), Some(9100));
    assert_eq!(
        wow_config::get_value::<u16>("InstanceServerPort"),
        Some(9101)
    );

    fs::remove_dir_all(root).expect("cleanup failed");
}
#[test]
fn world_network_config_uses_resolved_world_configs() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        r#"
WorldServerPort = 70000
InstanceServerPort = 70001
Expansion = 9
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(world_config_u16(&configs, "CONFIG_PORT_WORLD", 8085), 4464);
    assert_eq!(
        world_config_u16(&configs, "CONFIG_PORT_INSTANCE", 8086),
        4465
    );
    assert_eq!(world_config_u8(&configs, "CONFIG_EXPANSION", 2), 9);
}
#[test]
fn world_server_binary_delegates_to_the_library_composition_root() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binary_source = fs::read_to_string(manifest_dir.join("src/main.rs"))
        .expect("world-server binary source should be readable");

    assert!(binary_source.contains("world_server::run("));
    assert!(!binary_source.contains("start_world_listener"));
    assert!(!binary_source.contains("create_session"));
}
#[test]
fn library_composition_preserves_cpp_startup_and_shutdown_order() {
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"))
        .expect("world-server composition source should be readable");

    let mut cursor = 0;
    for stage in [
        "process_startup::initialize(args)",
        "database_startup::open_primary_databases().await?;",
        "database_startup::compose_world_catalog_ports(",
        "database_startup::open_hotfix_database().await?;",
        "database_startup::validate_runtime_schemas(",
        "realm_startup::initialize_availability(",
        "guid_allocator_startup::initialize_guid_allocators(",
        "world_startup::run_world_startup(",
    ] {
        let offset = source[cursor..]
            .find(stage)
            .unwrap_or_else(|| panic!("missing or reordered composition stage: {stage}"));
        cursor += offset + stage.len();
    }
    for call in [
        "process_startup::initialize(",
        "database_startup::open_primary_databases(",
        "database_startup::compose_world_catalog_ports(",
        "database_startup::open_hotfix_database(",
        "database_startup::validate_runtime_schemas(",
        "realm_startup::initialize_availability(",
        "world_startup::run_world_startup(",
    ] {
        assert_eq!(
            source.matches(call).count(),
            1,
            "the root must invoke {call} exactly once"
        );
    }
    let continuation = include_str!("../app/world_startup.rs");
    let mut cursor = 0;
    for stage in [
        "world_template_startup::load(",
        "hotfix_delivery_startup::load(",
        "account_admission_startup::load(",
        "condition_startup::load_world_conditions(",
        "let graveyard_store = Arc::new(",
        "npc_service_catalogs::load_spell_click(",
        "spell_world_startup::load_world_spell_catalogs(",
        "let spell_store = Arc::new(",
        "group_startup::load_group_startup(",
        "world_instance_startup::load_world_instance_managers(",
        "world_instance_startup::build_loaded_grid_caches(",
        "game_event_startup::start_system(",
        "let (game_event_quest_complete_tx, game_event_quest_complete_rx)",
        "tokio::spawn(run_game_event_quest_complete_processor_like_cpp(",
        "wow_database::build_player_name_query_port_like_cpp(",
        "session_persistence::build_session_persistence_ports(",
        "let session_resources = SessionResources {",
        "let session_resources = Arc::new(session_resources);",
        "serve::serve(",
    ] {
        assert_eq!(
            continuation.matches(stage).count(),
            1,
            "continuation must invoke {stage} exactly once"
        );
        let offset = continuation[cursor..]
            .find(stage)
            .unwrap_or_else(|| panic!("missing or reordered continuation stage: {stage}"));
        cursor += offset + stage.len();
    }
    let account_admission = include_str!("../app/account_admission_startup.rs");
    for stage in [
        "realm_startup::load_identity(",
        "let login_db = Arc::new(",
        "wow_world::session::registry::build_dispatch_table();",
    ] {
        assert_eq!(account_admission.matches(stage).count(), 1);
        assert!(!continuation.contains(stage));
    }
    let admission_call = continuation
        .find("account_admission_startup::load(")
        .unwrap();
    let admission_arguments = continuation[admission_call..]
        .split_once(".await")
        .unwrap()
        .0;
    for argument in ["login_db_slot,", "realm_availability,", "&stat_tables,"] {
        assert_eq!(admission_arguments.matches(argument).count(), 1);
    }
    // Follow each production delegation separately; concatenating sources would
    // accidentally impose file layout instead of the effective call order.
    for (phase, stages) in [
        (
            include_str!("../app/account_admission_startup.rs"),
            &[
                "realm_startup::load_identity(",
                "let login_db = Arc::new(",
                "let battle_pet_account_registry = Arc::new(",
                "wow_world::session::registry::build_dispatch_table();",
                "info!(\"Loaded {} packet handlers\", table.len());",
                "let account_lookup: Arc<dyn AccountLookup> = Arc::new(",
                "Ok((",
                "realm_identity,",
                "login_db,",
                "battle_pet_account_registry,",
                "table,",
                "account_lookup,",
            ][..],
        ),
        (
            include_str!("../app/process_startup.rs"),
            &[
                "let cli = WorldServerCliLikeCpp::parse_from(args);",
                "if cli.show_help",
                "if cli.show_version",
                "let world_runtime_state = Arc::new(",
                "let config_report = load_world_config(&cli)?;",
                "log_startup_banner_like_cpp(&config_report);",
                "let world_configs = wow_config::load_world_config_values();",
                "create_pid_file_from_config_like_cpp()?;",
                "let ip_location_store = Arc::new(",
            ][..],
        ),
        (
            include_str!("../app/realm_startup.rs"),
            &[
                "pub(super) async fn initialize_availability(",
                "clear_online_accounts_like_cpp(login_db, char_db, realm_id).await?;",
                "verify_world_db_version_like_cpp(world_db).await?;",
                "set_realm_offline(login_db, realm_id).await?;",
                "update_realm_list_once_like_cpp(login_db, &realm_list)",
                "spawn_realm_list_update_loop_like_cpp(",
                "pub(super) async fn load_identity(",
                "load_realm_info_from_snapshot_like_cpp(realm_list, realm_id)?;",
                "realm_name_records_from_snapshot_like_cpp(realm_list);",
                "load_realm_win64_auth_seed_like_cpp(login_db, realm_build).await?;",
                "let realm_external_address =",
                "let realm_local_address =",
            ][..],
        ),
        (
            include_str!("../app/serve.rs"),
            &[
                "let session_mgr =",
                "network_configuration::load(",
                "listener_startup::load(",
                "listener_startup::publish_realm_online(",
                "runtime_launch::load(",
                "runtime_supervision::supervise_and_shutdown(",
            ][..],
        ),
        (
            include_str!("../app/listener_startup.rs"),
            &[
                "let (realm_listener_ready_tx, realm_listener_ready_rx)",
                "let (instance_listener_ready_tx, instance_listener_ready_rx)",
                "let mut realm_handle = tokio::spawn(",
                "wow_network::start_world_listener(",
                "let mut instance_handle = tokio::spawn(",
                "wow_network::start_instance_listener(",
                "let realm_network_abort_handle =",
                "let instance_network_abort_handle =",
                "tokio::join!(realm_listener_ready_rx, instance_listener_ready_rx)",
                "let listener_start_error = match",
                "if let Some(error) = listener_start_error",
                "pub(super) async fn publish_realm_online(",
                "if let Err(error) = crate::set_realm_online(login_db, realm_id).await",
                "stop_world_network_like_cpp([",
                "return Err(error);",
            ][..],
        ),
        (
            include_str!("../app/runtime_launch.rs"),
            &[
                "let map_update_interval_ms =",
                "let legacy_creature_global_runtime_enabled =",
                "manager.set_tick_owner(",
                "let respawn_condition_interval_ms =",
                "let respawn_db_mutation_order =",
                "let respawn_db_producer_stop =",
                "spawn_respawn_db_writer_like_cpp(",
                "spawn_canonical_map_update_loop(",
                "spawn_legacy_creature_runtime_update_loop_like_cpp(",
                "spawn_group_ready_check_tick_loop(",
                "spawn_db_keepalive_loop_like_cpp(",
            ][..],
        ),
    ] {
        let mut cursor = 0;
        for stage in stages {
            let offset = phase[cursor..]
                .find(stage)
                .unwrap_or_else(|| panic!("missing or reordered delegated startup stage: {stage}"));
            cursor += offset + stage.len();
        }
    }
    let serving_source = include_str!("../app/serve.rs");
    for call in [
        "listener_startup::load(",
        "listener_startup::publish_realm_online(",
        "runtime_launch::load(",
        "runtime_supervision::supervise_and_shutdown(",
    ] {
        assert_eq!(
            serving_source.matches(call).count(),
            1,
            "the serving phase must invoke {call} exactly once"
        );
        assert!(
            !source.contains(call),
            "the root must delegate {call} to its serving phase"
        );
    }
    let database_source = include_str!("../app/database_startup.rs");
    let mut cursor = 0;
    for stage in [
        "pub(super) async fn open_primary_databases(",
        "LoginDatabase::open_with_pool_size(",
        "CharacterDatabase::open_with_pool_size(",
        "WorldDatabase::open_with_pool_size(",
        "pub(super) async fn open_hotfix_database(",
        "HotfixDatabase::open_with_pool_size(",
        "pub(super) async fn validate_runtime_schemas(",
        "let migration_manifest = wow_database::migration::bundled_manifest()?;",
        "wow_database::migration::DatabaseKind::Auth",
        "wow_database::migration::DatabaseKind::Characters",
        "wow_database::migration::DatabaseKind::World",
        "wow_database::migration::DatabaseKind::Hotfixes",
        "wow_database::migration::validate_runtime_schema(",
    ] {
        let offset = database_source[cursor..]
            .find(stage)
            .unwrap_or_else(|| panic!("missing or reordered database startup stage: {stage}"));
        cursor += offset + stage.len();
    }
    for call in [
        "LoginDatabase::open_with_pool_size(",
        "CharacterDatabase::open_with_pool_size(",
        "WorldDatabase::open_with_pool_size(",
        "HotfixDatabase::open_with_pool_size(",
        "wow_database::migration::validate_runtime_schema(",
    ] {
        assert_eq!(
            database_source.matches(call).count(),
            1,
            "the database child must retain one {call}"
        );
        assert!(
            !source.contains(call),
            "the root must delegate {call} to its database child"
        );
    }

    assert_eq!(
        serving_source
            .matches("runtime_supervision::supervise_and_shutdown(")
            .count(),
        1,
        "the serving phase must invoke the production supervisor exactly once"
    );
    let supervision_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app/runtime_supervision.rs"),
    )
    .expect("world-server supervision source should be readable");
    let mut cursor = 0;
    for stage in [
        "pub(super) async fn supervise_and_shutdown(",
        "wow_script::lifecycle::on_startup_like_cpp()",
        "shutdown_signal()",
        "active_session_registry.begin_shutdown_like_cpp();",
        "active_session_registry.close_tick_admission()",
        "active_session_registry.wait_for_quiescence(",
        "active_session_registry.enable_session_drain(receipt)",
        "kick_all_sessions_like_cpp(active_session_registry)",
        "update_sessions_shutdown_flush_once_like_cpp(",
        "stop_world_network_like_cpp([",
        "active_session_registry.request_session_stop_like_cpp();",
        "let sessions_drained = active_session_registry",
        "cancel_all_sessions_like_cpp()",
        "let battle_pet_operations_drained = battle_pet_account_registry",
        "active_session_registry.wait_for_terminal_settlement(",
        "active_session_registry.authorize_final_respawn_tick(receipt)",
        "respawn_db_producer_stop.store(true, Ordering::Release);",
        "stop_respawn_db_producer_like_cpp(",
        "active_session_registry.close_final_tick_admission();",
        "active_session_registry.wait_for_terminal_settlement(",
        "respawn_db_writer_tx.close_like_cpp();",
        "drop(respawn_db_writer_tx);",
        "drain_respawn_db_writer_like_cpp(",
        "game_event_quest_complete_handle.abort();",
        "db_keepalive_handle.abort();",
        "realm_list_update_handle.abort();",
        "clear_online_accounts_like_cpp(login_db, char_db, realm_id).await",
        "wow_script::lifecycle::on_shutdown_like_cpp()",
        "set_realm_offline(login_db, realm_id).await",
        "item_guid_allocator_advisory_lock.release_like_cpp().await",
        "Ok(process_exit_code_like_cpp(",
    ] {
        let offset = supervision_source[cursor..]
            .find(stage)
            .unwrap_or_else(|| panic!("missing or reordered supervision stage: {stage}"));
        cursor += offset + stage.len();
    }
}
