//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn gameobject_visual_despawn_creature_shared_vision_out_of_world_target_no_send_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_562);
    let target_guid = test_creature_guid(50_563);
    let gameobject_guid = test_gameobject_guid(605_077, 50_564);

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
        605_077,
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
        605_077,
        5_050_564,
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
            .get_typed_creature_mut(target_guid)
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
            .get_typed_creature_mut(target_guid)
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
async fn gameobject_visual_despawn_creature_shared_vision_requires_session_seer_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_546);
    let target_guid = test_creature_guid(50_547);
    let other_seer_guid = test_creature_guid(50_548);
    let gameobject_guid = test_gameobject_guid(605_070, 50_549);

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
        605_070,
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
        605_070,
        5_050_549,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(other_seer_guid);

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
}
#[tokio::test]
async fn gameobject_visual_despawn_creature_shared_vision_requires_target_list_viewer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_550);
    let target_guid = test_creature_guid(50_551);
    let gameobject_guid = test_gameobject_guid(605_071, 50_552);

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
        605_071,
        Position::new(12.0, 22.0, 32.0, 0.0),
        0,
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_071,
        5_050_552,
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
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .represented_gameobject_visual_despawns_delivered_like_cpp
            .is_empty()
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_creature_shared_vision_phase_range_and_have_at_client_gates_like_cpp()
 {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_553);
    let target_guid = test_creature_guid(50_554);
    let incompatible_phase_guid = test_gameobject_guid(605_072, 50_555);
    let out_of_range_guid = test_gameobject_guid(605_073, 50_556);
    let not_visible_guid = test_gameobject_guid(605_074, 50_557);
    let sendable_guid = test_gameobject_guid(605_075, 50_558);

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
        605_075,
        Position::new(12.0, 22.0, 32.0, 0.0),
        0,
        571,
        7,
    );
    {
        let mut guard = canonical.lock().unwrap();
        *guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(target_guid)
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
        605_072,
        5_050_555,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        out_of_range_guid,
        605_073,
        5_050_556,
        Position::new(10_000.0, 20_000.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        not_visible_guid,
        605_074,
        5_050_557,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        sendable_guid,
        605_075,
        5_050_558,
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
async fn gameobject_visual_despawn_dynamic_object_caster_viewer_receives_once_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_610);
    let dynamic_object_guid = test_dynamic_object_guid(605_110, 50_611);
    let gameobject_guid = test_gameobject_guid(605_111, 50_612);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_object_guid,
        viewer_guid,
        605_110,
        Position::new(12.0, 22.0, 32.0, 0.0),
        571,
        7,
    );
    configure_test_dynamic_object_for_visual_despawn_like_cpp(
        &canonical,
        571,
        7,
        dynamic_object_guid,
        120_000,
        Some(viewer_guid),
        None,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_111,
        5_050_612,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(dynamic_object_guid);

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
    assert_eq!(
        session
            .represented_gameobject_visual_despawns_delivered_like_cpp
            .len(),
        1
    );
}
#[tokio::test]
async fn gameobject_visual_despawn_dynamic_object_requires_session_seer_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_613);
    let dynamic_object_guid = test_dynamic_object_guid(605_112, 50_614);
    let other_seer_guid = test_dynamic_object_guid(605_113, 50_615);
    let gameobject_guid = test_gameobject_guid(605_114, 50_616);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_object_guid,
        viewer_guid,
        605_112,
        Position::new(12.0, 22.0, 32.0, 0.0),
        571,
        7,
    );
    configure_test_dynamic_object_for_visual_despawn_like_cpp(
        &canonical,
        571,
        7,
        dynamic_object_guid,
        120_000,
        None,
        None,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_114,
        5_050_616,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(other_seer_guid);

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
}
#[tokio::test]
async fn gameobject_visual_despawn_dynamic_object_requires_player_caster_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_617);
    let dynamic_object_guid = test_dynamic_object_guid(605_115, 50_618);
    let gameobject_guid = test_gameobject_guid(605_116, 50_619);
    let creature_caster_guid = test_creature_guid(50_620);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_object_guid,
        ObjectGuid::EMPTY,
        605_115,
        Position::new(12.0, 22.0, 32.0, 0.0),
        571,
        7,
    );
    configure_test_dynamic_object_for_visual_despawn_like_cpp(
        &canonical,
        571,
        7,
        dynamic_object_guid,
        120_000,
        None,
        None,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_116,
        5_050_619,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(dynamic_object_guid);

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

    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_dynamic_object_mut(dynamic_object_guid)
            .unwrap()
            .set_caster_guid(creature_caster_guid);
    }
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
}
#[tokio::test]
async fn gameobject_visual_despawn_dynamic_object_phase_range_and_have_at_client_gates_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_621);
    let dynamic_object_guid = test_dynamic_object_guid(605_117, 50_622);
    let incompatible_phase_guid = test_gameobject_guid(605_118, 50_623);
    let out_of_range_guid = test_gameobject_guid(605_119, 50_624);
    let not_visible_guid = test_gameobject_guid(605_120, 50_625);
    let sendable_guid = test_gameobject_guid(605_121, 50_626);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_object_guid,
        viewer_guid,
        605_117,
        Position::new(12.0, 22.0, 32.0, 0.0),
        571,
        7,
    );
    configure_test_dynamic_object_for_visual_despawn_like_cpp(
        &canonical,
        571,
        7,
        dynamic_object_guid,
        120_000,
        Some(viewer_guid),
        Some(PhaseShift::from_phases([10])),
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        incompatible_phase_guid,
        605_118,
        5_050_623,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        out_of_range_guid,
        605_119,
        5_050_624,
        Position::new(1_000.0, 1_000.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        not_visible_guid,
        605_120,
        5_050_625,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    add_canonical_visual_despawn_gameobject_like_cpp(
        &canonical,
        sendable_guid,
        605_121,
        5_050_626,
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
    session.represented_seer_guid_like_cpp = Some(dynamic_object_guid);

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
async fn gameobject_visual_despawn_shared_vision_requires_session_seer_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_526);
    let target_guid = ObjectGuid::create_player(1, 50_527);
    let other_seer_guid = ObjectGuid::create_player(1, 50_528);
    let gameobject_guid = test_gameobject_guid(605_061, 50_529);

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
        605_061,
        5_050_529,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_seer_guid_like_cpp = Some(other_seer_guid);

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
}
#[tokio::test]
async fn gameobject_visual_despawn_shared_vision_requires_target_list_viewer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_530);
    let target_guid = ObjectGuid::create_player(1, 50_531);
    let gameobject_guid = test_gameobject_guid(605_062, 50_532);

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
        605_062,
        5_050_532,
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
        0
    );

    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .represented_gameobject_visual_despawns_delivered_like_cpp
            .is_empty()
    );
}
