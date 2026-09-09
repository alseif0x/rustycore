//! Session scenarios exercising the represented visibility responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn force_update_visibility_repopulates_client_guids_after_login_clear_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 74_330);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let creature_guid = test_creature_guid(74_331);

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "LoginVisibility".to_string(),
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
    session.apply_move_init_active_mover_complete_like_cpp(0);
    let _ = drain_server_packet_bytes(&send_rx);

    let creature_position = Position::new(20.0, 20.0, 0.0, 0.0);
    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(creature_position.x, creature_position.y);
    manager.write().unwrap().add_creature(
        571,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            creature_guid,
            901,
            creature_position,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );

    session.last_visibility_pos = Some(player_position);
    session.client_visible_guids_like_cpp.clear();

    session.force_update_visibility_like_cpp().await;

    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&creature_guid),
        "C++ SendInitialPacketsAfterAddToMap::UpdateVisibilityForPlayer repopulates m_clientGUIDs after Map::AddPlayerToMap cleared it"
    );
    assert_eq!(session.last_visibility_pos, Some(player_position));
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        packets.iter().any(|packet| {
            packet.len() >= 2
                && u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::UpdateObject as u16
        }),
        "initial forced visibility should send CREATE update data for newly visible objects"
    );
}
#[tokio::test]
async fn far_sight_update_visibility_uses_represented_seer_position_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 492);
    let seer_guid = test_creature_guid(4920);
    let visible_creature_guid = test_creature_guid(4921);
    let visible_go_guid = test_gameobject_guid(4922, 4922);
    let player_position = Position::new(0.0, 0.0, 0.0, 0.0);
    let seer_position = Position::new(3000.0, 3000.0, 0.0, 0.0);
    let visible_position = Position::new(3010.0, 3010.0, 0.0, 0.0);

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightVisibility".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical player map");
    add_canonical_test_creature(&canonical, seer_guid, 4920, seer_position, 0);

    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(visible_position.x, visible_position.y);
    manager.write().unwrap().add_creature(
        571,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            visible_creature_guid,
            4921,
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
        ),
    );
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        visible_go_guid,
        4922,
        visible_position,
        3,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        visible_go_guid,
        7492,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
    );

    set_canonical_player_farsight_object_like_cpp(&canonical, player_guid, seer_guid);
    session.represented_seer_guid_like_cpp = Some(seer_guid);
    session.apply_move_init_active_mover_complete_like_cpp(0);

    assert_eq!(
        session.represented_visibility_source_position_like_cpp(),
        Some(seer_position)
    );

    tokio::time::timeout(Duration::from_secs(1), session.update_visibility())
        .await
        .expect("visibility must not re-enter the canonical map mutex");

    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&visible_creature_guid),
        "visibility should scan around represented m_seer position"
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&visible_go_guid),
        "canonical GO visibility should use represented m_seer position"
    );
    assert_eq!(session.last_visibility_pos, Some(seer_position));
}
#[tokio::test]
async fn far_sight_update_visibility_canonical_clear_resets_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_800);
    let stale_dynamic_object_guid = test_dynamic_object_guid(49_801, 49_801);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightClear".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.represented_seer_guid_like_cpp = Some(stale_dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.update_visibility().await;

    assert_eq!(session.represented_seer_guid_like_cpp(), Some(player_guid));
    assert_eq!(session.last_visibility_pos, Some(player_position));
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
        "canonical empty FarsightObject should emit exactly one represented VALUES-empty update"
    );
    assert_eq!(
        update_object_packet_count_like_cpp(&packets),
        1,
        "no unrelated visibility packet should be needed for the empty canonical map"
    );
}
#[tokio::test]
async fn far_sight_update_visibility_non_empty_canonical_keeps_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_802);
    let dynamic_object_guid = test_dynamic_object_guid(49_803, 49_803);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightKeep".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    set_canonical_player_farsight_object_like_cpp(&canonical, player_guid, dynamic_object_guid);
    session.represented_seer_guid_like_cpp = Some(dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.update_visibility().await;

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(dynamic_object_guid)
    );
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
#[tokio::test]
async fn far_sight_update_visibility_missing_canonical_player_keeps_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_804);
    let stale_dynamic_object_guid = test_dynamic_object_guid(49_805, 49_805);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightMissingCanonical".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    canonical.lock().unwrap().create_world_map(571, 0);
    session.represented_seer_guid_like_cpp = Some(stale_dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.update_visibility().await;

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(stale_dynamic_object_guid),
        "missing canonical Player is not proof of canonical farsight clear"
    );
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
        "missing canonical Player must not emit represented VALUES-empty update"
    );
}
#[test]
fn canonical_player_phase_shift_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_573);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PhaseOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    let active_phase = PhaseShift::from_phases([10]);
    assert!(session.set_represented_player_phase_shift_like_cpp(active_phase.clone()));
    assert_eq!(
        session.represented_player_phase_shift_like_cpp(),
        Some(active_phase.clone())
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.represented_player_phase_shift_like_cpp(),
        Some(active_phase)
    );

    let replacement_phase = PhaseShift::from_phases([20]);
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    *replacement.unit_mut().world_mut().phase_shift_mut() = replacement_phase.clone();
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.represented_player_phase_shift_like_cpp(), None);
    assert!(!session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([30])));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.unit().world().phase_shift().clone()
            }),
        Some(replacement_phase)
    );
}
#[test]
fn player_entry_visibility_refresh_skips_out_of_range_sessions_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let nearby_guid = ObjectGuid::create_player(1, 43);
    let far_guid = ObjectGuid::create_player(1, 44);
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (nearby_tx, nearby_rx) = flume::bounded(1);
    let (nearby_command_tx, nearby_command_rx) = flume::bounded(1);
    let (far_tx, far_rx) = flume::bounded(1);
    let (far_command_tx, far_command_rx) = flume::bounded(1);

    session.set_player_guid(Some(guid));
    session.set_player_registry(Arc::clone(&registry));
    session.set_player_position_like_cpp(Position::ZERO);

    registry.register_or_replace(
        nearby_guid,
        broadcast_info_with_command(nearby_guid, nearby_tx, nearby_command_tx),
        Default::default(),
    );
    let mut far_info = broadcast_info_with_command(far_guid, far_tx, far_command_tx);
    far_info.placement.position =
        Position::new(crate::map_manager::VISIBILITY_RADIUS + 1.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(far_guid, far_info, Default::default());

    session.notify_other_players_visibility_changed_like_cpp();

    let command = nearby_command_rx
        .try_recv()
        .expect("nearby visibility refresh command");
    let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) = command else {
        panic!("expected full visibility refresh command");
    };
    assert_eq!(command.map_id, 0);
    assert_eq!(command.instance_id, 0);
    assert!(nearby_rx.try_recv().is_err());
    assert!(far_rx.try_recv().is_err());
    assert!(far_command_rx.try_recv().is_err());
}
#[tokio::test]
async fn player_visibility_refresh_survives_full_command_queue_like_cpp() {
    let (mut source, _, _) = make_session();
    let (mut receiver, _, _) = make_session();
    let source_guid = ObjectGuid::create_player(1, 45);
    let receiver_guid = ObjectGuid::create_player(1, 46);
    let source_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let receiver_registry = Arc::new(PlayerRegistry::default());
    let (receiver_send_tx, _receiver_send_rx) = flume::bounded(1);
    let (full_command_tx, full_command_rx) = flume::bounded(1);

    full_command_tx
        .try_send(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id: 999,
                instance_id: 0,
            },
        ))
        .unwrap();

    source.set_player_guid(Some(source_guid));
    source.set_player_registry(Arc::clone(&source_registry));
    source.set_player_position_like_cpp(Position::ZERO);

    receiver.set_player_guid(Some(receiver_guid));
    receiver.set_player_registry(receiver_registry);
    receiver.set_player_position_like_cpp(Position::ZERO);
    receiver.set_state(SessionState::LoggedIn);

    let mut receiver_info =
        broadcast_info_with_command(receiver_guid, receiver_send_tx, full_command_tx);
    receiver_info.visibility_refresh_pending_like_cpp =
        Arc::clone(&receiver.visibility_refresh_pending_like_cpp);
    source_registry.register_or_replace(receiver_guid, receiver_info, Default::default());

    source.notify_other_players_visibility_changed_like_cpp();

    assert!(
        receiver
            .visibility_refresh_pending_like_cpp
            .load(Ordering::Acquire),
        "a full bounded queue must retain the C++ visibility notification"
    );
    assert_eq!(
        full_command_rx.len(),
        1,
        "the saturated queue remains untouched and the refresh is coalesced separately"
    );

    receiver
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        !receiver
            .visibility_refresh_pending_like_cpp
            .load(Ordering::Acquire)
    );
    assert_eq!(receiver.last_visibility_pos, Some(Position::ZERO));
}
#[test]
fn player_exit_visibility_refresh_uses_same_full_diff_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let nearby_guid = ObjectGuid::create_player(1, 43);
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (nearby_tx, nearby_rx) = flume::bounded(1);
    let (nearby_command_tx, nearby_command_rx) = flume::bounded(1);

    session.set_player_guid(Some(guid));
    session.set_player_registry(Arc::clone(&registry));
    session.set_player_position_like_cpp(Position::ZERO);

    let (self_tx, _self_rx) = flume::bounded(1);
    // Cleanup unregisters through the owning control channel (#243), so this
    // session's own entry must carry its command sender.
    registry.register_or_replace(
        guid,
        broadcast_info_with_command(guid, self_tx, session.session_command_tx()),
        Default::default(),
    );
    registry.register_or_replace(
        nearby_guid,
        broadcast_info_with_command(nearby_guid, nearby_tx, nearby_command_tx),
        Default::default(),
    );

    session.cleanup_shared_runtime_state();

    assert!(registry.runtime_recipient(guid).is_none());
    assert!(nearby_rx.try_recv().is_err());
    let command = nearby_command_rx
        .try_recv()
        .expect("nearby full visibility refresh command");
    let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) = command else {
        panic!("expected full visibility refresh command");
    };
    assert_eq!(command.map_id, 0);
    assert_eq!(command.instance_id, 0);
}
#[test]
fn visible_other_players_skips_out_of_visibility_range_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let nearby_guid = ObjectGuid::create_player(1, 43);
    let far_guid = ObjectGuid::create_player(1, 44);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let registry = Arc::new(PlayerRegistry::default());
    let (nearby_tx, _nearby_rx) = flume::bounded(1);
    let (far_tx, _far_rx) = flume::bounded(1);

    session.set_player_guid(Some(guid));
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::ZERO);
    add_canonical_test_player_on_map(&canonical, guid, Position::ZERO, 571, 0);
    add_canonical_test_player_on_map(&canonical, nearby_guid, Position::ZERO, 571, 0);
    add_canonical_test_player_on_map(
        &canonical,
        far_guid,
        Position::new(crate::map_manager::VISIBILITY_RADIUS + 1.0, 0.0, 0.0, 0.0),
        571,
        0,
    );

    let mut nearby_info = broadcast_info(nearby_guid, nearby_tx);
    nearby_info.placement.map_id = 571;
    registry.register_or_replace(nearby_guid, nearby_info, Default::default());
    let mut far_info = broadcast_info(far_guid, far_tx);
    far_info.placement.map_id = 571;
    far_info.placement.position =
        Position::new(crate::map_manager::VISIBILITY_RADIUS + 1.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(far_guid, far_info, Default::default());

    let visible = session.visible_other_players_from_registry_like_cpp(
        571,
        &Position::ZERO,
        crate::map_manager::VISIBILITY_RADIUS,
    );

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].0, nearby_guid);
}
#[tokio::test]
async fn player_visibility_diff_creates_then_removes_registry_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let viewer_guid = ObjectGuid::create_player(1, 47);
    let target_guid = ObjectGuid::create_player(1, 48);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let registry = Arc::new(PlayerRegistry::default());
    let (target_tx, _target_rx) = flume::bounded(1);

    session.set_player_guid(Some(viewer_guid));
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::ZERO);
    add_canonical_test_player_on_map(&canonical, viewer_guid, Position::ZERO, 571, 0);
    add_canonical_test_player_on_map(&canonical, target_guid, Position::ZERO, 571, 0);
    let mut target_info = broadcast_info(target_guid, target_tx);
    target_info.placement.map_id = 571;
    registry.register_or_replace(target_guid, target_info, Default::default());

    session.force_update_visibility_like_cpp().await;
    assert!(session.client_visible_guids_like_cpp.contains(&target_guid));
    let create = send_rx.try_recv().expect("player CREATE visibility diff");
    assert_eq!(
        u16::from_le_bytes(create[0..2].try_into().unwrap()),
        ServerOpcodes::UpdateObject as u16
    );

    assert!(registry.fixture_remove(target_guid));
    session.force_update_visibility_like_cpp().await;

    assert!(!session.client_visible_guids_like_cpp.contains(&target_guid));
    let out_of_range = send_rx
        .try_recv()
        .expect("player out-of-range visibility diff");
    assert_eq!(
        u16::from_le_bytes(out_of_range[0..2].try_into().unwrap()),
        ServerOpcodes::UpdateObject as u16
    );
    assert_eq!(
        u32::from_le_bytes(out_of_range[2..6].try_into().unwrap()),
        0,
        "C++ UpdateData::m_blockCount excludes the separate out-of-range GUID section"
    );
}
#[test]
fn player_visibility_create_uses_live_stats_and_customizations_like_cpp() {
    let guid = ObjectGuid::create_player(1, 51);
    let (send_tx, _send_rx) = flume::bounded(1);
    let registry = PlayerRegistry::default();
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    add_canonical_test_player_on_map(&canonical, guid, Position::ZERO, 571, 0);
    let vitals = (1_234, 4_567, PowerType::Mana, 321, 789, 456);
    assert!(set_vitals(&canonical, guid, vitals));
    assert!(
        with_canonical_player_at_mut_like_cpp(&canonical, guid, 571, 0, |player| {
            player.unit_mut().set_display_id(77, true);
            crate::canonical_player_access::set_player_visible_item_values_like_cpp(
                player,
                0,
                (12_345, 6, 7),
            );
            player.gameplay_state_mut().customizations = vec![
                wow_entities::PlayerCustomizationChoice {
                    option_id: 10,
                    choice_id: 20,
                },
                wow_entities::PlayerCustomizationChoice {
                    option_id: 30,
                    choice_id: 40,
                },
            ];
        })
        .is_some()
    );
    let mut info = broadcast_info(guid, send_tx);
    info.placement.map_id = 571;
    info.identity.class = 8;
    let expected_transport = wow_packet::packets::movement::TransportInfo {
        guid: ObjectGuid::create_transport(HighGuid::Transport, 7_004),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        o: 0.5,
        seat: 4,
        time: 123,
        prev_time: Some(122),
        vehicle_id: Some(99),
    };
    assert!(
        with_canonical_player_at_mut_like_cpp(&canonical, guid, 571, 0, |player| {
            player.gameplay_state_mut().transport = Some(wow_entities::PlayerTransportState {
                guid: expected_transport.guid,
                x: expected_transport.x,
                y: expected_transport.y,
                z: expected_transport.z,
                orientation: expected_transport.o,
                seat: expected_transport.seat,
                time: expected_transport.time,
                prev_time: expected_transport.prev_time,
                vehicle_id: expected_transport.vehicle_id,
            });
        })
        .is_some()
    );
    let expected_customizations = Arc::new(vec![
        wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate {
            option_id: 10,
            choice_id: 20,
        },
        wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate {
            option_id: 30,
            choice_id: 40,
        },
    ]);
    registry.register_or_replace(guid, info, Default::default());
    let snapshot = registry
        .player_visibility_create_candidates(
            ObjectGuid::create_player(1, 999),
            571,
            0,
            Position::ZERO,
            0.0,
            100.0,
        )
        .into_iter()
        .next()
        .expect("canonical player CREATE candidate");
    assert_eq!(snapshot.display_id, 77);
    assert_eq!(snapshot.visible_items[0], (12_345, 6, 7));
    let update = crate::handlers::character::player_visibility_create_update_from_snapshot_like_cpp(
        &snapshot, 571,
    );
    let [
        wow_packet::packets::update::UpdateBlock::CreateObject {
            create_data,
            movement: Some(movement),
            is_self,
            ..
        },
    ] = update.blocks.as_slice()
    else {
        panic!("expected one non-owner player CREATE block");
    };

    assert!(!is_self);
    assert_eq!(create_data.health, 1_234);
    assert_eq!(create_data.max_health, 4_567);
    assert_eq!(create_data.current_power0, 321);
    assert_eq!(create_data.max_mana, 789);
    assert_eq!(create_data.base_mana, 456);
    assert_eq!(
        &create_data.customizations,
        expected_customizations.as_ref()
    );
    let transport = movement
        .transport
        .as_ref()
        .expect("non-owner player transport attachment");
    assert_eq!(transport.guid, expected_transport.guid);
    assert_eq!(transport.x, 1.0);
    assert_eq!(transport.seat, 4);
    assert_eq!(transport.time, 123);
    assert_eq!(transport.prev_time, Some(122));
    assert_eq!(transport.vehicle_id, Some(99));
}
#[test]
fn visible_other_players_rejects_registry_only_target_like_cpp() {
    let (mut session, _, _) = make_session();
    let viewer_guid = ObjectGuid::create_player(1, 49);
    let target_guid = ObjectGuid::create_player(1, 50);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let registry = Arc::new(PlayerRegistry::default());
    let (target_tx, _target_rx) = flume::bounded(1);

    session.set_player_guid(Some(viewer_guid));
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::ZERO);
    add_canonical_test_player_on_map(&canonical, viewer_guid, Position::ZERO, 571, 0);

    let mut target_info = broadcast_info(target_guid, target_tx);
    target_info.placement.map_id = 571;
    registry.register_or_replace(target_guid, target_info, Default::default());

    assert!(
        session
            .visible_other_players_from_registry_like_cpp(
                571,
                &Position::ZERO,
                crate::map_manager::VISIBILITY_RADIUS,
            )
            .is_empty(),
        "C++ VisibleNotifier cannot visit a registry-only player"
    );
}
#[test]
fn visible_other_players_applies_canonical_phase_gate_like_cpp() {
    let (mut session, _, _) = make_session();
    let viewer_guid = ObjectGuid::create_player(1, 45);
    let target_guid = ObjectGuid::create_player(1, 46);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let registry = Arc::new(PlayerRegistry::default());
    let (target_tx, _target_rx) = flume::bounded(1);

    session.set_player_guid(Some(viewer_guid));
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));
    session.set_player_position_like_cpp(Position::ZERO);
    add_canonical_test_player_on_map(&canonical, viewer_guid, Position::ZERO, 571, 0);
    add_canonical_test_player_on_map(&canonical, target_guid, Position::ZERO, 571, 0);
    *canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(target_guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .phase_shift_mut() = PhaseShift::from_phases([20]);

    let mut target = broadcast_info(target_guid, target_tx);
    target.placement.map_id = 571;
    registry.register_or_replace(target_guid, target, Default::default());

    assert!(
        session
            .visible_other_players_from_registry_like_cpp(
                571,
                &Position::ZERO,
                crate::map_manager::VISIBILITY_RADIUS,
            )
            .is_empty(),
        "C++ VisibleNotifier excludes players outside the viewer's phase"
    );
}
#[test]
fn init_transport_filter_excludes_own_transport_and_wrong_phase_like_cpp() {
    let (mut session, _, _) = make_session();
    let own_transport = ObjectGuid::create_transport(HighGuid::Transport, 7001);
    let other_transport = ObjectGuid::create_transport(HighGuid::Transport, 7002);
    session.set_player_transport_guid_like_cpp(Some(own_transport));
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));

    assert!(
        !session
            .should_send_init_transport_like_cpp(own_transport, &PhaseShift::from_phases([10]),)
    );
    assert!(
        session
            .should_send_init_transport_like_cpp(other_transport, &PhaseShift::from_phases([10]),)
    );
    assert!(
        !session
            .should_send_init_transport_like_cpp(other_transport, &PhaseShift::from_phases([20]),)
    );
}
