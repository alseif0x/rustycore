//! Movement info and packets state definitions, part 1 of 2.
//!
//! Separated from the movement.rs root under #650. Behaviour is preserved.

use super::*;

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

/// Generic movement packet sent by the client for all movement opcodes.
#[derive(Debug, Clone)]
pub struct ClientPlayerMovement {
    pub info: MovementInfo,
}

impl ClientPlayerMovement {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let info = MovementInfo::read(pkt)?;
        Ok(Self { info })
    }
}

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

#[derive(Debug, Clone)]
pub struct MoveSetSpeed {
    pub opcode: ServerOpcodes,
    pub mover_guid: ObjectGuid,
    pub sequence_index: u32,
    pub speed: f32,
}

impl MoveSetSpeed {
    /// C++ `WorldPackets::Movement::MoveSetSpeed` is opcode-selected by move type.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(self.opcode);
        pkt.write_packed_guid(&self.mover_guid);
        pkt.write_uint32(self.sequence_index);
        pkt.write_float(self.speed);
        pkt.flush_bits();
        pkt.into_data()
    }
}

#[derive(Debug, Clone)]
pub struct MoveUpdateSpeed {
    pub opcode: ServerOpcodes,
    pub status: MovementInfo,
    pub speed: f32,
}

impl MoveUpdateSpeed {
    /// C++ `WorldPackets::Movement::MoveUpdateSpeed` is opcode-selected by move type.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(self.opcode);
        self.status.write(&mut pkt);
        pkt.write_float(self.speed);
        pkt.into_data()
    }
}

#[derive(Debug, Clone)]
pub struct MoveSplineSetSpeed {
    pub opcode: ServerOpcodes,
    pub mover_guid: ObjectGuid,
    pub speed: f32,
}

impl MoveSplineSetSpeed {
    /// C++ `WorldPackets::Movement::MoveSplineSetSpeed` is opcode-selected by move type.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(self.opcode);
        pkt.write_packed_guid(&self.mover_guid);
        pkt.write_float(self.speed);
        pkt.flush_bits();
        pkt.into_data()
    }
}

#[derive(Debug, Clone)]
pub struct MoveSetFlag {
    pub opcode: ServerOpcodes,
    pub mover_guid: ObjectGuid,
    pub sequence_index: u32,
}

impl MoveSetFlag {
    /// C++ `WorldPackets::Movement::MoveSetFlag` is opcode-selected by flag type.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(self.opcode);
        pkt.write_packed_guid(&self.mover_guid);
        pkt.write_uint32(self.sequence_index);
        pkt.flush_bits();
        pkt.into_data()
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
    pub(super) fn from_wire(value: u8) -> Self {
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

/// Broadcast to nearby players when a unit performs a same-map teleport.
///
/// Mirrors C++ `WorldPackets::Movement::MoveUpdateTeleport::Write` for the
/// represented no-movement-forces/no-speed-optionals case.
#[derive(Debug, Clone)]
pub struct MoveUpdateTeleport {
    pub status: MovementInfo,
}

impl ServerPacket for MoveUpdateTeleport {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveUpdateTeleport;

    fn write(&self, pkt: &mut WorldPacket) {
        self.status.write(pkt);
        pkt.write_uint32(0); // MovementForces count
        pkt.write_bit(false); // WalkSpeed
        pkt.write_bit(false); // RunSpeed
        pkt.write_bit(false); // RunBackSpeed
        pkt.write_bit(false); // SwimSpeed
        pkt.write_bit(false); // SwimBackSpeed
        pkt.write_bit(false); // FlightSpeed
        pkt.write_bit(false); // FlightBackSpeed
        pkt.write_bit(false); // TurnRate
        pkt.write_bit(false); // PitchRate
        pkt.flush_bits();
    }
}

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
