//! Movement info and packets regression scenarios, part 1 of 1.
//!
//! Moved out of the movement.rs root under #650; every test is unchanged.

use super::*;

#[test]
fn movement_info_write_includes_fall_data_when_falling_flag_is_set_like_cpp() {
    let mut info = MovementInfo {
        guid: ObjectGuid::create_player(1, 42),
        flags: MovementFlag::FALLING,
        time: 1234,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        jump: JumpInfo {
            fall_time: 0,
            z_speed: 5.0,
            has_direction: false,
            sin_angle: 0.25,
            cos_angle: 0.75,
            xy_speed: 6.0,
        },
        ..MovementInfo::default()
    };

    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);

    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let decoded = MovementInfo::read(&mut pkt).unwrap();
    assert_eq!(decoded.flags, MovementFlag::FALLING);
    assert_eq!(decoded.jump.fall_time, 0);
    assert_eq!(decoded.jump.z_speed, 5.0);
    assert!(decoded.jump.has_direction);
    assert_eq!(decoded.jump.sin_angle, 0.25);
    assert_eq!(decoded.jump.cos_angle, 0.75);
    assert_eq!(decoded.jump.xy_speed, 6.0);

    info.flags = MovementFlag::NONE;
    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let decoded = MovementInfo::read(&mut pkt).unwrap();
    assert_eq!(decoded.jump.fall_time, 0);
    assert!(!decoded.jump.has_direction);
}

#[test]
fn movement_info_preserves_standing_guid_and_inertia_like_cpp() {
    let info = MovementInfo {
        guid: ObjectGuid::create_player(1, 42),
        time: 77,
        position: Position::new(10.0, 20.0, 30.0, 1.5),
        standing_on_gameobject_guid: Some(ObjectGuid::create_world_object(
            HighGuid::GameObject,
            0,
            1,
            0,
            0,
            7,
            9001,
        )),
        inertia: Some(InertiaInfo {
            id: 12,
            x: 1.0,
            y: 2.0,
            z: 3.0,
            lifetime: 400,
        }),
        ..MovementInfo::default()
    };

    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);

    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let decoded = MovementInfo::read(&mut pkt).unwrap();
    assert_eq!(
        decoded.standing_on_gameobject_guid,
        Some(ObjectGuid::create_world_object(
            HighGuid::GameObject,
            0,
            1,
            0,
            0,
            7,
            9001,
        ))
    );
    let inertia = decoded.inertia.unwrap();
    assert_eq!(inertia.id, 12);
    assert_eq!(inertia.x, 1.0);
    assert_eq!(inertia.y, 2.0);
    assert_eq!(inertia.z, 3.0);
    assert_eq!(inertia.lifetime, 400);
}

#[test]
fn movement_transport_write_skips_zero_optional_values_like_cpp() {
    let info = MovementInfo {
        guid: ObjectGuid::create_player(1, 42),
        time: 77,
        position: Position::new(10.0, 20.0, 30.0, 1.5),
        transport: Some(TransportInfo {
            guid: ObjectGuid::create_world_object(HighGuid::Transport, 0, 1, 0, 0, 9, 100),
            x: 1.0,
            y: 2.0,
            z: 3.0,
            o: 4.0,
            seat: 2,
            time: 55,
            prev_time: Some(0),
            vehicle_id: Some(0),
        }),
        ..MovementInfo::default()
    };

    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);

    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let decoded = MovementInfo::read(&mut pkt).unwrap();
    let transport = decoded.transport.unwrap();
    assert_eq!(transport.prev_time, None);
    assert_eq!(transport.vehicle_id, None);
}

