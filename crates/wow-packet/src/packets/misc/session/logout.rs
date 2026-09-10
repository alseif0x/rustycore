//! Logout packets.
//!
//! Separated from session.rs under #689.

use super::*;

// ── NewWorld (SMSG 0x2594) ────────────────────────────────────────────

/// Client requests to log out.
pub struct LogoutRequest {
    pub idle_logout: bool,
}

impl ClientPacket for LogoutRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::LogoutRequest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let idle_logout = packet.read_bit()?;
        Ok(Self { idle_logout })
    }
}

/// Client cancels a pending logout.
pub struct LogoutCancel;

impl ClientPacket for LogoutCancel {
    const OPCODE: ClientOpcodes = ClientOpcodes::LogoutCancel;

    fn read(_packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

// ── LogoutResponse (SMSG 0x2683) ────────────────────────────────────

/// Server responds to a logout request.
pub struct LogoutResponse {
    pub logout_result: i32,
    pub instant: bool,
}

impl LogoutResponse {
    /// Successful instant logout.
    pub fn instant_ok() -> Self {
        Self {
            logout_result: 0,
            instant: true,
        }
    }

    /// Successful delayed logout (20s timer).
    pub fn delayed_ok() -> Self {
        Self {
            logout_result: 0,
            instant: false,
        }
    }
}

impl ServerPacket for LogoutResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.logout_result);
        pkt.write_bit(self.instant);
        pkt.flush_bits();
    }
}

// ── TransferPending (SMSG 0x25cd) ────────────────────────────────────

/// Server tells client logout is complete — return to character select.
pub struct LogoutComplete;

impl ServerPacket for LogoutComplete {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutComplete;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── LogoutCancelAck (SMSG 0x2685) ───────────────────────────────────

/// Server acknowledges logout cancellation.
pub struct LogoutCancelAck;

impl ServerPacket for LogoutCancelAck {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutCancelAck;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── Helper ──────────────────────────────────────────────────────────

/// Server response to CMSG_REQUEST_PLAYED_TIME.
///
/// C# ref: `MiscHandler.HandlePlayedTime` → `PlayedTime` packet.
/// Fields: TotalTime (u32), LevelTime (u32), TriggerEvent (bool).
pub struct PlayedTime {
    /// Total time the character has been played (seconds).
    pub total_time: u32,
    /// Time played at the current level (seconds).
    pub level_time: u32,
    /// Mirror of the client's TriggerScriptEvent flag.
    pub trigger_event: bool,
}

impl ServerPacket for PlayedTime {
    const OPCODE: ServerOpcodes = ServerOpcodes::PlayedTime;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.total_time);
        pkt.write_uint32(self.level_time);
        pkt.write_bit(self.trigger_event);
        pkt.flush_bits();
    }
}

// ── RatedPvpInfo ─────────────────────────────────────────────────────────────

/// Floating text "+XP" on screen when player earns experience.
/// C++ `WorldPackets::Character::LogXPGain::Write`.
pub struct LogXpGain {
    pub victim: ObjectGuid,
    pub original: i32, // base XP plus represented bonuses
    pub reason: u8,    // 0=Kill, 1=NoKill(quest/explore)
    pub amount: i32,   // base XP amount
    pub group_bonus: f32,
}

impl ServerPacket for LogXpGain {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogXpGain;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.victim);
        pkt.write_int32(self.original);
        pkt.write_uint8(self.reason);
        pkt.write_int32(self.amount);
        pkt.write_float(self.group_bonus);
    }
}
