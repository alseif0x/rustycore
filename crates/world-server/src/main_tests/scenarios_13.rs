//! Scenarios for [`super`], part 13.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn creature_melee_damage_delivery_filters_registry_state_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_057);
    let wrong_map = ObjectGuid::create_player(1, 58);
    let wrong_instance = ObjectGuid::create_player(1, 59);
    let not_in_world = ObjectGuid::create_player(1, 60);
    let missing = ObjectGuid::create_player(1, 61);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 0, Position::ZERO, true);
    let (wrong_instance_info, wrong_instance_rx) =
        make_registry_player_like_cpp(571, 9, Position::ZERO, true);
    let (not_in_world_info, not_in_world_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, false);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    registry.register_or_replace(wrong_instance, wrong_instance_info, Default::default());
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());

    let make_command =
        |victim_guid| wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid,
            map_id: 571,
            instance_id: 0,
            damage: 5,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 95,
            victim_health_state_revision_after: 1,
        };
    let commands = vec![
        make_command(wrong_map),
        make_command(wrong_instance),
        make_command(not_in_world),
        make_command(missing),
    ];
    let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 4);
    assert_eq!(summary.candidates_seen, 3);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);
    assert_eq!(summary.candidates_skipped_missing_victim, 1);
    assert!(wrong_map_rx.try_recv().is_err());
    assert!(wrong_instance_rx.try_recv().is_err());
    assert!(not_in_world_rx.try_recv().is_err());
}
#[test]
fn creature_melee_damage_delivery_poisoned_durable_rail_counts_send_failed_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 62);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_058);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "MeleeFull");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    let durable = Arc::clone(&info.durable_creature_runtime_commands_like_cpp);
    let _ = std::thread::spawn(move || {
        let _guard = durable.lock().unwrap();
        panic!("poison durable rail for delivery failure coverage");
    })
    .join();
    registry.register_or_replace(victim, info, Default::default());

    let commands = vec![
        wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 0,
            damage: 5,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 95,
            victim_health_state_revision_after: 1,
        },
    ];
    let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 1);
    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.send_failed, 1);
}
#[test]
fn creature_melee_damage_delivery_preserves_every_swing_when_general_queue_is_full_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 64);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_063);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx.clone(), "MeleeRetry");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    registry.register_or_replace(victim, info, Default::default());
    let command = wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: victim,
        map_id: 571,
        instance_id: 0,
        damage: 5,
        over_damage: -1,
        target_level: 80,
        victim_health_after: 95,
        victim_health_state_revision_after: 7,
    };
    command_tx
        .send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            command.clone(),
        ))
        .unwrap();

    let mut latest = command.clone();
    latest.victim_health_after = 90;
    latest.victim_health_state_revision_after = 8;
    let summary = deliver_creature_melee_damage_commands_like_cpp(&[command, latest], &registry);
    assert_eq!(summary.candidates_queued, 2);
    assert_eq!(summary.send_failed, 0);
    assert_eq!(command_rx.len(), 1, "bounded general queue remains full");
    let commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim);
    assert_eq!(
        commands.len(),
        2,
        "every committed swing remains observable"
    );
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(first) = &commands[0] else {
        panic!("expected first durable melee command");
    };
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(second) = &commands[1] else {
        panic!("expected second durable melee command");
    };
    assert_eq!(first.victim_health_after, 95);
    assert_eq!(first.victim_health_state_revision_after, 7);
    assert_eq!(second.victim_health_after, 90);
    assert_eq!(second.victim_health_state_revision_after, 8);
}
/// 4C.3 compatibility bridge: canonical health is applied once and the
/// final-health command is delivered outside all map locks. The outcome is
/// still marked unrepresented until full `CalculateMeleeDamage` exists.
#[test]
fn legacy_creature_melee_tick_delivers_compatibility_victim_command_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let victim = ObjectGuid::create_player(1, 63);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_059);
    let attacker_position = Position::new(5.0, 5.0, 0.0, 0.0);
    add_canonical_test_player_on_map_like_cpp(&canonical, victim, attacker_position, 0, 0, 100);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, attacker_position, 0, 0, 25);
    let mut world_creature =
        mirror_canonical_melee_test_creature_like_cpp(&canonical, attacker, 0, 0);
    world_creature.enter_combat(victim);
    world_creature.creature.ai_ownership_mut().swing_timer_ms = 0;

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(attacker_position.x),
            wow_world::map_manager::world_to_grid_y(attacker_position.y),
            world_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (victim_info, _victim_rx) = make_registry_player_like_cpp(0, 0, attacker_position, true);
    registry.register_or_replace(victim, victim_info, Default::default());

    let (outcome, delivery, plan_delivery) =
        run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
            &legacy,
            Some(&canonical),
            &registry,
        );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(delivery.commands_seen, 1);
    assert_eq!(delivery.candidates_seen, 1);
    assert_eq!(delivery.candidates_queued, 1);
    assert_eq!(plan_delivery.events_seen, 0);
    let command = match drain_durable_creature_runtime_commands_like_cpp(&registry, victim)
        .pop()
        .expect("victim session receives final-health melee command")
    {
        SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command) => command,
        other => panic!("expected ApplyCreatureMeleeDamageLikeCpp, got {other:?}"),
    };
    assert_eq!(command.attacker_guid, attacker);
    assert_eq!(command.victim_guid, victim);
    assert!((3..=5).contains(&command.damage));

    let canonical_health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(victim)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(canonical_health, command.victim_health_after);
    assert_eq!(canonical_health, 100 - u64::from(command.damage));
}
#[test]
fn legacy_creature_melee_tick_delivers_compatibility_creature_plan_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let victim = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9002, 90_060);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_061);
    let position = Position::new(5.0, 5.0, 0.0, 0.0);
    add_canonical_test_creature_on_map_like_cpp(&canonical, victim, position, 0, 0, 100);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, position, 0, 0, 25);
    let mut world_creature =
        mirror_canonical_melee_test_creature_like_cpp(&canonical, attacker, 0, 0);
    world_creature.enter_combat(victim);
    world_creature.creature.ai_ownership_mut().swing_timer_ms = 0;

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(position.x),
            wow_world::map_manager::world_to_grid_y(position.y),
            world_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let viewer = ObjectGuid::create_player(1, 91_030);
    let (viewer_info, viewer_rx) = make_registry_player_like_cpp(0, 0, position, true);
    registry.register_or_replace(viewer, viewer_info, Default::default());

    let (outcome, delivery, plan_delivery) =
        run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
            &legacy,
            Some(&canonical),
            &registry,
        );

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert!(outcome.commands.is_empty());
    assert_eq!(delivery.commands_seen, 0);
    assert_eq!(plan_delivery.events_seen, 2);
    assert_eq!(plan_delivery.candidates_queued, 2);
    for _ in 0..2 {
        let SessionCommand::SendIfVisibleLikeCpp(command) = viewer_rx
            .try_recv()
            .expect("viewer receives creature-victim melee fanout")
        else {
            panic!("expected SendIfVisibleLikeCpp");
        };
        assert!(command.source_guid == attacker || command.source_guid == victim);
        assert!(!command.packet_bytes.is_empty());
    }
}
/// 4A.3c bridge: lifecycle changes happen once under the global owner, then
/// matching sessions are woken to run their own visibility pass.
#[test]
fn legacy_creature_lifecycle_tick_refreshes_sessions_after_ready_respawn_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);
    legacy
        .write()
        .unwrap()
        .set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);

    let now = std::time::Instant::now();
    let creature_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_012);
    let mut world_creature = wow_world::map_manager::WorldCreature::new(
        creature_guid,
        9001,
        Position::new(20.0, 20.0, 0.0, 0.0),
        30,
        4,
        5,
        9,
        20.0,
        100,
        14,
        0,
        0,
    );
    world_creature
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(77, wow_constants::PhaseFlags::empty(), 1);
    let pending = wow_world::map_manager::pending_respawn_from_world_creature_like_cpp(
        &world_creature,
        now - std::time::Duration::from_secs(1),
        0,
    );
    legacy.write().unwrap().push_respawn(0, 0, pending);

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let same_a = ObjectGuid::create_player(1, 91_001);
    let (same_a_info, same_a_rx) = make_registry_player_like_cpp(0, 0, Position::ZERO, true);
    registry.register_or_replace(same_a, same_a_info, Default::default());
    let same_b = ObjectGuid::create_player(1, 91_002);
    let (same_b_info, same_b_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(9000.0, 0.0, 0.0, 0.0), true);
    registry.register_or_replace(same_b, same_b_info, Default::default());
    let wrong_map = ObjectGuid::create_player(1, 91_003);
    let (wrong_map_info, wrong_map_rx) = make_registry_player_like_cpp(1, 0, Position::ZERO, true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());

    let (outcome, delivery) = run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp(
        &legacy,
        Some(&canonical),
        &legacy_runtime_world_map_store_like_cpp(),
        now,
        &registry,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.canonical_inserts, 1);
    assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);
    assert_eq!(delivery.candidates_seen, 3);
    assert_eq!(delivery.candidates_queued, 2);
    assert_eq!(delivery.candidates_skipped_wrong_map, 1);

    for command in [
        same_a_rx.try_recv().expect("same-map session A refresh"),
        same_b_rx.try_recv().expect("same-map session B refresh"),
    ] {
        let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) = command else {
            panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
        };
        assert_eq!(command.map_id, 0);
        assert_eq!(command.instance_id, 0);
    }
    assert!(wrong_map_rx.try_recv().is_err());

    let canonical_guard = canonical.lock().unwrap();
    let typed = canonical_guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(creature_guid, Clone::clone)
        .expect("lifecycle bridge must sync canonical respawn");
    assert!(typed.unit().world().phase_shift().has_phase_like_cpp(77));
}
/// Slice 4A.4 test-only bridge: one gated global movement tick runs from a
/// spawned task, produces a `RuntimePlan`, syncs canonical state outside
/// the legacy lock, then delivers `SendIfVisibleLikeCpp` commands to
/// candidate sessions.
///
/// No production loop calls this yet; this only proves the cross-crate
/// task/ownership path with `GlobalLegacy` flipped only in the test.
#[tokio::test]
async fn legacy_creature_global_tick_task_delivers_movement_plan_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_009);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut world_creature = wow_world::map_manager::WorldCreature::new(
        guid, 9001, position, 25, 2, 3, 5, 20.0, 100, 14, 0, 0,
    );
    {
        let ai = world_creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
    }
    world_creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    world_creature.seed_runtime_rng_like_cpp(0x9009);

    let mut canonical_creature = world_creature.creature.clone();
    canonical_creature
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    canonical_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(canonical_creature).unwrap())
        .unwrap();

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(position.x),
            wow_world::map_manager::world_to_grid_y(position.y),
            world_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let near_a = ObjectGuid::create_player(1, 90_001);
    let (near_a_info, near_a_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(11.0, 10.0, 999.0, 0.0), true);
    registry.register_or_replace(near_a, near_a_info, Default::default());
    let near_b = ObjectGuid::create_player(1, 90_002);
    let (near_b_info, near_b_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(12.0, 10.0, -999.0, 0.0), true);
    registry.register_or_replace(near_b, near_b_info, Default::default());
    let wrong_map = ObjectGuid::create_player(1, 90_003);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(1, 0, Position::new(10.0, 10.0, 0.0, 0.0), true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());

    let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let legacy_for_task = Arc::clone(&legacy);
    let canonical_for_task = Arc::clone(&canonical);
    let registry_for_task = Arc::clone(&registry);
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        tokio::task::spawn_blocking(move || {
            run_legacy_creature_movement_tick_and_deliver_once_like_cpp(
                &legacy_for_task,
                Some(&canonical_for_task),
                &mmap_config,
                None,
                1,
                registry_for_task.as_ref(),
            )
        })
        .await
        .expect("legacy global tick task must not panic")
    });
    let (outcome, delivery) = handle.await.expect("tick task must complete");

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.movement_packets, 1);
    assert_eq!(outcome.canonical_syncs, 1);
    assert_eq!(delivery.events_seen, 1);
    assert_eq!(delivery.candidates_seen, 3);
    assert_eq!(delivery.candidates_queued, 2);
    assert_eq!(delivery.candidates_skipped_wrong_map, 1);

    for command in [
        near_a_rx.try_recv().expect("near player A command"),
        near_b_rx.try_recv().expect("near player B command"),
    ] {
        let SessionCommand::SendIfVisibleLikeCpp(command) = command else {
            panic!("expected SendIfVisibleLikeCpp command");
        };
        assert_eq!(command.source_guid, guid);
        assert_eq!(command.map_id, 0);
        assert_eq!(command.instance_id, 0);
        let opcode = u16::from_le_bytes([command.packet_bytes[0], command.packet_bytes[1]]);
        assert_eq!(opcode, wow_constants::ServerOpcodes::OnMonsterMove as u16);
    }
    assert!(wrong_map_rx.try_recv().is_err());

    let guard = canonical.lock().unwrap();
    let typed = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .expect("canonical creature record stays synced by the single-shot driver");
    assert_eq!(
        typed.ai_state(),
        wow_entities::CreatureAiState::WalkingRandom
    );
}
/// Combined runtime bridge: one test-only task runs lifecycle first
/// (map-owned despawn/respawn visibility refresh), then movement
/// (NearbyVisible MonsterMove fanout), then the transitional melee
/// compatibility bridge while retaining the unrepresented-outcome marker.
/// `GlobalLegacy` is enabled only inside the test.
#[tokio::test]
async fn legacy_creature_global_runtime_task_delivers_lifecycle_movement_and_melee_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let melee_victim = ObjectGuid::create_player(1, 92_004);
    let melee_position = Position::new(300.0, 300.0, 0.0, 0.0);
    add_canonical_test_player_on_map_like_cpp(&canonical, melee_victim, melee_position, 0, 0, 100);

    let moving_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_013);
    let moving_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut moving_creature = wow_world::map_manager::WorldCreature::new(
        moving_guid,
        9001,
        moving_position,
        25,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    {
        let ai = moving_creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
        ai.aggro_radius = 0.0;
    }
    moving_creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    moving_creature.seed_runtime_rng_like_cpp(0x900D);
    let mut canonical_moving = moving_creature.creature.clone();
    canonical_moving
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    canonical_moving
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(canonical_moving).unwrap())
        .unwrap();

    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_014);
    let mut corpse_creature = wow_world::map_manager::WorldCreature::new(
        corpse_guid,
        9001,
        Position::new(20.0, 20.0, 0.0, 0.0),
        10,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    corpse_creature.take_damage(10);
    corpse_creature.set_corpse_despawn_at(Some(
        std::time::Instant::now() - std::time::Duration::from_secs(1),
    ));

    let melee_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_015);
    add_canonical_test_creature_on_map_like_cpp(&canonical, melee_guid, melee_position, 0, 0, 25);
    let mut melee_creature =
        mirror_canonical_melee_test_creature_like_cpp(&canonical, melee_guid, 0, 0);
    melee_creature.enter_combat(melee_victim);
    melee_creature.creature.ai_ownership_mut().swing_timer_ms = 0;

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(moving_position.x),
            wow_world::map_manager::world_to_grid_y(moving_position.y),
            moving_creature,
        );
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(20.0),
            wow_world::map_manager::world_to_grid_y(20.0),
            corpse_creature,
        );
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(melee_position.x),
            wow_world::map_manager::world_to_grid_y(melee_position.y),
            melee_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = Arc::new(PlayerRegistry::default());
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let near_a = ObjectGuid::create_player(1, 92_001);
    let (near_a_info, near_a_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(11.0, 10.0, 999.0, 0.0), true);
    registry.register_or_replace(near_a, near_a_info, Default::default());
    let near_b = ObjectGuid::create_player(1, 92_002);
    let (near_b_info, near_b_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(12.0, 10.0, -999.0, 0.0), true);
    registry.register_or_replace(near_b, near_b_info, Default::default());
    let wrong_map = ObjectGuid::create_player(1, 92_003);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(1, 0, Position::new(10.0, 10.0, 0.0, 0.0), true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    let (mut victim_info, victim_rx) = make_registry_player_like_cpp(0, 0, melee_position, true);
    registry.register_or_replace(melee_victim, victim_info, Default::default());
    add_canonical_test_player_on_map_like_cpp(
        &canonical,
        near_a,
        Position::new(11.0, 10.0, 999.0, 0.0),
        0,
        0,
        100,
    );
    add_canonical_test_player_on_map_like_cpp(
        &canonical,
        near_b,
        Position::new(12.0, 10.0, -999.0, 0.0),
        0,
        0,
        100,
    );
    canonical.lock().unwrap().create_world_map(1, 0);
    add_canonical_test_player_on_map_like_cpp(
        &canonical,
        wrong_map,
        Position::new(10.0, 10.0, 0.0, 0.0),
        1,
        0,
        100,
    );
    let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let aggro_config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                wow_data::progression_rewards::FactionTemplateEntry {
                    id: 14,
                    faction: 72,
                    flags: 0,
                    faction_group: 0,
                    friend_group: 0,
                    enemy_group: 0,
                    enemies: [930, 0, 0, 0, 0, 0, 0, 0],
                    friend: [0; 8],
                },
                wow_data::progression_rewards::FactionTemplateEntry {
                    id: 1,
                    faction: 930,
                    flags: 0,
                    faction_group: 0,
                    friend_group: 0,
                    enemy_group: 0,
                    enemies: [0; 8],
                    friend: [0; 8],
                },
            ]),
        )),
        faction_store: Some(Arc::new(
            wow_data::progression_rewards::FactionStore::from_entries([
                wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
            ]),
        )),
        ..Default::default()
    };
    let legacy_for_task = Arc::clone(&legacy);
    let canonical_for_task = Arc::clone(&canonical);
    let registry_for_task = Arc::clone(&registry);
    let map_store_for_task = legacy_runtime_world_map_store_like_cpp();
    let tick_now = std::time::Instant::now() + std::time::Duration::from_millis(1);
    let handle = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
                &legacy_for_task,
                Some(&canonical_for_task),
                &map_store_for_task,
                &mmap_config,
                None,
                aggro_config,
                10,
                tick_now,
                registry_for_task.as_ref(),
                None,
                None,
                None,
                &Arc::new(Mutex::new(Default::default())),
            )
        })
        .await
        .expect("combined legacy runtime tick task must not panic")
    });
    let outcome = handle.await.expect("combined tick task must complete");

    assert!(!outcome.lifecycle.skipped_owner_not_global);
    assert_eq!(outcome.lifecycle.maps_seen, 1);
    assert_eq!(outcome.lifecycle.creatures_seen, 3);
    assert_eq!(outcome.lifecycle.corpses_despawned, 1);
    assert_eq!(outcome.lifecycle.refresh_map_keys, vec![(0, 0)]);
    assert_eq!(outcome.lifecycle_delivery.candidates_seen, 4);
    assert_eq!(outcome.lifecycle_delivery.candidates_queued, 3);
    assert_eq!(outcome.lifecycle_delivery.candidates_skipped_wrong_map, 1);

    assert!(!outcome.movement.skipped_owner_not_global);
    assert_eq!(outcome.movement.maps_seen, 1);
    assert_eq!(outcome.movement.creatures_seen, 2);
    assert_eq!(outcome.movement.movement_packets, 2);
    assert_eq!(outcome.movement_delivery.events_seen, 2);
    assert_eq!(outcome.movement_delivery.candidates_seen, 8);
    assert_eq!(outcome.movement_delivery.candidates_queued, 3);
    assert_eq!(outcome.movement_delivery.candidates_skipped_distance, 3);
    assert_eq!(outcome.movement_delivery.candidates_skipped_wrong_map, 2);

    assert!(!outcome.aggro.skipped_owner_not_global);
    assert_eq!(outcome.aggro.maps_seen, 1);
    assert_eq!(outcome.aggro.creatures_seen, 2);
    assert_eq!(outcome.aggro.candidates_seen, 3);
    assert_eq!(outcome.aggro.aggro_starts, 0);
    assert_eq!(outcome.aggro_delivery.commands_seen, 0);
    assert_eq!(outcome.aggro_delivery.candidates_queued, 0);

    assert!(!outcome.melee.skipped_owner_not_global);
    assert_eq!(outcome.melee.maps_seen, 1);
    assert_eq!(outcome.melee.creatures_seen, 2);
    assert_eq!(outcome.melee.swings_ready, 1);
    assert_eq!(outcome.melee.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.melee.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.melee.canonical_hits, 1);
    assert_eq!(outcome.melee_delivery.commands_seen, 1);
    assert_eq!(outcome.melee_delivery.candidates_seen, 1);
    assert_eq!(outcome.melee_delivery.candidates_queued, 1);
    assert_eq!(outcome.melee_plan_delivery.events_seen, 0);

    for command_rx in [&near_a_rx, &near_b_rx] {
        let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(refresh) = command_rx
            .try_recv()
            .expect("same-map player must receive lifecycle refresh")
        else {
            panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
        };
        assert_eq!(refresh.map_id, 0);
        assert_eq!(refresh.instance_id, 0);

        let SessionCommand::SendIfVisibleLikeCpp(move_command) = command_rx
            .try_recv()
            .expect("same-map player must receive movement command")
        else {
            panic!("expected SendIfVisibleLikeCpp command");
        };
        assert_eq!(move_command.source_guid, moving_guid);
        assert_eq!(move_command.map_id, 0);
        assert_eq!(move_command.instance_id, 0);
        let opcode =
            u16::from_le_bytes([move_command.packet_bytes[0], move_command.packet_bytes[1]]);
        assert_eq!(opcode, wow_constants::ServerOpcodes::OnMonsterMove as u16);
    }

    let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(refresh) = victim_rx
        .try_recv()
        .expect("victim same-map session must receive lifecycle refresh")
    else {
        panic!("expected RefreshVisibleWorldCreaturesLikeCpp command for victim");
    };
    assert_eq!(refresh.map_id, 0);
    assert_eq!(refresh.instance_id, 0);
    let SessionCommand::SendIfVisibleLikeCpp(chase_stop) = victim_rx
        .try_recv()
        .expect("melee victim receives its attacker's in-range chase stop")
    else {
        panic!("expected SendIfVisibleLikeCpp chase-stop command for victim");
    };
    assert_eq!(chase_stop.source_guid, melee_guid);
    assert_eq!(chase_stop.map_id, 0);
    assert_eq!(chase_stop.instance_id, 0);
    assert_eq!(
        u16::from_le_bytes([chase_stop.packet_bytes[0], chase_stop.packet_bytes[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(melee_command) =
        drain_durable_creature_runtime_commands_like_cpp(registry.as_ref(), melee_victim)
            .pop()
            .expect("victim must receive the compatibility melee command")
    else {
        panic!("expected ApplyCreatureMeleeDamageLikeCpp command for victim");
    };
    assert_eq!(melee_command.attacker_guid, melee_guid);
    assert_eq!(melee_command.victim_guid, melee_victim);
    assert!((3..=5).contains(&melee_command.damage));
    assert!(
        victim_rx.try_recv().is_err(),
        "victim receives only its own attacker's chase stop"
    );
    assert!(wrong_map_rx.try_recv().is_err());

    {
        let guard = legacy.read().unwrap();
        assert!(
            guard.find_creature(0, 0, corpse_guid).is_none(),
            "expired corpse must be removed by lifecycle before movement"
        );
        assert!(
            guard.find_creature(0, 0, moving_guid).is_some(),
            "alive moving creature must remain in the legacy map"
        );
        assert!(
            guard.find_creature(0, 0, melee_guid).is_some(),
            "alive melee creature must remain in the legacy map"
        );
    }
    let guard = canonical.lock().unwrap();
    let typed = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(moving_guid, Clone::clone)
        .expect("movement phase must keep canonical moving creature synced");
    assert_eq!(
        typed.ai_state(),
        wow_entities::CreatureAiState::WalkingRandom
    );
    let victim_health = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(melee_victim)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(victim_health, melee_command.victim_health_after);
    assert_eq!(victim_health, 100 - u64::from(melee_command.damage));
}
