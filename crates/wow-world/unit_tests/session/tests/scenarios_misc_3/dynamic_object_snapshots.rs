use super::*;

#[tokio::test]
async fn dynamic_object_values_snapshot_direct_player_vertical_only_separation_sends_like_cpp() {
    // C++ visibility distance is 2D (CanSeeOrDetect -> GetSightRange ->
    // IsWithinDist(obj, range, is3D=false); Object.cpp:1587-1609). A dynamic object at
    // the player's exact X/Y but a large Z offset is within the 2D sight range even
    // though its 3D distance exceeds it. A 3D check would wrongly drop it (the bug this
    // fixes); the 2D check sends the values update like C++.
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_530);
    let dynamic_guid = test_dynamic_object_guid(601_530, 50_531);

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
    // the sight range while the 2D distance stays 0.
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601_530,
        Position::new(10.0, 20.0, 30.0 + visibility_range + 25.0, 0.0),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 38.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
}

#[tokio::test]
async fn dynamic_object_values_snapshot_player_shared_vision_no_seer_gate_sends_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_526);
    let source_guid = ObjectGuid::create_player(1, 50_527);
    let dynamic_guid = test_dynamic_object_guid(601_526, 50_528);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
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
    let updated_position = Position::new(10.0 + visibility_range + 25.0, 20.0, 30.0, 0.0);
    add_canonical_test_player_on_map(
        &canonical,
        source_guid,
        Position::new(
            updated_position.x + 1.0,
            updated_position.y,
            updated_position.z,
            0.0,
        ),
        571,
        7,
    );
    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        source_guid,
        viewer_guid,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        source_guid,
        601_526,
        updated_position,
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 41.5);
    session
        .visibility_test_fixture_like_cpp
        .represented_seer_guid_like_cpp = Some(viewer_guid);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&dynamic_guid)
    );
    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        0
    );
}

#[tokio::test]
async fn dynamic_object_values_snapshot_shared_vision_requires_source_lists_viewer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_532);
    let source_guid = ObjectGuid::create_player(1, 50_533);
    let dynamic_guid = test_dynamic_object_guid(601_532, 50_534);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
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
    let updated_position = Position::new(10.0 + visibility_range + 25.0, 20.0, 30.0, 0.0);
    add_canonical_test_player_on_map(
        &canonical,
        source_guid,
        Position::new(
            updated_position.x + 1.0,
            updated_position.y,
            updated_position.z,
            0.0,
        ),
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        source_guid,
        601_532,
        updated_position,
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 43.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&dynamic_guid)
    );
}

#[tokio::test]
async fn dynamic_object_values_snapshot_repeated_process_pending_does_not_resend_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_421);
    let dynamic_guid = test_dynamic_object_guid(601021, 50_422);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601021,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 37.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    session.process_pending().await;
    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
}

#[tokio::test]
async fn dynamic_object_values_snapshot_later_same_bytes_resends_on_new_generation_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_441);
    let dynamic_guid = test_dynamic_object_guid(601041, 50_442);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601041,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 37.5);
    let first_generation = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .update_calls()
        .len();
    session.process_pending().await;
    session.process_pending().await;
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject],
        "same stable snapshot/generation must be consumed only once"
    );

    // Force a real value transition, then transition back to the original
    // radius before session consumption. The latest stable snapshot has the
    // same VALUES bytes as the first send, but a later `Map::Update`
    // generation, matching C++ per-drain delivery semantics.
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 38.5);
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 37.5);
    let second_generation = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .update_calls()
        .len();
    assert!(
        second_generation > first_generation,
        "test must exercise a later ManagedMap update generation"
    );

    session.process_pending().await;
    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject],
        "identical DynamicObject VALUES bytes from a later map update generation must resend once"
    );
}
