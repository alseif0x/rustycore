//! World state packets.
//!
//! Separated from world_state.rs under #689.

use super::*;

/// C++ `WorldPackets::WorldState::UpdateWorldState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateWorldState {
    pub variable_id: u32,
    pub value: i32,
    pub hidden: bool,
}

impl UpdateWorldState {
    pub fn new(variable_id: u32, value: i32) -> Self {
        Self {
            variable_id,
            value,
            hidden: false,
        }
    }
}

impl ServerPacket for UpdateWorldState {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateWorldState;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.variable_id);
        pkt.write_int32(self.value);
        pkt.write_bit(self.hidden);
        pkt.flush_bits();
    }
}

// ── PageText (SMSG 0x2719) ───────────────────────────────────────────

/// Time zone info sent to the client.
pub struct SetTimeZoneInformation {
    pub server_timezone: String,
    pub game_timezone: String,
    pub server_regional_timezone: String,
}

impl SetTimeZoneInformation {
    pub fn utc() -> Self {
        Self {
            server_timezone: "Etc/UTC".into(),
            game_timezone: "Etc/UTC".into(),
            server_regional_timezone: "Etc/UTC".into(),
        }
    }
}

impl ServerPacket for SetTimeZoneInformation {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetTimeZoneInformation;

    fn write(&self, pkt: &mut WorldPacket) {
        // 7-bit length-prefixed strings
        pkt.write_bits(self.server_timezone.len() as u32, 7);
        pkt.write_bits(self.game_timezone.len() as u32, 7);
        pkt.write_bits(self.server_regional_timezone.len() as u32, 7);
        pkt.flush_bits();

        pkt.write_string(&self.server_timezone);
        pkt.write_string(&self.game_timezone);
        pkt.write_string(&self.server_regional_timezone);
    }
}

// ── LoginSetTimeSpeed (SMSG 0x270d) ─────────────────────────────────

/// World state variables for the current zone. Sent after UpdateObject.
/// C++ `WorldPackets::WorldState::InitWorldStates`.
pub struct InitWorldStates {
    pub map_id: i32,
    pub area_id: i32,
    pub subarea_id: i32,
    pub world_states: Vec<(i32, i32)>,
}

impl InitWorldStates {
    pub fn new(map_id: i32, zone_id: i32) -> Self {
        Self {
            map_id,
            area_id: zone_id,
            subarea_id: 0,
            world_states: Vec::new(),
        }
    }

    pub fn with_world_states(
        map_id: i32,
        zone_id: i32,
        area_id: i32,
        world_states: Vec<(i32, i32)>,
    ) -> Self {
        Self {
            map_id,
            area_id: zone_id,
            subarea_id: area_id,
            world_states,
        }
    }
}

impl ServerPacket for InitWorldStates {
    const OPCODE: ServerOpcodes = ServerOpcodes::InitWorldStates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.map_id);
        pkt.write_int32(self.area_id);
        pkt.write_int32(self.subarea_id);
        pkt.write_uint32(self.world_states.len() as u32);
        for (variable_id, value) in &self.world_states {
            pkt.write_int32(*variable_id);
            pkt.write_int32(*value);
        }
    }
}

// ── UpdateTalentData (SMSG 0x25d7) ──────────────────────────────────

/// Hearthstone bind point. Sent during login.
pub struct BindPointUpdate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub map_id: u32,
    pub area_id: u32,
}

impl ServerPacket for BindPointUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::BindPointUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_float(self.x);
        pkt.write_float(self.y);
        pkt.write_float(self.z);
        pkt.write_uint32(self.map_id);
        pkt.write_uint32(self.area_id);
    }
}

// ── PlayerBound (SMSG 0x2ff8) ───────────────────────────────────────

/// World server info sent during login.
pub struct WorldServerInfo {
    pub difficulty_id: i32,
}

impl WorldServerInfo {
    pub fn default_open_world() -> Self {
        Self { difficulty_id: 0 }
    }
}

impl ServerPacket for WorldServerInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::WorldServerInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
        pkt.write_bit(false); // IsTournamentRealm
        pkt.write_bit(false); // XRealmPvpAlert
        pkt.write_bit(false); // RestrictedAccountMaxLevel.HasValue
        pkt.write_bit(false); // RestrictedAccountMaxMoney.HasValue
        pkt.write_bit(false); // InstanceGroupSize.HasValue
        pkt.flush_bits();
        // No optional fields written (all HasValue=false)
    }
}

// ── InitialSetup (SMSG 0x2580) ─────────────────────────────────────

/// Account-wide achievement criteria. Empty for fresh accounts.
pub struct AllAccountCriteria;

impl ServerPacket for AllAccountCriteria {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAccountCriteria;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Progress.Count
    }
}

// ── AllAchievementData (SMSG 0x2570) ─────────────────────────────────

/// Account-wide achievements. Empty for fresh accounts.
pub struct AllAchievementData;

impl ServerPacket for AllAchievementData {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAchievementData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Earned.Count
        pkt.write_int32(0); // Progress.Count
    }
}

// ── DbQueryBulk (CMSG 0x35e5) ─────────────────────────────────────

/// Sent after SuspendTokenResponse, before the client's WorldPortResponse.
/// C++ MovementPackets.cpp:696, Opcodes.cpp:1811: NewWorld on the realm connection.
pub struct NewWorld {
    pub map_id: u32,
    pub pos: wow_core::Position,
    /// C++ Player.h: 16 = Normal teleport, 21 = Seamless.
    pub reason: u32,
}

impl ServerPacket for NewWorld {
    const OPCODE: ServerOpcodes = ServerOpcodes::NewWorld;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
        // TeleportLocation: Pos (XYZO) + two unused int32 fields (-1, -1)
        pkt.write_float(self.pos.x);
        pkt.write_float(self.pos.y);
        pkt.write_float(self.pos.z);
        pkt.write_float(self.pos.orientation);
        pkt.write_int32(-1); // Unused901_1
        pkt.write_int32(-1); // Unused901_2
        pkt.write_uint32(self.reason);
        // MovementOffset (all zeros)
        pkt.write_float(0.0);
        pkt.write_float(0.0);
        pkt.write_float(0.0);
    }
}
