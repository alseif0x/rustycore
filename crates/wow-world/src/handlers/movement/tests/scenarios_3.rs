//! Movement handlers regression scenarios, part 3 of 3.
//!
//! Moved out of the movement.rs root under #654; every test is unchanged.

use super::*;

#[test]
fn transport_membership_reconciles_map_owner_on_attach_switch_and_detach_like_cpp() {
    let mut session = make_session();
    let canonical = std::sync::Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let player_guid = ObjectGuid::create_player(1, 1_080);
    let first_transport = ObjectGuid::create_transport(HighGuid::Transport, 1_081);
    let second_transport = ObjectGuid::create_transport(HighGuid::Transport, 1_082);
    install_canonical_player_and_transports_for_test(
        &canonical,
        player_guid,
        &[first_transport, second_transport],
    );
    session.set_canonical_map_manager(std::sync::Arc::clone(&canonical));
    session.set_player_guid(Some(player_guid));

    assert_eq!(
        session.reconcile_player_transport_membership_like_cpp(player_guid, Some(first_transport),),
        crate::session::MovementTransportMembershipLikeCpp::Attached(first_transport)
    );
    {
        let manager = canonical.lock().unwrap();
        let map = manager.find_map(571, 0).unwrap().map();
        assert!(
            map.get_typed_transport_like_cpp(first_transport)
                .unwrap()
                .passengers()
                .contains(&player_guid)
        );
    }

    assert_eq!(
        session
            .reconcile_player_transport_membership_like_cpp(player_guid, Some(second_transport),),
        crate::session::MovementTransportMembershipLikeCpp::Attached(second_transport)
    );
    let manager = canonical.lock().unwrap();
    let map = manager.find_map(571, 0).unwrap().map();
    assert!(
        !map.get_typed_transport_like_cpp(first_transport)
            .unwrap()
            .passengers()
            .contains(&player_guid)
    );
    assert!(
        map.get_typed_transport_like_cpp(second_transport)
            .unwrap()
            .passengers()
            .contains(&player_guid)
    );
    drop(manager);

    assert_eq!(
        session.reconcile_player_transport_membership_like_cpp(player_guid, None),
        crate::session::MovementTransportMembershipLikeCpp::Detached
    );
    let manager = canonical.lock().unwrap();
    let map = manager.find_map(571, 0).unwrap().map();
    assert!(
        map.get_typed_transport_like_cpp(second_transport)
            .unwrap()
            .passengers()
            .is_empty()
    );
}

#[test]
fn transport_membership_resets_when_requested_transport_is_not_canonical_like_cpp() {
    let mut session = make_session();
    let canonical = std::sync::Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let player_guid = ObjectGuid::create_player(1, 1_083);
    let missing_transport = ObjectGuid::create_transport(HighGuid::Transport, 1_084);
    install_canonical_player_and_transports_for_test(&canonical, player_guid, &[]);
    session.set_canonical_map_manager(canonical);
    session.set_player_guid(Some(player_guid));

    assert_eq!(
        session
            .reconcile_player_transport_membership_like_cpp(player_guid, Some(missing_transport),),
        crate::session::MovementTransportMembershipLikeCpp::Detached
    );
    assert_eq!(session.player_transport_guid_like_cpp(), None);
}

#[tokio::test]
async fn handle_move_set_vehicle_rec_ack_does_not_record_generic_ack_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 78);
    session.set_player_guid(Some(guid));

    session
        .handle_move_set_vehicle_rec_id_ack(
            ClientOpcodes::MoveSetVehicleRecIdAck,
            wow_packet::packets::vehicle::MoveSetVehicleRecIdAck {
                data: wow_packet::packets::movement::MovementAck {
                    status: MovementInfo {
                        guid: ObjectGuid::create_player(1, 79),
                        flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
                        position: wow_core::Position::new(f32::NAN, 20.0, 30.0, 1.5),
                        ..MovementInfo::default()
                    },
                    ack_index: 79,
                },
                vehicle_rec_id: 123,
            },
        )
        .await;

    assert!(session.movement_ack_events_like_cpp().is_empty());
}

