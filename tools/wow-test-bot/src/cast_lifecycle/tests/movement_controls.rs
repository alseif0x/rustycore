use super::*;

fn movement_controller() -> Controller {
    Controller::new(
        Options {
            source_path: "movement-unit-test".to_string(),
            plan: Plan {
                character_guid: None,
                observe_only: true,
                timeout_ms: 1_000,
                actions: Vec::new(),
                expect: Vec::new(),
                finish_policy: FinishPolicy::Logout,
            },
        },
        17,
        realm_id(),
    )
}

fn test_player_mover() -> Guid {
    let (low, high) = create_player_guid_raw(17, realm_id());
    Guid { low, high }
}

fn seed_movement_login(controller: &mut Controller) -> Vec<Outbound> {
    let mut payload = Vec::new();
    payload.extend(530i32.to_le_bytes());
    for coordinate in [1.25f32, -2.5, 8.0, 0.75] {
        payload.extend(coordinate.to_le_bytes());
    }
    payload.extend(0u32.to_le_bytes());
    controller
        .process_packet(Connection::Instance, SMSG_LOGIN_VERIFY_WORLD, &payload)
        .unwrap()
}

fn set_flag_payload(mover: Guid, sequence: u32) -> Vec<u8> {
    let mut payload = mover.packed();
    payload.extend(sequence.to_le_bytes());
    payload
}

fn expected_movement_ack(mover: Guid, flags: u32, sequence: u32, speed: Option<f32>) -> Vec<u8> {
    let mut body =
        build_client_movement_payload(mover.low, mover.high, flags, 0, 1.25, -2.5, 8.0, 0.75);
    body.extend(sequence.to_le_bytes());
    if let Some(speed) = speed {
        body.extend(speed.to_le_bytes());
    }
    body
}

#[test]
fn login_transfer_and_worldport_controls_use_cpp_routes_and_order() {
    let mut controller = movement_controller();
    let login_ack = seed_movement_login(&mut controller);
    assert_eq!(login_ack.len(), 1);
    assert_eq!(login_ack[0].connection, Connection::Instance);
    assert_eq!(login_ack[0].opcode, 0x3A46);
    assert_eq!(login_ack[0].body, 0u32.to_le_bytes().to_vec());

    // Repeated observation cannot send the login active-mover completion twice.
    assert!(seed_movement_login(&mut controller).is_empty());

    assert!(controller
        .process_packet(Connection::Realm, SMSG_TRANSFER_PENDING, &[])
        .unwrap()
        .is_empty());
    assert!(controller
        .process_packet(Connection::Instance, SMSG_TRANSFER_PENDING, &[])
        .is_err());

    // ResumeToken carries the same five-byte server shape but has no client ACK.
    let mut resume = 0x5566_7788u32.to_le_bytes().to_vec();
    resume.push(0x40);
    assert!(controller
        .process_packet(Connection::Instance, SMSG_RESUME_TOKEN, &resume)
        .unwrap()
        .is_empty());
    assert!(controller
        .process_packet(Connection::Realm, SMSG_RESUME_TOKEN, &resume)
        .is_err());

    let sequence = 0x1122_3344u32;
    let mut suspend = sequence.to_le_bytes().to_vec();
    suspend.push(0x80); // the two high reason bits; six padding bits are zero
    let response = controller
        .process_packet(Connection::Instance, SMSG_SUSPEND_TOKEN, &suspend)
        .unwrap();
    assert_eq!(response.len(), 1);
    assert_eq!(response[0].connection, Connection::Instance);
    assert_eq!(response[0].opcode, 0x376A);
    assert_eq!(response[0].body, sequence.to_le_bytes());
    assert!(controller
        .process_packet(Connection::Realm, SMSG_SUSPEND_TOKEN, &suspend)
        .is_err());

    // NewWorld::Write: map, XYZO, two unused u32s, reason, movement offset XYZ.
    let mut new_world = Vec::new();
    new_world.extend(1i32.to_le_bytes());
    for value in [100.0f32, 200.0, 30.0, 1.5] {
        new_world.extend(value.to_le_bytes());
    }
    new_world.extend((-1i32).to_le_bytes()); // TeleportLocation::Unused901_1 default
    new_world.extend((-1i32).to_le_bytes()); // TeleportLocation::Unused901_2 default
    new_world.extend(2u32.to_le_bytes());
    new_world.extend([0u8; 12]);
    let worldport = controller
        .process_packet(Connection::Realm, SMSG_NEW_WORLD, &new_world)
        .unwrap();
    assert_eq!(worldport.len(), 2);
    // The server accepts this ACK on either socket, but one instance TCP
    // stream preserves WorldPortResponse-before-visibility-ACK ordering.
    assert_eq!(
        worldport
            .iter()
            .map(|packet| packet.connection)
            .collect::<Vec<_>>(),
        [Connection::Instance, Connection::Instance]
    );
    assert_eq!(worldport[0].connection, Connection::Instance);
    assert_eq!(worldport[0].opcode, 0x35FA);
    assert!(worldport[0].body.is_empty());
    assert_eq!(worldport[1].connection, Connection::Instance);
    assert_eq!(worldport[1].opcode, 0x3A46);
    assert_eq!(worldport[1].body, 0u32.to_le_bytes().to_vec());
    assert_eq!(controller.evidence.movement_state.unwrap().map_id, Some(1));
    assert_eq!(
        controller
            .evidence
            .movement_state
            .unwrap()
            .active_mover_complete_count,
        2
    );
    assert!(controller
        .process_packet(Connection::Instance, SMSG_NEW_WORLD, &new_world)
        .is_err());
}

