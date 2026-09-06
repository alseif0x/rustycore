// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! travel capability handler tests.

use super::*;
use crate::session::RepresentedActivateTaxiLikeCpp;
use wow_packet::packets::misc::ERR_TAXITOOFARAWAY_LIKE_CPP;

#[tokio::test]
async fn world_port_response_ignores_ack_without_far_teleport_semaphore_like_cpp() {
    let (mut session, send_rx) = make_session();
    let destination = Position::new(11.0, 22.0, 33.0, 1.5);
    session.pending_teleport = Some((0, destination));
    session.set_state(crate::session::SessionState::Transfer);

    session
        .handle_world_port_response(WorldPacket::new_empty())
        .await;

    assert_eq!(session.pending_teleport, Some((0, destination)));
    assert_eq!(session.state(), crate::session::SessionState::Transfer);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn suspend_token_response_sends_new_world_for_far_teleport_like_cpp() {
    // C++ HandleSuspendTokenResponse (MovementHandler.cpp:239): on the client's suspend
    // ack during a far teleport, send SMSG_NEW_WORLD so it loads the destination map.
    // pending_teleport stays set (the later worldport ack consumes it). Without this the
    // client sits at 0% on the loading screen forever. #NEXT.R8.ENTITIES.1229.
    let (mut session, send_rx) = make_session();
    let destination = Position::new(11.0, 22.0, 33.0, 1.5);
    session.pending_teleport = Some((1, destination));
    session.set_represented_far_teleport_pending_like_cpp(true);
    session.set_state(crate::session::SessionState::Transfer);

    session
        .handle_suspend_token_response(WorldPacket::new_empty())
        .await;

    assert_eq!(
        std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::NewWorld as u16]
    );
    assert_eq!(session.pending_teleport, Some((1, destination)));
}

#[tokio::test]
async fn suspend_token_response_no_op_without_far_teleport_like_cpp() {
    // C++ HandleSuspendTokenResponse early-returns unless IsBeingTeleportedFar().
    let (mut session, send_rx) = make_session();
    session.pending_teleport = Some((1, Position::new(11.0, 22.0, 33.0, 1.5)));
    // far-teleport semaphore deliberately not set.

    session
        .handle_suspend_token_response(WorldPacket::new_empty())
        .await;

    assert_eq!(
        std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        Vec::<u16>::new()
    );
}

#[tokio::test]
async fn far_teleport_self_create_preserves_current_xp_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1C3);
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "TeleportXp".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        8,
        10,
        0,
    ));
    session.set_player_xp_like_cpp(1_234_567);
    session.set_player_next_level_xp_like_cpp(2_345_678);

    session
        .send_player_self_create_for_teleport_like_cpp(
            &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
        )
        .await;

    let bytes = send_rx.recv().expect("far teleport sends self CREATE");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::UpdateObject as u16
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 1_234_567i32.to_le_bytes()),
        "ActivePlayerData::XP must survive a far-map self CREATE"
    );
}

