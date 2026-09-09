//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn gameobject_visibility_on_destroy_uses_combat_reach_range_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_559);
    let gameobject_guid = test_gameobject_guid(605_059, 50_560);

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
    add_canonical_visibility_on_destroy_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_059,
        5_050_560,
        Position::new(10.0 + visibility_range + 2.0, 20.0, 30.0, 0.0),
        571,
        7,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(571, 7).unwrap().map_mut();
        map.get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .set_combat_reach(2.0);
        map.get_typed_game_object_mut(gameobject_guid)
            .unwrap()
            .world_mut()
            .set_combat_reach(2.0);
    }
    {
        let guard = canonical.lock().unwrap();
        let map = guard.find_map(571, 7).unwrap().map();
        let player_world = map.get_typed_player(player_guid).unwrap().unit().world();
        let gameobject_world = map.get_typed_game_object(gameobject_guid).unwrap().world();

        assert!(
            !gameobject_world
                .position()
                .is_within_dist(&player_world.position(), visibility_range)
        );
        assert!(gameobject_world.is_within_dist(player_world, visibility_range, true, true, true));
    }
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
#[tokio::test]
async fn gameobject_visibility_on_destroy_same_phase_sends_destroy_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_561);
    let gameobject_guid = test_gameobject_guid(605_061, 50_562);

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
        605_061,
        5_050_562,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    {
        let mut guard = canonical.lock().unwrap();
        *guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
    }
    session.record_represented_gameobject_phase_shift_like_cpp(
        gameobject_guid,
        PhaseShift::from_phases([10]),
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    assert_eq!(
        session.send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(),
        1
    );

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
#[tokio::test]
async fn gameobject_visibility_on_destroy_incompatible_phase_keeps_client_visible_guid_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_563);
    let gameobject_guid = test_gameobject_guid(605_063, 50_564);

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
        605_063,
        5_050_564,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    {
        let mut guard = canonical.lock().unwrap();
        *guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
    }
    session.record_represented_gameobject_phase_shift_like_cpp(
        gameobject_guid,
        PhaseShift::from_phases([20]),
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
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

    session.record_represented_gameobject_phase_shift_like_cpp(
        gameobject_guid,
        PhaseShift::from_phases([10]),
    );
    assert_eq!(
        session.send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(),
        1
    );
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
#[tokio::test]
async fn gameobject_visual_despawn_summary_visible_sends_despawn_once_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_509);
    let gameobject_guid = test_gameobject_guid(605_052, 50_510);

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
        605_052,
        5_050_510,
        Position::new(11.0, 21.0, 31.0, 0.0),
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

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );

    session.process_pending().await;
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
}
#[tokio::test]
async fn capture_point_delete_sends_removed_before_despawn_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_572);
    let gameobject_guid = test_gameobject_guid(605_072, 50_573);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_capture_point_delete_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_072,
        5_050_573,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    let summary = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .last_game_objects_update_summary();
    assert_eq!(
        summary.generic_capture_point_removed_guids.as_slice(),
        &[gameobject_guid]
    );
    assert_eq!(
        summary.generic_visual_despawn_guids.as_slice(),
        &[gameobject_guid]
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateCapturePoint,
            ServerOpcodes::GameObjectDespawn
        ]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );

    session.process_pending().await;
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
}
#[tokio::test]
async fn gameobject_visual_despawn_not_in_world_player_no_send_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_569);
    let gameobject_guid = test_gameobject_guid(605_071, 50_570);

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
        605_071,
        5_050_570,
        Position::new(11.0, 21.0, 31.0, 0.0),
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
    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .remove_from_world();
    }
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    assert!(
        session
            .represented_gameobject_visual_despawns_delivered_like_cpp
            .is_empty()
    );

    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
    }
    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_uses_2d_visibility_range_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_513);
    let gameobject_guid = test_gameobject_guid(605_054, 50_514);
    let player_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let gameobject_position = Position::new(11.0, 21.0, 10_000.0, 0.0);

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
    assert!(!gameobject_position.is_within_dist(&player_position, visibility_range));
    assert!(gameobject_position.is_within_dist_2d(&player_position, visibility_range));
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_054,
        5_050_514,
        gameobject_position,
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

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_incompatible_phase_keeps_client_visible_guid_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_515);
    let gameobject_guid = test_gameobject_guid(605_055, 50_516);

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
        605_055,
        5_050_516,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    {
        let mut guard = canonical.lock().unwrap();
        *guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
    }
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
        .represented_gameobject_phase_shifts
        .insert(gameobject_guid, PhaseShift::from_phases([20]));
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

    session
        .represented_gameobject_phase_shifts
        .insert(gameobject_guid, PhaseShift::from_phases([10]));
    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_mismatched_seer_without_vehicle_preserves_delivery_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_517);
    let gameobject_guid = test_gameobject_guid(605_056, 50_518);
    let seer_guid = test_dynamic_object_guid(605_057, 50_519);

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
        605_056,
        5_050_518,
        Position::new(11.0, 21.0, 31.0, 0.0),
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
    session.represented_seer_guid_like_cpp = Some(seer_guid);

    session.process_pending().await;

    let opcodes = drain_server_opcodes(&send_rx);
    assert!(!opcodes.contains(&ServerOpcodes::GameObjectDespawn));
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    assert!(
        session
            .represented_gameobject_visual_despawns_delivered_like_cpp
            .is_empty()
    );

    session.represented_seer_guid_like_cpp = Some(player_guid);
    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_shared_vision_viewer_receives_once_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_523);
    let target_guid = ObjectGuid::create_player(1, 50_524);
    let gameobject_guid = test_gameobject_guid(605_060, 50_525);

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
    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        target_guid,
        viewer_guid,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_060,
        5_050_525,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(target_guid);

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        1
    );

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
}
#[tokio::test]
async fn gameobject_visual_despawn_player_shared_vision_out_of_world_target_no_send_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_559);
    let target_guid = ObjectGuid::create_player(1, 50_560);
    let gameobject_guid = test_gameobject_guid(605_076, 50_561);

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
    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        target_guid,
        viewer_guid,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_076,
        5_050_561,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(target_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .remove_from_world();
    }
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(target_guid);

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    assert!(
        session
            .represented_gameobject_visual_despawns_delivered_like_cpp
            .is_empty()
    );

    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(target_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
    }
    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_creature_shared_vision_viewer_receives_once_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_543);
    let target_guid = test_creature_guid(50_544);
    let gameobject_guid = test_gameobject_guid(605_069, 50_545);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        target_guid,
        605_069,
        Position::new(12.0, 22.0, 32.0, 0.0),
        0,
        571,
        7,
    );
    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        target_guid,
        viewer_guid,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_069,
        5_050_545,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(target_guid);

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
}
