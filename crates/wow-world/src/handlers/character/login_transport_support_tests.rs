use super::*;

#[test]
fn persisted_transport_login_resolves_valid_offset_to_current_world_position_like_cpp() {
    let guid = ObjectGuid::create_transport(HighGuid::Transport, 7_002);
    let offset = Position::new(10.0, 20.0, 3.0, 0.25);
    let transport_create = MapTransportCreateLikeCpp {
        guid_low: 7_002,
        entry: 192_241,
        display_id: 3_012,
        scale: 1.0,
        taxi_path_id: 784,
        move_speed: 30,
        accel_rate: 10,
        allow_stopping: false,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        gameobject_flags: 0,
        faction_template: 0,
    };
    let transport_position = TransportCreatePositionLikeCpp {
        map_id: 571,
        position: Position::new(100.0, 200.0, 10.0, PI / 2.0),
        timer_ms: 1,
        total_time_ms: 2,
    };
    let resolved = validate_persisted_transport_login_like_cpp(
        guid,
        offset,
        transport_position,
        transport_create,
    )
    .expect("valid passenger attachment");

    assert_eq!(resolved.guid, guid);
    assert_eq!(resolved.map_id, 571);
    assert_eq!(resolved.offset, offset);
    assert_eq!(
        resolved.transport_create, transport_create,
        "SendInitSelf must not need a second DB query for the own transport CREATE"
    );
    assert_eq!(
        resolved.transport_position, transport_position,
        "SendInitSelf must reuse the validated path-time snapshot"
    );
    assert!((resolved.world_position.x - 80.0).abs() < 0.001);
    assert!((resolved.world_position.y - 210.0).abs() < 0.001);
    assert!((resolved.world_position.z - 13.0).abs() < 0.001);
    assert!((resolved.world_position.orientation - (PI / 2.0 + 0.25)).abs() < 0.001);
}

#[test]
fn persisted_transport_login_rejects_corrupt_offsets_and_world_coordinates_like_cpp() {
    let guid = ObjectGuid::create_transport(HighGuid::Transport, 7_003);
    let transport_create = MapTransportCreateLikeCpp {
        guid_low: 7_003,
        entry: 192_241,
        display_id: 3_012,
        scale: 1.0,
        taxi_path_id: 784,
        move_speed: 30,
        accel_rate: 10,
        allow_stopping: false,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        gameobject_flags: 0,
        faction_template: 0,
    };
    let valid_transport = TransportCreatePositionLikeCpp {
        map_id: 571,
        position: Position::ZERO,
        timer_ms: 1,
        total_time_ms: 2,
    };

    for invalid_offset in [
        Position::new(250.01, 0.0, 0.0, 0.0),
        Position::new(0.0, -250.01, 0.0, 0.0),
        Position::new(0.0, 0.0, f32::INFINITY, 0.0),
        Position::new(0.0, 0.0, 0.0, f32::NAN),
    ] {
        assert!(
            validate_persisted_transport_login_like_cpp(
                guid,
                invalid_offset,
                valid_transport,
                transport_create,
            )
            .is_none()
        );
    }

    assert!(
        validate_persisted_transport_login_like_cpp(
            guid,
            Position::new(1.0, 0.0, 0.0, 0.0),
            TransportCreatePositionLikeCpp {
                position: Position::new(Position::MAP_HALFSIZE_LIKE_CPP, 0.0, 0.0, 0.0,),
                ..valid_transport
            },
            transport_create,
        )
        .is_none(),
        "C++ rejects an attachment whose calculated world coordinate is outside the map"
    );
}

#[test]
fn persisted_transport_login_requires_saved_map_in_transport_route_like_cpp() {
    assert!(transport_route_contains_saved_map_like_cpp(
        [0, 1, 571],
        571
    ));
    assert!(
        !transport_route_contains_saved_map_like_cpp([0, 1, 571], 530),
        "C++ GetTransport(savedMap) rejects a same-GUID transport absent from that map"
    );
}
