//! Session scenarios exercising the represented movement responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn teleport_to_far_map_clears_transport_server_time_override_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 808);
    let destination = Position::new(113.0, 213.0, 43.0, 2.8);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_active_player_transport_server_time_like_cpp(42_000);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportClearTransportTime".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    assert!(session.represented_far_teleport_pending_like_cpp());
    assert_eq!(
        session.active_player_local_flags_like_cpp()
            & PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
        0,
        "C++ Player::TeleportTo removes PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME after SMSG_TRANSFER_PENDING"
    );
    assert_eq!(
        session.active_player_transport_server_time_like_cpp(),
        0,
        "C++ Player::TeleportTo calls SetTransportServerTime(0)"
    );
}
#[tokio::test]
async fn teleport_to_preflight_abort_preserves_transport_server_time_override_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 809);
    let destination = Position::new(114.0, 214.0, 44.0, 2.9);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_active_player_transport_server_time_like_cpp(43_000);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportRejectKeepsTransportTime".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        DEATH_KNIGHT_START_MAP_LIKE_CPP,
        1,
        CLASS_DEATH_KNIGHT_LIKE_CPP,
        58,
        0,
    ));

    session.teleport_to(571, destination).await;

    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 571,
            arg: 1,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
        }
        .to_bytes()
    );
    assert_ne!(
        session.active_player_local_flags_like_cpp()
            & PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
        0,
        "C++ Player::TeleportTo returns before clearing transport server time on preflight aborts"
    );
    assert_eq!(
        session.active_player_transport_server_time_like_cpp(),
        43_000
    );
}
#[tokio::test]
async fn teleport_to_valid_seamless_suppresses_transfer_pending_and_uses_reason_two_like_cpp() {
    use wow_packet::ServerPacket;

    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 811);
    let destination = Position::new(116.0, 216.0, 46.0, 3.1);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: 0,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_active_player_transport_server_time_like_cpp(44_000);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportSeamless".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_canonical_map_manager(shared_canonical_map_manager());
    assert!(
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .is_some()
    );

    session
        .teleport_to_with_options(0, destination, TELE_TO_SEAMLESS_LIKE_CPP)
        .await;

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::SuspendToken],
        "C++ Player::TeleportTo suppresses TransferPending for valid seamless far teleports"
    );
    assert_eq!(
        packets.last().expect("SMSG_SUSPEND_TOKEN"),
        &wow_packet::packets::misc::SuspendToken {
            // SequenceIndex = m_movementCounter (0 here: no movement-control packets sent
            // in the test). Must match the ResumeToken index. #NEXT.R8.ENTITIES.1229.
            sequence_index: 0,
            reason: 2,
        }
        .to_bytes()
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    assert_ne!(
        session.active_player_local_flags_like_cpp()
            & PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
        0,
        "C++ clears transport server time only in the TransferPending branch"
    );
    assert_eq!(
        session.active_player_transport_server_time_like_cpp(),
        44_000
    );
}
#[tokio::test]
async fn teleport_to_invalid_seamless_falls_back_to_normal_transfer_like_cpp() {
    use wow_packet::ServerPacket;

    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 812);
    let destination = Position::new(117.0, 217.0, 47.0, 3.2);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_active_player_transport_server_time_like_cpp(45_000);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportInvalidSeamless".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    session
        .teleport_to_with_options(0, destination, TELE_TO_SEAMLESS_LIKE_CPP)
        .await;

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ],
        "C++ Player::TeleportTo clears TELE_TO_SEAMLESS when cosmetic-map parents do not match"
    );
    assert_eq!(
        packets.last().expect("SMSG_SUSPEND_TOKEN"),
        &wow_packet::packets::misc::SuspendToken {
            // SequenceIndex = m_movementCounter (0 here). #NEXT.R8.ENTITIES.1229.
            sequence_index: 0,
            reason: 1,
        }
        .to_bytes()
    );
    assert_eq!(session.active_player_transport_server_time_like_cpp(), 0);
}
#[tokio::test]
async fn teleport_to_player_logout_suppresses_transfer_packets_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 813);
    let destination = Position::new(118.0, 218.0, 48.0, 3.3);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_active_player_transport_server_time_like_cpp(46_000);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportLogout".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_logout_like_cpp(true);

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat],
        "C++ Player::TeleportTo suppresses both TransferPending and SuspendToken during PlayerLogout"
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    assert_ne!(
        session.active_player_local_flags_like_cpp()
            & PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
        0
    );
    assert_eq!(
        session.active_player_transport_server_time_like_cpp(),
        46_000
    );
}
#[tokio::test]
async fn teleport_to_far_map_removes_current_player_dynamic_objects_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 801);
    let other_player_guid = ObjectGuid::create_player(1, 802);
    let owned_dynamic_guid = test_dynamic_object_guid(601_801, 50_801);
    let other_dynamic_guid = test_dynamic_object_guid(601_802, 50_802);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(108.0, 208.0, 38.0, 2.3);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportRemoveDynObjects".to_string(),
        source_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source_position, 571, 0);
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        owned_dynamic_guid,
        player_guid,
        60_181,
        source_position,
        571,
        0,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        other_dynamic_guid,
        other_player_guid,
        60_182,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        0,
    );

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    let manager = canonical.lock().unwrap();
    let map = manager.find_map(571, 0).unwrap().map();
    assert!(
        map.get_typed_dynamic_object(owned_dynamic_guid).is_none(),
        "C++ Player::TeleportTo calls Unit::RemoveAllDynObjects before transfer"
    );
    assert!(
        map.get_typed_dynamic_object(other_dynamic_guid).is_some(),
        "RemoveAllDynObjects must not remove DynamicObjects owned by another caster"
    );
}
#[tokio::test]
async fn teleport_to_far_map_removes_current_player_area_triggers_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 803);
    let other_player_guid = ObjectGuid::create_player(1, 804);
    let owned_area_trigger_guid = test_area_trigger_guid(601_803, 50_803);
    let other_area_trigger_guid = test_area_trigger_guid(601_804, 50_804);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(109.0, 209.0, 39.0, 2.4);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportRemoveAreaTriggers".to_string(),
        source_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source_position, 571, 0);
    add_canonical_test_area_trigger_on_map(
        &canonical,
        owned_area_trigger_guid,
        player_guid,
        60_183,
        source_position,
        571,
        0,
    );
    add_canonical_test_area_trigger_on_map(
        &canonical,
        other_area_trigger_guid,
        other_player_guid,
        60_184,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        0,
    );

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    let manager = canonical.lock().unwrap();
    let map = manager.find_map(571, 0).unwrap().map();
    assert!(
        map.get_area_trigger(owned_area_trigger_guid).is_none(),
        "C++ Player::TeleportTo calls Unit::RemoveAllAreaTriggers before transfer"
    );
    assert!(
        map.get_area_trigger(other_area_trigger_guid).is_some(),
        "RemoveAllAreaTriggers must not remove AreaTriggers owned by another caster"
    );
}
#[tokio::test]
async fn teleport_to_blocks_unescaped_death_knight_leaving_start_map_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 796);
    let destination = Position::new(104.0, 204.0, 34.0, 1.9);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportDkReject".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        DEATH_KNIGHT_START_MAP_LIKE_CPP,
        1,
        CLASS_DEATH_KNIGHT_LIKE_CPP,
        58,
        0,
    ));

    session.teleport_to(571, destination).await;

    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 571,
            arg: 1,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::TeleportTo returns before SMSG_TRANSFER_PENDING for unescaped DKs leaving map 609"
    );
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_ne!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn teleport_to_dk_escape_abort_still_masks_movement_flags_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 828);
    let destination = Position::new(104.0, 204.0, 34.0, 1.9);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportDkRejectMovementReset".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        DEATH_KNIGHT_START_MAP_LIKE_CPP,
        1,
        CLASS_DEATH_KNIGHT_LIKE_CPP,
        58,
        0,
    ));
    session.set_player_movement_flags_like_cpp(
        MovementFlag::DISABLE_GRAVITY
            | MovementFlag::HOVER
            | MovementFlag::FORWARD
            | MovementFlag::FALLING
            | MovementFlag::FLYING,
    );
    session.set_player_movement_jump_like_cpp(wow_packet::packets::movement::JumpInfo {
        fall_time: 1_500,
        z_speed: 4.25,
        has_direction: true,
        sin_angle: 0.25,
        cos_angle: 0.75,
        xy_speed: 6.5,
    });

    session.teleport_to(571, destination).await;

    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 571,
            arg: 1,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
        }
        .to_bytes()
    );
    assert_eq!(
        session.player_movement_flags_like_cpp(),
        MovementFlag::DISABLE_GRAVITY | MovementFlag::HOVER,
        "C++ resets movement flags before the far-branch DK escape abort"
    );
    assert_eq!(
        session.player_movement_jump_like_cpp().fall_time,
        0,
        "C++ Player::TeleportTo calls MovementInfo::ResetJump before the DK escape abort"
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::TeleportTo returns before SMSG_TRANSFER_PENDING for unescaped DKs leaving map 609"
    );
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_ne!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn teleport_to_same_map_sends_move_teleport_and_sets_near_pending_like_cpp() {
    use wow_packet::ServerPacket;

    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 814);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(119.0, 219.0, 49.0, 3.4);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleport".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_canonical_map_manager(shared_canonical_map_manager());
    assert!(
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .is_some()
    );

    session.teleport_to(571, destination).await;

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport],
        "C++ same-map Player::TeleportTo sends SMSG_MOVE_TELEPORT, not TransferPending"
    );
    assert_eq!(
        packets.last().expect("SMSG_MOVE_TELEPORT"),
        &wow_packet::packets::movement::MoveTeleport {
            mover_guid: player_guid,
            position: destination,
            facing: destination.orientation,
            sequence_index: 0,
            preload_world: 0,
            transport_guid: None,
        }
        .to_bytes()
    );
    assert!(session.near_teleport_pending_like_cpp());
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_ne!(session.state, SessionState::Transfer);
    assert_eq!(session.fall_information_like_cpp(), (0, source.z));
}
#[tokio::test]
async fn teleport_to_same_map_masks_movement_flags_before_near_teleport_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    let player_guid = ObjectGuid::create_player(1, 827);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(119.0, 219.0, 49.0, 3.4);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportMovementReset".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical world map/player");
    session.mutate_canonical_player_like_cpp(|player| {
        let motion = &mut player.unit_mut().subsystems_mut().motion;
        motion.start_spline(77, 1_000);
        motion.launch_generic_movement(MovementGeneratorKind::Effect, 3, 1_000, None);
    });
    session.set_player_movement_flags_like_cpp(
        MovementFlag::ROOT
            | MovementFlag::CAN_FLY
            | MovementFlag::FORWARD
            | MovementFlag::FLYING
            | MovementFlag::FALLING
            | MovementFlag::SPLINE_ELEVATION,
    );
    session.set_player_movement_jump_like_cpp(wow_packet::packets::movement::JumpInfo {
        fall_time: 2_250,
        z_speed: 8.0,
        has_direction: true,
        sin_angle: 0.4,
        cos_angle: 0.6,
        xy_speed: 7.0,
    });

    session.teleport_to(571, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport]
    );
    assert_eq!(
        session.player_movement_flags_like_cpp(),
        MovementFlag::ROOT | MovementFlag::CAN_FLY,
        "C++ Player::TeleportTo keeps only MOVEMENTFLAG_MASK_HAS_PLAYER_STATUS_OPCODE before the same-map branch"
    );
    assert_eq!(
        session.player_movement_jump_like_cpp().fall_time,
        0,
        "C++ Player::TeleportTo resets MovementInfo::jump before the same-map branch"
    );
    {
        let canonical = canonical.lock().unwrap();
        let player = canonical
            .find_map(571, 0)
            .and_then(|map| map.map().get_typed_player(player_guid))
            .expect("canonical player after teleport");
        assert_eq!(
            player.unit().movement_flags_like_cpp(),
            MovementFlag::ROOT | MovementFlag::CAN_FLY,
            "C++ Player::TeleportTo writes the masked flags back to Unit::m_movementInfo"
        );
        let motion = &player.unit().subsystems().motion;
        assert!(
            motion.spline.finalized,
            "C++ Player::TeleportTo calls Unit::DisableSpline before the same-map branch"
        );
        assert!(
            !motion
                .active_generators
                .iter()
                .any(|generator| generator.kind == MovementGeneratorKind::Effect),
            "C++ Player::TeleportTo removes EFFECT_MOTION_TYPE before the same-map branch"
        );
    }
    assert!(session.near_teleport_pending_like_cpp());
}
#[tokio::test]
async fn teleport_to_same_map_forces_vehicle_exit_before_near_teleport_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let player_guid = ObjectGuid::create_player(1, 830);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(119.0, 219.0, 49.0, 3.4);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_player_registry(Arc::clone(&registry));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportVehicleExit".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK);
    session.player_vehicle_seat_id_like_cpp = Some(1004);
    session.register_in_player_registry();

    session.teleport_to(571, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport]
    );
    assert!(
        session.player_vehicle_seat_flags_like_cpp.is_none(),
        "C++ Player::TeleportTo calls ExitVehicle directly; this is not gated by client seat-exit permissions"
    );
    assert!(session.player_vehicle_seat_id_like_cpp.is_none());
    let info = registry
        .party_member(player_guid)
        .expect("registered player");
    assert!(!info.in_vehicle);
    assert_eq!(info.party_member_vehicle_seat, 0);
    assert!(session.near_teleport_pending_like_cpp());
}