#[test]
fn movement_ack_packets_read_cpp_field_order() {
    let info = MovementInfo {
        guid: ObjectGuid::create_player(1, 42),
        time: 1234,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        ..MovementInfo::default()
    };

    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);
    pkt.write_int32(77);
    pkt.write_float(7.5);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let speed_ack = MovementSpeedAck::read(&mut pkt).unwrap();
    assert_eq!(speed_ack.ack.status.guid, info.guid);
    assert_eq!(speed_ack.ack.ack_index, 77);
    assert_eq!(speed_ack.speed, 7.5);

    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);
    pkt.write_int32(78);
    pkt.write_bit(true);
    pkt.write_float(8.0);
    pkt.write_float(9.0);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let knockback = MoveKnockBackAck::read(&mut pkt).unwrap();
    assert_eq!(knockback.ack.ack_index, 78);
    assert_eq!(
        knockback.speeds,
        Some(MoveKnockBackSpeeds {
            horz_speed: 8.0,
            vert_speed: 9.0
        })
    );

    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);
    pkt.write_int32(79);
    pkt.write_float(2.25);
    pkt.write_uint32(123);
    pkt.write_uint8(4);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let collision = MoveSetCollisionHeightAck::read(&mut pkt).unwrap();
    assert_eq!(collision.data.ack_index, 79);
    assert_eq!(collision.height, 2.25);
    assert_eq!(collision.mount_display_id, 123);
    assert_eq!(collision.reason, 4);
}

#[test]
fn movement_time_spline_and_teleport_ack_packets_read_cpp_field_order() {
    let guid = ObjectGuid::create_player(1, 42);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.write_uint32(250);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let skipped = MoveTimeSkipped::read(&mut pkt).unwrap();
    assert_eq!(skipped.mover_guid, guid);
    assert_eq!(skipped.time_skipped, 250);

    let info = MovementInfo {
        guid,
        time: 1234,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        ..MovementInfo::default()
    };
    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);
    pkt.write_int32(9001);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let spline = MoveSplineDone::read(&mut pkt).unwrap();
    assert_eq!(spline.status.guid, guid);
    assert_eq!(spline.spline_id, 9001);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.write_int32(11);
    pkt.write_int32(12);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let teleport = MoveTeleportAck::read(&mut pkt).unwrap();
    assert_eq!(teleport.mover_guid, guid);
    assert_eq!(teleport.ack_index, 11);
    assert_eq!(teleport.move_time, 12);
}

#[test]
fn move_teleport_writes_cpp_no_transport_field_order() {
    let guid = ObjectGuid::create_player(1, 42);
    let packet = MoveTeleport {
        mover_guid: guid,
        position: Position::new(1.25, 2.5, 3.75, 4.0),
        facing: 4.0,
        sequence_index: 77,
        preload_world: 0,
        transport_guid: None,
    };

    let bytes = packet.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::MoveTeleport as u16
    );
    assert_eq!(pkt.read_int64().unwrap(), guid.low_value());
    assert_eq!(pkt.read_int64().unwrap(), guid.high_value());
    assert_eq!(pkt.read_uint32().unwrap(), 77);
    assert_eq!(pkt.read_float().unwrap(), 1.25);
    assert_eq!(pkt.read_float().unwrap(), 2.5);
    assert_eq!(pkt.read_float().unwrap(), 3.75);
    assert_eq!(pkt.read_float().unwrap(), 4.0);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert!(!pkt.read_bit().unwrap());
    assert!(!pkt.read_bit().unwrap());
}

#[test]
fn move_update_teleport_writes_cpp_no_forces_no_speeds_field_order() {
    let guid = ObjectGuid::create_player(1, 43);
    let status = MovementInfo {
        guid,
        time: 987,
        position: Position::new(12.0, 13.0, 14.0, 1.25),
        ..MovementInfo::default()
    };
    let packet = MoveUpdateTeleport {
        status: status.clone(),
    };

    let bytes = packet.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::MoveUpdateTeleport as u16
    );
    let parsed_status = MovementInfo::read(&mut pkt).expect("MovementInfo status");
    assert_eq!(parsed_status.guid, guid);
    assert_eq!(parsed_status.time, 987);
    assert_eq!(parsed_status.position, status.position);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    for _ in 0..9 {
        assert!(!pkt.read_bit().unwrap());
    }
}

