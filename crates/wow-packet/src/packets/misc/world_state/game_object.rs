//! Game object packets.
//!
//! Separated from world_state.rs under #689.

use super::*;

// ── FishNotHooked (SMSG 0x26cf) ─────────────────────────────────────

/// Opens a gameobject-backed interaction UI.
pub struct GameObjectInteraction {
    pub object_guid: ObjectGuid,
    pub interaction_type: i32,
}

impl ServerPacket for GameObjectInteraction {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectInteraction;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_int32(self.interaction_type);
    }
}

// ── GameObjectCustomAnim (SMSG 0x25c4) ───────────────────────────────

/// Broadcasts a custom animation for a gameobject.
pub struct GameObjectCustomAnim {
    pub object_guid: ObjectGuid,
    pub custom_anim: u32,
    pub play_as_despawn: bool,
}

impl ServerPacket for GameObjectCustomAnim {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectCustomAnim;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint32(self.custom_anim);
        pkt.write_bit(self.play_as_despawn);
        pkt.flush_bits();
    }
}

// ── GameObjectDespawn (SMSG 0x25c5) ─────────────────────────────────

/// Notifies the client that a gameobject despawned.
pub struct GameObjectDespawn {
    pub object_guid: ObjectGuid,
}

impl ServerPacket for GameObjectDespawn {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectDespawn;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── CapturePointRemoved (SMSG 0xbadd/UNKNOWN placeholder) ────────────

/// C++ `WorldPackets::Battleground::CapturePointRemoved`.
///
/// The legacy C++ opcode table still marks this battleground packet as
/// `0xBADD`; the archived TrinityCore source marks it as `UNKNOWN_OPCODE` too.
/// Rust cannot model two `ServerOpcodes` enum variants with the same numeric
/// placeholder, so this serializer intentionally shares the current
/// `UpdateCapturePoint` placeholder while preserving the distinct packet type
/// and payload shape.
pub struct CapturePointRemoved {
    pub capture_point_guid: ObjectGuid,
}

impl ServerPacket for CapturePointRemoved {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.capture_point_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── GameObjectSetStateLocal (SMSG 0x2806) ───────────────────────────

/// Sets a gameobject state only for the receiving client.
pub struct GameObjectSetStateLocal {
    pub object_guid: ObjectGuid,
    pub state: u8,
}

impl ServerPacket for GameObjectSetStateLocal {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectSetStateLocal;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint8(self.state);
    }
}

// ── AnimKit control packets ────────────────────────────────────────

/// C++ `WorldPackets::Battleground::UpdateCapturePoint`.
pub struct UpdateCapturePoint {
    pub guid: ObjectGuid,
    pub position: Position,
    pub state: u8,
    pub capture_time_ms: u32,
    pub capture_total_duration_ms: u32,
}

impl ServerPacket for UpdateCapturePoint {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_uint8(self.state);

        if matches!(self.state, 2 | 3) {
            pkt.write_uint32(self.capture_time_ms);
            pkt.write_uint32(self.capture_total_duration_ms);
        }
    }
}
