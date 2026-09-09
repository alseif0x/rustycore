//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn gameobject_visual_despawn_shared_vision_phase_range_and_have_at_client_gates_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_533);
    let target_guid = ObjectGuid::create_player(1, 50_534);
    let incompatible_phase_guid = test_gameobject_guid(605_063, 50_535);
    let out_of_range_guid = test_gameobject_guid(605_064, 50_536);
    let not_visible_guid = test_gameobject_guid(605_065, 50_537);
    let sendable_guid = test_gameobject_guid(605_066, 50_538);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    add_canonical_test_player_on_map(
        &canonical,
        target_guid,
        Position::new(12.0, 22.0, 32.0, 0.0),
        571,
        7,
    );
    {
        let mut guard = canonical.lock().unwrap();
        *guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(target_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
    }
    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        target_guid,
        viewer_guid,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        incompatible_phase_guid,
        605_063,
        5_050_535,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        out_of_range_guid,
        605_064,
        5_050_536,
        Position::new(10_000.0, 20_000.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        not_visible_guid,
        605_065,
        5_050_537,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        sendable_guid,
        605_066,
        5_050_538,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .represented_gameobject_phase_shifts
        .insert(incompatible_phase_guid, PhaseShift::from_phases([20]));
    session
        .represented_gameobject_phase_shifts
        .insert(out_of_range_guid, PhaseShift::from_phases([10]));
    session
        .represented_gameobject_phase_shifts
        .insert(not_visible_guid, PhaseShift::from_phases([10]));
    session
        .represented_gameobject_phase_shifts
        .insert(sendable_guid, PhaseShift::from_phases([10]));
    session
        .client_visible_guids_like_cpp
        .insert(incompatible_phase_guid);
    session
        .client_visible_guids_like_cpp
        .insert(out_of_range_guid);
    session.client_visible_guids_like_cpp.insert(sendable_guid);
    session.represented_seer_guid_like_cpp = Some(target_guid);

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        1
    );

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert_eq!(
        session
            .represented_gameobject_visual_despawns_delivered_like_cpp
            .len(),
        1
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&incompatible_phase_guid)
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&out_of_range_guid)
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&not_visible_guid)
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&sendable_guid)
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_direct_blocked_until_shared_vision_qualifies_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_539);
    let target_guid = ObjectGuid::create_player(1, 50_540);
    let dynamic_seer_guid = test_dynamic_object_guid(605_067, 50_541);
    let gameobject_guid = test_gameobject_guid(605_068, 50_542);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    add_canonical_test_player_on_map(
        &canonical,
        target_guid,
        Position::new(12.0, 22.0, 32.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_068,
        5_050_542,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(dynamic_seer_guid);

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .represented_gameobject_visual_despawns_delivered_like_cpp
            .is_empty()
    );

    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        target_guid,
        viewer_guid,
    );
    session.represented_seer_guid_like_cpp = Some(target_guid);
    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_mismatched_seer_with_vehicle_sends_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_520);
    let gameobject_guid = test_gameobject_guid(605_058, 50_521);
    let seer_guid = test_dynamic_object_guid(605_059, 50_522);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_058,
        5_050_521,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(seer_guid);
    let mut vehicle_kit = Vehicle::new(
        player_guid,
        TypeId::Player,
        Position::new(10.0, 20.0, 30.0, 0.0),
        77,
        88,
        std::iter::empty(),
    );
    vehicle_kit.install();
    session.player_mount_vehicle_kit_like_cpp = Some(vehicle_kit);

    session.process_pending().await;

    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::GameObjectDespawn));
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_out_of_range_keeps_client_visible_guid_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_511);
    let gameobject_guid = test_gameobject_guid(605_053, 50_512);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_053,
        5_050_512,
        Position::new(10_000.0, 20_000.0, 30.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 7)
            .unwrap()
            .last_game_objects_update_summary()
            .generic_visual_despawn_guids
            .as_slice(),
        &[gameobject_guid]
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visibility_on_destroy_out_of_range_keeps_client_visible_guid_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_507);
    let gameobject_guid = test_gameobject_guid(605_051, 50_508);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_visibility_on_destroy_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_051,
        5_050_508,
        Position::new(10_000.0, 20_000.0, 30.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 7)
            .unwrap()
            .last_game_objects_update_summary()
            .generic_visibility_on_destroy_guids
            .as_slice(),
        &[gameobject_guid]
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visibility_on_destroy_vertical_only_separation_sends_destroy_like_cpp() {
    // C++ visibility distance is 2D (CanSeeOrDetect -> IsWithinDist is3D=false;
    // Object.cpp:1587-1609). A GameObject at the player's exact X/Y but a large Z offset
    // is within the 2D sight range even though its 3D distance exceeds it. The is3D=false
    // flag on the destroy-fanout distance check sends the destroy like C++; a 3D check
    // would wrongly keep it client-visible.
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_540);
    let gameobject_guid = test_gameobject_guid(605_090, 50_541);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    let visibility_range = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .map()
        .visibility_range();
    // Same X/Y as the player (10, 20); Z far enough above that the 3D distance exceeds
    // the sight range (plus combat reach) while the 2D distance stays 0.
    add_canonical_visibility_on_destroy_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_090,
        5_050_541,
        Position::new(10.0, 20.0, 30.0 + visibility_range + 100.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 7)
            .unwrap()
            .last_game_objects_update_summary()
            .generic_visibility_on_destroy_guids
            .as_slice(),
        &[gameobject_guid]
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[test]
fn gameobject_interaction_resolves_canonical_map_object_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(777, 9);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        2,
        1,
        10,
        0,
    ));

    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(gameobject_guid);
    gameobject.world_mut().object_mut().set_entry(777);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::new(14.9, 0.0, 0.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();

    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert_eq!(
        session.represented_gameobject_can_interact_with_like_cpp(gameobject_guid, 5.0),
        Some(RepresentedGameObjectAccessLikeCpp {
            entry: 777,
            position: Position::new(14.9, 0.0, 0.0, 0.0),
        })
    );

    session.set_player_position_like_cpp(Position::new(20.1, 0.0, 0.0, 0.0));
    assert_eq!(
        session.represented_gameobject_can_interact_with_like_cpp(gameobject_guid, 5.0),
        None
    );
}
#[test]
fn visible_world_creatures_use_map_grid_and_phase_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 26_820);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let visible_guid = test_creature_guid(20);
    let far_guid = test_creature_guid(21);
    let visible_pos = Position::new(20.0, 20.0, 0.0, 0.0);
    let far_pos = Position::new(5000.0, 5000.0, 0.0, 0.0);

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VisibilityOwner".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical viewer map");
    for (guid, position) in [(visible_guid, visible_pos), (far_guid, far_pos)] {
        let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        manager.write().unwrap().add_creature(
            571,
            0,
            grid_x,
            grid_y,
            crate::map_manager::WorldCreature::new(
                guid, 900, position, 100, 80, 1, 2, 0.0, 1, 35, 0, 0,
            ),
        );
    }

    let visible = session.visible_world_creatures_from_map_like_cpp(571, &player_position);

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid(), visible_guid);
}
#[test]
fn visible_world_creatures_prefer_legacy_runtime_duplicate_with_active_spline_like_cpp() {
    use wow_constants::movement::MovementFlag;

    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 76_010);
    let creature_guid = test_creature_guid(76_011);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let creature_position = Position::new(20.0, 20.0, 0.0, 0.0);

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VisibleCreatureViewer".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical viewer map");
    add_canonical_test_creature_on_map_with_world_state(
        &canonical,
        creature_guid,
        76_011,
        creature_position,
        0,
        571,
        0,
        true,
    );

    let mut legacy_creature = crate::map_manager::WorldCreature::new(
        creature_guid,
        76_011,
        creature_position,
        100,
        12,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
    );
    legacy_creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    legacy_creature
        .creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    legacy_creature
        .begin_move_spline_like_cpp(Position::new(24.0, 20.0, 0.0, 0.0))
        .expect("legacy runtime spline must launch");
    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(creature_position.x, creature_position.y);
    manager
        .write()
        .unwrap()
        .add_creature(571, 0, grid_x, grid_y, legacy_creature);

    let visible = session.visible_world_creatures_from_map_like_cpp(571, &player_position);

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid(), creature_guid);
    assert!(
        visible[0].active_move_spline_like_cpp().is_some(),
        "C++ has one map Creature; Rust visibility must keep the legacy active MoveSpline for duplicate canonical/legacy GUIDs"
    );
    assert!(
        MovementFlag::from_bits_retain(visible[0].create_data.movement_flags)
            .contains(MovementFlag::FORWARD),
        "active legacy spline should carry C++ MoveSplineInit::Launch movement flags into CREATE"
    );
}
#[test]
fn active_creature_update_guids_use_cpp_cell_activation_area() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let active_guid = test_creature_guid(40);
    let inactive_guid = test_creature_guid(41);

    session.set_map_manager(Arc::clone(&manager));
    session.set_player_map_position_like_cpp(571, Position::new(0.0, 0.0, 0.0, 0.0));

    for (guid, position) in [
        (active_guid, Position::new(40.0, 40.0, 0.0, 0.0)),
        (inactive_guid, Position::new(260.0, 260.0, 0.0, 0.0)),
    ] {
        let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        manager.write().unwrap().add_creature(
            571,
            0,
            grid_x,
            grid_y,
            crate::map_manager::WorldCreature::new(
                guid, 900, position, 100, 80, 1, 2, 0.0, 1, 35, 0, 0,
            ),
        );
    }

    let active = session.active_world_creature_guids_for_update_like_cpp();

    assert_eq!(active, vec![active_guid]);
}
#[test]
fn creature_message_to_set_gate_requires_have_at_client_phase_map_and_range_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let guid = test_creature_guid(9301);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let visible_position = Position::new(20.0, 20.0, 0.0, 0.0);
    let far_position = Position::new(5000.0, 5000.0, 0.0, 0.0);

    session.set_player_map_position_like_cpp(571, player_position);
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));

    let mut creature = crate::map_manager::WorldCreature::new(
        guid,
        900,
        visible_position,
        100,
        80,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
    );
    let _ = creature.creature.unit_mut().world_mut().set_map(571, 0);
    creature
        .creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    *creature.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);

    assert!(
        !session.represented_can_receive_creature_message_to_set_like_cpp(guid, &creature, false),
        "C++ MessageDistDeliverer::SendPacket requires HaveAtClient"
    );

    session.client_visible_guids_like_cpp.insert(guid);
    assert!(
        session.represented_can_receive_creature_message_to_set_like_cpp(guid, &creature, false),
        "same phase + in range + HaveAtClient must receive"
    );

    let mut wrong_phase = creature.clone();
    *wrong_phase
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut() = PhaseShift::from_phases([20]);
    assert!(
        !session.represented_can_receive_creature_message_to_set_like_cpp(
            guid,
            &wrong_phase,
            false
        ),
        "MessageDistDeliverer rejects players outside source phase"
    );

    let mut far_creature = creature.clone();
    far_creature.creature.set_ai_position(far_position);
    assert!(
        !session.represented_can_receive_creature_message_to_set_like_cpp(
            guid,
            &far_creature,
            false
        ),
        "MessageDistDeliverer applies source GetVisibilityRange distance"
    );

    let mut wrong_map = crate::map_manager::WorldCreature::new(
        test_creature_guid(9302),
        900,
        visible_position,
        100,
        80,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
    );
    let _ = wrong_map.creature.unit_mut().world_mut().set_map(530, 0);
    wrong_map
        .creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    *wrong_map.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
    session
        .client_visible_guids_like_cpp
        .insert(wrong_map.guid());
    assert!(
        !session.represented_can_receive_creature_message_to_set_like_cpp(
            wrong_map.guid(),
            &wrong_map,
            false
        ),
        "Cell::VisitWorldObjects cannot deliver across maps"
    );
}
#[test]
fn visible_creatures_skip_not_in_world_canonical_objects_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 9303);
    let in_world_guid = test_creature_guid(9304);
    let removed_guid = test_creature_guid(9305);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VisibleCreatureViewer".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        in_world_guid,
        9304,
        Position::new(20.0, 20.0, 0.0, 0.0),
        571,
        0,
        80,
    );
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        removed_guid,
        9305,
        Position::new(21.0, 21.0, 0.0, 0.0),
        571,
        0,
        80,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(571, 0).unwrap().map_mut();
        *map.get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
        *map.get_typed_creature_mut(in_world_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
        map.get_typed_creature_mut(in_world_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        *map.get_typed_creature_mut(removed_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
        map.get_typed_creature_mut(removed_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .remove_from_world();
    }

    let visible = session
        .visible_creatures_from_canonical_map_like_cpp(571, &player_position)
        .expect("canonical map");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid(), in_world_guid);
}
#[test]
fn visible_gameobjects_use_canonical_map_cells_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let visible_guid = test_gameobject_guid(910, 30);
    let far_guid = test_gameobject_guid(911, 31);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    canonical.lock().unwrap().create_world_map(571, 0);

    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        visible_guid,
        910,
        Position::new(20.0, 20.0, 0.0, 0.0),
        3,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        visible_guid,
        7000,
        1.5,
        [0.0, 0.0, 0.0, 1.0],
    );
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        far_guid,
        911,
        Position::new(5000.0, 5000.0, 0.0, 0.0),
        3,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        far_guid,
        7001,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
    );

    let visible = session
        .visible_gameobjects_from_canonical_map_like_cpp(571, &player_position, 800.0)
        .expect("canonical map");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, visible_guid);
    assert_eq!(visible[0].entry, 910);
    assert_eq!(visible[0].display_id, 7000);
    assert_eq!(visible[0].go_type, 3);
    assert_eq!(visible[0].scale, 1.5);
}
