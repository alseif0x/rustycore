//! Scenarios for [`super`], part 10.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn loaded_grid_area_trigger_record_returns_area_trigger_record_like_cpp() {
    let spawn_id = 88;
    let create_properties_id = 2001;
    let template_id = 9001;
    let mut store = SpawnStore::new();
    let spawn = SpawnData {
        object_type: SpawnObjectType::AreaTrigger,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: create_properties_id,
        spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 1.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_area_trigger_spawn(&spawn);
    let metadata = super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
        store,
        BTreeMap::new(),
    )
    .with_area_trigger_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::super::spawn_store_loader::AreaTriggerSpawnRuntimeRowLikeCpp {
            spawn_id,
            create_properties_id: wow_data::AreaTriggerIdLikeCpp {
                id: create_properties_id,
                is_custom: false,
            },
            spell_for_visuals: None,
        },
    )]));
    let template_store =
        area_trigger_template_store_for_loaded_grid_like_cpp(create_properties_id, template_id);
    let mut map = wow_map::Map::new(571, 0, 0, 60_000);

    let record = build_loaded_grid_area_trigger_record_like_cpp(
        &mut map,
        SpawnObjectType::AreaTrigger,
        spawn_id,
        &metadata,
        &template_store,
    )
    .expect("loaded-grid AreaTrigger builder should return loaded-grid records");
    let area_trigger = record
        .primary_record
        .area_trigger()
        .expect("builder should return a typed AreaTrigger MapObjectRecord");

    assert_eq!(
        record.primary_record.kind(),
        wow_entities::AccessorObjectKind::AreaTrigger
    );
    assert_eq!(area_trigger.spawn_id(), spawn_id);
    assert!(area_trigger.is_static_spawn());
    assert_eq!(
        area_trigger.world().guid().high_type(),
        wow_core::guid::HighGuid::AreaTrigger
    );
    assert_eq!(u32::from(area_trigger.world().guid().map_id()), 571);
    assert_eq!(area_trigger.world().guid().entry(), template_id);
    assert_eq!(area_trigger.world().guid().counter(), 1);
    assert_eq!(
        area_trigger.create_properties_id().unwrap().id,
        create_properties_id
    );
    assert_eq!(area_trigger.template_id().unwrap().id, template_id);
    assert_eq!(area_trigger.data().spell_visual_id, 0);
}
#[test]
fn loaded_grid_area_trigger_loader_materializes_loaded_map_grid_like_cpp() {
    let spawn_id = 89;
    let create_properties_id = 2002;
    let template_id = 9002;
    let mut store = SpawnStore::new();
    let spawn = SpawnData {
        object_type: SpawnObjectType::AreaTrigger,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: create_properties_id,
        spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 1.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_area_trigger_spawn(&spawn);
    let metadata = super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
        store,
        BTreeMap::new(),
    )
    .with_area_trigger_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::super::spawn_store_loader::AreaTriggerSpawnRuntimeRowLikeCpp {
            spawn_id,
            create_properties_id: wow_data::AreaTriggerIdLikeCpp {
                id: create_properties_id,
                is_custom: false,
            },
            spell_for_visuals: None,
        },
    )]));
    let template_store =
        area_trigger_template_store_for_loaded_grid_like_cpp(create_properties_id, template_id);
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(571, 0);
    manager
        .find_map_mut(571, 0)
        .expect("created map")
        .map_mut()
        .ensure_grid_loaded(&wow_map::map::cell_from_world(1.0, 2.0));

    let summary = load_loaded_grid_area_triggers_like_cpp(&mut manager, &metadata, &template_store);

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.loaded_grids_evaluated, 1);
    assert_eq!(summary.grid_not_loaded, 0);
    assert_eq!(summary.metadata_entries, 1);
    assert_eq!(summary.loaded_grid_primary_records, 1);
    assert_eq!(summary.loaded_area_trigger_guids.len(), 1);
    assert_eq!(summary.add_to_map_errors, 0);
    let area_trigger = manager
        .find_map_mut(571, 0)
        .expect("created map")
        .map()
        .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
        .expect("AreaTrigger should be materialized on the loaded grid");
    assert_eq!(
        summary.loaded_area_trigger_guids,
        vec![area_trigger.world().guid()]
    );
    assert_eq!(area_trigger.spawn_id(), spawn_id);
    assert_eq!(area_trigger.template_id().unwrap().id, template_id);
    assert_eq!(
        area_trigger.world().guid().high_type(),
        wow_core::guid::HighGuid::AreaTrigger
    );

    let second = load_loaded_grid_area_triggers_like_cpp(&mut manager, &metadata, &template_store);
    assert_eq!(second.maps_evaluated, 1);
    assert_eq!(second.loaded_grids_evaluated, 1);
    assert_eq!(second.metadata_entries, 0);
    assert_eq!(second.loaded_grid_primary_records, 0);
    assert!(second.loaded_area_trigger_guids.is_empty());
    assert_eq!(second.skipped_already_loaded, 1);
}
#[test]
fn loaded_grid_gameobject_respawn_record_returns_gameobject_record_like_cpp() {
    let spawn_id = 77;
    let entry = 9001;
    let mut store = SpawnStore::new();
    let spawn = SpawnData {
        object_type: SpawnObjectType::GameObject,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: entry,
        spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 1.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 30,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_object_spawn(&spawn, |_| false);
    let metadata = super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
        store,
        BTreeMap::new(),
    )
    .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::super::spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
            spawn_id,
            rotation: [0.0, 0.0, 0.0, 1.0],
            anim_progress: 55,
            state: 1,
            string_id: "live-gameobject".to_string(),
            spawn_time_secs: 30,
        },
    )]));
    let mut data = [0; wow_entities::MAX_GAMEOBJECT_DATA];
    data[11] = 1;
    let mut caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    caches.gameobject_template_store = Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry,
                go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                display_id: 44,
                name: "Live Loaded GO".to_string(),
                size: 1.0,
                data,
                content_tuning_id: 0,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: String::new(),
                addon: None,
            },
        ]),
    );
    let mut map = wow_map::Map::new(571, 0, 0, 60_000);
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id,
        entry,
        respawn_time: 1_234,
        grid_id: 7,
    });

    let record = build_loaded_grid_gameobject_respawn_record_like_cpp(
        &mut map,
        SpawnObjectType::GameObject,
        spawn_id,
        &metadata,
        &caches,
    )
    .expect("loaded-grid GameObject builder should return loaded-grid records");
    let game_object = record
        .primary_record
        .game_object()
        .expect("builder should return a typed GameObject MapObjectRecord");

    assert_eq!(
        record.primary_record.kind(),
        wow_entities::AccessorObjectKind::GameObject
    );
    assert_eq!(game_object.spawn_id(), spawn_id);
    assert_eq!(
        game_object.world().guid().high_type(),
        wow_core::guid::HighGuid::GameObject
    );
    assert_eq!(u32::from(game_object.world().guid().map_id()), 571);
    assert_eq!(game_object.world().guid().entry(), entry);
    assert_eq!(game_object.world().guid().counter(), 1);
    assert_eq!(
        game_object.respawn_time(),
        0,
        "ProcessRespawns erases due timer before LoadFromDB, so new GO observes no map respawn time"
    );
}
#[test]
fn loaded_grid_creature_respawn_record_variable_level_returns_creature_record_like_cpp() {
    let spawn_id = 54_984;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(67, 571, SpawnGroupFlags::NONE, spawn_id)]);
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "variable-level-live".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let mut caches = variable_loaded_grid_creature_respawn_caches_like_cpp(entry);
    caches.realm_id = 7;
    let mut map = wow_map::Map::new(571, 0, 2, 60_000);
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id,
        entry,
        respawn_time: 0,
        grid_id: 7,
    });

    let record = build_loaded_grid_creature_respawn_record_like_cpp(
        &mut map,
        SpawnObjectType::Creature,
        spawn_id,
        &metadata,
        &caches,
    )
    .expect("variable-level loaded-grid Creature builder should no longer block");
    let creature = record
        .primary_record
        .creature()
        .expect("builder should return a typed Creature MapObjectRecord");
    let level = creature.ai_level();

    assert!((18..=20).contains(&level));
    assert_eq!(
        record.primary_record.kind(),
        wow_entities::AccessorObjectKind::Creature
    );
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    assert_eq!(
        creature.guid().high_type(),
        wow_core::guid::HighGuid::Creature
    );
    assert_eq!(creature.guid().realm_id(), 7);
    assert_eq!(u32::from(creature.guid().map_id()), 571);
    assert_eq!(creature.guid().entry(), entry);
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
    assert_eq!(creature.ai_max_health(), u64::from(level) * 20);
    assert_eq!(creature.ai_current_health(), creature.ai_max_health());
}
#[test]
fn login_grid_load_preserves_precreated_dungeon_kind_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_map_entry(
        33,
        77,
        0,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(33, wow_data::map::MAP_INSTANCE);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        33,
        Some(77),
        Position::ZERO,
    );

    assert!(!outcome.map_unavailable);
    assert!(!outcome.map_created);
    assert!(matches!(
        canonical.lock().unwrap().find_map(33, 77).unwrap().kind(),
        wow_map::ManagedMapKind::Dungeon { .. }
    ));
}
#[test]
fn login_grid_load_accepts_authoritative_garrison_world_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(1_151, 0);
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 1_151,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        1_151,
        Some(0),
        Position::ZERO,
    );

    assert!(!outcome.map_unavailable);
    assert!(!outcome.map_created);
    assert!(matches!(
        canonical.lock().unwrap().find_map(1_151, 0).unwrap().kind(),
        wow_map::ManagedMapKind::World
    ));
}
#[test]
fn login_grid_load_does_not_fabricate_missing_instanceable_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(33, wow_data::map::MAP_INSTANCE);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        33,
        Some(77),
        Position::ZERO,
    );

    assert!(outcome.map_unavailable);
    assert!(!outcome.map_created);
    assert!(canonical.lock().unwrap().find_map(33, 77).is_none());
}
#[test]
fn login_grid_load_rejects_stale_dungeon_zero_world_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(33, 0);
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(33, wow_data::map::MAP_INSTANCE);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        33,
        None,
        Position::ZERO,
    );

    assert!(outcome.map_unavailable);
    assert!(!outcome.grid_loaded_now);
    assert!(matches!(
        canonical.lock().unwrap().find_map(33, 0).unwrap().kind(),
        wow_map::ManagedMapKind::World
    ));
}
#[test]
fn login_grid_load_can_materialize_missing_common_world_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(571, wow_data::map::MAP_COMMON);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        571,
        None,
        Position::ZERO,
    );

    assert!(!outcome.map_unavailable);
    assert!(outcome.map_created);
    assert!(matches!(
        canonical.lock().unwrap().find_map(571, 0).unwrap().kind(),
        wow_map::ManagedMapKind::World
    ));
}
#[test]
fn login_grid_load_does_not_guess_missing_faction_split_world_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(609, wow_data::map::MAP_COMMON);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        609,
        None,
        Position::ZERO,
    );

    assert!(outcome.map_unavailable);
    assert!(!outcome.map_created);
    assert!(canonical.lock().unwrap().find_map(609, 0).is_none());
}
#[test]
fn login_grid_load_mirrors_already_loaded_canonical_creature_to_legacy_like_cpp() {
    let spawn_id = 70_001;
    let entry = 42;
    let position = Position::new(1_000.0, 1_000.0, 0.0, 0.0);
    let guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, spawn_id as i64);

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(entry);
        creature.unit_mut().world_mut().set_map(571, 0).unwrap();
        creature.unit_mut().world_mut().relocate(position);
        creature.unit_mut().world_mut().object_mut().add_to_world();
        creature.set_spawn_id(spawn_id);

        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
            .expect("test canonical creature add to map");
    }

    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id: 571,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::default_group(),
            id: entry,
            spawn_point: SpawnPosition::new(
                position.x,
                position.y,
                position.z,
                position.orientation,
            ),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 120,
            spawn_difficulties: vec![0],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            store,
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(571, wow_data::map::MAP_COMMON);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        571,
        Some(0),
        position,
    );

    assert_eq!(outcome.skipped_already_loaded, 1);
    assert_eq!(outcome.creature_records_added, 0);
    assert_eq!(
        outcome.legacy_creature_mirrors, 1,
        "C++ has one Map object store; Rust's temporary canonical/legacy split must mirror already-loaded canonical creatures into the legacy tick world"
    );
    assert!(
        legacy.read().unwrap().find_creature(571, 0, guid).is_some(),
        "already-loaded canonical creature must be present in legacy MapManager so the creature tick can move it"
    );
}
#[test]
fn login_grid_load_materializes_visible_adjacent_ngrid_creature_like_cpp() {
    let spawn_id = 304_317;
    let entry = 3_114;
    let player_position = Position::new(545.38, -4209.53, 15.9, 0.0);
    let creature_position = Position::new(520.972, -4209.32, 15.9, 0.0);
    let player_cell = wow_map::cell_from_world(player_position.x, player_position.y);
    let creature_cell = wow_map::cell_from_world(creature_position.x, creature_position.y);
    assert_ne!(
        player_cell.grid_x(),
        creature_cell.grid_x(),
        "regression setup must place the player and nearby creature across a C++ NGrid boundary"
    );
    assert!(
        player_position.distance_2d(&creature_position)
            <= wow_world::map_manager::VISIBILITY_RADIUS,
        "regression creature must be inside the client visibility radius"
    );

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::default_group(),
            id: entry,
            spawn_point: SpawnPosition::new(
                creature_position.x,
                creature_position.y,
                creature_position.z,
                creature_position.orientation,
            ),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 120,
            spawn_difficulties: vec![0],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            store,
            BTreeMap::new(),
        )
        .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            super::super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id,
                model_id: 111,
                equipment_id: 0,
                wander_distance: 8.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                npc_flags: None,
                unit_flags: None,
                unit_flags2: None,
                unit_flags3: None,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                rooted: false,
                chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                interaction_pause_timer_ms:
                    wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                string_id: "visible-adjacent-grid-creature".to_string(),
                spawn_time_secs: 120,
            },
        )])),
    ));
    let mut caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    caches.realm_id = 7;
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    canonical.lock().unwrap().create_world_map(1, 0);
    let map_store = loaded_grid_map_store_like_cpp(1, wow_data::map::MAP_COMMON);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        1,
        Some(0),
        player_position,
    );

    assert_eq!(outcome.creature_records_added, 1);
    assert_eq!(outcome.legacy_creature_mirrors, 1);
    assert_eq!(outcome.load_record_missing, 0);
    assert_eq!(outcome.add_to_map_errors, 0);
    let guid = {
        let guard = canonical.lock().unwrap();
        guard
            .find_map(1, 0)
            .expect("login grid load should create the world map")
            .map()
            .get_creature_by_spawn_id_like_cpp(spawn_id)
            .expect("visible adjacent NGrid creature should be materialized")
            .guid()
    };
    assert_eq!(guid.realm_id(), 7);
    assert!(
        legacy.read().unwrap().find_creature(1, 0, guid).is_some(),
        "materialized canonical creature must be mirrored into the legacy visible/tick world"
    );
    assert!(
        legacy
            .read()
            .unwrap()
            .get_visible_creatures(
                1,
                0,
                player_position.x,
                player_position.y,
                player_position.z
            )
            .iter()
            .any(|creature| creature.guid() == guid),
        "nearby creature from the adjacent C++ NGrid must be visible after login"
    );
}
#[test]
fn login_grid_load_materializes_area_triggers_like_cpp() {
    let spawn_id = 70_101;
    let create_properties_id = 2003;
    let template_id = 9003;
    let position = Position::new(1.0, 2.0, 3.0, 0.5);

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let mut store = SpawnStore::new();
    store.add_area_trigger_spawn(&SpawnData {
        object_type: SpawnObjectType::AreaTrigger,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: create_properties_id,
        spawn_point: SpawnPosition::new(position.x, position.y, position.z, position.orientation),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    });
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            store,
            BTreeMap::new(),
        )
        .with_area_trigger_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            super::super::spawn_store_loader::AreaTriggerSpawnRuntimeRowLikeCpp {
                spawn_id,
                create_properties_id: wow_data::AreaTriggerIdLikeCpp {
                    id: create_properties_id,
                    is_custom: false,
                },
                spell_for_visuals: None,
            },
        )])),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates =
        area_trigger_template_store_for_loaded_grid_like_cpp(create_properties_id, template_id);
    canonical.lock().unwrap().create_world_map(571, 0);
    let map_store = loaded_grid_map_store_like_cpp(571, wow_data::map::MAP_COMMON);

    let outcome = super::super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        571,
        Some(0),
        position,
    );

    assert_eq!(outcome.area_trigger_records_added, 1);
    assert_eq!(outcome.load_record_missing, 0);
    assert_eq!(outcome.add_to_map_errors, 0);
    let guard = canonical.lock().unwrap();
    let area_trigger = guard
        .find_map(571, 0)
        .expect("login grid load should create the map")
        .map()
        .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
        .expect("login grid load should materialize DB-backed AreaTrigger");
    assert_eq!(area_trigger.spawn_id(), spawn_id);
    assert_eq!(area_trigger.template_id().unwrap().id, template_id);
    assert_eq!(
        area_trigger.world().guid().high_type(),
        wow_core::guid::HighGuid::AreaTrigger
    );
}