#[test]
fn movement_force_ack_packets_read_cpp_field_order() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let force_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 9, 88);
    let info = MovementInfo {
        guid: player_guid,
        time: 1234,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        ..MovementInfo::default()
    };
    let force = MovementForce {
        id: force_guid,
        origin: [1.0, 2.0, 3.0],
        direction: [4.0, 5.0, 6.0],
        transport_id: 7,
        magnitude: 8.5,
        unused_910: 9,
        force_type: MovementForceType::Gravity,
    };

    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);
    pkt.write_int32(33);
    force.write(&mut pkt);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let apply = MoveApplyMovementForceAck::read(&mut pkt).unwrap();
    assert_eq!(apply.ack.status.guid, player_guid);
    assert_eq!(apply.ack.ack_index, 33);
    assert_eq!(apply.force, force);

    let mut pkt = WorldPacket::new_empty();
    info.write(&mut pkt);
    pkt.write_int32(34);
    pkt.write_packed_guid(&force_guid);
    let mut pkt = WorldPacket::from_bytes(pkt.data());
    let remove = MoveRemoveMovementForceAck::read(&mut pkt).unwrap();
    assert_eq!(remove.ack.status.guid, player_guid);
    assert_eq!(remove.ack.ack_index, 34);
    assert_eq!(remove.id, force_guid);
}

#[test]
fn movement_ack_update_packets_write_cpp_field_order() {
    let guid = ObjectGuid::create_player(1, 42);
    let info = MovementInfo {
        guid,
        time: 1234,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        ..MovementInfo::default()
    };
    let force = MovementForce {
        id: ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 9, 88),
        origin: [1.0, 2.0, 3.0],
        direction: [4.0, 5.0, 6.0],
        transport_id: 7,
        magnitude: 8.5,
        unused_910: 9,
        force_type: MovementForceType::Gravity,
    };

    let update = MoveUpdateApplyMovementForce {
        status: info.clone(),
        force: force.clone(),
    };
    let bytes = update.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let decoded_info = MovementInfo::read(&mut pkt).unwrap();
    let decoded_force = MovementForce::read(&mut pkt).unwrap();
    assert_eq!(decoded_info.guid, guid);
    assert_eq!(decoded_force, force);

    let remove = MoveUpdateRemoveMovementForce {
        status: info.clone(),
        trigger_guid: force.id,
    };
    let bytes = remove.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let decoded_info = MovementInfo::read(&mut pkt).unwrap();
    let decoded_guid = pkt.read_packed_guid().unwrap();
    assert_eq!(decoded_info.guid, guid);
    assert_eq!(decoded_guid, force.id);

    let skipped = MoveSkipTime {
        mover_guid: guid,
        time_skipped: 250,
    };
    let bytes = skipped.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_uint32().unwrap(), 250);

    let magnitude = MoveUpdateModMovementForceMagnitude {
        status: info,
        speed: 1.25,
    };
    let bytes = magnitude.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let decoded_info = MovementInfo::read(&mut pkt).unwrap();
    assert_eq!(decoded_info.guid, guid);
    assert_eq!(pkt.read_float().unwrap(), 1.25);
}

