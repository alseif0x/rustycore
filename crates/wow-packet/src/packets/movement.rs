// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet definitions.
//!
//! Handles all CMSG_MOVE_* client packets (player movement) and server-side
//! movement packets (MoveUpdate, OnMonsterMove).

use wow_constants::movement::{MovementFlag, MovementFlag2, MovementFlags3};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position};
use wow_movement::{
    AnimTierTransition as MoveAnimTierTransition, MonsterMoveType, MoveSpline, MoveSplineFlag,
    SpellEffectExtraData as MoveSpellEffectExtraData,
};

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

// ── MovementInfo ─────────────────────────────────────────────────

/// Full movement info parsed from any CMSG_MOVE_* packet.
///
/// Binary layout (from C# PacketHandlerExtensions.Read):
/// ```text
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
    pub inertia: Option<InertiaInfo>,
    pub adv_flying: Option<AdvFlyingInfo>,
    pub standing_on_gameobject_guid: Option<ObjectGuid>,
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
            inertia: None,
            adv_flying: None,
            standing_on_gameobject_guid: None,
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

#[derive(Debug, Clone)]
pub struct InertiaInfo {
    pub id: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub lifetime: u32,
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

            let prev_time = if has_prev {
                Some(pkt.read_uint32()?)
            } else {
                None
            };
            let vehicle_id = if has_vehicle_id {
                Some(pkt.read_int32()?)
            } else {
                None
            };

            Some(TransportInfo {
                guid: tguid,
                x: tx,
                y: ty,
                z: tz,
                o: to_,
                seat,
                time: ttime,
                prev_time,
                vehicle_id,
            })
        } else {
            None
        };

        let standing_on_gameobject_guid = if has_standing_on_go {
            Some(pkt.read_packed_guid()?)
        } else {
            None
        };

        let inertia = if has_inertia {
            Some(InertiaInfo {
                id: pkt.read_int32()?,
                x: pkt.read_float()?,
                y: pkt.read_float()?,
                z: pkt.read_float()?,
                lifetime: pkt.read_uint32()?,
            })
        } else {
            None
        };

        let adv_flying = if has_adv_flying {
            let fwd = pkt.read_float()?;
            let up = pkt.read_float()?;
            Some(AdvFlyingInfo {
                forward_velocity: fwd,
                up_velocity: up,
            })
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
            JumpInfo {
                fall_time,
                z_speed,
                has_direction,
                sin_angle,
                cos_angle,
                xy_speed,
            }
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
            inertia,
            adv_flying,
            standing_on_gameobject_guid,
        })
    }

    /// Write movement info to a packet (for MoveUpdate broadcasts).
    pub fn write(&self, pkt: &mut WorldPacket) {
        let has_transport = self.transport.is_some();
        let has_fall_direction = self
            .flags
            .intersects(MovementFlag::FALLING | MovementFlag::FALLING_FAR);
        let has_fall = has_fall_direction || self.jump.fall_time != 0;
        let has_inertia = self.inertia.is_some();
        let has_adv_flying = self.adv_flying.is_some();
        let has_standing_on_gameobject_guid = self.standing_on_gameobject_guid.is_some();

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

        pkt.write_bit(has_standing_on_gameobject_guid);
        pkt.write_bit(has_transport);
        pkt.write_bit(has_fall);
        pkt.write_bit(false); // has_spline
        pkt.write_bit(false); // height_change_failed
        pkt.write_bit(false); // remote_time_valid
        pkt.write_bit(has_inertia);
        pkt.write_bit(has_adv_flying);
        pkt.flush_bits();

        if let Some(t) = &self.transport {
            let prev_time = t.prev_time.filter(|time| *time != 0);
            let vehicle_id = t.vehicle_id.filter(|id| *id != 0);

            pkt.write_packed_guid(&t.guid);
            pkt.write_float(t.x);
            pkt.write_float(t.y);
            pkt.write_float(t.z);
            pkt.write_float(t.o);
            pkt.write_int8(t.seat);
            pkt.write_uint32(t.time);
            pkt.write_bit(prev_time.is_some());
            pkt.write_bit(vehicle_id.is_some());
            pkt.flush_bits();
            if let Some(pt) = prev_time {
                pkt.write_uint32(pt);
            }
            if let Some(vid) = vehicle_id {
                pkt.write_int32(vid);
            }
        }

        if let Some(guid) = &self.standing_on_gameobject_guid {
            pkt.write_packed_guid(guid);
        }

        if let Some(inertia) = &self.inertia {
            pkt.write_int32(inertia.id);
            pkt.write_float(inertia.x);
            pkt.write_float(inertia.y);
            pkt.write_float(inertia.z);
            pkt.write_uint32(inertia.lifetime);
        }

        if let Some(af) = &self.adv_flying {
            pkt.write_float(af.forward_velocity);
            pkt.write_float(af.up_velocity);
        }

        if has_fall {
            pkt.write_uint32(self.jump.fall_time);
            pkt.write_float(self.jump.z_speed);
            pkt.write_bit(has_fall_direction);
            pkt.flush_bits();
            if has_fall_direction {
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

// ── Movement ACK client packets ──────────────────────────────────

/// C++ `WorldPackets::Movement::MovementAck`.
#[derive(Debug, Clone)]
pub struct MovementAck {
    pub status: MovementInfo,
    pub ack_index: i32,
}

impl MovementAck {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            status: MovementInfo::read(pkt)?,
            ack_index: pkt.read_int32()?,
        })
    }
}

/// Generic ACK packet used by root, hover, water-walk and similar movement toggles.
#[derive(Debug, Clone)]
pub struct MovementAckMessage {
    pub ack: MovementAck,
}

impl MovementAckMessage {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            ack: MovementAck::read(pkt)?,
        })
    }
}

/// ACK packet carrying a movement speed or movement-force magnitude.
#[derive(Debug, Clone)]
pub struct MovementSpeedAck {
    pub ack: MovementAck,
    pub speed: f32,
}

