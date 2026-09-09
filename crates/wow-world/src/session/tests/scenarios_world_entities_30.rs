//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_lifecycle_tick_once_clears_timer_when_canonical_won_like_cpp() {
    use crate::map_manager::{
        RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp, world_to_grid_coords,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let now = Instant::now();
    let queued_guid = test_creature_guid(90_012);
    let live_guid = test_creature_guid(90_013);
    let mut queued_world_creature = crate::map_manager::WorldCreature::new(
        queued_guid,
        9002,
        Position::new(8.0, 9.0, 10.0, 1.5),
        35,
        4,
        5,
        9,
        20.0,
        101,
        14,
        0,
        0,
    );
    queued_world_creature.creature.set_spawn_id(90_012);
    let pending = pending_respawn_from_world_creature_like_cpp(
        &queued_world_creature,
        now - Duration::from_secs(1),
        0,
    );
    let spawn_id = pending.spawn_id;
    let mut live_world_creature = crate::map_manager::WorldCreature::new(
        live_guid,
        9002,
        Position::new(8.0, 9.0, 10.0, 1.5),
        35,
        4,
        5,
        9,
        20.0,
        101,
        14,
        0,
        0,
    );
    live_world_creature.creature.set_spawn_id(spawn_id);

    {
        let mut guard = manager.write().unwrap();
        let (grid_x, grid_y) = world_to_grid_coords(
            live_world_creature.position().x,
            live_world_creature.position().y,
        );
        assert!(
            guard.add_creature(0, 0, grid_x, grid_y, live_world_creature),
            "test setup simulates canonical ProcessRespawns already mirroring the spawn under a regenerated GUID"
        );
        assert!(
            guard
                .save_pending_respawn_time_like_cpp(0, 0, &pending, now, unix_now())
                .is_some(),
            "test setup must leave the stale persisted timer that the ready queue clears"
        );
        guard.push_respawn(0, 0, pending);
        assert!(
            guard
                .persisted_respawn_time_like_cpp(
                    0,
                    0,
                    wow_map::SpawnObjectType::Creature,
                    spawn_id,
                )
                .is_some()
        );
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.corpses_despawned, 0);
    assert_eq!(
        outcome.respawns_processed, 0,
        "legacy must not insert a duplicate when the creature is already present"
    );
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(matches!(
        outcome.respawn_db_mutations[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));
    assert_eq!(outcome.canonical_inserts, 0);
    assert_eq!(outcome.canonical_respawn_removes, 0);
    assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);

    let guard = manager.read().unwrap();
    assert!(
        guard.find_creature(0, 0, live_guid).is_some(),
        "canonical-won creature must remain in legacy"
    );
    assert!(
        guard.find_creature(0, 0, queued_guid).is_none(),
        "the stale queued GUID must not be inserted when its persistent spawn is already live"
    );
    assert_eq!(guard.respawn_queue_len(0, 0), 0);
    assert_eq!(
        guard.persisted_respawn_time_like_cpp(0, 0, wow_map::SpawnObjectType::Creature, spawn_id,),
        None,
        "stale legacy persisted timer must be cleared before the next death"
    );
}
#[test]
fn legacy_creature_lifecycle_tick_once_ignores_dead_spawn_id_duplicate_like_cpp() {
    use crate::map_manager::{
        RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp, world_to_grid_coords,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let now = Instant::now();
    let queued_guid = test_creature_guid(90_017);
    let dead_guid = test_creature_guid(90_018);
    let spawn_id = 90_017;
    let mut queued = crate::map_manager::WorldCreature::new(
        queued_guid,
        9006,
        Position::new(20.0, 21.0, 22.0, 1.25),
        55,
        8,
        9,
        13,
        20.0,
        105,
        14,
        0,
        0,
    );
    queued.creature.set_spawn_id(spawn_id);
    let pending =
        pending_respawn_from_world_creature_like_cpp(&queued, now - Duration::from_secs(1), 0);

    let mut dead = crate::map_manager::WorldCreature::new(
        dead_guid,
        9006,
        Position::new(23.0, 24.0, 25.0, 1.5),
        55,
        8,
        9,
        13,
        20.0,
        105,
        14,
        0,
        0,
    );
    dead.creature.set_spawn_id(spawn_id);
    assert!(dead.take_damage(55));
    dead.creature.runtime_state_mut().save_respawn_requested = false;
    dead.set_corpse_despawn_at(Some(now + Duration::from_secs(60)));

    {
        let mut guard = manager.write().unwrap();
        let (grid_x, grid_y) = world_to_grid_coords(dead.position().x, dead.position().y);
        assert!(guard.add_creature(0, 0, grid_x, grid_y, dead));
        assert!(
            guard
                .save_pending_respawn_time_like_cpp(0, 0, &pending, now, unix_now())
                .is_some()
        );
        guard.push_respawn(0, 0, pending);
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(matches!(
        outcome.respawn_db_mutations[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));
    let guard = manager.read().unwrap();
    assert!(guard.find_creature(0, 0, queued_guid).unwrap().is_alive());
    assert!(!guard.find_creature(0, 0, dead_guid).unwrap().is_alive());
    assert_eq!(guard.respawn_queue_len(0, 0), 0);
}
#[test]
fn legacy_creature_lifecycle_tick_once_respawns_synthetic_spawn_id_collision_like_cpp() {
    use crate::map_manager::{
        RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp, world_to_grid_coords,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let now = Instant::now();
    let synthetic_guid = test_creature_guid(90_014);
    let persistent_guid = test_creature_guid(90_015);
    let synthetic = crate::map_manager::WorldCreature::new(
        synthetic_guid,
        9003,
        Position::new(11.0, 12.0, 13.0, 0.5),
        40,
        5,
        6,
        10,
        20.0,
        102,
        14,
        0,
        0,
    );
    let pending =
        pending_respawn_from_world_creature_like_cpp(&synthetic, now - Duration::from_secs(1), 0);
    assert!(!pending.persistent_spawn);

    let mut persistent = crate::map_manager::WorldCreature::new(
        persistent_guid,
        9004,
        Position::new(14.0, 15.0, 16.0, 0.75),
        45,
        6,
        7,
        11,
        20.0,
        103,
        14,
        0,
        0,
    );
    persistent.creature.set_spawn_id(pending.spawn_id);

    {
        let mut guard = manager.write().unwrap();
        let (grid_x, grid_y) =
            world_to_grid_coords(persistent.position().x, persistent.position().y);
        assert!(guard.add_creature(0, 0, grid_x, grid_y, persistent));
        guard.push_respawn(0, 0, pending);
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert_eq!(outcome.respawns_processed, 1);
    assert!(outcome.respawn_db_mutations.is_empty());
    assert_eq!(outcome.canonical_inserts, 1);
    assert_eq!(outcome.canonical_respawn_removes, 0);

    let guard = manager.read().unwrap();
    let synthetic = guard
        .find_creature(0, 0, synthetic_guid)
        .expect("GUID-low collision with a DB spawn id must not discard a synthetic respawn");
    assert_eq!(synthetic.creature.spawn_id(), 0);
    assert!(guard.find_creature(0, 0, persistent_guid).is_some());
    assert_eq!(guard.respawn_queue_len(0, 0), 0);
}
#[test]
fn two_sessions_same_map_under_global_owner_do_not_session_tick_creature() {
    // Two sessions share the same Arc<RwLock<MapManager>>. With GlobalLegacy,
    // neither session must advance the shared creature via session ticks.
    // Contrast: with Session (default) both would tick the same creature
    // (double tick — the bug GlobalLegacy is designed to prevent in Slice 4).
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let guid = test_creature_guid(90_004);
    let (mut session1, _, recv1) = make_session();
    register_test_creature(&mut session1, manager.clone(), guid, 100);
    session1
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
        })
        .unwrap();

    let (mut session2, _, recv2) = make_session();
    session2.set_map_manager(manager.clone());
    session2.current_map_id = 0;

    let start_position = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .expect("creature must exist")
        .position();

    // Both sessions read GlobalLegacy → neither runs tick_creatures_sync.
    let owner1 = session1.runtime_tick_owner_like_cpp();
    let owner2 = session2.runtime_tick_owner_like_cpp();
    assert_eq!(owner1, RuntimeTickOwner::GlobalLegacy);
    assert_eq!(owner2, RuntimeTickOwner::GlobalLegacy);

    if owner1 == RuntimeTickOwner::Session {
        session1.tick_creatures_sync();
    }
    if owner2 == RuntimeTickOwner::Session {
        session2.tick_creatures_sync();
    }

    let end_position = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .expect("creature must exist")
        .position();
    assert_eq!(
        end_position, start_position,
        "creature position must be unchanged when GlobalLegacy suppresses session creature ticks"
    );

    assert!(recv1.try_recv().is_err(), "session1 must not send packets");
    assert!(recv2.try_recv().is_err(), "session2 must not send packets");
}
#[test]
fn respawn_queue_lives_in_map_not_in_session_like_cpp() {
    // After a creature is despawned and queued for respawn, the pending entry
    // must be stored in the map's respawn_queue (MapInstance), not in WorldSession.
    let (mut session, _, _) = setup_dead_creature_past_despawn(91_001);

    // First tick: despawn fires and pushes to the MAP queue.
    let output = session.run_creatures_tick();
    session.flush_runtime_output(output);

    // Verify the pending respawn is in the map, not in a session field.
    let map_id = session.player_map_id_like_cpp();
    let queue_len = session
        .map_manager
        .as_ref()
        .map(|m| {
            m.read()
                .unwrap_or_else(|p| p.into_inner())
                .respawn_queue_len(map_id, 0)
        })
        .unwrap_or(0);
    assert_eq!(queue_len, 1, "pending respawn must be in the map queue");
}
#[test]
fn single_session_respawn_produces_create_packet_like_cpp() {
    // 1 session: kill creature → corpse despawn → respawn ready → run_creatures_tick
    // must produce an SMSG_UPDATE_OBJECT CREATE block.
    // This verifies byte-identity of the single-session path: the same opcode
    // sequence that was produced before the ownership migration still fires.
    let (mut session, send_rx, _guid) = setup_dead_creature_past_despawn(91_002);

    // Tick 1: despawn — removes the creature from the world and enqueues respawn.
    let output = session.run_creatures_tick();
    session.flush_runtime_output(output);

    // Drain any packets sent so far (DESTROY block).
    while send_rx.try_recv().is_ok() {}

    // Force the queued respawn to be immediately ready.
    force_respawn_ready(&mut session);

    // Tick 2: respawn fires — must produce an SMSG_UPDATE_OBJECT CREATE block.
    let output2 = session.run_creatures_tick();

    // The CREATE block must be in the RuntimeOutput packets before flush.
    let found_create = output2.packets.iter().any(|bytes| {
        bytes.len() >= 2
            && u16::from_le_bytes([bytes[0], bytes[1]]) == ServerOpcodes::UpdateObject as u16
    });

    assert!(
        found_create,
        "respawn tick must produce SMSG_UPDATE_OBJECT CREATE in RuntimeOutput"
    );
}
#[test]
fn two_sessions_share_respawn_queue_drain_is_not_duplicated_like_cpp() {
    // Two sessions sharing the same Arc<RwLock<MapManager>>:
    // a respawn enqueued by session A must be drained exactly once —
    // a second drain (by session B) must return nothing.
    let manager = shared_map_manager();
    let (mut session_a, _, send_rx_a) = make_session();
    let (mut session_b, _, _send_rx_b) = make_session();

    session_a.set_map_manager(manager.clone());
    session_a.current_map_id = 0;
    session_b.set_map_manager(manager.clone());
    session_b.current_map_id = 0;

    let past = Instant::now() - Duration::from_secs(1);

    // Push one pending respawn via session A.
    let dummy_guid = test_creature_guid(91_003);
    let create_data = test_creature_create_data(dummy_guid, 9001, 10);
    let pending = crate::map_manager::PendingRespawn {
        respawn_at: past,
        spawn_id: dummy_guid.low_value() as u64,
        persistent_spawn: true,
        home_pos: Position::new(0.0, 0.0, 0.0, 0.0),
        create_data,
        max_hp: 10,
        level: 1,
        min_dmg: 1,
        max_dmg: 2,
        combat_log_stats: wow_entities::CreatureCombatLogStatsLikeCpp::default(),
        spell_hit_aura_source_authority_like_cpp: false,
        spell_cast_log_aura_source_authority_like_cpp: false,
        aggro_radius: 5.0,
        wander_distance: 0.0,
        flags_extra: 0,
        static_flags: [0; 8],
        ai_name: String::new(),
        script_name: String::new(),
        string_id: None,
        addon: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        default_movement_type: wow_entities::MovementGeneratorType::Idle,
        waypoint_path_id: 0,
        npc_flags: 0,
        unit_flags: 0,
        map_id: 0,
        loot_id: 0,
        skin_loot_id: 0,
        gold_min: 0,
        gold_max: 0,
        respawn_delay_secs: 30,
        selected_equipment_id: 0,
        original_equipment_id: 0,
        boss_id: None,
        dungeon_encounter_id: 0,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        terrain_swap_map: -1,
        phase_shift: PhaseShift::default(),
    };
    session_a.push_map_respawn_like_cpp(0, 0, pending);

    // Queue must have exactly 1 entry.
    {
        let mgr = manager.read().unwrap_or_else(|p| p.into_inner());
        assert_eq!(
            mgr.respawn_queue_len(0, 0),
            1,
            "queue must have 1 entry after push"
        );
    }

    // Session A drains — gets 1 entry.
    let drained_a = session_a.drain_ready_map_respawns_like_cpp(0, 0, Instant::now());
    assert_eq!(
        drained_a.len(),
        1,
        "session A must drain the 1 ready respawn"
    );

    // Queue is now empty.
    {
        let mgr = manager.read().unwrap_or_else(|p| p.into_inner());
        assert_eq!(
            mgr.respawn_queue_len(0, 0),
            0,
            "queue must be empty after drain"
        );
    }

    // Session B drains — must get nothing (no duplicates).
    let drained_b = session_b.drain_ready_map_respawns_like_cpp(0, 0, Instant::now());
    assert!(
        drained_b.is_empty(),
        "second drain by session B must return empty — no duplicate respawns"
    );

    // Channel must be empty (no side effects from push/drain helpers).
    assert!(
        send_rx_a.try_recv().is_err(),
        "helpers must not send packets"
    );
}
#[test]
fn step_creature_movement_random_should_wander_returns_monster_move_and_state_walking_random() {
    let guid = test_creature_guid(200_001);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    // Trigger random wander: delay=0, wander_radius=3.0, move_start_ms=0.
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
    }
    creature.seed_runtime_rng_like_cpp(0x5757);
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false, // disable pathfinding → simple straight-line spline
        ..Default::default()
    };

    let result =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);

    // Must return Some with a serialised MonsterMove packet.
    assert!(
        result.is_some(),
        "Random + should_wander must produce a MonsterMove packet"
    );
    let bytes = result.unwrap();
    // Opcode bytes [0..2] must equal OnMonsterMove.
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(
        opcode,
        wow_constants::ServerOpcodes::OnMonsterMove as u16,
        "packet opcode must be OnMonsterMove"
    );
    // State must have transitioned to WalkingRandom.
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::WalkingRandom,
        "state must be WalkingRandom after launching a wander spline"
    );
    assert_eq!(
        creature.runtime_motion_master_ticks_like_cpp(),
        1,
        "the map movement frame must call MotionMaster::Update once"
    );
}
#[test]
fn step_creature_movement_chase_priority_interrupts_active_random_spline_like_cpp() {
    let guid = test_creature_guid(200_013);
    let target = ObjectGuid::create_player(1, 20_013);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
    }
    creature.seed_runtime_rng_like_cpp(0x2013);
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };

    let first =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);
    assert!(first.is_some(), "precondition: random launches a spline");
    assert!(creature.active_move_spline_like_cpp().is_some());

    creature.enter_combat(target);
    let interrupted =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200)
            .expect("active chase must stop the lower-priority random spline");

    assert_eq!(
        u16::from_le_bytes([interrupted[0], interrupted[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(wow_movement::MovementGeneratorType::Chase)
    );
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), 2);
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::InCombat,
        "the inline random generator must not run below active chase"
    );
}
/// Chase must ask Detour for a route to its victim and walk *around* an
/// obstacle, mirroring C++ `ChaseMovementGenerator::Update`
/// (`ChaseMovementGenerator.cpp:154-236`), which builds a `PathGenerator`
/// for the victim and launches `init.MovebyPath(_path->GetPath())`.
#[test]
fn step_creature_movement_chase_paths_around_real_navmesh_obstacle_like_cpp() {
    use wow_recastdetour::test_fixtures::{
        OBSTACLE_TILE_CELL_SIZE, obstacle_hole_bounds, write_obstacle_ring_mmaps_like_cpp,
    };

    const MAP_ID: u32 = 1;
    let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
    // Same geometry as the waypoint end-to-end test: the creature and its
    // victim sit on opposite ring cells of the middle row, so the direct
    // segment crosses the unwalkable centre cell.
    let start = Position::new(half + OBSTACLE_TILE_CELL_SIZE, half, 0.0, 0.0);
    let victim_position = Position::new(
        half + OBSTACLE_TILE_CELL_SIZE,
        half + 2.0 * OBSTACLE_TILE_CELL_SIZE,
        0.0,
        0.0,
    );

    let root = std::env::temp_dir().join(format!(
        "rustycore-step-chase-obstacle-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_obstacle_ring_mmaps_like_cpp(&root, MAP_ID, &[(start.x, start.y)]);

    let guid = test_creature_guid(200_025);
    let victim_guid = test_creature_guid(200_026);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(MAP_ID, 0)
        .expect("bind the fixture map");
    creature.creature.set_ai_position(start);
    creature.creature.set_ai_home_position(start);
    creature.enter_combat(victim_guid);

    let worker = crate::map_manager::WorldMMapPathfinderWorkerLikeCpp::spawn(&root);
    let config = MMapRuntimeConfigLikeCpp {
        data_dir: root.display().to_string(),
        enabled: true,
        ..Default::default()
    };
    let target = crate::map_manager::ChaseTargetSnapshotLikeCpp {
        guid: victim_guid,
        position: victim_position,
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };

    let bytes = step_creature_movement_like_cpp(
        &mut creature,
        guid,
        &config,
        Some(&worker),
        None,
        Some(target),
        200,
    )
    .expect("chase must launch a MonsterMove toward the victim");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(wow_movement::MovementGeneratorType::Chase)
    );

    let spline = creature
        .active_move_spline_like_cpp()
        .expect("chase launched a spline");
    let points = spline.create_object_path_points_like_cpp();
    assert!(
        points.len() > 4,
        "chase must carry navmesh waypoints, not a straight line: {points:?}"
    );

    let (hole_detour_x, hole_detour_z) = obstacle_hole_bounds();
    for point in points.iter() {
        let inside_hole = hole_detour_z.contains(&point.x) && hole_detour_x.contains(&point.y);
        assert!(
            !inside_hole,
            "chase point {point:?} walks through the obstacle: {points:?}"
        );
    }
    assert!(
        points
            .iter()
            .any(|point| point.x < *hole_detour_z.start() || point.x > *hole_detour_z.end()),
        "the chase route never leaves the blocked row: {points:?}"
    );

    // C++ bails out of the whole tick without a spline when the path is
    // NOPATH; here it succeeded, so the creature must be marked as chasing.
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(wow_constants::UnitState::CHASE_MOVE.bits()),
        "a launched chase sets UNIT_STATE_CHASE_MOVE"
    );

    let _ = std::fs::remove_dir_all(&root);
}
/// Evade return must walk a navmesh route home instead of snapping there.
/// C++ `HomeMovementGenerator<Creature>::SetTargetLocation` launches
/// `init.MoveTo(GetHomePosition())` with `generatePath = true`
/// (`HomeMovementGenerator.cpp:60-82`).
#[test]
fn step_creature_movement_home_paths_around_real_navmesh_obstacle_like_cpp() {
    use wow_recastdetour::test_fixtures::{
        OBSTACLE_TILE_CELL_SIZE, obstacle_hole_bounds, write_obstacle_ring_mmaps_like_cpp,
    };

    const MAP_ID: u32 = 1;
    let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
    let home = Position::new(half + OBSTACLE_TILE_CELL_SIZE, half, 0.0, 0.0);
    let away = Position::new(
        half + OBSTACLE_TILE_CELL_SIZE,
        half + 2.0 * OBSTACLE_TILE_CELL_SIZE,
        0.0,
        0.0,
    );

    let root = std::env::temp_dir().join(format!(
        "rustycore-step-home-obstacle-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_obstacle_ring_mmaps_like_cpp(&root, MAP_ID, &[(home.x, home.y)]);

    let guid = test_creature_guid(200_027);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(MAP_ID, 0)
        .expect("bind the fixture map");
    // Home is across the obstacle from where the creature stands.
    creature.creature.set_ai_home_position(home);
    creature.creature.set_ai_position(away);
    creature
        .creature
        .set_ai_state(wow_entities::CreatureAiState::Returning);

    let worker = crate::map_manager::WorldMMapPathfinderWorkerLikeCpp::spawn(&root);
    let config = MMapRuntimeConfigLikeCpp {
        data_dir: root.display().to_string(),
        enabled: true,
        ..Default::default()
    };

    let bytes = step_creature_movement_like_cpp(
        &mut creature,
        guid,
        &config,
        Some(&worker),
        None,
        None,
        200,
    )
    .expect("evade return must launch a MonsterMove toward home");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );

    let spline = creature
        .active_move_spline_like_cpp()
        .expect("home launched a spline");
    let points = spline.create_object_path_points_like_cpp();
    assert!(
        points.len() > 4,
        "the return trip must be a navmesh route, not a teleport/straight line: {points:?}"
    );
    let (hole_detour_x, hole_detour_z) = obstacle_hole_bounds();
    for point in points.iter() {
        let inside_hole = hole_detour_z.contains(&point.x) && hole_detour_x.contains(&point.y);
        assert!(
            !inside_hole,
            "home point {point:?} crosses the obstacle: {points:?}"
        );
    }
    // C++ `SetTargetLocation` adds `UNIT_STATE_ROAMING_MOVE` before launching.
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(wow_constants::UnitState::ROAMING_MOVE.bits())
    );
    // The creature must still be returning: C++ only finalizes once the
    // spline reports finalized.
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::Returning,
        "the home generator stays alive until its spline finalizes"
    );

    let _ = std::fs::remove_dir_all(&root);
}
#[test]
fn step_creature_movement_idle_zero_wander_radius_stays_still_like_cpp() {
    let guid = test_creature_guid(200_011);
    let mut creature = make_test_world_creature(guid);
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 0.0;
    }
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };

    let result =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);

    assert!(
        result.is_none(),
        "C++ Creature::Create forces RANDOM_MOTION_TYPE to IDLE_MOTION_TYPE when m_wanderDistance is zero"
    );
    assert_eq!(creature.state(), wow_entities::CreatureAiState::Idle);
    assert!(creature.creature.ai_ownership().move_target.is_none());
}
#[test]
fn step_creature_movement_idle_positive_wander_radius_stays_still_like_cpp() {
    let guid = test_creature_guid(200_012);
    let mut creature = make_test_world_creature(guid);
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
    }
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };

    let result =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);

    assert!(
        result.is_none(),
        "C++ IdleMovementGenerator must not use RandomMovementGenerator wander distance"
    );
    assert_eq!(creature.state(), wow_entities::CreatureAiState::Idle);
    assert!(creature.creature.ai_ownership().move_target.is_none());
}