#[tokio::test]
async fn move_time_skipped_broadcasts_skip_time_to_other_players_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
    let (self_tx, self_rx) = flume::bounded(1);
    let (other_tx, other_rx) = flume::bounded(1);
    let (self_command_tx, self_command_rx) = flume::bounded(1);
    let (other_command_tx, other_command_rx) = flume::bounded(1);

    session.set_player_guid(Some(guid));
    session.set_player_registry(std::sync::Arc::clone(&registry));
    session.set_player_position_like_cpp(wow_core::Position::ZERO);
    session.set_player_movement_time_like_cpp(100);
    registry.register_or_replace(
        guid,
        broadcast_info_with_command(guid, self_tx, self_command_tx),
        Default::default(),
    );
    registry.register_or_replace(
        other_guid,
        broadcast_info_with_command(other_guid, other_tx, other_command_tx),
        Default::default(),
    );

    session
        .handle_move_time_skipped(wow_packet::packets::movement::MoveTimeSkipped {
            mover_guid: guid,
            time_skipped: 25,
        })
        .await;

    assert!(self_rx.try_recv().is_err());
    assert!(other_rx.try_recv().is_err());
    assert!(self_command_rx.try_recv().is_err());
    let command = other_command_rx
        .try_recv()
        .expect("visible movement-set command");
    let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp move-skip-time command");
    };
    assert_eq!(command.source_guid, guid);
    let bytes = command.packet_bytes;
    let pkt = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveSkipTime)
    );
    assert_eq!(session.player_movement_time_like_cpp(), 125);
}

#[tokio::test]
async fn move_time_skipped_updates_and_routes_from_controlled_mover_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 44);
    let mover_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 779, 1_244);
    let observer_guid = ObjectGuid::create_player(1, 45);
    let player_position = Position::new(1_000.0, 1_000.0, 0.0, 0.0);
    let mover_position = Position::new(10.0, 20.0, 0.0, 0.0);
    let manager = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let (self_tx, _self_rx) = flume::bounded(1);
    let (observer_tx, _observer_rx) = flume::bounded(1);
    let (self_command_tx, self_command_rx) = flume::bounded(1);
    let (observer_command_tx, observer_command_rx) = flume::bounded(1);

    session.set_player_guid(Some(player_guid));
    session.set_player_moved_unit_guid_like_cpp(mover_guid);
    session.set_player_position_like_cpp(player_position);
    session.set_player_registry(Arc::clone(&registry));
    session.set_map_manager(Arc::clone(&manager));

    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(mover_position.x, mover_position.y);
    manager.write().unwrap().add_creature(
        0,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            mover_guid,
            779,
            mover_position,
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

    let mut self_info = broadcast_info_with_command(player_guid, self_tx, self_command_tx);
    self_info.placement.position = player_position;
    registry.register_or_replace(player_guid, self_info, Default::default());
    let mut observer_info =
        broadcast_info_with_command(observer_guid, observer_tx, observer_command_tx);
    observer_info.placement.position = mover_position;
    registry.register_or_replace(observer_guid, observer_info, Default::default());

    session
        .handle_move_time_skipped(wow_packet::packets::movement::MoveTimeSkipped {
            mover_guid,
            time_skipped: 25,
        })
        .await;

    let movement_time = manager
        .read()
        .unwrap()
        .find_creature(0, 0, mover_guid)
        .expect("controlled mover")
        .creature
        .unit()
        .movement_time_like_cpp();
    assert_eq!(movement_time, 25);
    assert!(self_command_rx.try_recv().is_err());
    let command = observer_command_rx
        .try_recv()
        .expect("observer near the controlled mover");
    let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp move-skip-time command");
    };
    assert_eq!(command.source_guid, mover_guid);
    let packet = wow_packet::WorldPacket::from_bytes(&command.packet_bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveSkipTime)
    );
}

