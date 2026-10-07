// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the movement handlers moved to
//! `wow-world-application` (#1263 F5). The entry point dispatches through the
//! registered production thunk, so the release context built by
//! [`super::super::movement_host`] is exercised by the caller.
//! `SetActiveMover` and `MoveSetCollisionHeightAck` have no test caller and
//! therefore no entry point here.

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;
use wow_packet::packets::movement::MoveTimeSkipped;
use wow_packet::packets::vehicle::MoveSetVehicleRecIdAck;

use crate::session::WorldSession;

async fn dispatch_registered_like_cpp(
    session: &mut WorldSession,
    opcode: ClientOpcodes,
    pkt: WorldPacket,
) {
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == opcode)
        .expect("registered movement handler");
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(session, &catalogs, pkt).await;
}

/// Rebuilds the client wire body of `CMSG_MOVE_TIME_SKIPPED`.
fn move_time_skipped_wire_like_cpp(pkt: &MoveTimeSkipped) -> WorldPacket {
    let mut wire = WorldPacket::new_empty();
    wire.write_packed_guid(&pkt.mover_guid);
    wire.write_uint32(pkt.time_skipped);
    wire
}

/// Rebuilds the client wire body of `CMSG_MOVE_SET_VEHICLE_REC_ID_ACK`.
///
/// C++ `WorldPackets::Vehicle::MoveSetVehicleRecIdAck::Read` reads the shared
/// movement ACK and then the vehicle record id.
fn move_set_vehicle_rec_id_ack_wire_like_cpp(pkt: &MoveSetVehicleRecIdAck) -> WorldPacket {
    let mut wire = WorldPacket::new_empty();
    pkt.data.status.write(&mut wire);
    wire.write_int32(pkt.data.ack_index);
    wire.write_int32(pkt.vehicle_rec_id);
    wire
}

impl WorldSession {
    pub async fn handle_move_time_skipped(&mut self, pkt: MoveTimeSkipped) {
        dispatch_registered_like_cpp(
            self,
            ClientOpcodes::MoveTimeSkipped,
            move_time_skipped_wire_like_cpp(&pkt),
        )
        .await;
    }

    pub async fn handle_move_set_vehicle_rec_id_ack(
        &mut self,
        opcode: ClientOpcodes,
        pkt: MoveSetVehicleRecIdAck,
    ) {
        dispatch_registered_like_cpp(
            self,
            opcode,
            move_set_vehicle_rec_id_ack_wire_like_cpp(&pkt),
        )
        .await;
    }
}
