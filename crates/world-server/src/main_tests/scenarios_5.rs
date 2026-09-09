//! Scenarios for [`super`], part 5.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn chat_strict_link_checking_kick_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("ChatStrictLinkChecking.Kick = 1\n")
        .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_ne!(
        world_config_u8(&configs, "CONFIG_CHAT_STRICT_LINK_CHECKING_KICK", 0),
        0
    );
}
#[test]
fn chat_level_requirements_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "ChatLevelReq.Channel = 2\n\
         ChatLevelReq.Whisper = 3\n\
         ChatLevelReq.Emote = 4\n\
         ChatLevelReq.Say = 5\n\
         ChatLevelReq.Yell = 6\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_u8(&configs, "CONFIG_CHAT_CHANNEL_LEVEL_REQ", 1),
        2
    );
    assert_eq!(
        world_config_u8(&configs, "CONFIG_CHAT_WHISPER_LEVEL_REQ", 1),
        3
    );
    assert_eq!(
        world_config_u8(&configs, "CONFIG_CHAT_EMOTE_LEVEL_REQ", 1),
        4
    );
    assert_eq!(world_config_u8(&configs, "CONFIG_CHAT_SAY_LEVEL_REQ", 1), 5);
    assert_eq!(
        world_config_u8(&configs, "CONFIG_CHAT_YELL_LEVEL_REQ", 1),
        6
    );
}
#[test]
fn chat_listen_ranges_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "ListenRange.Say = 40\n\
         ListenRange.TextEmote = 41\n\
         ListenRange.Yell = 301\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_f32(&configs, "CONFIG_LISTEN_RANGE_SAY", 25.0),
        40.0
    );
    assert_eq!(
        world_config_f32(&configs, "CONFIG_LISTEN_RANGE_TEXTEMOTE", 25.0),
        41.0
    );
    assert_eq!(
        world_config_f32(&configs, "CONFIG_LISTEN_RANGE_YELL", 300.0),
        301.0
    );
}
#[test]
fn declined_names_are_forced_for_russian_realm_categories_like_cpp() {
    let categories = wow_data::CfgCategoriesStore::from_entries([
        wow_data::CfgCategoriesEntry {
            id: 1,
            name: "Development".to_string(),
            locale_mask: 0,
            create_charset_mask: 0x01,
            existing_charset_mask: 0,
            flags: 0,
            order: 0,
        },
        wow_data::CfgCategoriesEntry {
            id: 12,
            name: "Russian".to_string(),
            locale_mask: 0,
            create_charset_mask: 0x04,
            existing_charset_mask: 0,
            flags: 0,
            order: 0,
        },
    ]);

    assert!(!declined_names_used_for_realm_category_like_cpp(
        false,
        1,
        &categories
    ));
    assert!(declined_names_used_for_realm_category_like_cpp(
        false,
        12,
        &categories
    ));
    assert!(declined_names_used_for_realm_category_like_cpp(
        true,
        1,
        &categories
    ));
}
#[test]
fn chat_flood_config_uses_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "ChatFlood.MessageCount = 2\n\
         ChatFlood.MessageDelay = 3\n\
         ChatFlood.AddonMessageCount = 4\n\
         ChatFlood.AddonMessageDelay = 5\n\
         ChatFlood.MuteTime = 6\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_MESSAGE_COUNT", 10),
        2
    );
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_MESSAGE_DELAY", 1),
        3
    );
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_ADDON_MESSAGE_COUNT", 100),
        4
    );
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_ADDON_MESSAGE_DELAY", 1),
        5
    );
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_MUTE_TIME", 10),
        6
    );
}
#[test]
fn max_overspeed_pings_reads_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("MaxOverspeedPings = 7\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_u32(&configs, "CONFIG_MAX_OVERSPEED_PINGS", 2),
        7
    );
}
#[test]
fn socket_timeouts_read_cpp_world_config_keys_as_seconds() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "SocketTimeOutTime = 120000\nSocketTimeOutTimeActive = 45000\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let timeouts = wow_network::SocketTimeoutsLikeCpp {
        unauthenticated_secs: u64::from(world_config_u32(
            &configs,
            "CONFIG_SOCKET_TIMEOUTTIME",
            900,
        )),
        active_secs: u64::from(world_config_u32(
            &configs,
            "CONFIG_SOCKET_TIMEOUTTIME_ACTIVE",
            60,
        )),
    };

    assert_eq!(
        timeouts,
        wow_network::SocketTimeoutsLikeCpp {
            unauthenticated_secs: 120,
            active_secs: 45,
        }
    );
}
#[test]
fn packet_spoof_config_reads_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "PacketSpoof.Policy = 2\nPacketSpoof.BanMode = 2\nPacketSpoof.BanDuration = 12345\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let packet_spoof = wow_world::PacketSpoofConfigLikeCpp {
        policy: world_config_u32(&configs, "CONFIG_PACKET_SPOOF_POLICY", 1),
        ban_mode: world_config_u32(&configs, "CONFIG_PACKET_SPOOF_BANMODE", 0),
        ban_duration_secs: world_config_u32(&configs, "CONFIG_PACKET_SPOOF_BANDURATION", 86_400),
    };

    assert_eq!(
        packet_spoof,
        wow_world::PacketSpoofConfigLikeCpp {
            policy: 2,
            ban_mode: 2,
            ban_duration_secs: 12_345,
        }
    );
}
#[test]
fn mmap_runtime_config_uses_cpp_world_config_key_and_data_dir() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        r#"
DataDir = "/srv/wow-data"
mmap.enablePathFinding = 0
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let mmap_config = mmap_runtime_config_like_cpp(&configs, HashSet::from([1]));
    assert_eq!(mmap_config.data_dir, "/srv/wow-data");
    assert!(!mmap_config.enabled);
    assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(0));
    assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(1));
}
#[test]
fn mmap_runtime_config_applies_cpp_disable_mgr_map_gate() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("mmap.enablePathFinding = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    let mmap_config = mmap_runtime_config_like_cpp(&configs, HashSet::from([571]));
    assert!(mmap_config.pathfinding_enabled_for_map_like_cpp(0));
    assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(571));
}
#[test]
fn canonical_spawn_group_initializer_applies_mapid_conditions_on_new_maps() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([(10, 571), (11, 530)])));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp([
        mapid_condition(10, 571),
        mapid_condition(11, 571),
    ]));
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        Arc::clone(&metadata),
        condition_store,
        Arc::new(PersistedRespawnTimesLikeCpp::default()),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let group_571 = metadata
        .lock()
        .expect("test metadata lock")
        .spawn_group_templates()
        .get(&10)
        .expect("test group 10")
        .clone();
    let map_571 = manager.create_world_map(571, 0);
    assert!(
        map_571
            .map()
            .is_spawn_group_active_like_cpp(Some(&group_571))
    );

    let group_530 = metadata
        .lock()
        .expect("test metadata lock")
        .spawn_group_templates()
        .get(&11)
        .expect("test group 11")
        .clone();
    let map_530 = manager.create_world_map(530, 0);
    assert!(
        !map_530
            .map()
            .is_spawn_group_active_like_cpp(Some(&group_530))
    );
}
#[test]
fn canonical_spawn_group_initializer_does_not_reexecute_for_existing_map() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([(20, 571)])));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp([
        mapid_condition(20, 530),
    ]));
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        Arc::clone(&metadata),
        condition_store,
        Arc::new(PersistedRespawnTimesLikeCpp::default()),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let group = metadata
        .lock()
        .expect("test metadata lock")
        .spawn_group_templates()
        .get(&20)
        .expect("test group 20")
        .clone();
    let map = manager.create_world_map(571, 0);
    assert!(!map.map().is_spawn_group_active_like_cpp(Some(&group)));
    map.map_mut()
        .set_spawn_group_active_like_cpp(Some(&group), true);
    assert!(map.map().is_spawn_group_active_like_cpp(Some(&group)));

    let existing = manager.create_world_map(571, 0);
    assert!(existing.map().is_spawn_group_active_like_cpp(Some(&group)));
}
#[test]
fn canonical_spawn_group_initializer_no_groups_is_noop() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([])));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::default());
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        condition_store,
        Arc::new(PersistedRespawnTimesLikeCpp::default()),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let map = manager.create_world_map(999, 0);
    assert!(
        map.map()
            .spawn_group_state()
            .toggled_spawn_group_ids()
            .is_empty()
    );
}
#[test]
fn canonical_map_creation_loads_persisted_respawns_for_world_maps_before_spawn_groups() {
    let mut store = SpawnStore::new();
    let mut creature = test_spawn(77, 571);
    creature.id = 7001;
    creature.spawn_point = SpawnPosition::new(533.0, -533.0, 12.0, 1.0);
    store.add_object_spawn(&creature, |_| false);
    let mut gameobject = test_spawn(88, 571);
    gameobject.object_type = SpawnObjectType::GameObject;
    gameobject.id = 9001;
    gameobject.spawn_point = SpawnPosition::new(-100.0, 200.0, 13.0, 2.0);
    store.add_object_spawn(&gameobject, |_| false);
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            store,
            BTreeMap::new(),
        ),
    ));
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 77,
            entry: 7001,
            respawn_time: 12345,
            grid_id: wow_map::compute_grid_coord(creature.spawn_point.x, creature.spawn_point.y)
                .get_id(),
        },
    );
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 88,
            entry: 9001,
            respawn_time: 67890,
            grid_id: wow_map::compute_grid_coord(
                gameobject.spawn_point.x,
                gameobject.spawn_point.y,
            )
            .get_id(),
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        Arc::new(ConditionEntriesByTypeStore::default()),
        Arc::new(snapshot),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let map = manager.create_world_map(571, 0);
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 77),
        12345
    );
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, 88),
        67890
    );
    assert_eq!(
        map.map()
            .get_respawn_info_like_cpp(SpawnObjectType::Creature, 77)
            .expect("creature respawn loaded")
            .grid_id,
        wow_map::compute_grid_coord(creature.spawn_point.x, creature.spawn_point.y).get_id()
    );
}
#[test]
fn canonical_map_creation_init_pools_before_persisted_respawns_and_spawn_groups() {
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(88, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 10, group)
        .expect("test pool group");
    pool_mgr.add_auto_spawn_pool_like_cpp(571, 10);

    let mut store = SpawnStore::new();
    let mut gameobject = test_spawn(88, 571);
    gameobject.object_type = SpawnObjectType::GameObject;
    gameobject.id = 9001;
    gameobject.spawn_point = SpawnPosition::new(-100.0, 200.0, 13.0, 2.0);
    store.add_object_spawn(&gameobject, |_| false);
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            store,
            BTreeMap::new(),
        )
        .with_pool_mgr_like_cpp(pool_mgr),
    ));
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 88,
            entry: 9001,
            respawn_time: 67890,
            grid_id: wow_map::compute_grid_coord(
                gameobject.spawn_point.x,
                gameobject.spawn_point.y,
            )
            .get_id(),
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        Arc::new(ConditionEntriesByTypeStore::default()),
        Arc::new(snapshot),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let map = manager.create_world_map(571, 0);
    assert!(
        map.map()
            .pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(88)
    );
    assert_eq!(
        map.map()
            .pool_data_like_cpp()
            .get_spawned_objects_like_cpp(10),
        1
    );
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, 88),
        67890
    );
}
#[test]
fn canonical_map_creation_skips_persisted_respawns_for_dungeon_maps() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([])));
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 1),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 12345,
            grid_id: 7,
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        Arc::new(ConditionEntriesByTypeStore::default()),
        Arc::new(snapshot),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let map = manager.create_map_entry(
        571,
        1,
        0,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
}
#[test]
fn canonical_map_creation_skips_persisted_respawns_for_instanceable_world_kind_like_cpp() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([])));
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(1_151, 42),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 12_345,
            grid_id: 7,
        },
    );
    let map_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 1_151,
        instance_type: wow_data::map::MAP_SCENARIO,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        Arc::new(ConditionEntriesByTypeStore::default()),
        Arc::new(snapshot),
        Arc::new(map_store),
    );

    // The current canonical manager represents garrisons as `World`, but
    // C++ gates respawn persistence on `MapEntry::Instanceable()`.
    let map = manager.create_world_map(1_151, 42);
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
}
#[test]
fn persisted_respawn_loader_rejects_invalid_areatrigger_and_missing_metadata_rows() {
    let metadata = test_spawn_metadata([]);
    let mut report = PersistedRespawnLoadReportLikeCpp::default();

    assert!(
        persisted_respawn_info_from_row_like_cpp(
            RespawnPersistenceRowLikeCpp {
                object_type_raw: 99,
                spawn_id: 1,
                respawn_time: 10,
                map_id: 571,
                instance_id: 0,
            },
            &metadata,
            &mut report,
        )
        .is_none()
    );
    assert!(
        persisted_respawn_info_from_row_like_cpp(
            RespawnPersistenceRowLikeCpp {
                object_type_raw: 256,
                spawn_id: 1,
                respawn_time: 10,
                map_id: 571,
                instance_id: 0,
            },
            &metadata,
            &mut report,
        )
        .is_none()
    );
    assert!(
        persisted_respawn_info_from_row_like_cpp(
            RespawnPersistenceRowLikeCpp {
                object_type_raw: SpawnObjectType::AreaTrigger as u16,
                spawn_id: 1,
                respawn_time: 10,
                map_id: 571,
                instance_id: 0,
            },
            &metadata,
            &mut report,
        )
        .is_none()
    );
    assert!(
        persisted_respawn_info_from_row_like_cpp(
            RespawnPersistenceRowLikeCpp {
                object_type_raw: SpawnObjectType::Creature as u16,
                spawn_id: 404,
                respawn_time: 10,
                map_id: 571,
                instance_id: 0,
            },
            &metadata,
            &mut report,
        )
        .is_none()
    );

    assert_eq!(report.rows, 4);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.invalid_type, 2);
    assert_eq!(report.unsupported_area_trigger, 1);
    assert_eq!(report.missing_spawn_metadata, 1);
}
// C++ anchors for the focused condition-update helper tests:
// - Maps/Map.cpp:666-688 (`Map::Update` respawn timer calls `UpdateSpawnGroupConditions`).
// - Maps/Map.cpp:2471-2502 (`UpdateSpawnGroupConditions` branch order).
// - Maps/Map.cpp:2427-2453 (map-owned spawn-group toggle state).
// - GameObject.cpp:772-779 and 4256-4277 (capture-point paths trigger condition updates).
#[test]
fn spawn_group_condition_update_set_inactive_applies_for_failed_automatic_group() {
    let metadata = test_spawn_metadata([(30, 571)]);
    let condition_store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(30, 530)]);
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let group = metadata
        .spawn_group_templates()
        .get(&30)
        .expect("test group 30");
    let map = manager.create_world_map(571, 0);
    assert!(map.map().is_spawn_group_active_like_cpp(Some(group)));

    let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
        map,
        &metadata,
        &condition_store,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    );

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].group_id, 30);
    assert_eq!(
        outcomes[0].action,
        wow_map::map::SpawnGroupConditionActionLikeCpp::SetInactive
    );
    assert!(matches!(
        outcomes[0].applied_change,
        Some(
            wow_map::SpawnGroupActiveChange::Toggled
                | wow_map::SpawnGroupActiveChange::ClearedToggle
        )
    ));
    assert!(!map.map().is_spawn_group_active_like_cpp(Some(group)));
}
#[test]
fn spawn_group_condition_update_set_inactive_executes_spawn_active_seam_and_despawn_toggles() {
    let metadata = test_spawn_metadata_with_flags([
        (40, 571, SpawnGroupFlags::NONE),
        (41, 571, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE),
    ]);
    let condition_store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
        mapid_condition(40, 571),
        mapid_condition(41, 530),
    ]);
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let spawn_group = metadata
        .spawn_group_templates()
        .get(&40)
        .expect("test group 40");
    let despawn_group = metadata
        .spawn_group_templates()
        .get(&41)
        .expect("test group 41");
    let map = manager.create_world_map(571, 0);
    map.map_mut()
        .set_spawn_group_inactive_like_cpp(Some(spawn_group));
    assert!(!map.map().is_spawn_group_active_like_cpp(Some(spawn_group)));
    assert!(
        map.map()
            .is_spawn_group_active_like_cpp(Some(despawn_group))
    );

    let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
        map,
        &metadata,
        &condition_store,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    );

    let spawn_outcome = outcomes
        .iter()
        .find(|outcome| outcome.group_id == 40)
        .expect("spawn outcome");
    assert_eq!(
        spawn_outcome.action,
        wow_map::map::SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
    );
    assert_eq!(spawn_outcome.applied_change, None);
    let spawn = spawn_outcome
        .spawn_outcome
        .as_ref()
        .expect("condition-success spawn executes active-state seam");
    assert_eq!(spawn.blocked_missing_group, 0);
    assert_eq!(spawn.blocked_system_group, 0);
    assert_eq!(
        spawn.applied_active_change,
        Some(wow_map::SpawnGroupActiveChange::ClearedToggle)
    );
    let despawn_outcome = outcomes
        .iter()
        .find(|outcome| outcome.group_id == 41)
        .expect("despawn outcome");
    assert_eq!(
        despawn_outcome.action,
        wow_map::map::SpawnGroupConditionActionLikeCpp::condition_failure_despawn()
    );
    assert_eq!(despawn_outcome.applied_change, None);
    let despawn = despawn_outcome
        .despawn_outcome
        .expect("condition-failure despawn executes");
    assert_eq!(despawn.blocked_missing_group, 0);
    assert_eq!(despawn.blocked_system_group, 0);
    assert_eq!(
        despawn.applied_inactive_change,
        Some(wow_map::SpawnGroupActiveChange::Toggled)
    );
    assert!(map.map().is_spawn_group_active_like_cpp(Some(spawn_group)));
    assert!(
        !map.map()
            .is_spawn_group_active_like_cpp(Some(despawn_group))
    );
}
#[test]
fn spawn_group_condition_update_set_inactive_no_groups_is_noop() {
    let metadata = test_spawn_metadata([]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let map = manager.create_world_map(999, 0);

    let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
        map,
        &metadata,
        &condition_store,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    );

    assert!(outcomes.is_empty());
    assert!(
        map.map()
            .spawn_group_state()
            .toggled_spawn_group_ids()
            .is_empty()
    );
}
#[test]
fn respawn_condition_scheduler_like_cpp_waits_fires_and_resets() {
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

    assert!(!scheduler.update(40));
    assert_eq!(scheduler.timer_ms(), 60);
    assert!(!scheduler.update(59));
    assert_eq!(scheduler.timer_ms(), 1);
    assert!(scheduler.update(1));
    assert_eq!(scheduler.timer_ms(), 100);
    assert!(scheduler.update(150));
    assert_eq!(scheduler.timer_ms(), 100);
    assert!(!scheduler.update(25));
    assert_eq!(scheduler.timer_ms(), 75);
}
#[test]
fn game_event_scheduler_like_cpp_waits_fires_resets_and_installs_dynamic_delay() {
    let mut scheduler = CanonicalGameEventSchedulerLikeCpp::start_system(100);

    assert_eq!(scheduler.interval_ms(), 100);
    assert!(!scheduler.update(40));
    assert_eq!(scheduler.timer_ms(), 60);
    assert!(!scheduler.update(59));
    assert_eq!(scheduler.timer_ms(), 1);
    assert!(scheduler.update(1));
    assert_eq!(scheduler.timer_ms(), 100);

    scheduler.set_interval_and_reset(250);
    assert_eq!(scheduler.interval_ms(), 250);
    assert_eq!(scheduler.timer_ms(), 250);
    assert!(!scheduler.update(249));
    assert_eq!(scheduler.timer_ms(), 1);
    assert!(scheduler.update(1));
    assert_eq!(scheduler.timer_ms(), 250);

    scheduler.set_interval_and_reset(u64::from(u32::MAX) + 1);
    assert_eq!(scheduler.interval_ms(), u32::MAX);
    assert_eq!(scheduler.timer_ms(), u32::MAX);
    scheduler.set_interval_and_reset(0);
    assert_eq!(scheduler.interval_ms(), 1);
    assert_eq!(scheduler.timer_ms(), 1);
}
#[test]
fn game_event_start_system_first_update_records_negative_spawn_then_init_update_skips_it() {
    let event = spawn_store_loader::GameEventDataLikeCpp {
        event_id: 1,
        start: 100,
        end: 1_000,
        occurence: 10,
        length: 2,
        ..spawn_store_loader::GameEventDataLikeCpp::default()
    };
    let store =
        spawn_store_loader::GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(1))
            .with_event_like_cpp(event);
    let mut metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
        SpawnStore::default(),
        BTreeMap::new(),
    )
    .with_game_events_like_cpp(store);

    metadata.clear_active_game_events_like_cpp();
    let start_outcome = metadata.update_game_events_like_cpp(650, false, |_| false);
    assert_eq!(start_outcome.negative_spawn_event_ids, vec![-1]);
    assert_eq!(start_outcome.next_update_delay_millis, 51_000);
    let mut scheduler =
        CanonicalGameEventSchedulerLikeCpp::start_system(start_outcome.next_update_delay_millis);
    assert_eq!(scheduler.interval_ms(), 51_000);

    assert!(scheduler.update(51_000));
    let tick_outcome = metadata.update_game_events_like_cpp(650, true, |_| false);
    scheduler.set_interval_and_reset(tick_outcome.next_update_delay_millis);
    assert!(tick_outcome.negative_spawn_event_ids.is_empty());
    assert_eq!(
        scheduler.interval_ms(),
        tick_outcome.next_update_delay_millis as u32
    );
}
#[test]
fn game_event_db_bridge_materializes_semantic_save_with_zero_next_start_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            state_raw: 2,
            next_start: 0,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.start_outcomes = vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
        spawn_store_loader::GameEventStartSummaryLikeCpp {
            event_id: 1,
            state_before_raw: 1,
            state_after_raw: 2,
            active_added: true,
            active_was_present: false,
            apply_new_event_requested: true,
            save_world_event_state_requested: true,
            force_game_event_update_requested: false,
            completed: false,
        },
    )];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.saves_queued, 1);
    assert_eq!(summary.operations.len(), 1);
    assert_game_event_save_operation_like_cpp(&summary.operations[0], 1, 2, 0);
}