impl MovementSpeedAck {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            ack: MovementAck::read(pkt)?,
            speed: pkt.read_float()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveKnockBackSpeeds {
    pub horz_speed: f32,
    pub vert_speed: f32,
}

#[derive(Debug, Clone)]
pub struct MoveKnockBackAck {
    pub ack: MovementAck,
    pub speeds: Option<MoveKnockBackSpeeds>,
}

impl MoveKnockBackAck {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let ack = MovementAck::read(pkt)?;
        let speeds = if pkt.read_bit()? {
            Some(MoveKnockBackSpeeds {
                horz_speed: pkt.read_float()?,
                vert_speed: pkt.read_float()?,
            })
        } else {
            None
        };
        Ok(Self { ack, speeds })
    }
}

#[derive(Debug, Clone)]
pub struct MoveSetCollisionHeightAck {
    pub data: MovementAck,
    pub height: f32,
    pub mount_display_id: u32,
    pub reason: u8,
}

impl MoveSetCollisionHeightAck {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            data: MovementAck::read(pkt)?,
            height: pkt.read_float()?,
            mount_display_id: pkt.read_uint32()?,
            reason: pkt.read_uint8()?,
        })
    }
}

pub const UPDATE_COLLISION_HEIGHT_REASON_SCALE_LIKE_CPP: u8 = 0;
pub const UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP: u8 = 1;
pub const UPDATE_COLLISION_HEIGHT_REASON_FORCE_LIKE_CPP: u8 = 2;

#[derive(Debug, Clone)]
pub struct MoveSetCollisionHeight {
    pub mover_guid: ObjectGuid,
    pub sequence_index: u32,
    pub height: f32,
    pub scale: f32,
    pub reason: u8,
    pub mount_display_id: u32,
    pub scale_duration: i32,
}

impl ServerPacket for MoveSetCollisionHeight {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveSetCollisionHeight;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.mover_guid);
        pkt.write_uint32(self.sequence_index);
        pkt.write_float(self.height);
        pkt.write_float(self.scale);
        pkt.write_uint8(self.reason);
        pkt.write_uint32(self.mount_display_id);
        pkt.write_int32(self.scale_duration);
        pkt.flush_bits();
    }
}

#[derive(Debug, Clone)]
pub struct MoveUpdateCollisionHeight {
    pub status: MovementInfo,
    pub height: f32,
    pub scale: f32,
}

impl ServerPacket for MoveUpdateCollisionHeight {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveUpdateCollisionHeight;

    fn write(&self, pkt: &mut WorldPacket) {
        self.status.write(pkt);
        pkt.write_float(self.height);
        pkt.write_float(self.scale);
    }
}

/// C++ `MovementForceType`, stored as two bits on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementForceType {
    SingleDirectional,
    Gravity,
    Unknown(u8),
}

impl MovementForceType {
    fn from_wire(value: u8) -> Self {
        match value {
            0 => Self::SingleDirectional,
            1 => Self::Gravity,
            value => Self::Unknown(value),
        }
    }

    pub fn to_wire(self) -> u8 {
        match self {
            Self::SingleDirectional => 0,
            Self::Gravity => 1,
            Self::Unknown(value) => value & 0x03,
        }
    }
}

/// C++ `MovementForce` wire shape.
#[derive(Debug, Clone, PartialEq)]
pub struct MovementForce {
    pub id: ObjectGuid,
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub transport_id: u32,
    pub magnitude: f32,
    pub unused_910: i32,
    pub force_type: MovementForceType,
}

impl MovementForce {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            id: pkt.read_packed_guid()?,
            origin: [pkt.read_float()?, pkt.read_float()?, pkt.read_float()?],
            direction: [pkt.read_float()?, pkt.read_float()?, pkt.read_float()?],
            transport_id: pkt.read_uint32()?,
            magnitude: pkt.read_float()?,
            unused_910: pkt.read_int32()?,
            force_type: MovementForceType::from_wire(pkt.read_bits(2)? as u8),
        })
    }

    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.id);
        for value in self.origin {
            pkt.write_float(value);
        }
        for value in self.direction {
            pkt.write_float(value);
        }
        pkt.write_uint32(self.transport_id);
        pkt.write_float(self.magnitude);
        pkt.write_int32(self.unused_910);
        pkt.write_bits(u32::from(self.force_type.to_wire()), 2);
        pkt.flush_bits();
    }
}

#[derive(Debug, Clone)]
pub struct MoveApplyMovementForceAck {
    pub ack: MovementAck,
    pub force: MovementForce,
}

impl MoveApplyMovementForceAck {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            ack: MovementAck::read(pkt)?,
            force: MovementForce::read(pkt)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MoveRemoveMovementForceAck {
    pub ack: MovementAck,
    pub id: ObjectGuid,
}

impl MoveRemoveMovementForceAck {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            ack: MovementAck::read(pkt)?,
            id: pkt.read_packed_guid()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MoveUpdateApplyMovementForce {
    pub status: MovementInfo,
    pub force: MovementForce,
}

impl ServerPacket for MoveUpdateApplyMovementForce {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveUpdateApplyMovementForce;

    fn write(&self, pkt: &mut WorldPacket) {
        self.status.write(pkt);
        self.force.write(pkt);
    }
}

#[derive(Debug, Clone)]
pub struct MoveUpdateRemoveMovementForce {
    pub status: MovementInfo,
    pub trigger_guid: ObjectGuid,
}

impl ServerPacket for MoveUpdateRemoveMovementForce {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveUpdateRemoveMovementForce;

    fn write(&self, pkt: &mut WorldPacket) {
        self.status.write(pkt);
        pkt.write_packed_guid(&self.trigger_guid);
    }
}

#[derive(Debug, Clone)]
pub struct MoveUpdateKnockBack {
    pub status: MovementInfo,
}

impl ServerPacket for MoveUpdateKnockBack {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveUpdateKnockBack;

    fn write(&self, pkt: &mut WorldPacket) {
        self.status.write(pkt);
    }
}

#[derive(Debug, Clone)]
pub struct MoveSkipTime {
    pub mover_guid: ObjectGuid,
    pub time_skipped: u32,
}

impl ServerPacket for MoveSkipTime {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveSkipTime;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.mover_guid);
        pkt.write_uint32(self.time_skipped);
    }
}

#[derive(Debug, Clone)]
pub struct MoveUpdateModMovementForceMagnitude {
    pub status: MovementInfo,
    pub speed: f32,
}

impl ServerPacket for MoveUpdateModMovementForceMagnitude {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveUpdateModMovementForceMagnitude;

