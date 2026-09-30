//! Original world-entity scenarios; inputs, bodies and assertions retained.

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
    session
        .visibility_test_fixture_like_cpp
        .represented_seer_guid_like_cpp = Some(target_guid);

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
            .visibility_publication.represented_gameobject_visual_despawns_delivered_like_cpp
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
    session
        .visibility_test_fixture_like_cpp
        .represented_seer_guid_like_cpp = Some(dynamic_seer_guid);

    assert_eq!(
        session.send_represented_gameobject_visual_despawn_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .visibility_publication.represented_gameobject_visual_despawns_delivered_like_cpp
            .is_empty()
    );

    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        target_guid,
        viewer_guid,
    );
    session
        .visibility_test_fixture_like_cpp
        .represented_seer_guid_like_cpp = Some(target_guid);
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
    session
        .visibility_test_fixture_like_cpp
        .represented_seer_guid_like_cpp = Some(seer_guid);
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