#[test]
fn monster_move_single_destination_writes_cpp_spline_order() {
    let mover = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9, 88);
    let packet = MonsterMove::single_destination(
        mover,
        Position::new(1.0, 2.0, 3.0, 0.0),
        77,
        1_500,
        0x0040_0000,
        Position::new(10.0, 20.0, 30.0, 0.0),
    );
    let bytes = packet.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);

    assert_eq!(pkt.read_packed_guid().unwrap(), mover);
    assert_eq!(pkt.read_float().unwrap(), 1.0);
    assert_eq!(pkt.read_float().unwrap(), 2.0);
    assert_eq!(pkt.read_float().unwrap(), 3.0);
    assert_eq!(pkt.read_uint32().unwrap(), 77);
    assert_eq!(pkt.read_float().unwrap(), 10.0);
    assert_eq!(pkt.read_float().unwrap(), 20.0);
    assert_eq!(pkt.read_float().unwrap(), 30.0);
    // C++ MovementMonsterSpline writes CrzTeleport(1) + StopDistanceTolerance(3)
    // before the spline; the spline's first integer write flushes those
    // bits into their own byte before Flags.
    assert!(!pkt.has_bit().unwrap()); // CrzTeleport
    assert_eq!(pkt.read_bits(3).unwrap(), 0); // StopDistanceTolerance
    assert_eq!(pkt.read_uint32().unwrap(), 0x0040_0000); // Flags
    assert_eq!(pkt.read_int32().unwrap(), 0); // Elapsed
    assert_eq!(pkt.read_uint32().unwrap(), 1_500); // MoveTime
    assert_eq!(pkt.read_uint32().unwrap(), 0); // FadeObjectTime
    assert_eq!(pkt.read_uint8().unwrap(), 0); // Mode
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY); // TransportGUID
    assert_eq!(pkt.read_int8().unwrap(), -1); // VehicleSeat
    assert_eq!(pkt.read_bits(2).unwrap(), 0); // Face
    assert_eq!(pkt.read_bits(16).unwrap(), 1); // Points.len()
    assert!(!pkt.has_bit().unwrap()); // VehicleExitVoluntary
    assert!(!pkt.has_bit().unwrap()); // Interpolate
    assert_eq!(pkt.read_bits(16).unwrap(), 0); // PackedDeltas.len()
    assert!(!pkt.has_bit().unwrap()); // SplineFilter
    assert!(!pkt.has_bit().unwrap()); // SpellEffectExtraData
    assert!(!pkt.has_bit().unwrap()); // JumpExtraData
    assert!(!pkt.has_bit().unwrap()); // AnimTierTransition
    assert_eq!(pkt.read_float().unwrap(), 10.0);
    assert_eq!(pkt.read_float().unwrap(), 20.0);
    assert_eq!(pkt.read_float().unwrap(), 30.0);
    assert!(pkt.is_empty());
}

#[test]
fn monster_move_matches_real_cpp_waypoint_capture_bytes() {
    // Packet 89 from the 111,420-byte raw C++ source capture recorded by
    // `crates/capture-diff/flows/loot-single-item-claim/capture-provenance/
    // cpp.capture-manifest.json` as artifact `cpp.pkt` (SHA-256
    // a25f2c2bbf60de6cda7e32f305d732733017e711eb474dd5dbf6e007690143a8).
    // This is not the checked-in 795-byte normalized flow `cpp.pkt`
    // (SHA-256 a84abf8f1d067fc68a9fdbe1608243479b0740a703b07444097ab5af0aebf487).
    // It is a real instance-socket SMSG_ON_MONSTER_MOVE emitted by
    // MoveSplineInit::Launch for a compressed waypoint spline.
    const CPP_BODY: &[u8] = &[
        0x03, 0xBF, 0x0B, 0x01, 0x40, 0x3E, 0x15, 0x40, 0x42, 0x04, 0x20, 0xEC, 0xB9, 0x27, 0xC5,
        0xE1, 0xD2, 0x1F, 0x45, 0x12, 0xE5, 0x95, 0x42, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x11, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
        0x00, 0x00, 0x40, 0x00, 0xA0, 0x33, 0xB7, 0x27, 0xC5, 0xA4, 0x58, 0x22, 0x45, 0x04, 0xDB,
        0x95, 0x42, 0x00, 0x00, 0x02, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0x3F, 0x00, 0x00, 0x08, 0x3F, 0x00,
        0x00, 0x88, 0x3E, 0x00, 0x00, 0x08, 0x3E, 0x00, 0x00, 0x88, 0x3D, 0x00,
    ];
    let mover = ObjectGuid::new(0x2000_0442_4015_3E40, 0x010B);
    let packet = MonsterMove {
        mover_guid: mover,
        current_pos: Position::new(
            f32::from_bits(0xC527_B9EC),
            f32::from_bits(0x451F_D2E1),
            f32::from_bits(0x4295_E512),
            0.0,
        ),
        spline: MovementMonsterSpline {
            id: 6,
            movement: MovementSpline {
                flags: 0x0030_0000,
                move_time: 16_145,
                points: vec![Position::new(
                    f32::from_bits(0xC527_B733),
                    f32::from_bits(0x4522_58A4),
                    f32::from_bits(0x4295_DB04),
                    0.0,
                )],
                packed_deltas: vec![
                    [0.0, 16.0, 0.0],
                    [0.0, 12.0, 0.0],
                    [0.0, 8.0, 0.0],
                    [0.0, 4.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, -3.75, 0.0],
                    [0.0, -7.75, 0.0],
                    [0.0, -11.75, 0.0],
                    [0.0, -15.75, 0.0],
                    [0.0, -19.75, 0.0],
                ],
                ..MovementSpline::default()
            },
            ..MovementMonsterSpline::default()
        },
    };

    let bytes = packet.to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::OnMonsterMove as u16
    );
    assert_eq!(&bytes[2..], CPP_BODY);
}

