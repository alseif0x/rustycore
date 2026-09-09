//! Scenarios for [`super`], part 11.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn loaded_grid_creature_spawn_group_spawn_record_does_not_require_respawn_timer_like_cpp() {
    let spawn_id = 54_985;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(68, 571, SpawnGroupFlags::NONE, spawn_id)]);
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
            string_id: "condition-spawn-no-timer".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    let mut map = wow_map::Map::new(571, 0, 0, 60_000);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );

    let record = build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
        &mut map,
        SpawnObjectType::Creature,
        spawn_id,
        &metadata,
        &caches,
    )
    .expect("SpawnGroupSpawn loaded-grid Creature loader must not require a respawn timer");
    let creature = record
        .primary_record
        .creature()
        .expect("builder should return a typed Creature MapObjectRecord");

    assert_eq!(creature.respawn_time(), 0);
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    assert_eq!(creature.guid().entry(), entry);
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
}
#[test]
fn spawn_group_condition_update_spawn_loads_loaded_grid_creature_without_respawn_timer_like_cpp() {
    let spawn_id = 54_986;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(69, 571, SpawnGroupFlags::NONE, spawn_id)]);
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
            string_id: "condition-spawn-caller-no-timer".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let condition_store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(69, 571)]);
    let caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let group = metadata
        .spawn_group_templates()
        .get(&69)
        .expect("test group 69")
        .clone();
    let map = manager.create_world_map(571, 0);
    map.map_mut()
        .set_spawn_group_inactive_like_cpp(Some(&group));
    assert!(map.map_mut().load_grid(0.0, 0.0));
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );

    let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
        map,
        &metadata,
        &condition_store,
        &caches,
    );

    let spawn_outcome = outcomes
        .iter()
        .find(|outcome| outcome.group_id == 69)
        .and_then(|outcome| outcome.spawn_outcome.as_ref())
        .expect("condition-success SpawnGroupSpawn outcome");
    assert_eq!(spawn_outcome.executed_loaded_grid_spawns, 1);
    assert_eq!(spawn_outcome.blocked_loaded_grid_creature_loads, 0);
    assert_eq!(spawn_outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(spawn_outcome.skipped_respawn_timer_active, 0);
    assert_eq!(map.map().map_object_count(), 1);
    let creature = map
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("loaded-grid Creature should be indexed by spawn id");
    assert_eq!(creature.respawn_time(), 0);
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
}
#[test]
fn spawn_group_condition_update_tick_mirrors_loaded_grid_creature_to_legacy_like_cpp() {
    let spawn_id = 54_987;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(69, 571, SpawnGroupFlags::NONE, spawn_id)]);
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
            string_id: "condition-spawn-caller-legacy-mirror".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let condition_store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(69, 571)]);
    let caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let group = metadata
        .spawn_group_templates()
        .get(&69)
        .expect("test group 69")
        .clone();
    let map = manager.create_world_map(571, 0);
    map.map_mut()
        .set_spawn_group_inactive_like_cpp(Some(&group));
    assert!(map.map_mut().load_grid(0.0, 0.0));
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        Some(&legacy),
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &caches,
    )
    .expect("scheduler fires and condition spawn executes");

    assert_eq!(summary.condition_spawn_executed_loaded_grid_spawns, 1);
    assert_eq!(summary.condition_spawn_legacy_creature_mirrors, 1);
    let creature = manager
        .find_map(571, 0)
        .expect("canonical map")
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("canonical loaded-grid creature");
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
    assert!(
        legacy
            .read()
            .unwrap()
            .find_creature(571, 0, creature.guid())
            .is_some(),
        "C++ AddToMap has one live runtime; Rust split runtime must mirror internal wow-map loaded-grid inserts into legacy"
    );
}
#[test]
fn loaded_grid_creature_respawn_record_vehicle_template_uses_creature_low_vehicle_high_like_cpp() {
    let spawn_id = 54_988;
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
            string_id: "vehicle-template-live".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let mut caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 101);
    caches.realm_id = 7;
    let entry_accessory = wow_entities::VehicleAccessory {
        accessory_entry: 7001,
        seat_id: 1,
        is_minion: false,
        summoned_type: 6,
        summon_time_ms: 3_000,
    };
    let spawn_accessory = wow_entities::VehicleAccessory {
        accessory_entry: 8001,
        seat_id: 2,
        is_minion: true,
        summoned_type: 8,
        summon_time_ms: 4_000,
    };
    caches.vehicle_accessory_store = Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
        [(spawn_id, vec![spawn_accessory])],
        [(entry, vec![entry_accessory])],
    ));
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
    .expect("vehicle-template loaded-grid Creature builder should resolve");
    let creature = record
        .primary_record
        .creature()
        .expect("builder should return a typed Creature MapObjectRecord");

    assert_eq!(
        creature.guid().high_type(),
        wow_core::guid::HighGuid::Vehicle
    );
    assert_eq!(creature.guid().realm_id(), 7);
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
    assert_eq!(creature.guid().entry(), entry);
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
    let kit = creature
        .unit()
        .subsystems()
        .vehicle
        .kit
        .as_ref()
        .expect("VehicleEntry-backed template should create a local kit");
    assert_eq!(kit.kit_id(), 101);
    assert!(kit.active());
    assert!(!kit.installed());
    assert_eq!(kit.seat_count(), 2);
    assert_eq!(kit.usable_seat_num(), 1);
    let outcome = creature
        .unit()
        .subsystems()
        .vehicle
        .last_create_outcome
        .as_ref()
        .expect("CreateVehicleKit evidence should be recorded");
    assert!(outcome.created);
    assert_eq!(outcome.seat_count, 2);
    assert_eq!(outcome.usable_seat_num, 1);
    assert!(outcome.update_display_power_represented);
    assert!(!outcome.send_set_vehicle_rec_id_represented);
    let reset_context = creature
        .add_to_world_vehicle_reset_context_like_cpp()
        .expect("VehicleEntry-backed template should build AddToWorld reset context");
    assert!(!reset_context.is_mechanical_creature);
    assert!(!reset_context.is_world_boss);
    assert_eq!(reset_context.accessories, vec![spawn_accessory]);
}
#[test]
fn loaded_grid_creature_respawn_record_vehicle_high_guid_without_kit_when_vehicle_row_missing_like_cpp()
 {
    let mut metadata = test_spawn_metadata_with_flags([(67, 571, SpawnGroupFlags::NONE)]);
    let spawn_id = 1;
    let entry = 42;
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
            string_id: "vehicle-template-missing-row".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let mut caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 101);
    caches.vehicle_store = Arc::new(wow_data::VehicleStore::from_entries([]));
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
    .expect("vehicle-template loaded-grid Creature builder should still resolve");
    let creature = record
        .primary_record
        .creature()
        .expect("builder should return a typed Creature MapObjectRecord");

    assert_eq!(
        creature.guid().high_type(),
        wow_core::guid::HighGuid::Vehicle
    );
    assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
    assert!(creature.unit().subsystems().vehicle.kit.is_none());
    let outcome = creature
        .unit()
        .subsystems()
        .vehicle
        .last_create_outcome
        .as_ref()
        .expect("CreateVehicleKit false evidence should be recorded");
    assert_eq!(outcome.kit_id, Some(101));
    assert!(!outcome.created);
    assert!(!outcome.update_display_power_represented);
}
/// (1) NearbyVisible: players on a different map_id are not enqueued.
/// C++ anchor: MessageDistDeliverer::Visit — map-id check before distance.
#[test]
fn nearby_visible_filters_by_map_id_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 1);
    let (info, command_rx) = make_registry_player_like_cpp(530, 0, Position::ZERO, true); // wrong map
    registry.register_or_replace(guid, info, Default::default());

    let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert!(command_rx.try_recv().is_err());
}
/// (2) NearbyVisible: players on a different instance_id are not enqueued.
/// Slice 4A.1b requirement — instance separation.
#[test]
fn nearby_visible_filters_by_instance_id_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 2);
    let (info, command_rx) = make_registry_player_like_cpp(571, 99, Position::ZERO, true); // wrong instance
    registry.register_or_replace(guid, info, Default::default());

    let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert!(command_rx.try_recv().is_err());
}
/// (3) NearbyVisible: players not in world are not enqueued.
/// C++ anchor: MessageDistDeliverer::Visit — `Player::IsInWorld()` gate.
#[test]
fn nearby_visible_filters_is_in_world_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 3);
    let (info, command_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, false); // not in world
    registry.register_or_replace(guid, info, Default::default());

    let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);
    assert!(command_rx.try_recv().is_err());
}
/// (4) NearbyVisible: 2D distance check excludes players beyond range when
/// `required_3d == false` — Z-axis is ignored.
/// C++ anchor: GridNotifiersImpl.h MessageDistDeliverer::Visit ~43-46.
#[test]
fn nearby_visible_uses_2d_distance_when_required_3d_false_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    // Player is far on Z but close in XY — should be INCLUDED with 2D check.
    let near_guid = ObjectGuid::create_player(1, 4);
    let (near_info, near_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(5.0, 0.0, 1000.0, 0.0), true);
    registry.register_or_replace(near_guid, near_info, Default::default());

    // Player is far in XY — should be EXCLUDED.
    let far_guid = ObjectGuid::create_player(1, 5);
    let (far_info, far_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(200.0, 0.0, 0.0, 0.0), true);
    registry.register_or_replace(far_guid, far_info, Default::default());

    let source = Position::new(0.0, 0.0, 0.0, 0.0);
    let event = make_nearby_visible_event_like_cpp(571, 0, source, 100.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1, "only the XY-near player");
    assert_eq!(summary.candidates_skipped_distance, 1);
    assert!(near_rx.try_recv().is_ok(), "near player got command");
    assert!(far_rx.try_recv().is_err(), "far player did not get command");
}
/// (5) NearbyVisible: 3D distance check excludes players beyond range when
/// `required_3d == true` — Z-axis contributes to distance.
/// C++ anchor: GridNotifiersImpl.h MessageDistDeliverer::Visit ~43-46.
#[test]
fn nearby_visible_uses_3d_distance_when_required_3d_true_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    // Player is close in XY but far on Z — should be EXCLUDED with 3D check.
    let near_xy_guid = ObjectGuid::create_player(1, 6);
    let (near_xy_info, near_xy_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(5.0, 0.0, 200.0, 0.0), true);
    registry.register_or_replace(near_xy_guid, near_xy_info, Default::default());

    // Player is close in 3D — should be INCLUDED.
    let near_3d_guid = ObjectGuid::create_player(1, 7);
    let (near_3d_info, near_3d_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(3.0, 3.0, 3.0, 0.0), true);
    registry.register_or_replace(near_3d_guid, near_3d_info, Default::default());

    let source = Position::new(0.0, 0.0, 0.0, 0.0);
    let event = make_nearby_visible_event_like_cpp(571, 0, source, 10.0, true);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1, "only the 3D-near player");
    assert_eq!(summary.candidates_skipped_distance, 1);
    assert!(near_xy_rx.try_recv().is_err(), "far-Z player excluded");
    assert!(near_3d_rx.try_recv().is_ok(), "3D-near player included");
}
#[test]
fn nearby_visible_durable_uses_committed_fifo_instead_of_bounded_queue() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 8);
    let (info, command_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let durable = Arc::clone(&info.durable_creature_runtime_commands_like_cpp);
    registry.register_or_replace(guid, info, Default::default());

    let event = wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::NearbyVisibleDurable {
            source_guid: make_source_guid(),
            map_id: 571,
            instance_id: 0,
            source_position: Position::ZERO,
            range: 100.0,
            required_3d: false,
        },
        packet_bytes: vec![0xAA],
    };
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1);
    assert!(command_rx.try_recv().is_err());
    let drained = durable.lock().unwrap().drain_like_cpp();
    assert!(matches!(
        drained.as_slice(),
        [SessionCommand::SendIfVisibleLikeCpp(command)]
            if command.packet_bytes == vec![0xAA]
    ));
}
#[test]
fn creature_spell_start_go_is_one_atomic_observer_command_without_victim_drain_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim_guid = ObjectGuid::create_player(1, 80);
    let observer_guid = ObjectGuid::create_player(1, 81);
    let (victim_info, _victim_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let (observer_info, _observer_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(25.0, 0.0, 0.0, 0.0), true);
    victim_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    observer_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    registry.register_or_replace(victim_guid, victim_info, Default::default());
    registry.register_or_replace(observer_guid, observer_info, Default::default());

    let (plan, start_bytes, basic_go_bytes, full_go_bytes) =
        make_creature_spell_runtime_plan_like_cpp(victim_guid);
    let plan_delivery = deliver_runtime_plan_like_cpp(&plan, &registry);
    assert_eq!(plan_delivery.events_seen, 1);
    assert_eq!(plan_delivery.candidates_seen, 2);
    assert_eq!(plan_delivery.candidates_queued, 2);

    // Drain the observer first: its START/GO delivery does not depend on
    // the victim session making progress on its own durable FIFO.
    let observer_commands =
        drain_durable_creature_runtime_commands_like_cpp(&registry, observer_guid);
    let [SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(cast)] =
        observer_commands.as_slice()
    else {
        panic!("observer must receive one atomic START+GO command: {observer_commands:?}");
    };
    assert_eq!(cast.start_packet_bytes, start_bytes);
    assert_eq!(
        cast.go_packet_bytes, basic_go_bytes,
        "a receiver without advanced combat logging commits the basic frame"
    );
    assert_ne!(cast.go_packet_bytes, full_go_bytes);
    assert_eq!(
        u16::from_le_bytes(cast.start_packet_bytes[..2].try_into().unwrap()),
        ServerOpcodes::SpellStart as u16
    );
    assert_eq!(
        u16::from_le_bytes(cast.go_packet_bytes[..2].try_into().unwrap()),
        ServerOpcodes::SpellGo as u16
    );

    // The victim's untouched FIFO retains one indivisible copy as well.
    let victim_commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim_guid);
    let [SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(victim_cast)] =
        victim_commands.as_slice()
    else {
        panic!("victim FIFO must contain one atomic START+GO command: {victim_commands:?}");
    };
    assert_eq!(victim_cast.start_packet_bytes, start_bytes);
    assert_eq!(victim_cast.go_packet_bytes, basic_go_bytes);
}
#[test]
fn creature_spell_plan_skips_invisible_observer_but_reaches_victim_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim_guid = ObjectGuid::create_player(1, 82);
    let invisible_observer_guid = ObjectGuid::create_player(1, 83);
    let (victim_info, _victim_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let (invisible_observer_info, _observer_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(250.0, 0.0, 0.0, 0.0), true);
    // Both viewers already have the caster at client; only range separates
    // them here.
    victim_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    invisible_observer_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    registry.register_or_replace(victim_guid, victim_info, Default::default());
    registry.register_or_replace(
        invisible_observer_guid,
        invisible_observer_info,
        Default::default(),
    );

    let (plan, start_bytes, basic_go_bytes, full_go_bytes) =
        make_creature_spell_runtime_plan_like_cpp(victim_guid);
    let plan_delivery = deliver_runtime_plan_like_cpp(&plan, &registry);
    assert_eq!(plan_delivery.events_seen, 1);
    assert_eq!(plan_delivery.candidates_seen, 2);
    assert_eq!(plan_delivery.candidates_queued, 1);
    assert_eq!(plan_delivery.candidates_skipped_distance, 1);

    assert!(
        drain_durable_creature_runtime_commands_like_cpp(&registry, invisible_observer_guid)
            .is_empty(),
        "out-of-range observer must not receive START or GO"
    );

    let victim_commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim_guid);
    let [SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(cast)] = victim_commands.as_slice()
    else {
        panic!("victim must retain one atomic START+GO command: {victim_commands:?}");
    };
    assert_eq!(cast.start_packet_bytes, start_bytes);
    assert_eq!(cast.go_packet_bytes, basic_go_bytes);
}
/// C++ selects recipients inside `SendSpellGo` from each viewer's
/// `HaveAtClient`, so a viewer that does not have the caster at client when
/// the cast resolves never gets a command — becoming visible afterwards
/// cannot deliver the older cast.
#[test]
fn creature_spell_plan_commits_have_at_client_at_resolution_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim_guid = ObjectGuid::create_player(1, 84);
    let unaware_guid = ObjectGuid::create_player(1, 85);
    let (victim_info, _victim_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let (unaware_info, _unaware_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(25.0, 0.0, 0.0, 0.0), true);
    victim_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    let unaware_visibility = unaware_info.client_visible_guids_like_cpp.clone();
    registry.register_or_replace(victim_guid, victim_info, Default::default());
    registry.register_or_replace(unaware_guid, unaware_info, Default::default());

    let (plan, _start_bytes, _basic_go_bytes, _full_go_bytes) =
        make_creature_spell_runtime_plan_like_cpp(victim_guid);
    let plan_delivery = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(plan_delivery.candidates_seen, 2);
    assert_eq!(plan_delivery.candidates_queued, 1);
    assert_eq!(plan_delivery.candidates_skipped_not_visible, 1);
    assert_eq!(plan_delivery.candidates_skipped_distance, 0);

    // The caster becoming visible after the cast resolved must not conjure a
    // command that was never committed.
    unaware_visibility.insert(make_source_guid());
    assert!(
        drain_durable_creature_runtime_commands_like_cpp(&registry, unaware_guid).is_empty(),
        "a viewer that was not selected at commit time receives nothing"
    );
    assert_eq!(
        drain_durable_creature_runtime_commands_like_cpp(&registry, victim_guid).len(),
        1
    );
}
/// (6) MapBroadcastVisible: enqueues all players on the same map/instance
/// regardless of distance, but respects map/instance/in_world.
/// C++ anchor: WorldObject::SendMessageToSet map-wide broadcast path.
#[test]
fn map_broadcast_visible_ignores_distance_but_respects_map_instance_in_world_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();

    // In range player — correct map/instance.
    let in_guid = ObjectGuid::create_player(1, 10);
    let (in_info, in_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(9999.0, 9999.0, 0.0, 0.0), true);
    registry.register_or_replace(in_guid, in_info, Default::default());

    // Wrong map.
    let wrong_map_guid = ObjectGuid::create_player(1, 11);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 0, Position::ZERO, true);
    registry.register_or_replace(wrong_map_guid, wrong_map_info, Default::default());

    // Not in world.
    let no_world_guid = ObjectGuid::create_player(1, 12);
    let (no_world_info, no_world_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, false);
    registry.register_or_replace(no_world_guid, no_world_info, Default::default());

    let event = wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::MapBroadcastVisible {
            map_id: 571,
            instance_id: 0,
        },
        packet_bytes: vec![0xCC],
    };
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1);
    assert!(in_rx.try_recv().is_ok(), "valid player got command");
    assert!(wrong_map_rx.try_recv().is_err(), "wrong-map excluded");
    assert!(no_world_rx.try_recv().is_err(), "not-in-world excluded");
}
/// (7) ExplicitPlayer: command sent to exactly one GUID, no other sessions.
/// C++ anchor: WorldObject::SendMessageToSet explicit receiver path.
#[test]
fn explicit_player_routes_only_to_target_guid_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let target_guid = ObjectGuid::create_player(1, 20);
    let other_guid = ObjectGuid::create_player(1, 21);
    let (target_info, target_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let (other_info, other_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    registry.register_or_replace(target_guid, target_info, Default::default());
    registry.register_or_replace(other_guid, other_info, Default::default());

    let event = wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::ExplicitPlayer(target_guid),
        packet_bytes: vec![0xDD],
    };
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1);
    assert!(target_rx.try_recv().is_ok(), "target received command");
    assert!(other_rx.try_recv().is_err(), "other session NOT notified");
}
#[test]
fn runtime_directory_delivery_rejects_replaced_recipient_generation() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 22);
    let (first_info, first_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    registry.register_or_replace(guid, first_info, Default::default());
    let stale = registry.runtime_recipient(guid).expect("first recipient");

    let (second_info, second_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let current = registry.register_or_replace(guid, second_info, Default::default());
    let command = SessionCommand::KickLikeCpp(wow_world::session::mailbox::KickLikeCppCommand {
        reason: "stale runtime delivery".to_string(),
    });

    assert_eq!(
        registry.try_send_current_command(stale.registration, command),
        Err(wow_world::session::directory::PlayerDirectorySendError::StaleRegistration)
    );
    assert!(first_rx.try_recv().is_err());
    assert!(second_rx.try_recv().is_err());

    registry
        .try_send_current_command(
            current,
            SessionCommand::KickLikeCpp(wow_world::session::mailbox::KickLikeCppCommand {
                reason: "current runtime delivery".to_string(),
            }),
        )
        .expect("current recipient remains addressable");
    assert!(second_rx.try_recv().is_ok());
}
/// (8) SelfOnly: NO broadcast global; increments self_only_skipped counter.
/// Guarantees SelfOnly events are not distributed to any registry session.
/// C++ anchor: WorldObject::SendMessageToSet — self-send path bypasses
/// MessageDistDeliverer entirely.
#[test]
fn self_only_does_not_broadcast_to_any_session_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    // Even with a matching player in registry, SelfOnly must NOT deliver.
    let guid = ObjectGuid::create_player(1, 30);
    let (info, command_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    registry.register_or_replace(guid, info, Default::default());

    let event = wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::SelfOnly,
        packet_bytes: vec![0xEE],
    };
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.self_only_skipped, 1, "must count skipped SelfOnly");
    assert_eq!(
        summary.candidates_queued, 0,
        "must NOT broadcast to registry"
    );
    assert_eq!(summary.candidates_seen, 0, "no candidates should be seen");
    assert!(
        command_rx.try_recv().is_err(),
        "session must NOT receive command"
    );
}
/// (9) try_send on a full channel increments send_failed and does NOT block.
/// Backpressure requirement from Slice 4A.1b spec.
#[test]
fn full_command_channel_increments_send_failed_and_does_not_block_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 40);

    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    // Drop the receiver so try_send returns Err::Disconnected immediately.
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    drop(command_rx);

    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "Full");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    info.placement.position = Position::ZERO;
    registry.register_or_replace(guid, info, Default::default());

    let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 1000.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(
        summary.send_failed, 1,
        "disconnected channel counted as send_failed"
    );
    assert_eq!(summary.candidates_queued, 0);
}
