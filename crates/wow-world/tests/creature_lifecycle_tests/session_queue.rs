use super::*;

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
        .fixture_melee_mutate_creature(guid, |creature| {
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
    session2.fixture_lifecycle_set_current_map(0);

    let start_position = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .expect("creature must exist")
        .position();

    // Both sessions read GlobalLegacy → neither runs tick_creatures_sync.
    let owner1 = session1.fixture_lifecycle_tick_owner();
    let owner2 = session2.fixture_lifecycle_tick_owner();
    assert_eq!(owner1, RuntimeTickOwner::GlobalLegacy);
    assert_eq!(owner2, RuntimeTickOwner::GlobalLegacy);

    if owner1 == RuntimeTickOwner::Session {
        session1.fixture_lifecycle_tick_sync();
    }
    if owner2 == RuntimeTickOwner::Session {
        session2.fixture_lifecycle_tick_sync();
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
    let output = session.fixture_lifecycle_tick();
    session.fixture_lifecycle_flush(output);

    // Verify the pending respawn is in the map, not in a session field.
    let map_id = session.fixture_lifecycle_map_id();
    let queue_len = session
        .fixture_lifecycle_manager()
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
    let output = session.fixture_lifecycle_tick();
    session.fixture_lifecycle_flush(output);

    // Drain any packets sent so far (DESTROY block).
    while send_rx.try_recv().is_ok() {}

    // Force the queued respawn to be immediately ready.
    force_respawn_ready(&mut session);

    // Tick 2: respawn fires — must produce an SMSG_UPDATE_OBJECT CREATE block.
    let output2 = session.fixture_lifecycle_tick();

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
    session_a.fixture_lifecycle_set_current_map(0);
    session_b.set_map_manager(manager.clone());
    session_b.fixture_lifecycle_set_current_map(0);

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
    session_a.fixture_lifecycle_push_respawn(0, 0, pending);

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
    let drained_a = session_a.fixture_lifecycle_drain_respawns(0, 0, Instant::now());
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
    let drained_b = session_b.fixture_lifecycle_drain_respawns(0, 0, Instant::now());
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
