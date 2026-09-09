//! Scenarios for [`super`], part 14.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

/// 4B.2a smoke: exercise the real experimental production loop wrapper,
/// not only the single-shot bridge.  The loop remains disabled by default;
/// this test flips `GlobalLegacy` explicitly, waits for one visible
/// movement command, then aborts the forever-running task.
#[tokio::test]
async fn legacy_creature_runtime_loop_smoke_delivers_visible_work_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let creature_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 94_001);
    let creature_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut world_creature = wow_world::map_manager::WorldCreature::new(
        creature_guid,
        9001,
        creature_position,
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
        let ai = world_creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
        ai.aggro_radius = 0.0;
        ai.swing_timer_ms = u64::MAX;
    }
    world_creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    world_creature.seed_runtime_rng_like_cpp(0x9401);

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
            wow_world::map_manager::world_to_grid_x(creature_position.x),
            wow_world::map_manager::world_to_grid_y(creature_position.y),
            world_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let player = ObjectGuid::create_player(1, 94_002);
    let (player_info, player_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(11.0, 10.0, 0.0, 0.0), true);
    registry.register_or_replace(player, player_info, Default::default());

    let handle = spawn_legacy_creature_runtime_update_loop_like_cpp(
        true,
        Arc::clone(&legacy),
        Arc::clone(&canonical),
        Arc::new(legacy_runtime_world_map_store_like_cpp()),
        wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        },
        None,
        wow_world::session::LegacyCreatureAggroConfigLikeCpp::default(),
        1,
        None,
        Arc::new(Mutex::new(())),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        None,
        Arc::clone(&registry),
    );

    let command = tokio::time::timeout(std::time::Duration::from_secs(2), player_rx.recv_async())
        .await
        .expect("runtime loop should deliver visible work")
        .expect("command channel should stay open");
    handle.abort();
    let _ = handle.await;

    let SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp movement command");
    };
    assert_eq!(command.source_guid, creature_guid);
    assert_eq!(command.map_id, 0);
    assert_eq!(command.instance_id, 0);
    let opcode = u16::from_le_bytes([command.packet_bytes[0], command.packet_bytes[1]]);
    assert_eq!(opcode, wow_constants::ServerOpcodes::OnMonsterMove as u16);

    let guard = canonical.lock().unwrap();
    let typed = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(creature_guid, Clone::clone)
        .expect("production loop must keep canonical creature synced");
    assert_eq!(
        typed.ai_state(),
        wow_entities::CreatureAiState::WalkingRandom
    );
}
#[tokio::test]
async fn legacy_respawn_producer_stop_runs_final_lifecycle_flush_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let creature_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 95_001);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut creature = wow_world::map_manager::WorldCreature::new(
        creature_guid,
        9001,
        position,
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
    creature.creature.set_spawn_id(95_001);
    creature.take_damage(10);
    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(position.x),
            wow_world::map_manager::world_to_grid_y(position.y),
            creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let producer_stop = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let writer_tx = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    let writer_probe = writer_tx.clone();
    let handle = spawn_legacy_creature_runtime_update_loop_like_cpp(
        true,
        legacy,
        canonical,
        Arc::new(legacy_runtime_world_map_store_like_cpp()),
        wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        },
        None,
        wow_world::session::LegacyCreatureAggroConfigLikeCpp::default(),
        1,
        Some(writer_tx),
        Arc::new(Mutex::new(())),
        producer_stop,
        None,
        Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp()),
    );

    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("stopping producer must finish its final lifecycle tick")
        .expect("final lifecycle tick must not panic");
    let mut mailbox = writer_probe
        .mailbox
        .state
        .lock()
        .expect("respawn DB mailbox lock");
    assert_eq!(mailbox.queue.pending_len(), 1);
    let mutation = mailbox
        .queue
        .take_due(Instant::now())
        .expect("final lifecycle tick must persist the pending death")
        .pending
        .mutation;
    assert!(matches!(
        mutation,
        RespawnPersistenceMutationLikeCpp::Save { .. }
    ));
    assert_eq!(mailbox.queue.pending_len(), 0);
}
