// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet definitions.
//!
//! Handles all CMSG_MOVE_* client packets (player movement) and server-side
//! movement packets (MoveUpdate, OnMonsterMove).

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_constants::movement::{MovementFlag, MovementFlag2, MovementFlags3};
use wow_core::{ObjectGuid, Position};

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

// ── MovementInfo ─────────────────────────────────────────────────

/// Full movement info parsed from any CMSG_MOVE_* packet.
///
/// Binary layout (from C# PacketHandlerExtensions.Read):
/// ```
/// PackedGuid  guid
/// u32         movement_flags
/// u32         movement_flags2
/// u32         movement_flags3
/// u32         time (ms)
/// f32         x, y, z, orientation
/// f32         pitch
/// f32         step_up_start_elevation
/// u32         remove_movement_forces_count
/// u32         move_index
/// PackedGuid  × remove_movement_forces_count (GUIDs to remove)
/// bit         has_standing_on_gameobject_guid
/// bit         has_transport
/// bit         has_fall
/// bit         has_spline
/// bit         height_change_failed
/// bit         remote_time_valid
/// bit         has_inertia
/// bit         has_adv_flying
/// [flush]
/// [transport info if has_transport]
/// [standing_on_guid if has_standing_on_gameobject_guid]
/// [inertia if has_inertia]
/// [adv_flying if has_adv_flying]
/// [fall info if has_fall]
/// ```
#[derive(Debug, Clone)]
pub struct MovementInfo {
    pub guid: ObjectGuid,
    pub flags: MovementFlag,
    pub flags2: MovementFlag2,
    pub flags3: MovementFlags3,
    pub time: u32,
    pub position: Position,
    pub pitch: f32,
    pub step_up_start_elevation: f32,
    pub jump: JumpInfo,
    pub transport: Option<TransportInfo>,
    pub adv_flying: Option<AdvFlyingInfo>,
}

