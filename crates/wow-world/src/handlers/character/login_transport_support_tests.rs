use super::*;
use wow_entities::GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT as GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP;
use wow_packet::packets::movement::TransportInfo;

#[test]
fn init_self_orders_transport_attached_player_and_fellow_passenger_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let passenger_guid = ObjectGuid::create_player(1, 43);
    let transport_guid = ObjectGuid::create_transport(HighGuid::Transport, 7_001);
    let mut player_update = UpdateObject::create_player(
        player_guid,
        1,
        8,
        0,
        80,
        49,
        &Position::ZERO,
        571,
        0,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    player_update.set_player_movement_transport_like_cpp(TransportInfo {
        guid: transport_guid,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        o: 0.5,
        seat: -1,
        time: 0,
        prev_time: None,
        vehicle_id: None,
    });
    let transport_block = UpdateObject::create_transport_block(
        GameObjectCreateData {
            guid: transport_guid,
            entry: 1,
            dynamic_flags: 0,
            display_id: 2,
            go_type: GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP,
            position: Position::ZERO,
            rotation: [0.0, 0.0, 0.0, 1.0],
            anim_progress: 255,
            state: wow_entities::GoState::Ready as i8,
            art_kit: 0,
            created_by: ObjectGuid::EMPTY,
            faction_template: 0,
            gameobject_flags: 0x0010_0028,
            world_effect_id: 0,
            scale: 1.0,
            level: 1_000,
            parent_rotation: [0.0, 0.0, 0.0, 1.0],
        },
        0,
    );
    let mut passenger_update = UpdateObject::create_player(
        passenger_guid,
        1,
        8,
        0,
        80,
        49,
        &Position::ZERO,
        571,
        0,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let passenger_block = passenger_update
        .blocks
        .pop()
        .expect("fellow passenger CREATE");

    assert_eq!(
        compose_init_self_create_blocks_like_cpp(
            &mut player_update,
            Vec::new(),
            Some((transport_guid, transport_block)),
            vec![passenger_block],
        ),
        Some(transport_guid)
    );
    assert_eq!(player_update.num_updates, 3);
    assert!(matches!(
        player_update.blocks.first(),
        Some(UpdateBlock::CreateTransport { guid, .. }) if *guid == transport_guid
    ));
    let Some(UpdateBlock::CreateObject {
        guid,
        movement: Some(movement),
        is_self: true,
        ..
    }) = player_update.blocks.get(1)
    else {
        panic!("expected attached self player after its transport");
    };
    assert_eq!(*guid, player_guid);
    assert_eq!(
        movement.transport.as_ref().map(|transport| transport.guid),
        Some(transport_guid)
    );
    assert!(matches!(
        player_update.blocks.get(2),
        Some(UpdateBlock::CreateObject {
            guid,
            is_self: false,
            ..
        }) if *guid == passenger_guid
    ));
}

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