    fn write(&self, pkt: &mut WorldPacket) {
        self.status.write(pkt);
        pkt.write_float(self.speed);
    }
}

#[derive(Debug, Clone)]
pub struct MoveTimeSkipped {
    pub mover_guid: ObjectGuid,
    pub time_skipped: u32,
}

impl MoveTimeSkipped {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            mover_guid: pkt.read_packed_guid()?,
            time_skipped: pkt.read_uint32()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MoveSplineDone {
    pub status: MovementInfo,
    pub spline_id: i32,
}

impl MoveSplineDone {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            status: MovementInfo::read(pkt)?,
            spline_id: pkt.read_int32()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MoveTeleportAck {
    pub mover_guid: ObjectGuid,
    pub ack_index: i32,
    pub move_time: i32,
}

impl MoveTeleportAck {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            mover_guid: pkt.read_packed_guid()?,
            ack_index: pkt.read_int32()?,
            move_time: pkt.read_int32()?,
        })
    }
}

// ── MoveTeleport (SMSG_MOVE_TELEPORT) ────────────────────────────

/// Sent to the moved player on a same-map teleport.
///
/// Mirrors C++ `WorldPackets::Movement::MoveTeleport::Write` for the
/// no-transport/no-vehicle case currently represented by Rust near teleports.
#[derive(Debug, Clone)]
pub struct MoveTeleport {
    pub mover_guid: ObjectGuid,
    pub position: Position,
    pub facing: f32,
    pub sequence_index: u32,
    pub preload_world: u8,
    pub transport_guid: Option<ObjectGuid>,
}

impl ServerPacket for MoveTeleport {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveTeleport;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_guid(&self.mover_guid);
        pkt.write_uint32(self.sequence_index);
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_float(self.position.z);
        pkt.write_float(self.facing);
        pkt.write_uint8(self.preload_world);
        pkt.write_bit(self.transport_guid.is_some());
        pkt.write_bit(false); // Vehicle payload not represented yet.
        pkt.flush_bits();
        if let Some(transport_guid) = self.transport_guid {
            pkt.write_guid(&transport_guid);
        }
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
/// Mirrors C++ `WorldPackets::Movement::MonsterMove`:
/// `MoverGUID`, current XYZ position, then `MovementMonsterSpline`.
#[derive(Debug, Clone)]
pub struct MonsterMove {
    pub mover_guid: ObjectGuid,
    pub current_pos: Position,
    pub spline: MovementMonsterSpline,
}

impl MonsterMove {
    pub fn single_destination(
        mover_guid: ObjectGuid,
        current_pos: Position,
        spline_id: u32,
        move_time_ms: u32,
        spline_flags: u32,
        destination: Position,
    ) -> Self {
        Self {
            mover_guid,
            current_pos,
            spline: MovementMonsterSpline {
                id: spline_id,
                destination,
                movement: MovementSpline {
                    flags: spline_flags,
                    move_time: move_time_ms,
                    points: vec![destination],
                    ..MovementSpline::default()
                },
                ..MovementMonsterSpline::default()
            },
        }
    }
}

impl ServerPacket for MonsterMove {
    const OPCODE: ServerOpcodes = ServerOpcodes::OnMonsterMove;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.mover_guid);
        write_xyz(pkt, self.current_pos);
        self.spline.write(pkt);
    }
}

#[derive(Debug, Clone)]
pub struct MovementMonsterSpline {
    pub id: u32,
    pub destination: Position,
    pub crz_teleport: bool,
    pub stop_distance_tolerance: u8,
    pub movement: MovementSpline,
}

impl Default for MovementMonsterSpline {
    fn default() -> Self {
        Self {
            id: 0,
            destination: Position::ZERO,
            crz_teleport: false,
            stop_distance_tolerance: 0,
            movement: MovementSpline::default(),
        }
    }
}

impl MovementMonsterSpline {
    #[must_use]
    pub fn from_move_spline(move_spline: &MoveSpline) -> Self {
        let mut flags = move_spline.flags();
        if move_spline.is_cyclic() {
            flags.insert(MoveSplineFlag::ENTER_CYCLE);
        }
        flags.remove(MoveSplineFlag::MASK_NO_MONSTER_MOVE);

        let path_data = move_spline.monster_move_path_data();
        Self {
            id: move_spline.id(),
            destination: move_spline.final_destination().unwrap_or(Position::ZERO),
            movement: MovementSpline {
                flags: flags.bits(),
                face: MonsterMoveFace::from_move_spline(move_spline),
                move_time: move_spline.duration_ms().max(0) as u32,
                fade_object_time: if flags.contains(MoveSplineFlag::FADE_OBJECT) {
                    move_spline.effect_start_time_ms().max(0) as u32
                } else {
                    0
                },
                points: path_data.points,
                packed_deltas: path_data.packed_deltas,
                spell_effect_extra: move_spline.spell_effect_extra().map(|data| {
                    MonsterSplineSpellEffectExtraData::from_move_data(
                        data,
                        move_spline.vertical_acceleration(),
                    )
                }),
                jump_extra: (flags.contains(MoveSplineFlag::PARABOLIC)
                    && (move_spline.spell_effect_extra().is_none()
                        || move_spline.effect_start_time_ms() != 0))
                    .then(|| MonsterSplineJumpExtraData {
                        jump_gravity: move_spline.vertical_acceleration(),
                        start_time: move_spline.effect_start_time_ms().max(0) as u32,
                        duration: 0,
                    }),
                anim_tier_transition: (flags.contains(MoveSplineFlag::ANIMATION))
                    .then_some(move_spline.anim_tier())
                    .flatten()
                    .map(|anim_tier| {
                        MonsterSplineAnimTierTransition::from_move_data(
                            anim_tier,
                            move_spline.effect_start_time_ms().max(0) as u32,
                        )
                    }),
                ..MovementSpline::default()
            },
            ..Self::default()
        }
    }

    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.id);
        write_xyz(pkt, self.destination);
        pkt.write_bit(self.crz_teleport);
        pkt.write_bits(u32::from(self.stop_distance_tolerance & 0x07), 3);
        self.movement.write(pkt);
    }
}