#[test]
fn monster_move_stop_writes_cpp_stop_tolerance_without_done_flag() {
    let mover = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9, 88);
    let packet = MonsterMoveStop {
        mover_guid: mover,
        current_pos: Position::new(1.0, 2.0, 3.0, 0.0),
        spline_id: 78,
    };
    let bytes = packet.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);

    assert_eq!(pkt.read_packed_guid().unwrap(), mover);
    assert_eq!(pkt.read_float().unwrap(), 1.0);
    assert_eq!(pkt.read_float().unwrap(), 2.0);
    assert_eq!(pkt.read_float().unwrap(), 3.0);
    assert_eq!(pkt.read_uint32().unwrap(), 78);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert!(!pkt.has_bit().unwrap()); // CrzTeleport
    assert_eq!(pkt.read_bits(3).unwrap(), 2); // StopDistanceTolerance
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Flags
    assert_eq!(pkt.read_int32().unwrap(), 0); // Elapsed
    assert_eq!(pkt.read_uint32().unwrap(), 0); // MoveTime
    assert_eq!(pkt.read_uint32().unwrap(), 0); // FadeObjectTime
    assert_eq!(pkt.read_uint8().unwrap(), 0); // Mode
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY); // TransportGUID
    assert_eq!(pkt.read_int8().unwrap(), -1); // VehicleSeat
    assert_eq!(pkt.read_bits(2).unwrap(), 0); // Face
    assert_eq!(pkt.read_bits(16).unwrap(), 0); // Points.len()
    assert!(!pkt.has_bit().unwrap()); // VehicleExitVoluntary
    assert!(!pkt.has_bit().unwrap()); // Interpolate
    assert_eq!(pkt.read_bits(16).unwrap(), 0); // PackedDeltas.len()
    assert!(!pkt.has_bit().unwrap()); // SplineFilter
    assert!(!pkt.has_bit().unwrap()); // SpellEffectExtraData
    assert!(!pkt.has_bit().unwrap()); // JumpExtraData
    assert!(!pkt.has_bit().unwrap()); // AnimTierTransition
    assert!(pkt.is_empty());
}