#[tokio::test]
async fn movement_force_acks_validate_and_route_from_controlled_mover_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 46);
    let mover_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 780, 1_245);
    let observer_guid = ObjectGuid::create_player(1, 47);
    let player_position = Position::new(1_000.0, 1_000.0, 0.0, 0.0);
    let mover_position = Position::new(10.0, 20.0, 0.0, 0.0);
    let manager = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let (self_tx, _self_rx) = flume::bounded(1);
    let (observer_tx, _observer_rx) = flume::bounded(1);
    let (self_command_tx, self_command_rx) = flume::bounded(1);
    let (observer_command_tx, observer_command_rx) = flume::bounded(4);

    session.set_player_guid(Some(player_guid));
    session.set_player_moved_unit_guid_like_cpp(mover_guid);
    session.set_player_position_like_cpp(player_position);
    session.set_player_registry(Arc::clone(&registry));
    session.set_map_manager(Arc::clone(&manager));

    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(mover_position.x, mover_position.y);
    let mut mover = crate::map_manager::WorldCreature::new(
        mover_guid,
        780,
        mover_position,
        100,
        80,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
    );
    mover
        .creature
        .unit_mut()
        .set_movement_force_mod_magnitude_like_cpp(1.25);
    manager
        .write()
        .unwrap()
        .add_creature(0, 0, grid_x, grid_y, mover);

    let mut self_info = broadcast_info_with_command(player_guid, self_tx, self_command_tx);
    self_info.placement.position = player_position;
    registry.register_or_replace(player_guid, self_info, Default::default());
    let mut observer_info =
        broadcast_info_with_command(observer_guid, observer_tx, observer_command_tx);
    observer_info.placement.position = mover_position;
    registry.register_or_replace(observer_guid, observer_info, Default::default());

    let force_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 9, 89);
    let force = wow_packet::packets::movement::MovementForce {
        id: force_guid,
        origin: [1.0, 2.0, 3.0],
        direction: [4.0, 5.0, 6.0],
        transport_id: 0,
        magnitude: 7.0,
        unused_910: 0,
        force_type: wow_packet::packets::movement::MovementForceType::Gravity,
    };
    let status = MovementInfo {
        guid: mover_guid,
        time: 1_000,
        position: mover_position,
        ..MovementInfo::default()
    };

    session
        .handle_move_apply_movement_force_ack(
            wow_packet::packets::movement::MoveApplyMovementForceAck {
                ack: wow_packet::packets::movement::MovementAck {
                    status: status.clone(),
                    ack_index: 46,
                },
                force: force.clone(),
            },
        )
        .await;
    let command = observer_command_rx
        .try_recv()
        .expect("observer near controlled mover receives apply force update");
    let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp apply-force command");
    };
    assert_eq!(command.source_guid, mover_guid);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&command.packet_bytes).server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveUpdateApplyMovementForce)
    );

    session
        .handle_move_remove_movement_force_ack(
            wow_packet::packets::movement::MoveRemoveMovementForceAck {
                ack: wow_packet::packets::movement::MovementAck {
                    status: status.clone(),
                    ack_index: 47,
                },
                id: force_guid,
            },
        )
        .await;
    let command = observer_command_rx
        .try_recv()
        .expect("observer near controlled mover receives remove force update");
    let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp remove-force command");
    };
    assert_eq!(command.source_guid, mover_guid);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&command.packet_bytes).server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveUpdateRemoveMovementForce)
    );

    session.set_movement_force_mod_magnitude_changes_like_cpp(1);
    session
        .handle_movement_speed_ack(
            ClientOpcodes::MoveSetModMovementForceMagnitudeAck,
            wow_packet::packets::movement::MovementSpeedAck {
                ack: wow_packet::packets::movement::MovementAck {
                    status,
                    ack_index: 48,
                },
                speed: 1.25,
            },
        )
        .await;
    let command = observer_command_rx
        .try_recv()
        .expect("observer near controlled mover receives magnitude update");
    let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp magnitude command");
    };
    assert_eq!(command.source_guid, mover_guid);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&command.packet_bytes).server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveUpdateModMovementForceMagnitude)
    );

    session
        .handle_move_knock_back_ack(wow_packet::packets::movement::MoveKnockBackAck {
            ack: wow_packet::packets::movement::MovementAck {
                status: MovementInfo {
                    guid: mover_guid,
                    time: 1_001,
                    position: mover_position,
                    ..MovementInfo::default()
                },
                ack_index: 49,
            },
            speeds: None,
        })
        .await;
    let command = observer_command_rx
        .try_recv()
        .expect("observer receives knockback update from the Player source");
    let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp knockback command");
    };
    assert_eq!(command.source_guid, player_guid);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&command.packet_bytes).server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveUpdateKnockBack)
    );

    session
        .handle_move_apply_movement_force_ack(
            wow_packet::packets::movement::MoveApplyMovementForceAck {
                ack: wow_packet::packets::movement::MovementAck {
                    status: MovementInfo {
                        guid: player_guid,
                        time: 1_001,
                        position: player_position,
                        ..MovementInfo::default()
                    },
                    ack_index: 50,
                },
                force,
            },
        )
        .await;
    assert!(
        observer_command_rx.try_recv().is_err(),
        "a force ACK for a non-active mover must not publish"
    );
    assert!(self_command_rx.try_recv().is_err());
}