#[derive(Debug, Clone)]
pub struct MovementSpline {
    pub flags: u32,
    pub face: MonsterMoveFace,
    pub elapsed: i32,
    pub move_time: u32,
    pub fade_object_time: u32,
    pub points: Vec<Position>,
    pub mode: u8,
    pub vehicle_exit_voluntary: bool,
    pub interpolate: bool,
    pub transport_guid: ObjectGuid,
    pub vehicle_seat: i8,
    pub packed_deltas: Vec<[f32; 3]>,
    pub spline_filter: Option<MonsterSplineFilter>,
    pub spell_effect_extra: Option<MonsterSplineSpellEffectExtraData>,
    pub jump_extra: Option<MonsterSplineJumpExtraData>,
    pub anim_tier_transition: Option<MonsterSplineAnimTierTransition>,
}

impl Default for MovementSpline {
    fn default() -> Self {
        Self {
            flags: 0,
            face: MonsterMoveFace::Normal,
            elapsed: 0,
            move_time: 0,
            fade_object_time: 0,
            points: Vec::new(),
            mode: 0,
            vehicle_exit_voluntary: false,
            interpolate: false,
            transport_guid: ObjectGuid::EMPTY,
            vehicle_seat: -1,
            packed_deltas: Vec::new(),
            spline_filter: None,
            spell_effect_extra: None,
            jump_extra: None,
            anim_tier_transition: None,
        }
    }
}

