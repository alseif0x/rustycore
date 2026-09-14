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
