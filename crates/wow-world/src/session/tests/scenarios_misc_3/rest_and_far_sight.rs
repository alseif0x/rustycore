use super::*;

#[tokio::test]
async fn post_add_flushes_deferred_rest_flag_update_after_world_states_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 74_334);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PostAddRest".to_string(),
        player_position,
        571,
        1,
        1,
        10,
        0,
    ));
    session.set_player_zone_area_like_cpp(20, 102);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 20,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP,
        },
        wow_data::AreaTableEntry {
            id: 102,
            continent_id: 571,
            parent_area_id: 20,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_FACTION_AREA_LIKE_CPP, 0));
    let _ = drain_server_packet_bytes(&send_rx);

    session
        .send_initial_packets_after_add_to_map(player_guid, &player_position, 571, false)
        .await;

    let opcodes: Vec<_> = drain_server_packet_bytes(&send_rx)
        .iter()
        .filter_map(|packet| {
            (packet.len() >= 2).then(|| u16::from_le_bytes([packet[0], packet[1]]))
        })
        .collect();
    let init_world_states_index = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::InitWorldStates as u16)
        .expect("post-add InitWorldStates");
    let rest_update_indices: Vec<_> = opcodes
        .iter()
        .enumerate()
        .filter_map(|(index, opcode)| {
            (*opcode == ServerOpcodes::UpdateObject as u16).then_some(index)
        })
        .collect();
    assert_eq!(
        rest_update_indices.len(),
        1,
        "the deferred zone rest transition flushes as one final PlayerFlags update"
    );
    assert!(rest_update_indices[0] > init_world_states_index);
    assert_eq!(
        session
            .rest_mgr_test_fixture_like_cpp
            .represented_rest_flag_mask_like_cpp,
        0
    );
}

#[tokio::test]
async fn far_sight_process_pending_canonical_clear_resets_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_906);
    let stale_dynamic_object_guid = test_dynamic_object_guid(49_907, 49_907);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_state(SessionState::LoggedIn);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightProcessPendingClear".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .visibility_test_fixture_like_cpp
        .represented_seer_guid_like_cpp = Some(stale_dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.process_pending().await;

    assert_eq!(session.represented_seer_guid_like_cpp(), Some(player_guid));
    assert_eq!(
        session.last_visibility_pos, None,
        "live tick consumption should invalidate the visibility throttle without requiring update_visibility"
    );
    let expected_farsight_clear = expected_active_player_farsight_object_values_update_like_cpp(
        player_guid,
        session.player_map_id_like_cpp(),
        ObjectGuid::EMPTY,
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .filter(|bytes| *bytes == &expected_farsight_clear)
            .count(),
        1,
        "logged-in process_pending should emit exactly one represented VALUES-empty update"
    );
    assert_eq!(
        update_object_packet_count_like_cpp(&packets),
        1,
        "process_pending should not need update_visibility to consume canonical farsight clear"
    );
}

#[tokio::test]
async fn far_sight_process_pending_non_logged_in_keeps_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_908);
    let stale_dynamic_object_guid = test_dynamic_object_guid(49_909, 49_909);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightProcessPendingAuthed".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .visibility_test_fixture_like_cpp
        .represented_seer_guid_like_cpp = Some(stale_dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.process_pending().await;

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(stale_dynamic_object_guid),
        "non-logged-in process_pending must not consume canonical farsight clear"
    );
    assert_eq!(session.last_visibility_pos, Some(player_position));
    let expected_farsight_clear = expected_active_player_farsight_object_values_update_like_cpp(
        player_guid,
        session.player_map_id_like_cpp(),
        ObjectGuid::EMPTY,
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        !packets
            .iter()
            .any(|bytes| bytes == &expected_farsight_clear),
        "non-logged-in process_pending must not emit represented VALUES-empty update"
    );
}

#[tokio::test]
async fn far_sight_process_pending_non_empty_canonical_keeps_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_910);
    let dynamic_object_guid = test_dynamic_object_guid(49_911, 49_911);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_state(SessionState::LoggedIn);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightProcessPendingKeep".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    set_canonical_player_farsight_object_like_cpp(&canonical, player_guid, dynamic_object_guid);
    session
        .visibility_test_fixture_like_cpp
        .represented_seer_guid_like_cpp = Some(dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.process_pending().await;

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(dynamic_object_guid),
        "non-empty canonical FarsightObject must preserve represented m_seer"
    );
    assert_eq!(session.last_visibility_pos, Some(player_position));
    let expected_farsight_clear = expected_active_player_farsight_object_values_update_like_cpp(
        player_guid,
        session.player_map_id_like_cpp(),
        ObjectGuid::EMPTY,
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        !packets
            .iter()
            .any(|bytes| bytes == &expected_farsight_clear),
        "non-empty canonical FarsightObject must not emit represented VALUES-empty update"
    );
}