#[test]
fn initial_controller_failure_handoff_retains_the_failing_packet() {
    let mut controller = movement_controller();
    let failed_body = [0xAA, 0xBB];
    assert!(controller
        .record_and_process_packet(Connection::Instance, SMSG_SUSPEND_TOKEN, &failed_body,)
        .is_err());

    let mut result = BotRunResult::default();
    preserve_controller_failure(
        controller,
        &mut result,
        "initial controller failure".to_string(),
    );
    let evidence = result.cast_lifecycle.expect("failure evidence handoff");
    assert_eq!(evidence.events.len(), 1);
    assert_eq!(evidence.events[0].opcode, SMSG_SUSPEND_TOKEN);
    assert_eq!(evidence.events[0].body_hex, "aabb");
    assert!(evidence.events[0].error.is_some());
    assert!(evidence
        .failure
        .as_deref()
        .is_some_and(|failure| failure.contains("movement control decode failed")));
}

#[test]
fn root_walk_and_speed_controls_emit_exact_movementinfo_ack_bytes() {
    let mut controller = movement_controller();
    seed_movement_login(&mut controller);
    let mover = test_player_mover();
    assert!(controller
        .process_packet(Connection::Realm, SMSG_MOVE_ROOT, &[])
        .is_err());
    assert!(controller
        .process_packet(Connection::Realm, SMSG_MOVE_SET_RUN_SPEED, &[])
        .is_err());
    let controls = [
        (SMSG_MOVE_ROOT, 0x3A0E, 11, 0x0000_0400, None),
        (SMSG_MOVE_UNROOT, 0x3A0F, 12, 0, None),
        (SMSG_MOVE_SET_WATER_WALK, 0x3A1D, 13, 0x0400_0000, None),
        (SMSG_MOVE_SET_LAND_WALK, 0x3A1D, 14, 0, None),
        (SMSG_MOVE_SET_RUN_SPEED, 0x3A0B, 15, 0, Some(7.0f32)),
        (SMSG_MOVE_SET_SWIM_SPEED, 0x3A0D, 16, 0, Some(4.5f32)),
    ];
    for (incoming, response_opcode, sequence, flags, speed) in controls {
        let mut payload = set_flag_payload(mover, sequence);
        if let Some(speed) = speed {
            payload.extend(speed.to_le_bytes());
        }
        let response = controller
            .process_packet(Connection::Instance, incoming, &payload)
            .unwrap();
        assert_eq!(response.len(), 1);
        assert_eq!(response[0].connection, Connection::Instance);
        assert_eq!(response[0].opcode, response_opcode);
        assert_eq!(
            response[0].body,
            expected_movement_ack(mover, flags, sequence, speed)
        );
    }

    let mut wrong_mover = set_flag_payload(Guid { low: 18, high: 0 }, 1);
    assert!(controller
        .process_packet(Connection::Instance, SMSG_MOVE_ROOT, &wrong_mover)
        .is_err());
    wrong_mover = set_flag_payload(mover, 1);
    wrong_mover.push(0); // exact MoveSetFlag has no trailing fields
    assert!(controller
        .process_packet(Connection::Instance, SMSG_MOVE_ROOT, &wrong_mover)
        .is_err());
}

