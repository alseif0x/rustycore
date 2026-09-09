//! Movement handlers regression scenarios, part 3 of 3.
//!
//! Moved out of the movement.rs root under #654; every test is unchanged.

use super::*;

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