impl MovementSpline {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.flags);
        pkt.write_int32(self.elapsed);
        pkt.write_uint32(self.move_time);
        pkt.write_uint32(self.fade_object_time);
        pkt.write_uint8(self.mode);
        pkt.write_packed_guid(&self.transport_guid);
        pkt.write_int8(self.vehicle_seat);
        pkt.write_bits(u32::from(self.face.kind()), 2);
        pkt.write_bits(self.points.len() as u32, 16);
        pkt.write_bit(self.vehicle_exit_voluntary);
        pkt.write_bit(self.interpolate);
        pkt.write_bits(self.packed_deltas.len() as u32, 16);
        pkt.write_bit(self.spline_filter.is_some());
        pkt.write_bit(self.spell_effect_extra.is_some());
        pkt.write_bit(self.jump_extra.is_some());
        pkt.write_bit(self.anim_tier_transition.is_some());
        pkt.flush_bits();

        if let Some(spline_filter) = &self.spline_filter {
            spline_filter.write(pkt);
        }

        match self.face {
            MonsterMoveFace::Normal => {}
            MonsterMoveFace::FacingSpot(pos) => write_xyz(pkt, pos),
            MonsterMoveFace::FacingTarget {
                direction,
                target_guid,
            } => {
                pkt.write_float(direction);
                pkt.write_packed_guid(&target_guid);
            }
            MonsterMoveFace::FacingAngle(direction) => pkt.write_float(direction),
        }

        for point in &self.points {
            write_xyz(pkt, *point);
        }

        for [x, y, z] in &self.packed_deltas {
            pkt.write_packed_xyz(*x, *y, *z);
        }

        if let Some(spell_effect_extra) = &self.spell_effect_extra {
            spell_effect_extra.write(pkt);
        }
        if let Some(jump_extra) = &self.jump_extra {
            jump_extra.write(pkt);
        }
        if let Some(anim_tier_transition) = &self.anim_tier_transition {
            anim_tier_transition.write(pkt);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MonsterMoveFace {
    Normal,
    FacingSpot(Position),
    FacingTarget {
        direction: f32,
        target_guid: ObjectGuid,
    },
    FacingAngle(f32),
}

impl MonsterMoveFace {
    fn from_move_spline(move_spline: &MoveSpline) -> Self {
        let facing = move_spline.facing();
        match facing.kind {
            MonsterMoveType::Normal => Self::Normal,
            MonsterMoveType::FacingSpot => Self::FacingSpot(facing.spot),
            MonsterMoveType::FacingTarget => Self::FacingTarget {
                direction: facing.angle,
                target_guid: facing.target,
            },
            MonsterMoveType::FacingAngle => Self::FacingAngle(facing.angle),
        }
    }

    fn kind(self) -> u8 {
        match self {
            MonsterMoveFace::Normal => 0,
            MonsterMoveFace::FacingSpot(_) => 1,
            MonsterMoveFace::FacingTarget { .. } => 2,
            MonsterMoveFace::FacingAngle(_) => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MonsterSplineFilterKey {
    pub index: i16,
    pub speed: u16,
}

impl MonsterSplineFilterKey {
    fn write(self, pkt: &mut WorldPacket) {
        pkt.write_int16(self.index);
        pkt.write_uint16(self.speed);
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MonsterSplineFilter {
    pub filter_keys: Vec<MonsterSplineFilterKey>,
    pub filter_flags: u8,
    pub base_speed: f32,
    pub start_offset: i16,
    pub dist_to_prev_filter_key: f32,
    pub added_to_start: i16,
}

impl MonsterSplineFilter {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.filter_keys.len() as u32);
        pkt.write_float(self.base_speed);
        pkt.write_int16(self.start_offset);
        pkt.write_float(self.dist_to_prev_filter_key);
        pkt.write_int16(self.added_to_start);
        for filter_key in &self.filter_keys {
            filter_key.write(pkt);
        }
        pkt.write_bits(u32::from(self.filter_flags & 0x03), 2);
        pkt.flush_bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonsterSplineSpellEffectExtraData {
    pub target_guid: ObjectGuid,
    pub spell_visual_id: u32,
    pub progress_curve_id: u32,
    pub parabolic_curve_id: u32,
    pub jump_gravity: f32,
}

impl MonsterSplineSpellEffectExtraData {
    fn from_move_data(data: MoveSpellEffectExtraData, jump_gravity: f32) -> Self {
        Self {
            target_guid: data.target,
            spell_visual_id: data.spell_visual_id,
            progress_curve_id: data.progress_curve_id,
            parabolic_curve_id: data.parabolic_curve_id,
            jump_gravity,
        }
    }

    fn write(self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.target_guid);
        pkt.write_uint32(self.spell_visual_id);
        pkt.write_uint32(self.progress_curve_id);
        pkt.write_uint32(self.parabolic_curve_id);
        pkt.write_float(self.jump_gravity);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MonsterSplineJumpExtraData {
    pub jump_gravity: f32,
    pub start_time: u32,
    pub duration: u32,
}

impl MonsterSplineJumpExtraData {
    fn write(self, pkt: &mut WorldPacket) {
        pkt.write_float(self.jump_gravity);
        pkt.write_uint32(self.start_time);
        pkt.write_uint32(self.duration);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MonsterSplineAnimTierTransition {
    pub tier_transition_id: i32,
    pub start_time: u32,
    pub end_time: u32,
    pub anim_tier: u8,
}

impl MonsterSplineAnimTierTransition {
    fn from_move_data(data: MoveAnimTierTransition, start_time: u32) -> Self {
        Self {
            tier_transition_id: data.tier_transition_id as i32,
            start_time,
            end_time: 0,
            anim_tier: data.anim_tier,
        }
    }

    fn write(self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.tier_transition_id);
        pkt.write_uint32(self.start_time);
        pkt.write_uint32(self.end_time);
        pkt.write_uint8(self.anim_tier);
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
        MonsterMove {
            mover_guid: self.mover_guid,
            current_pos: self.current_pos,
            spline: MovementMonsterSpline {
                id: self.spline_id,
                stop_distance_tolerance: 2,
                ..MovementMonsterSpline::default()
            },
        }
        .write(pkt);
    }
}

fn write_xyz(pkt: &mut WorldPacket, position: Position) {
    pkt.write_float(position.x);
    pkt.write_float(position.y);
    pkt.write_float(position.z);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_packet::WorldPacket;
    use wow_core::guid::HighGuid;
    use wow_movement::{AnimTierTransition, FacingInfo, MoveSplineInitArgs, SpellEffectExtraData};

    #[test]
    fn movement_info_write_includes_fall_data_when_falling_flag_is_set_like_cpp() {
        let mut info = MovementInfo {
            guid: ObjectGuid::create_player(1, 42),
            flags: MovementFlag::FALLING,
            time: 1234,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            jump: JumpInfo {
                fall_time: 0,
                z_speed: 5.0,
                has_direction: false,
                sin_angle: 0.25,
                cos_angle: 0.75,
                xy_speed: 6.0,
            },
            ..MovementInfo::default()
        };

        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);

        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let decoded = MovementInfo::read(&mut pkt).unwrap();
        assert_eq!(decoded.flags, MovementFlag::FALLING);
        assert_eq!(decoded.jump.fall_time, 0);
        assert_eq!(decoded.jump.z_speed, 5.0);
        assert!(decoded.jump.has_direction);
        assert_eq!(decoded.jump.sin_angle, 0.25);
        assert_eq!(decoded.jump.cos_angle, 0.75);
        assert_eq!(decoded.jump.xy_speed, 6.0);

        info.flags = MovementFlag::NONE;
        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let decoded = MovementInfo::read(&mut pkt).unwrap();
        assert_eq!(decoded.jump.fall_time, 0);
        assert!(!decoded.jump.has_direction);
    }

    #[test]
    fn movement_info_preserves_standing_guid_and_inertia_like_cpp() {
        let info = MovementInfo {
            guid: ObjectGuid::create_player(1, 42),
            time: 77,
            position: Position::new(10.0, 20.0, 30.0, 1.5),
            standing_on_gameobject_guid: Some(ObjectGuid::create_world_object(
                HighGuid::GameObject,
                0,
                1,
                0,
                0,
                7,
                9001,
            )),
            inertia: Some(InertiaInfo {
                id: 12,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                lifetime: 400,
            }),
            ..MovementInfo::default()
        };

        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);

        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let decoded = MovementInfo::read(&mut pkt).unwrap();
        assert_eq!(
            decoded.standing_on_gameobject_guid,
            Some(ObjectGuid::create_world_object(
                HighGuid::GameObject,
                0,
                1,
                0,
                0,
                7,
                9001,
            ))
        );
        let inertia = decoded.inertia.unwrap();
        assert_eq!(inertia.id, 12);
        assert_eq!(inertia.x, 1.0);
        assert_eq!(inertia.y, 2.0);
        assert_eq!(inertia.z, 3.0);
        assert_eq!(inertia.lifetime, 400);
    }

    #[test]
    fn movement_transport_write_skips_zero_optional_values_like_cpp() {
        let info = MovementInfo {
            guid: ObjectGuid::create_player(1, 42),
            time: 77,
            position: Position::new(10.0, 20.0, 30.0, 1.5),
            transport: Some(TransportInfo {
                guid: ObjectGuid::create_world_object(HighGuid::Transport, 0, 1, 0, 0, 9, 100),
                x: 1.0,
                y: 2.0,
                z: 3.0,
                o: 4.0,
                seat: 2,
                time: 55,
                prev_time: Some(0),
                vehicle_id: Some(0),
            }),
            ..MovementInfo::default()
        };

        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);

        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let decoded = MovementInfo::read(&mut pkt).unwrap();
        let transport = decoded.transport.unwrap();
        assert_eq!(transport.prev_time, None);
        assert_eq!(transport.vehicle_id, None);
    }

    #[test]
    fn movement_ack_packets_read_cpp_field_order() {
        let info = MovementInfo {
            guid: ObjectGuid::create_player(1, 42),
            time: 1234,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            ..MovementInfo::default()
        };

        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);
        pkt.write_int32(77);
        pkt.write_float(7.5);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let speed_ack = MovementSpeedAck::read(&mut pkt).unwrap();
        assert_eq!(speed_ack.ack.status.guid, info.guid);
        assert_eq!(speed_ack.ack.ack_index, 77);
        assert_eq!(speed_ack.speed, 7.5);

        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);
        pkt.write_int32(78);
        pkt.write_bit(true);
        pkt.write_float(8.0);
        pkt.write_float(9.0);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let knockback = MoveKnockBackAck::read(&mut pkt).unwrap();
        assert_eq!(knockback.ack.ack_index, 78);
        assert_eq!(
            knockback.speeds,
            Some(MoveKnockBackSpeeds {
                horz_speed: 8.0,
                vert_speed: 9.0
            })
        );

        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);
        pkt.write_int32(79);
        pkt.write_float(2.25);
        pkt.write_uint32(123);
        pkt.write_uint8(4);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let collision = MoveSetCollisionHeightAck::read(&mut pkt).unwrap();
        assert_eq!(collision.data.ack_index, 79);
        assert_eq!(collision.height, 2.25);
        assert_eq!(collision.mount_display_id, 123);
        assert_eq!(collision.reason, 4);
    }

    #[test]
    fn movement_time_spline_and_teleport_ack_packets_read_cpp_field_order() {
        let guid = ObjectGuid::create_player(1, 42);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.write_uint32(250);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let skipped = MoveTimeSkipped::read(&mut pkt).unwrap();
        assert_eq!(skipped.mover_guid, guid);
        assert_eq!(skipped.time_skipped, 250);

        let info = MovementInfo {
            guid,
            time: 1234,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            ..MovementInfo::default()
        };
        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);
        pkt.write_int32(9001);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let spline = MoveSplineDone::read(&mut pkt).unwrap();
        assert_eq!(spline.status.guid, guid);
        assert_eq!(spline.spline_id, 9001);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.write_int32(11);
        pkt.write_int32(12);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let teleport = MoveTeleportAck::read(&mut pkt).unwrap();
        assert_eq!(teleport.mover_guid, guid);
        assert_eq!(teleport.ack_index, 11);
        assert_eq!(teleport.move_time, 12);
    }

    #[test]
    fn move_teleport_writes_cpp_no_transport_field_order() {
        let guid = ObjectGuid::create_player(1, 42);
        let packet = MoveTeleport {
            mover_guid: guid,
            position: Position::new(1.25, 2.5, 3.75, 4.0),
            facing: 4.0,
            sequence_index: 77,
            preload_world: 0,
            transport_guid: None,
        };

        let bytes = packet.to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::MoveTeleport as u16
        );
        assert_eq!(pkt.read_int64().unwrap(), guid.low_value());
        assert_eq!(pkt.read_int64().unwrap(), guid.high_value());
        assert_eq!(pkt.read_uint32().unwrap(), 77);
        assert_eq!(pkt.read_float().unwrap(), 1.25);
        assert_eq!(pkt.read_float().unwrap(), 2.5);
        assert_eq!(pkt.read_float().unwrap(), 3.75);
        assert_eq!(pkt.read_float().unwrap(), 4.0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert!(!pkt.read_bit().unwrap());
        assert!(!pkt.read_bit().unwrap());
    }

    #[test]
    fn movement_force_ack_packets_read_cpp_field_order() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let force_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 9, 88);
        let info = MovementInfo {
            guid: player_guid,
            time: 1234,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            ..MovementInfo::default()
        };
        let force = MovementForce {
            id: force_guid,
            origin: [1.0, 2.0, 3.0],
            direction: [4.0, 5.0, 6.0],
            transport_id: 7,
            magnitude: 8.5,
            unused_910: 9,
            force_type: MovementForceType::Gravity,
        };

        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);
        pkt.write_int32(33);
        force.write(&mut pkt);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let apply = MoveApplyMovementForceAck::read(&mut pkt).unwrap();
        assert_eq!(apply.ack.status.guid, player_guid);
        assert_eq!(apply.ack.ack_index, 33);
        assert_eq!(apply.force, force);

        let mut pkt = WorldPacket::new_empty();
        info.write(&mut pkt);
        pkt.write_int32(34);
        pkt.write_packed_guid(&force_guid);
        let mut pkt = WorldPacket::from_bytes(pkt.data());
        let remove = MoveRemoveMovementForceAck::read(&mut pkt).unwrap();
        assert_eq!(remove.ack.status.guid, player_guid);
        assert_eq!(remove.ack.ack_index, 34);
        assert_eq!(remove.id, force_guid);
    }

    #[test]
    fn movement_ack_update_packets_write_cpp_field_order() {
        let guid = ObjectGuid::create_player(1, 42);
        let info = MovementInfo {
            guid,
            time: 1234,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            ..MovementInfo::default()
        };
        let force = MovementForce {
            id: ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 9, 88),
            origin: [1.0, 2.0, 3.0],
            direction: [4.0, 5.0, 6.0],
            transport_id: 7,
            magnitude: 8.5,
            unused_910: 9,
            force_type: MovementForceType::Gravity,
        };

        let update = MoveUpdateApplyMovementForce {
            status: info.clone(),
            force: force.clone(),
        };
        let bytes = update.to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        let decoded_info = MovementInfo::read(&mut pkt).unwrap();
        let decoded_force = MovementForce::read(&mut pkt).unwrap();
        assert_eq!(decoded_info.guid, guid);
        assert_eq!(decoded_force, force);

        let remove = MoveUpdateRemoveMovementForce {
            status: info.clone(),
            trigger_guid: force.id,
        };
        let bytes = remove.to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        let decoded_info = MovementInfo::read(&mut pkt).unwrap();
        let decoded_guid = pkt.read_packed_guid().unwrap();
        assert_eq!(decoded_info.guid, guid);
        assert_eq!(decoded_guid, force.id);

        let skipped = MoveSkipTime {
            mover_guid: guid,
            time_skipped: 250,
        };
        let bytes = skipped.to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_packed_guid().unwrap(), guid);
        assert_eq!(pkt.read_uint32().unwrap(), 250);

        let magnitude = MoveUpdateModMovementForceMagnitude {
            status: info,
            speed: 1.25,
        };
        let bytes = magnitude.to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        let decoded_info = MovementInfo::read(&mut pkt).unwrap();
        assert_eq!(decoded_info.guid, guid);
        assert_eq!(pkt.read_float().unwrap(), 1.25);
    }

    #[test]
    fn monster_move_single_destination_writes_cpp_spline_order() {
        let mover = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9, 88);
        let packet = MonsterMove::single_destination(
            mover,
            Position::new(1.0, 2.0, 3.0, 0.0),
            77,
            1_500,
            0x0040_0000,
            Position::new(10.0, 20.0, 30.0, 0.0),
        );
        let bytes = packet.to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);

        assert_eq!(pkt.read_packed_guid().unwrap(), mover);
        assert_eq!(pkt.read_float().unwrap(), 1.0);
        assert_eq!(pkt.read_float().unwrap(), 2.0);
        assert_eq!(pkt.read_float().unwrap(), 3.0);
        assert_eq!(pkt.read_uint32().unwrap(), 77);
        assert_eq!(pkt.read_float().unwrap(), 10.0);
        assert_eq!(pkt.read_float().unwrap(), 20.0);
        assert_eq!(pkt.read_float().unwrap(), 30.0);
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_bits(3).unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0x0040_0000);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 1_500);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_int8().unwrap(), -1);
        assert_eq!(pkt.read_bits(2).unwrap(), 0);
        assert_eq!(pkt.read_bits(16).unwrap(), 1);
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_bits(16).unwrap(), 0);
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_float().unwrap(), 10.0);
        assert_eq!(pkt.read_float().unwrap(), 20.0);
        assert_eq!(pkt.read_float().unwrap(), 30.0);
        assert!(pkt.is_empty());
    }

    #[test]
    fn monster_move_stop_writes_cpp_stop_tolerance_without_done_flag() {
        let mover = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9, 88);
        let packet = MonsterMoveStop {
            mover_guid: mover,
            current_pos: Position::new(1.0, 2.0, 3.0, 0.0),
            spline_id: 78,
        };
        let bytes = packet.to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);

        assert_eq!(pkt.read_packed_guid().unwrap(), mover);
        assert_eq!(pkt.read_float().unwrap(), 1.0);
        assert_eq!(pkt.read_float().unwrap(), 2.0);
        assert_eq!(pkt.read_float().unwrap(), 3.0);
        assert_eq!(pkt.read_uint32().unwrap(), 78);
        assert_eq!(pkt.read_float().unwrap(), 0.0);
        assert_eq!(pkt.read_float().unwrap(), 0.0);
        assert_eq!(pkt.read_float().unwrap(), 0.0);
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_bits(3).unwrap(), 2);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_int8().unwrap(), -1);
        assert_eq!(pkt.read_bits(2).unwrap(), 0);
        assert_eq!(pkt.read_bits(16).unwrap(), 0);
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_bits(16).unwrap(), 0);
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(pkt.is_empty());
    }

    #[test]
    fn monster_move_writes_cpp_face_angle_and_packed_deltas() {
        let mover = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9, 88);
        let packet = MonsterMove {
            mover_guid: mover,
            current_pos: Position::new(0.0, 0.0, 0.0, 0.0),
            spline: MovementMonsterSpline {
                id: 79,
                destination: Position::new(12.0, 0.0, 0.0, 0.0),
                movement: MovementSpline {
                    face: MonsterMoveFace::FacingAngle(1.25),
                    points: vec![Position::new(12.0, 0.0, 0.0, 0.0)],
                    packed_deltas: vec![[1.0, -2.0, 3.0]],
                    ..MovementSpline::default()
                },
                ..MovementMonsterSpline::default()
            },
        };
        let bytes = packet.to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);

        assert_eq!(pkt.read_packed_guid().unwrap(), mover);
        for _ in 0..3 {
            pkt.read_float().unwrap();
        }
        assert_eq!(pkt.read_uint32().unwrap(), 79);
        for _ in 0..3 {
            pkt.read_float().unwrap();
        }
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_bits(3).unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_int8().unwrap(), -1);
        assert_eq!(pkt.read_bits(2).unwrap(), 3);
        assert_eq!(pkt.read_bits(16).unwrap(), 1);
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_bits(16).unwrap(), 1);
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_float().unwrap(), 1.25);
        assert_eq!(pkt.read_float().unwrap(), 12.0);
        assert_eq!(pkt.read_float().unwrap(), 0.0);
        assert_eq!(pkt.read_float().unwrap(), 0.0);
        let expected_packed = ((1.0f32 / 0.25) as i32 as u32 & 0x7ff)
            | (((-2.0f32 / 0.25) as i32 as u32 & 0x7ff) << 11)
            | (((3.0f32 / 0.25) as i32 as u32 & 0x3ff) << 22);
        assert_eq!(pkt.read_uint32().unwrap(), expected_packed);
        assert!(pkt.is_empty());
    }

    #[test]
    fn movement_spline_writes_cpp_optional_payload_order() {
        let target = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        MovementSpline {
            spline_filter: Some(MonsterSplineFilter {
                filter_keys: vec![MonsterSplineFilterKey {
                    index: -2,
                    speed: 35,
                }],
                filter_flags: 3,
                base_speed: 1.5,
                start_offset: -4,
                dist_to_prev_filter_key: 2.5,
                added_to_start: 6,
            }),
            points: vec![Position::xyz(4.0, 5.0, 6.0)],
            spell_effect_extra: Some(MonsterSplineSpellEffectExtraData {
                target_guid: target,
                spell_visual_id: 11,
                progress_curve_id: 22,
                parabolic_curve_id: 33,
                jump_gravity: 44.5,
            }),
            jump_extra: Some(MonsterSplineJumpExtraData {
                jump_gravity: 9.25,
                start_time: 55,
                duration: 66,
            }),
            anim_tier_transition: Some(MonsterSplineAnimTierTransition {
                tier_transition_id: -77,
                start_time: 88,
                end_time: 99,
                anim_tier: 3,
            }),
            ..MovementSpline::default()
        }
        .write(&mut pkt);
        let mut pkt = WorldPacket::from_bytes(pkt.data());

        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_int8().unwrap(), -1);
        assert_eq!(pkt.read_bits(2).unwrap(), 0);
        assert_eq!(pkt.read_bits(16).unwrap(), 1);
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_bits(16).unwrap(), 0);
        assert!(pkt.has_bit().unwrap());
        assert!(pkt.has_bit().unwrap());
        assert!(pkt.has_bit().unwrap());
        assert!(pkt.has_bit().unwrap());

        assert_eq!(pkt.read_uint32().unwrap(), 1);
        assert_eq!(pkt.read_float().unwrap(), 1.5);
        assert_eq!(pkt.read_int16().unwrap(), -4);
        assert_eq!(pkt.read_float().unwrap(), 2.5);
        assert_eq!(pkt.read_int16().unwrap(), 6);
        assert_eq!(pkt.read_int16().unwrap(), -2);
        assert_eq!(pkt.read_uint16().unwrap(), 35);
        assert_eq!(pkt.read_bits(2).unwrap(), 3);

        assert_eq!(pkt.read_float().unwrap(), 4.0);
        assert_eq!(pkt.read_float().unwrap(), 5.0);
        assert_eq!(pkt.read_float().unwrap(), 6.0);

        assert_eq!(pkt.read_packed_guid().unwrap(), target);
        assert_eq!(pkt.read_uint32().unwrap(), 11);
        assert_eq!(pkt.read_uint32().unwrap(), 22);
        assert_eq!(pkt.read_uint32().unwrap(), 33);
        assert_eq!(pkt.read_float().unwrap(), 44.5);
        assert_eq!(pkt.read_float().unwrap(), 9.25);
        assert_eq!(pkt.read_uint32().unwrap(), 55);
        assert_eq!(pkt.read_uint32().unwrap(), 66);
        assert_eq!(pkt.read_int32().unwrap(), -77);
        assert_eq!(pkt.read_uint32().unwrap(), 88);
        assert_eq!(pkt.read_uint32().unwrap(), 99);
        assert_eq!(pkt.read_uint8().unwrap(), 3);
        assert!(pkt.is_empty());
    }

    #[test]
    fn movement_monster_spline_from_move_spline_matches_cpp_mapping() {
        let target = ObjectGuid::create_player(1, 55);
        let args = MoveSplineInitArgs {
            path: vec![
                Position::xyz(0.0, 0.0, 0.0),
                Position::xyz(10.0, 0.0, 0.0),
                Position::xyz(20.0, 0.0, 0.0),
            ],
            facing: FacingInfo {
                kind: MonsterMoveType::FacingTarget,
                target,
                angle: 1.75,
                ..FacingInfo::default()
            },
            flags: MoveSplineFlag::UNCOMPRESSED_PATH | MoveSplineFlag::PARABOLIC,
            velocity: 10.0,
            vertical_acceleration: 12.5,
            effect_start_time_ms: 250,
            spline_id: 123,
            spell_effect_extra: Some(SpellEffectExtraData {
                target,
                spell_visual_id: 777,
                progress_curve_id: 888,
                parabolic_curve_id: 999,
            }),
            ..MoveSplineInitArgs::default()
        };
        let mut move_spline = MoveSpline::new();
        move_spline.initialize(&args).unwrap();
        move_spline.finalize();

        let packet_spline = MovementMonsterSpline::from_move_spline(&move_spline);

        assert_eq!(packet_spline.id, 123);
        assert_eq!(packet_spline.destination, Position::xyz(20.0, 0.0, 0.0));
        assert_eq!(
            packet_spline.movement.flags,
            (MoveSplineFlag::UNCOMPRESSED_PATH | MoveSplineFlag::PARABOLIC).bits()
        );
        assert_eq!(
            packet_spline.movement.face,
            MonsterMoveFace::FacingTarget {
                direction: 1.75,
                target_guid: target,
            }
        );
        assert_eq!(
            packet_spline.movement.points,
            vec![Position::xyz(10.0, 0.0, 0.0), Position::xyz(20.0, 0.0, 0.0)]
        );
        assert!(packet_spline.movement.packed_deltas.is_empty());
        assert_eq!(
            packet_spline.movement.spell_effect_extra,
            Some(MonsterSplineSpellEffectExtraData {
                target_guid: target,
                spell_visual_id: 777,
                progress_curve_id: 888,
                parabolic_curve_id: 999,
                jump_gravity: 12.5,
            })
        );
        assert_eq!(
            packet_spline.movement.jump_extra,
            Some(MonsterSplineJumpExtraData {
                jump_gravity: 12.5,
                start_time: 250,
                duration: 0,
            })
        );
    }

    #[test]
    fn movement_monster_spline_from_move_spline_maps_animation_tier_like_cpp() {
        let mut flags = MoveSplineFlag::empty();
        flags.enable_animation();
        let args = MoveSplineInitArgs {
            path: vec![Position::xyz(0.0, 0.0, 0.0), Position::xyz(10.0, 0.0, 0.0)],
            flags,
            velocity: 10.0,
            effect_start_time_ms: 125,
            anim_tier: Some(AnimTierTransition {
                tier_transition_id: 44,
                anim_tier: 2,
            }),
            ..MoveSplineInitArgs::default()
        };
        let mut move_spline = MoveSpline::new();
        move_spline.initialize(&args).unwrap();

        let packet_spline = MovementMonsterSpline::from_move_spline(&move_spline);

        assert_eq!(
            packet_spline.movement.anim_tier_transition,
            Some(MonsterSplineAnimTierTransition {
                tier_transition_id: 44,
                start_time: 125,
                end_time: 0,
                anim_tier: 2,
            })
        );
        assert!(packet_spline.movement.jump_extra.is_none());
    }

    #[test]
    fn move_set_collision_height_matches_cpp_opcode_and_tail() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = MoveSetCollisionHeight {
            mover_guid: guid,
            sequence_index: 7,
            height: 1.5,
            scale: 1.0,
            reason: UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP,
            mount_display_id: 4321,
            scale_duration: 0,
        };
        let bytes = pkt.to_bytes();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2e13);
        assert!(bytes.len() > 23);
        assert_eq!(
            &bytes[bytes.len() - 21..bytes.len() - 17],
            &7u32.to_le_bytes()
        );
        assert_eq!(
            &bytes[bytes.len() - 17..bytes.len() - 13],
            &1.5f32.to_le_bytes()
        );
        assert_eq!(
            &bytes[bytes.len() - 13..bytes.len() - 9],
            &1.0f32.to_le_bytes()
        );
        assert_eq!(
            bytes[bytes.len() - 9],
            UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP
        );
        assert_eq!(
            &bytes[bytes.len() - 8..bytes.len() - 4],
            &4321u32.to_le_bytes()
        );
        assert_eq!(&bytes[bytes.len() - 4..], &0i32.to_le_bytes());
    }

    #[test]
    fn move_update_collision_height_matches_cpp_opcode_and_tail() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = MoveUpdateCollisionHeight {
            status: MovementInfo {
                guid,
                position: Position::xyz(1.0, 2.0, 3.0),
                time: 77,
                ..MovementInfo::default()
            },
            height: 1.5,
            scale: 1.0,
        };
        let bytes = pkt.to_bytes();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2ddf);
        assert!(bytes.len() > 10);
        assert_eq!(
            &bytes[bytes.len() - 8..bytes.len() - 4],
            &1.5f32.to_le_bytes()
        );
        assert_eq!(&bytes[bytes.len() - 4..], &1.0f32.to_le_bytes());
    }
}