#[tokio::test]
async fn world_port_response_clears_far_teleport_semaphore_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let destination = Position::new(11.0, 22.0, 33.0, 1.5);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(MapStore::from_entries([
        MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "WorldportAck".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session.pending_teleport = Some((0, destination));
    session.set_represented_far_teleport_pending_like_cpp(true);
    session.set_state(crate::session::SessionState::Transfer);

    session
        .handle_world_port_response(WorldPacket::new_empty())
        .await;

    assert_eq!(session.pending_teleport, None);
    assert!(!session.represented_far_teleport_pending_like_cpp());
    assert_eq!(session.player_map_id_like_cpp(), 0);
    assert_eq!(session.player_position_like_cpp(), Some(destination));
    assert_eq!(session.state(), crate::session::SessionState::LoggedIn);
    {
        let manager = canonical.lock().unwrap();
        assert!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_player(player_guid)
                .is_none(),
            "C++ HandleMoveWorldportAck removes a still-in-world player from the old map before adding to the destination"
        );
        let player = manager
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap();
        assert_eq!(player.unit().world().map_id(), 0);
        assert_eq!(player.unit().world().position(), destination);
    }
    // C++ HandleMoveWorldportAck (non-seamless) replays the init sequence on the client's
    // worldport ack (#NEXT.R8.ENTITIES.1229). SMSG_NEW_WORLD is NOT here — it is sent
    // earlier from handle_suspend_token_response. This handler starts with ResumeToken,
    // then the before-add control packets SetMovedUnit (MoveSetActiveMover) + a fresh
    // TimeSyncRequest, then (no nearby objects on the destination test map, so the AddToMap
    // refresh emits nothing) the full SendInitialPacketsAfterAddToMap helper — post-add
    // PhaseShiftChange, InitWorldStates for the destination map, LoadCufProfiles (no auras
    // on the test player), and the PhasingHandler::OnMapChange PhaseShiftChange. The
    // UpdateObject after TimeSyncRequest is SendInitSelf (the player's own ActivePlayer
    // create for the destination map — C++ Map::AddPlayerToMap initPlayer=true). The final
    // send_stat_update emits nothing in this minimal test (no stat stores configured).
    assert_eq!(
        std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::ResumeToken as u16,
            ServerOpcodes::MoveSetActiveMover as u16,
            ServerOpcodes::TimeSyncRequest as u16,
            ServerOpcodes::UpdateObject as u16,
            ServerOpcodes::PhaseShiftChange as u16,
            ServerOpcodes::InitWorldStates as u16,
            ServerOpcodes::LoadCufProfiles as u16,
            ServerOpcodes::PhaseShiftChange as u16,
        ]
    );
}

#[tokio::test]
async fn world_port_response_recomputes_destination_rest_state_post_add_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let data_dir = unique_temp_data_dir("world-port-rest-state");
    let player_guid = ObjectGuid::create_player(1, 43);
    let destination = Position::new(0.0, 0.0, 9.0, 0.25);
    write_no_area_map_file_like_cpp(&data_dir, 571, destination.x, destination.y, 300);

    session.set_mmap_runtime_config_like_cpp(crate::session::MMapRuntimeConfigLikeCpp {
        data_dir: data_dir.to_string_lossy().to_string(),
        enabled: true,
        disabled_map_ids: HashSet::new(),
    });
    session.set_map_store(Arc::new(MapStore::from_entries([
        map_entry(0, wow_data::map::MAP_COMMON),
        map_entry(571, wow_data::map::MAP_COMMON),
    ])));
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        area_entry(20, 0, wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP),
        area_entry(200, 0, 0),
        area_entry(300, 200, AREA_FLAG_IS_SUBZONE_LIKE_CPP),
    ])));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "WorldportRest".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        0,
        1,
        1,
        10,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        0,
        0,
    );
    session.set_player_zone_area_like_cpp(10, 10);
    assert!(session.update_zone_represented_like_cpp(20, 20));
    assert!(
        session.represented_is_resting_like_cpp(),
        "pre-teleport city rest flag should be active before moving to wilderness"
    );
    while send_rx.try_recv().is_ok() {}

    session.pending_teleport = Some((571, destination));
    session.set_represented_far_teleport_pending_like_cpp(true);
    session.set_state(crate::session::SessionState::Transfer);

    session
        .handle_world_port_response(WorldPacket::new_empty())
        .await;

    assert_eq!(session.player_zone_area_like_cpp(), Some((200, 300)));
    assert!(
        !session.represented_is_resting_like_cpp(),
        "C++ HandleMoveWorldportAck calls UpdateZone in SendInitialPacketsAfterAddToMap before later rest-state saves observe flags"
    );
    assert_eq!(session.state(), crate::session::SessionState::LoggedIn);
    let opcodes: Vec<_> = std::iter::from_fn(|| send_rx.try_recv().ok())
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect();
    let init_world_states_index = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::InitWorldStates as u16)
        .expect("post-add init sends InitWorldStates");
    let last_update_index = opcodes
        .iter()
        .rposition(|opcode| *opcode == ServerOpcodes::UpdateObject as u16)
        .expect("self-create/rest transition sends UpdateObject");
    assert!(
        last_update_index > init_world_states_index,
        "the destination RESTING field update must follow post-add InitWorldStates"
    );
}