impl Default for MovementInfo {
    fn default() -> Self {
        Self {
            guid: ObjectGuid::EMPTY,
            flags: MovementFlag::NONE,
            flags2: MovementFlag2::NONE,
            flags3: MovementFlags3::NONE,
            time: 0,
            position: Position::ZERO,
            pitch: 0.0,
            step_up_start_elevation: 0.0,
            jump: JumpInfo::default(),
            transport: None,
            adv_flying: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct JumpInfo {
    pub fall_time: u32,
    pub z_speed: f32,
    pub has_direction: bool,
    pub sin_angle: f32,
    pub cos_angle: f32,
    pub xy_speed: f32,
}

#[derive(Debug, Clone)]
pub struct TransportInfo {
    pub guid: ObjectGuid,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
    pub seat: i8,
    pub time: u32,
    pub prev_time: Option<u32>,
    pub vehicle_id: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct AdvFlyingInfo {
    pub forward_velocity: f32,
    pub up_velocity: f32,
}

impl MovementInfo {
    /// Parse from packet buffer (after opcode has been consumed).
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid = pkt.read_packed_guid()?;
        let flags = MovementFlag::from_bits_truncate(pkt.read_uint32()?);
        let flags2 = MovementFlag2::from_bits_truncate(pkt.read_uint32()?);
        let flags3 = MovementFlags3::from_bits_truncate(pkt.read_uint32()?);
        let time = pkt.read_uint32()?;
        let x = pkt.read_float()?;
        let y = pkt.read_float()?;
        let z = pkt.read_float()?;
        let o = pkt.read_float()?;
        let pitch = pkt.read_float()?;
        let step_up_start_elevation = pkt.read_float()?;

        let remove_forces_count = pkt.read_uint32()?;
        let _move_index = pkt.read_uint32()?;

        // skip force GUIDs
        for _ in 0..remove_forces_count {
            pkt.read_packed_guid()?;
        }

        let has_standing_on_go = pkt.has_bit()?;
        let has_transport = pkt.has_bit()?;
        let has_fall = pkt.has_bit()?;
        let _has_spline = pkt.has_bit()?;
        let _height_change_failed = pkt.has_bit()?;
        let _remote_time_valid = pkt.has_bit()?;
        let has_inertia = pkt.has_bit()?;
        let has_adv_flying = pkt.has_bit()?;

        let transport = if has_transport {
            let tguid = pkt.read_packed_guid()?;
            let tx = pkt.read_float()?;
            let ty = pkt.read_float()?;
            let tz = pkt.read_float()?;
            let to_ = pkt.read_float()?;
            let seat = pkt.read_int8()?;
            let ttime = pkt.read_uint32()?;

            let has_prev = pkt.has_bit()?;
            let has_vehicle_id = pkt.has_bit()?;
            // bit reader auto-resets on next byte read

            let prev_time = if has_prev { Some(pkt.read_uint32()?) } else { None };
            let vehicle_id = if has_vehicle_id { Some(pkt.read_int32()?) } else { None };

            Some(TransportInfo {
                guid: tguid,
                x: tx, y: ty, z: tz, o: to_,
                seat,
                time: ttime,
                prev_time,
                vehicle_id,
            })
        } else {
            None
        };

        // standing on gameobject guid (skip)
        if has_standing_on_go {
            pkt.read_packed_guid()?;
        }

        // inertia (skip)
        if has_inertia {
            let _id = pkt.read_int32()?;
            let _fx = pkt.read_float()?;
            let _fy = pkt.read_float()?;
            let _fz = pkt.read_float()?;
            let _lifetime = pkt.read_uint32()?;
        }

        let adv_flying = if has_adv_flying {
            let fwd = pkt.read_float()?;
            let up = pkt.read_float()?;
            Some(AdvFlyingInfo { forward_velocity: fwd, up_velocity: up })
        } else {
            None
        };

        let jump = if has_fall {
            let fall_time = pkt.read_uint32()?;
            let z_speed = pkt.read_float()?;
            let has_direction = pkt.has_bit()?;
            // bit reader auto-resets on next byte read
            let (sin_angle, cos_angle, xy_speed) = if has_direction {
                (pkt.read_float()?, pkt.read_float()?, pkt.read_float()?)
            } else {
                (0.0, 0.0, 0.0)
            };
            JumpInfo { fall_time, z_speed, has_direction, sin_angle, cos_angle, xy_speed }
        } else {
            JumpInfo::default()
        };

        Ok(MovementInfo {
            guid,
            flags,
            flags2,
            flags3,
            time,
            position: Position::new(x, y, z, o),
            pitch,
            step_up_start_elevation,
            jump,
            transport,
            adv_flying,
        })
    }

    /// Write movement info to a packet (for MoveUpdate broadcasts).
    pub fn write(&self, pkt: &mut WorldPacket) {
        let has_transport = self.transport.is_some();
        let has_fall = self.jump.fall_time != 0;
        let has_adv_flying = self.adv_flying.is_some();

        pkt.write_packed_guid(&self.guid);
        pkt.write_uint32(self.flags.bits());
        pkt.write_uint32(self.flags2.bits());
        pkt.write_uint32(self.flags3.bits());
        pkt.write_uint32(self.time);
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_float(self.position.z);
        pkt.write_float(self.position.orientation);
        pkt.write_float(self.pitch);
        pkt.write_float(self.step_up_start_elevation);

        pkt.write_uint32(0u32); // remove_forces_count
        pkt.write_uint32(0u32); // move_index

        pkt.write_bit(false); // has_standing_on_go
        pkt.write_bit(has_transport);
        pkt.write_bit(has_fall);
        pkt.write_bit(false); // has_spline
        pkt.write_bit(false); // height_change_failed
        pkt.write_bit(false); // remote_time_valid
        pkt.write_bit(false); // has_inertia
        pkt.write_bit(has_adv_flying);
        pkt.flush_bits();

        if let Some(t) = &self.transport {
            pkt.write_packed_guid(&t.guid);
            pkt.write_float(t.x);
            pkt.write_float(t.y);
            pkt.write_float(t.z);
            pkt.write_float(t.o);
            pkt.write_int8(t.seat);
            pkt.write_uint32(t.time);
            pkt.write_bit(t.prev_time.is_some());
            pkt.write_bit(t.vehicle_id.is_some());
            pkt.flush_bits();
            if let Some(pt) = t.prev_time { pkt.write_uint32(pt); }
            if let Some(vid) = t.vehicle_id { pkt.write_int32(vid); }
        }

        if let Some(af) = &self.adv_flying {
            pkt.write_float(af.forward_velocity);
            pkt.write_float(af.up_velocity);
        }

        if has_fall {
            pkt.write_uint32(self.jump.fall_time);
            pkt.write_float(self.jump.z_speed);
            pkt.write_bit(self.jump.has_direction);
            pkt.flush_bits();
            if self.jump.has_direction {
                pkt.write_float(self.jump.sin_angle);
                pkt.write_float(self.jump.cos_angle);
                pkt.write_float(self.jump.xy_speed);
            }
        }
    }
}

// ── ClientPlayerMovement (CMSG_MOVE_*) ───────────────────────────

/// Generic movement packet sent by the client for all movement opcodes.
#[derive(Debug, Clone)]
pub struct ClientPlayerMovement {
    pub info: MovementInfo,
}

/// Macro to implement ClientPacket for all movement opcodes.
macro_rules! impl_movement_client_packet {
    ($opcode:ident) => {
        // We can't use a macro for const OPCODE easily with multiple,
        // so we implement a shared read function instead.
    };
}

impl ClientPlayerMovement {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let info = MovementInfo::read(pkt)?;
        Ok(Self { info })
    }
}

// ── MoveUpdate (SMSG_MOVE_UPDATE) ────────────────────────────────

/// Broadcast a player's movement to nearby players.
#[derive(Debug, Clone)]
pub struct MoveUpdate {
    pub info: MovementInfo,
}

impl ServerPacket for MoveUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        self.info.write(pkt);
    }
}