#[test]
fn monster_move_writes_cpp_face_angle_and_packed_deltas() {
    let mover = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9, 88);
    let packet = MonsterMove {
        mover_guid: mover,
        current_pos: Position::new(0.0, 0.0, 0.0, 0.0),
        spline: MovementMonsterSpline {
            id: 79,
            destination: Position::new(12.0, 0.0, 0.0, 0.0),
            movement: MovementSpline {
                face: MonsterMoveFace::FacingAngle(1.25),
                points: vec![Position::new(12.0, 0.0, 0.0, 0.0)],
                packed_deltas: vec![[1.0, -2.0, 3.0]],
                ..MovementSpline::default()
            },
            ..MovementMonsterSpline::default()
        },
    };
    let bytes = packet.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);

    assert_eq!(pkt.read_packed_guid().unwrap(), mover);
    for _ in 0..3 {
        pkt.read_float().unwrap();
    }
    assert_eq!(pkt.read_uint32().unwrap(), 79);
    assert_eq!(pkt.read_float().unwrap(), 12.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert!(!pkt.has_bit().unwrap()); // CrzTeleport
    assert_eq!(pkt.read_bits(3).unwrap(), 0); // StopDistanceTolerance
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Flags
    assert_eq!(pkt.read_int32().unwrap(), 0); // Elapsed
    assert_eq!(pkt.read_uint32().unwrap(), 0); // MoveTime
    assert_eq!(pkt.read_uint32().unwrap(), 0); // FadeObjectTime
    assert_eq!(pkt.read_uint8().unwrap(), 0); // Mode
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY); // TransportGUID
    assert_eq!(pkt.read_int8().unwrap(), -1); // VehicleSeat
    assert_eq!(pkt.read_bits(2).unwrap(), 3); // Face = FacingAngle
    assert_eq!(pkt.read_bits(16).unwrap(), 1); // Points.len()
    assert!(!pkt.has_bit().unwrap()); // VehicleExitVoluntary
    assert!(!pkt.has_bit().unwrap()); // Interpolate
    assert_eq!(pkt.read_bits(16).unwrap(), 1); // PackedDeltas.len()
    assert!(!pkt.has_bit().unwrap()); // SplineFilter
    assert!(!pkt.has_bit().unwrap()); // SpellEffectExtraData
    assert!(!pkt.has_bit().unwrap()); // JumpExtraData
    assert!(!pkt.has_bit().unwrap()); // AnimTierTransition
    assert_eq!(pkt.read_float().unwrap(), 1.25);
    assert_eq!(pkt.read_float().unwrap(), 12.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    let expected_packed = ((1.0f32 / 0.25) as i32 as u32 & 0x7ff)
        | (((-2.0f32 / 0.25) as i32 as u32 & 0x7ff) << 11)
        | (((3.0f32 / 0.25) as i32 as u32 & 0x3ff) << 22);
    assert_eq!(pkt.read_uint32().unwrap(), expected_packed);
    assert!(pkt.is_empty());
}

#[test]
fn movement_spline_writes_cpp_optional_payload_order() {
    let target = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    MovementSpline {
        spline_filter: Some(MonsterSplineFilter {
            filter_keys: vec![MonsterSplineFilterKey {
                index: -2,
                speed: 35,
            }],
            filter_flags: 3,
            base_speed: 1.5,
            start_offset: -4,
            dist_to_prev_filter_key: 2.5,
            added_to_start: 6,
        }),
        points: vec![Position::xyz(4.0, 5.0, 6.0)],
        spell_effect_extra: Some(MonsterSplineSpellEffectExtraData {
            target_guid: target,
            spell_visual_id: 11,
            progress_curve_id: 22,
            parabolic_curve_id: 33,
            jump_gravity: 44.5,
        }),
        jump_extra: Some(MonsterSplineJumpExtraData {
            jump_gravity: 9.25,
            start_time: 55,
            duration: 66,
        }),
        anim_tier_transition: Some(MonsterSplineAnimTierTransition {
            tier_transition_id: -77,
            start_time: 88,
            end_time: 99,
            anim_tier: 3,
        }),
        ..MovementSpline::default()
    }
    .write(&mut pkt);
    let mut pkt = WorldPacket::from_bytes(pkt.data());

    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_int8().unwrap(), -1);
    assert_eq!(pkt.read_bits(2).unwrap(), 0);
    assert_eq!(pkt.read_bits(16).unwrap(), 1);
    assert!(!pkt.has_bit().unwrap());
    assert!(!pkt.has_bit().unwrap());
    assert_eq!(pkt.read_bits(16).unwrap(), 0);
    assert!(pkt.has_bit().unwrap());
    assert!(pkt.has_bit().unwrap());
    assert!(pkt.has_bit().unwrap());
    assert!(pkt.has_bit().unwrap());

    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_float().unwrap(), 1.5);
    assert_eq!(pkt.read_int16().unwrap(), -4);
    assert_eq!(pkt.read_float().unwrap(), 2.5);
    assert_eq!(pkt.read_int16().unwrap(), 6);
    assert_eq!(pkt.read_int16().unwrap(), -2);
    assert_eq!(pkt.read_uint16().unwrap(), 35);
    assert_eq!(pkt.read_bits(2).unwrap(), 3);

    assert_eq!(pkt.read_float().unwrap(), 4.0);
    assert_eq!(pkt.read_float().unwrap(), 5.0);
    assert_eq!(pkt.read_float().unwrap(), 6.0);

    assert_eq!(pkt.read_packed_guid().unwrap(), target);
    assert_eq!(pkt.read_uint32().unwrap(), 11);
    assert_eq!(pkt.read_uint32().unwrap(), 22);
    assert_eq!(pkt.read_uint32().unwrap(), 33);
    assert_eq!(pkt.read_float().unwrap(), 44.5);
    assert_eq!(pkt.read_float().unwrap(), 9.25);
    assert_eq!(pkt.read_uint32().unwrap(), 55);
    assert_eq!(pkt.read_uint32().unwrap(), 66);
    assert_eq!(pkt.read_int32().unwrap(), -77);
    assert_eq!(pkt.read_uint32().unwrap(), 88);
    assert_eq!(pkt.read_uint32().unwrap(), 99);
    assert_eq!(pkt.read_uint8().unwrap(), 3);
    assert!(pkt.is_empty());
}

