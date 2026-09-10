//! Connection packets.
//!
//! Separated from session.rs under #689.

use super::*;

// ── ConnectionStatus (SMSG 0x2809) ─────────────────────────────────

/// BattleNet connection status sent at end of session init.
pub struct ConnectionStatus {
    pub state: u8,
    pub suppress_notification: bool,
}

impl ServerPacket for ConnectionStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattleNetConnectionStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(u32::from(self.state), 2);
        pkt.write_bit(self.suppress_notification);
        pkt.flush_bits();
    }
}

// ── SetTimeZoneInformation (SMSG 0x2677) ────────────────────────────

/// Response to ServerTimeOffsetRequest. Sends the current realm time.
pub struct ServerTimeOffset {
    pub time: i64,
}

impl ServerTimeOffset {
    /// Current time.
    pub fn now() -> Self {
        Self {
            time: unix_timestamp(),
        }
    }
}

impl ServerPacket for ServerTimeOffset {
    const OPCODE: ServerOpcodes = ServerOpcodes::ServerTimeOffset;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.time);
    }
}

// ── InitWorldStates (SMSG 0x2746) ─────────────────────────────────

/// Time synchronization request. The client uses this to sync its clock.
/// Critical for loading — client expects this before it can finish.
pub struct TimeSyncRequest {
    pub sequence_index: u32,
}

impl ServerPacket for TimeSyncRequest {
    const OPCODE: ServerOpcodes = ServerOpcodes::TimeSyncRequest;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
    }
}

// ── TimeSyncResponse (CMSG 0x3a3d) ──────────────────────────────────

/// Client response to a TimeSyncRequest. Contains the client's time
/// at the moment it received the request, plus the server's sequence index.
///
/// The server must keep sending periodic TimeSyncRequests (every 5-10s)
/// or the client's internal time sync state becomes inconsistent and crashes.
pub struct TimeSyncResponse {
    pub client_time: u32,
    pub sequence_index: u32,
}

impl ClientPacket for TimeSyncResponse {
    const OPCODE: ClientOpcodes = ClientOpcodes::TimeSyncResponse;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let sequence_index = packet.read_uint32()?;
        let client_time = packet.read_uint32()?;
        Ok(Self {
            client_time,
            sequence_index,
        })
    }
}

// ── MoveSetActiveMover (SMSG 0x2dd5) ───────────────────────────────

/// Sent on the instance connection after TransferPending.
/// Tells the client to pause movement processing during map transfer.
/// C# ref: MovementPackets.SuspendToken (ConnectionType.Instance)
pub struct SuspendToken {
    /// Movement counter (sequence index). Send 1 for simple teleports.
    pub sequence_index: u32,
    /// 1 = Normal teleport, 2 = Seamless teleport.
    pub reason: u32,
}

impl ServerPacket for SuspendToken {
    const OPCODE: ServerOpcodes = ServerOpcodes::SuspendToken;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
        pkt.write_bits(self.reason, 2);
        pkt.flush_bits();
    }
}

// ── ResumeToken (SMSG 0x25a9) ────────────────────────────────────────

/// Sent after WorldPortResponse to resume movement processing.
/// C# ref: MovementPackets.ResumeToken (ConnectionType.Instance)
pub struct ResumeToken {
    pub sequence_index: u32,
    /// 1 = Normal, 2 = Seamless.
    pub reason: u32,
}

impl ServerPacket for ResumeToken {
    const OPCODE: ServerOpcodes = ServerOpcodes::ResumeToken;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
        pkt.write_bits(self.reason, 2);
        pkt.flush_bits();
    }
}

// ── Auction empty results ─────────────────────────────────────────────────────

/// SMSG_QUERY_TIME_RESPONSE — server time response to CMSG_QUERY_TIME.
/// C# ref: QueryPackets.QueryTimeResponse → WriteInt64(CurrentTime)
pub struct QueryTimeResponse {
    /// Current server Unix timestamp (seconds).
    pub current_time: i64,
}

impl ServerPacket for QueryTimeResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryTimeResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.current_time);
    }
}