// ── MonsterMove (SMSG_ON_MONSTER_MOVE) ───────────────────────────

/// Server moves a creature/NPC along a spline path.
///
/// Simplified version: single straight-line move with one destination point.
///
/// Wire format (simplified, no cyclic/uncompressed):
/// ```
/// PackedGuid  mover_guid
/// Vector3     current_pos (f32 x3)
/// u32         spline_id
/// u32         move_time_ms
/// u32         spline_flags
/// u8          face_type  (0=none)
/// f32         face_direction (if face_type == 4)
/// i32         points_count  (count of dest points)
/// Vector3     destination point (last point, then packed deltas)
/// i32         packed_deltas_count (0 for single point)
/// ```
#[derive(Debug, Clone)]
pub struct MonsterMove {
    pub mover_guid: ObjectGuid,
    pub current_pos: Position,
    pub spline_id: u32,
    pub move_time_ms: u32,
    /// SplineFlag: 0x400 = UncompressedPath, 0x800 = Cyclic, default 0
    pub spline_flags: u32,
    pub destination: Position,
}

impl ServerPacket for MonsterMove {
    const OPCODE: ServerOpcodes = ServerOpcodes::OnMonsterMove;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.mover_guid);
        // current position
        pkt.write_float(self.current_pos.x);
        pkt.write_float(self.current_pos.y);
        pkt.write_float(self.current_pos.z);
        // spline id
        pkt.write_uint32(self.spline_id);
        // MovementMonsterSpline::Write
        // move time
        pkt.write_uint32(self.move_time_ms);
        // spline flags (0 = default linear)
        pkt.write_uint32(self.spline_flags);
        // face type (0 = none/direction)
        pkt.write_uint8(0u8);
        // face direction (written when face_type == 4, FACE_ANGLE)
        // For face_type 0 we skip face data.

        // No AnimTierTransition (spline flags don't have Animation)
        // No JumpExtraData
        // No FadeObjectTime

        // Points (uncompressed path = false, so last point + packed deltas)
        // For a single-destination move: 1 destination point, 0 packed deltas
        pkt.write_int32(1i32); // points count
        pkt.write_float(self.destination.x);
        pkt.write_float(self.destination.y);
        pkt.write_float(self.destination.z);
        pkt.write_int32(0i32); // packed deltas count
    }
}

// ── MonsterMoveStop ───────────────────────────────────────────────

/// Stops a creature's current spline movement.
#[derive(Debug, Clone)]
pub struct MonsterMoveStop {
    pub mover_guid: ObjectGuid,
    pub current_pos: Position,
    pub spline_id: u32,
}

impl ServerPacket for MonsterMoveStop {
    const OPCODE: ServerOpcodes = ServerOpcodes::OnMonsterMove;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.mover_guid);
        pkt.write_float(self.current_pos.x);
        pkt.write_float(self.current_pos.y);
        pkt.write_float(self.current_pos.z);
        pkt.write_uint32(self.spline_id);
        // move_time = 0 → stop
        pkt.write_uint32(0u32);
        // SplineFlag::Done = 0x100
        pkt.write_uint32(0x100u32);
        pkt.write_uint8(0u8); // face_type = none
        pkt.write_int32(0i32); // no points
        pkt.write_int32(0i32); // no packed deltas
    }
}

// ── SetActiveMover (CMSG 0x3A3C) ──────────────────────────────────

/// Client sets which unit is currently being moved (should be player's own GUID).
/// Sent after login and when switching controlled units (e.g., vehicles).
///
/// C#: `SetActiveMover` in MovementPackets.cs
#[derive(Debug, Clone)]
pub struct SetActiveMover {
    pub active_mover: ObjectGuid,
}

impl ClientPacket for SetActiveMover {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetActiveMover;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let active_mover = pkt.read_packed_guid()?;
        Ok(Self { active_mover })
    }
}

// ── MoveInitActiveMoverComplete (CMSG 0x3A46) ─────────────────────

/// Client acknowledges that the active mover has been fully initialized.
/// Sent after login; server may update transport timing flags.
///
/// C#: `MoveInitActiveMoverComplete` in MovementPackets.cs
#[derive(Debug, Clone)]
pub struct MoveInitActiveMoverComplete {
    /// Ticks relative to server time (used for transport sync).
    pub ticks: u32,
}

impl ClientPacket for MoveInitActiveMoverComplete {
    const OPCODE: ClientOpcodes = ClientOpcodes::MoveInitActiveMoverComplete;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let ticks = pkt.read_uint32()?;
        Ok(Self { ticks })
    }
}