#[tokio::test]
async fn world_port_response_activates_pvp_item_levels_for_flagged_map_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let destination = Position::new(7.0, 8.0, 9.0, 0.25);
    let mut pvp_item_level_map = map_entry(30, wow_data::map::MAP_COMMON);
    pvp_item_level_map.flags2 = 0x40;
    session.set_map_store(Arc::new(MapStore::from_entries([
        map_entry(571, wow_data::map::MAP_COMMON),
        pvp_item_level_map,
    ])));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "WorldportPvpItemLevel".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session.pending_teleport = Some((30, destination));
    session.set_represented_far_teleport_pending_like_cpp(true);
    session.set_state(crate::session::SessionState::Transfer);

    session
        .handle_world_port_response(WorldPacket::new_empty())
        .await;

    assert!(
        session.represented_using_pvp_item_levels_like_cpp(),
        "C++ Player::UpdateItemLevelAreaBasedScaling activates PvP item levels when MapEntry::Flags[1] has 0x40"
    );
}

#[tokio::test]
async fn world_port_response_deactivates_pvp_item_levels_for_normal_map_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let destination = Position::new(7.0, 8.0, 9.0, 0.25);
    let mut pvp_item_level_map = map_entry(30, wow_data::map::MAP_COMMON);
    pvp_item_level_map.flags2 = 0x40;
    session.set_map_store(Arc::new(MapStore::from_entries([
        pvp_item_level_map,
        map_entry(571, wow_data::map::MAP_COMMON),
    ])));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "WorldportNormalItemLevel".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        30,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        30,
        0,
    );
    let _ = session.set_represented_using_pvp_item_levels_like_cpp(true);
    session.pending_teleport = Some((571, destination));
    session.set_represented_far_teleport_pending_like_cpp(true);
    session.set_state(crate::session::SessionState::Transfer);

    session
        .handle_world_port_response(WorldPacket::new_empty())
        .await;

    assert!(
        !session.represented_using_pvp_item_levels_like_cpp(),
        "C++ Player::UpdateItemLevelAreaBasedScaling clears PvP item levels after leaving map/PvP activity"
    );
}

#[tokio::test]
async fn activate_taxi_without_interactable_flight_master_replies_too_far_like_cpp() {
    let (mut session, send_rx) = make_session();
    let vendor = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9, 77);

    session
        .handle_activate_taxi(activate_taxi_packet(vendor, 12, 101, 202))
        .await;

    let encoded = send_rx.try_recv().unwrap();
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::ActivateTaxiReply)
    );
    packet.skip_opcode();
    assert_eq!(
        packet.read_bits(4).unwrap(),
        u32::from(ERR_TAXITOOFARAWAY_LIKE_CPP)
    );
    assert_eq!(packet.remaining(), 0);
    assert!(
        session
            .represented_activate_taxi_requests_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn activate_taxi_records_represented_request_for_flight_master_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let vendor = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9, 77);
    let position = Position::new(10.0, 0.0, 0.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_alive_like_cpp(true);
    session.set_player_faction_template_like_cpp(35);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "TaxiTester".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(&canonical, player_guid, position, 571, 0);
    add_canonical_flight_master_for_misc_test(&canonical, vendor, position);

    session
        .handle_activate_taxi(activate_taxi_packet(vendor, 12, 101, 202))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_activate_taxi_requests_like_cpp(),
        &[RepresentedActivateTaxiLikeCpp {
            vendor,
            node: 12,
            ground_mount_id: 101,
            flying_mount_id: 202,
            preferred_mount_display: 0,
        }]
    );
}

#[tokio::test]
async fn set_taxi_benchmark_mode_sets_and_clears_player_flag_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 9010);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    let mut enable = WorldPacket::new_empty();
    enable.write_bit(true);
    enable.flush_bits();
    enable.reset_read();
    session.handle_set_taxi_benchmark_mode(enable).await;
    assert!(session.represented_taxi_benchmark_mode_like_cpp());

    let mut disable = WorldPacket::new_empty();
    disable.write_bit(false);
    disable.flush_bits();
    disable.reset_read();
    session.handle_set_taxi_benchmark_mode(disable).await;
    assert!(!session.represented_taxi_benchmark_mode_like_cpp());
}

#[tokio::test]
async fn set_taxi_benchmark_mode_short_packet_does_not_change_flag_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session
        .handle_set_taxi_benchmark_mode(WorldPacket::from_bytes(&[]))
        .await;

    assert!(!session.represented_taxi_benchmark_mode_like_cpp());
}