#[test]
fn movement_monster_spline_from_move_spline_matches_cpp_mapping() {
    let target = ObjectGuid::create_player(1, 55);
    let args = MoveSplineInitArgs {
        path: vec![
            Position::xyz(0.0, 0.0, 0.0),
            Position::xyz(10.0, 0.0, 0.0),
            Position::xyz(20.0, 0.0, 0.0),
        ],
        facing: FacingInfo {
            kind: MonsterMoveType::FacingTarget,
            target,
            angle: 1.75,
            ..FacingInfo::default()
        },
        flags: MoveSplineFlag::UNCOMPRESSED_PATH | MoveSplineFlag::PARABOLIC,
        velocity: 10.0,
        vertical_acceleration: 12.5,
        effect_start_time_ms: 250,
        spline_id: 123,
        spell_effect_extra: Some(SpellEffectExtraData {
            target,
            spell_visual_id: 777,
            progress_curve_id: 888,
            parabolic_curve_id: 999,
        }),
        ..MoveSplineInitArgs::default()
    };
    let mut move_spline = MoveSpline::new();
    move_spline.initialize(&args).unwrap();
    move_spline.finalize();

    let packet_spline = MovementMonsterSpline::from_move_spline(&move_spline);

    assert_eq!(packet_spline.id, 123);
    assert_eq!(packet_spline.destination, Position::ZERO);
    assert_eq!(
        packet_spline.movement.flags,
        (MoveSplineFlag::UNCOMPRESSED_PATH | MoveSplineFlag::PARABOLIC).bits()
    );
    assert_eq!(
        packet_spline.movement.face,
        MonsterMoveFace::FacingTarget {
            direction: 1.75,
            target_guid: target,
        }
    );
    assert_eq!(
        packet_spline.movement.points,
        vec![Position::xyz(10.0, 0.0, 0.0), Position::xyz(20.0, 0.0, 0.0)]
    );
    assert!(packet_spline.movement.packed_deltas.is_empty());
    assert_eq!(
        packet_spline.movement.spell_effect_extra,
        Some(MonsterSplineSpellEffectExtraData {
            target_guid: target,
            spell_visual_id: 777,
            progress_curve_id: 888,
            parabolic_curve_id: 999,
            jump_gravity: 12.5,
        })
    );
    assert_eq!(
        packet_spline.movement.jump_extra,
        Some(MonsterSplineJumpExtraData {
            jump_gravity: 12.5,
            start_time: 250,
            duration: 0,
        })
    );
}

