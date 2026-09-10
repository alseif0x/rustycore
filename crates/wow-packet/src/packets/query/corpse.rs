//! Corpse packets.
//!
//! Separated from query.rs under #689.

use super::*;

// ── CMSG_QUERY_CORPSE_LOCATION_FROM_CLIENT (0x3662) ─────────────────

/// C++ `WorldPackets::Query::QueryCorpseLocationFromClient`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryCorpseLocationFromClient {
    pub player: ObjectGuid,
}

impl ClientPacket for QueryCorpseLocationFromClient {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryCorpseLocationFromClient;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            player: packet.read_guid()?,
        })
    }
}

// ── SMSG_CORPSE_LOCATION (0x264F) ───────────────────────────────────

/// C++ `WorldPackets::Query::CorpseLocation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorpseLocation {
    pub player: ObjectGuid,
    pub transport: ObjectGuid,
    pub position: Position,
    pub actual_map_id: i32,
    pub map_id: i32,
    pub valid: bool,
}

impl CorpseLocation {
    pub fn not_found_like_cpp(player: ObjectGuid) -> Self {
        Self {
            player,
            transport: ObjectGuid::EMPTY,
            position: Position::ZERO,
            actual_map_id: 0,
            map_id: 0,
            valid: false,
        }
    }
}

impl ServerPacket for CorpseLocation {
    const OPCODE: ServerOpcodes = ServerOpcodes::CorpseLocation;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.valid);
        pkt.flush_bits();

        pkt.write_guid(&self.player);
        pkt.write_int32(self.actual_map_id);
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_float(self.position.z);
        pkt.write_int32(self.map_id);
        pkt.write_guid(&self.transport);
    }
}

// ── CMSG_QUERY_CORPSE_TRANSPORT (0x3663) ────────────────────────────

/// C++ `WorldPackets::Query::QueryCorpseTransport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryCorpseTransport {
    pub player: ObjectGuid,
    pub transport: ObjectGuid,
}

impl ClientPacket for QueryCorpseTransport {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryCorpseTransport;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            player: packet.read_guid()?,
            transport: packet.read_guid()?,
        })
    }
}

// ── SMSG_CORPSE_TRANSPORT_QUERY (0x2712) ────────────────────────────

/// C++ `WorldPackets::Query::CorpseTransportQuery`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorpseTransportQuery {
    pub player: ObjectGuid,
    pub position: Position,
    pub facing: f32,
}

impl CorpseTransportQuery {
    pub fn not_found_like_cpp(player: ObjectGuid) -> Self {
        Self {
            player,
            position: Position::ZERO,
            facing: 0.0,
        }
    }
}

impl ServerPacket for CorpseTransportQuery {
    const OPCODE: ServerOpcodes = ServerOpcodes::CorpseTransportQuery;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_guid(&self.player);
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_float(self.position.z);
        pkt.write_float(self.facing);
    }
}