#[test]
fn root_clears_forward_and_fall_state_before_building_root_ack() {
    let mut controller = movement_controller();
    seed_movement_login(&mut controller);
    let mover = test_player_mover();
    controller
        .movement_action(&Action::Movement {
            kind: MovementKind::StartForward,
            time_ms: 100,
            x: 1.25,
            y: -2.5,
            z: 8.0,
            orientation: 0.75,
            fall: None,
        })
        .unwrap();
    controller
        .movement_action(&Action::Movement {
            kind: MovementKind::Heartbeat,
            time_ms: 101,
            x: 1.25,
            y: -2.5,
            z: 4.0,
            orientation: 0.75,
            fall: Some(FallData {
                fall_time_ms: 800,
                z_speed: -2.0,
                sin_angle: 0.0,
                cos_angle: 1.0,
                xy_speed: 0.0,
            }),
        })
        .unwrap();

    let sequence = 77;
    let response = controller
        .process_packet(
            Connection::Instance,
            SMSG_MOVE_ROOT,
            &set_flag_payload(mover, sequence),
        )
        .unwrap();
    assert_eq!(response.len(), 1);
    let mut expected = build_client_movement_payload(
        mover.low,
        mover.high,
        0x0000_0400,
        101,
        1.25,
        -2.5,
        4.0,
        0.75,
    );
    expected.extend(sequence.to_le_bytes());
    assert_eq!(response[0].opcode, 0x3A0E);
    assert_eq!(response[0].body, expected);
    assert_eq!(
        controller.evidence.movement_state.unwrap().flags,
        0x0000_0400
    );
}

#[test]
fn unsupported_movement_controls_fail_instead_of_being_ignored() {
    let mut controller = movement_controller();
    seed_movement_login(&mut controller);
    for opcode in [
        0x2E05, // can-fly
        0x2DFF, // feather-fall
        0x2DF5, // flight back speed
        0x2DF4, // flight speed
        0x2E01, // hovering
        0x2E00, // normal fall
        0x2DF8, // pitch rate
        0x2DF1, // run back speed
        0x2DF3, // swim back speed
        0x2DF7, // turn rate
        0x2DF6, // walk speed
    ] {
        let error = controller
            .process_packet(Connection::Instance, opcode, &[])
            .unwrap_err();
        assert!(
            error.to_string().contains("unsupported movement control"),
            "opcode 0x{opcode:04X} was silently accepted: {error}"
        );
    }
}