#[test]
fn movement_monster_spline_from_move_spline_maps_animation_tier_like_cpp() {
    let mut flags = MoveSplineFlag::empty();
    flags.enable_animation();
    let args = MoveSplineInitArgs {
        path: vec![Position::xyz(0.0, 0.0, 0.0), Position::xyz(10.0, 0.0, 0.0)],
        flags,
        velocity: 10.0,
        effect_start_time_ms: 125,
        anim_tier: Some(AnimTierTransition {
            tier_transition_id: 44,
            anim_tier: 2,
        }),
        ..MoveSplineInitArgs::default()
    };
    let mut move_spline = MoveSpline::new();
    move_spline.initialize(&args).unwrap();

    let packet_spline = MovementMonsterSpline::from_move_spline(&move_spline);

    assert_eq!(
        packet_spline.movement.anim_tier_transition,
        Some(MonsterSplineAnimTierTransition {
            tier_transition_id: 44,
            start_time: 125,
            end_time: 0,
            anim_tier: 2,
        })
    );
    assert!(packet_spline.movement.jump_extra.is_none());
}

#[test]
fn move_set_collision_height_matches_cpp_opcode_and_tail() {
    let guid = ObjectGuid::create_player(1, 42);
    let pkt = MoveSetCollisionHeight {
        mover_guid: guid,
        sequence_index: 7,
        height: 1.5,
        scale: 1.0,
        reason: UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP,
        mount_display_id: 4321,
        scale_duration: 0,
    };
    let bytes = pkt.to_bytes();

    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2e13);
    assert!(bytes.len() > 23);
    assert_eq!(
        &bytes[bytes.len() - 21..bytes.len() - 17],
        &7u32.to_le_bytes()
    );
    assert_eq!(
        &bytes[bytes.len() - 17..bytes.len() - 13],
        &1.5f32.to_le_bytes()
    );
    assert_eq!(
        &bytes[bytes.len() - 13..bytes.len() - 9],
        &1.0f32.to_le_bytes()
    );
    assert_eq!(
        bytes[bytes.len() - 9],
        UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP
    );
    assert_eq!(
        &bytes[bytes.len() - 8..bytes.len() - 4],
        &4321u32.to_le_bytes()
    );
    assert_eq!(&bytes[bytes.len() - 4..], &0i32.to_le_bytes());
}

#[test]
fn move_update_collision_height_matches_cpp_opcode_and_tail() {
    let guid = ObjectGuid::create_player(1, 42);
    let pkt = MoveUpdateCollisionHeight {
        status: MovementInfo {
            guid,
            position: Position::xyz(1.0, 2.0, 3.0),
            time: 77,
            ..MovementInfo::default()
        },
        height: 1.5,
        scale: 1.0,
    };
    let bytes = pkt.to_bytes();

    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2ddf);
    assert!(bytes.len() > 10);
    assert_eq!(
        &bytes[bytes.len() - 8..bytes.len() - 4],
        &1.5f32.to_le_bytes()
    );
    assert_eq!(&bytes[bytes.len() - 4..], &1.0f32.to_le_bytes());
}

#[test]
fn move_spline_set_speed_matches_cpp_opcode_and_tail() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 123, 456);
    let bytes = MoveSplineSetSpeed {
        opcode: ServerOpcodes::MoveSplineSetRunSpeed,
        mover_guid: guid,
        speed: 14.0,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::MoveSplineSetRunSpeed as u16
    );
    assert!(bytes.len() > 6);
    assert_eq!(&bytes[bytes.len() - 4..], &14.0f32.to_le_bytes());
}

#[test]
fn move_set_flag_matches_cpp_opcode_and_tail() {
    let guid = ObjectGuid::create_player(1, 42);
    let bytes = MoveSetFlag {
        opcode: ServerOpcodes::MoveSetCanFly,
        mover_guid: guid,
        sequence_index: 77,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::MoveSetCanFly as u16
    );
    assert!(bytes.len() > 6);
    assert_eq!(&bytes[bytes.len() - 4..], &77u32.to_le_bytes());
}