#[test]
fn near_teleport_and_typed_movement_preserve_control_flags_and_fall_layout() {
    let mut controller = movement_controller();
    seed_movement_login(&mut controller);
    let mover = test_player_mover();
    for (opcode, sequence) in [(SMSG_MOVE_ROOT, 1), (SMSG_MOVE_SET_WATER_WALK, 2)] {
        controller
            .process_packet(
                Connection::Instance,
                opcode,
                &set_flag_payload(mover, sequence),
            )
            .unwrap();
    }
    let rooted_start = Action::Movement {
        kind: MovementKind::StartForward,
        time_ms: 100,
        x: 1.25,
        y: -2.5,
        z: 8.0,
        orientation: 0.75,
        fall: None,
    };
    assert!(controller.movement_action(&rooted_start).is_err());

    controller
        .process_packet(
            Connection::Instance,
            SMSG_MOVE_UNROOT,
            &set_flag_payload(mover, 3),
        )
        .unwrap();
    let (start_opcode, start_body) = controller.movement_action(&rooted_start).unwrap();
    assert_eq!(start_opcode, 0x39E4);
    let expected_start = build_client_movement_payload(
        mover.low,
        mover.high,
        0x0400_0001,
        100,
        1.25,
        -2.5,
        8.0,
        0.75,
    );
    assert_eq!(start_body, expected_start);

    let (stop_opcode, stop_body) = controller
        .movement_action(&Action::Movement {
            kind: MovementKind::Stop,
            time_ms: 101,
            x: 1.5,
            y: -2.0,
            z: 8.0,
            orientation: 1.0,
            fall: None,
        })
        .unwrap();
    assert_eq!(stop_opcode, 0x39E6);
    assert_eq!(
        u32::from_le_bytes(
            stop_body[mover.packed().len()..mover.packed().len() + 4]
                .try_into()
                .unwrap()
        ),
        0x0400_0000,
        "stop clears motion while retaining water-walking control"
    );

    let fall = FallData {
        fall_time_ms: 800,
        z_speed: -7.5,
        sin_angle: 0.0,
        cos_angle: 1.0,
        xy_speed: 0.0,
    };
    let (heartbeat_opcode, heartbeat) = controller
        .movement_action(&Action::Movement {
            kind: MovementKind::Heartbeat,
            time_ms: 102,
            x: 1.5,
            y: -2.0,
            z: 4.0,
            orientation: 1.0,
            fall: Some(fall),
        })
        .unwrap();
    assert_eq!(heartbeat_opcode, 0x3A10);
    let mut expected_fall =
        build_client_movement_payload(mover.low, mover.high, 0x0400_0800, 102, 1.5, -2.0, 4.0, 1.0);
    *expected_fall.last_mut().unwrap() |= 0x20;
    expected_fall.extend(fall.fall_time_ms.to_le_bytes());
    expected_fall.extend(fall.z_speed.to_le_bytes());
    expected_fall.push(0x80);
    expected_fall.extend(fall.sin_angle.to_le_bytes());
    expected_fall.extend(fall.cos_angle.to_le_bytes());
    expected_fall.extend(fall.xy_speed.to_le_bytes());
    assert_eq!(heartbeat, expected_fall);

    let (fall_land_opcode, fall_land) = controller
        .movement_action(&Action::Movement {
            kind: MovementKind::FallLand,
            time_ms: 902,
            x: 1.5,
            y: -2.0,
            z: 0.0,
            orientation: 1.0,
            fall: Some(fall),
        })
        .unwrap();
    assert_eq!(fall_land_opcode, 0x39FB);
    let mut expected_fall_land =
        build_client_movement_payload(mover.low, mover.high, 0x0400_0800, 902, 1.5, -2.0, 0.0, 1.0);
    *expected_fall_land.last_mut().unwrap() |= 0x20;
    expected_fall_land.extend(fall.fall_time_ms.to_le_bytes());
    expected_fall_land.extend(fall.z_speed.to_le_bytes());
    expected_fall_land.push(0x80);
    expected_fall_land.extend(fall.sin_angle.to_le_bytes());
    expected_fall_land.extend(fall.cos_angle.to_le_bytes());
    expected_fall_land.extend(fall.xy_speed.to_le_bytes());
    assert_eq!(fall_land, expected_fall_land);
    assert_eq!(
        controller.evidence.movement_state.unwrap().flags,
        0x0400_0000
    );

    let mut teleport = mover.packed();
    teleport.extend(0xA1B2_C3D4u32.to_le_bytes());
    for value in [3.0f32, 4.0, 5.0, 1.25] {
        teleport.extend(value.to_le_bytes());
    }
    teleport.extend([0u8, 0u8]); // preload world=false, no optional transport/vehicle data
    let response = controller
        .process_packet(Connection::Instance, SMSG_MOVE_TELEPORT, &teleport)
        .unwrap();
    assert_eq!(response.len(), 1);
    assert_eq!(response[0].connection, Connection::Instance);
    assert_eq!(response[0].opcode, 0x39FA);
    let mut expected_teleport_ack = mover.packed();
    expected_teleport_ack.extend(0xA1B2_C3D4u32.to_le_bytes());
    expected_teleport_ack.extend(902u32.to_le_bytes());
    assert_eq!(response[0].body, expected_teleport_ack);
    assert_eq!(
        controller.evidence.movement_state.unwrap().map_id,
        Some(530)
    );

    for optional_bits in [0x40, 0x80] {
        let mut unsupported_teleport = mover.packed();
        unsupported_teleport.extend(9u32.to_le_bytes());
        for value in [3.0f32, 4.0, 5.0, 1.25] {
            unsupported_teleport.extend(value.to_le_bytes());
        }
        unsupported_teleport.extend([0, optional_bits]);
        let error = controller
            .process_packet(
                Connection::Instance,
                SMSG_MOVE_TELEPORT,
                &unsupported_teleport,
            )
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("transport and vehicle data are unsupported"));
    }
}

#[test]
fn typed_movement_rejects_unsupported_kinds_and_non_finite_fall_data() {
    let unsupported: std::result::Result<Action, _> = serde_json::from_str(
        r#"{"action":"movement","kind":"teleport","x":0.0,"y":0.0,"z":0.0,"orientation":0.0}"#,
    );
    assert!(unsupported.is_err());

    let mut plan = Plan {
        character_guid: None,
        observe_only: false,
        timeout_ms: 1_000,
        actions: vec![Action::Movement {
            kind: MovementKind::FallLand,
            time_ms: 10,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: 0.0,
            fall: None,
        }],
        expect: vec![ExpectedRule::Packet {
            connection: Connection::Instance,
            opcode: SMSG_SPELL_GO,
            body_hex: String::new(),
        }],
        finish_policy: FinishPolicy::Logout,
    };
    assert!(plan::validate_plan(&plan).is_err());
    plan.actions[0] = Action::Movement {
        kind: MovementKind::Heartbeat,
        time_ms: 10,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        orientation: 0.0,
        fall: Some(FallData {
            fall_time_ms: 10,
            z_speed: f32::NAN,
            sin_angle: 0.0,
            cos_angle: 1.0,
            xy_speed: 0.0,
        }),
    };
    assert!(plan::validate_plan(&plan).is_err());
}
