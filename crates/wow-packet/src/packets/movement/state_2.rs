//! Movement info and packets state definitions, part 2 of 2.
//!
//! Separated from the movement.rs root under #650. Behaviour is preserved.

use super::*;

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
            // C++ `MonsterMove::InitializeSplineData` leaves
            // `MovementMonsterSpline::Destination` at its default value for
            // SMSG_ON_MONSTER_MOVE; only the nested MovementSpline path carries
            // the destination.
            destination: Position::ZERO,
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
        // C++ `operator<<(MovementSpline)` opens with `data << uint32(Flags)`,
        // and every `ByteBuffer::operator<<` routes through
        // `ByteBuffer::append()`, whose first line is `FlushBits()`
        // (ByteBuffer.cpp:100). That flushes the 4 bits written just before by
        // `MovementMonsterSpline::write` (CrzTeleport + StopDistanceTolerance)
        // into their own byte BEFORE the flags u32. Omitting this flush shifts
        // the entire spline tail one byte earlier on the wire; the 3.4.3 client
        // then discards the spline and the creature appears frozen.
        pkt.flush_bits();
        pkt.write_uint32_unflushed(self.flags);
        pkt.write_int32_unflushed(self.elapsed);
        pkt.write_uint32_unflushed(self.move_time);
        pkt.write_uint32_unflushed(self.fade_object_time);
        pkt.write_uint8_unflushed(self.mode);
        pkt.write_packed_guid_unflushed(&self.transport_guid);
        pkt.write_int8_unflushed(self.vehicle_seat);
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
    pub(super) fn from_move_spline(move_spline: &MoveSpline) -> Self {
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

    pub(super) fn kind(self) -> u8 {
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
    pub(super) fn write(self, pkt: &mut WorldPacket) {
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
    pub(super) fn write(&self, pkt: &mut WorldPacket) {
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
    pub(super) fn from_move_data(data: MoveSpellEffectExtraData, jump_gravity: f32) -> Self {
        Self {
            target_guid: data.target,
            spell_visual_id: data.spell_visual_id,
            progress_curve_id: data.progress_curve_id,
            parabolic_curve_id: data.parabolic_curve_id,
            jump_gravity,
        }
    }

    pub(super) fn write(self, pkt: &mut WorldPacket) {
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
    pub(super) fn write(self, pkt: &mut WorldPacket) {
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
    pub(super) fn from_move_data(data: MoveAnimTierTransition, start_time: u32) -> Self {
        Self {
            tier_transition_id: data.tier_transition_id as i32,
            start_time,
            end_time: 0,
            anim_tier: data.anim_tier,
        }
    }

    pub(super) fn write(self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.tier_transition_id);
        pkt.write_uint32(self.start_time);
        pkt.write_uint32(self.end_time);
        pkt.write_uint8(self.anim_tier);
    }
}

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

pub(super) fn write_xyz(pkt: &mut WorldPacket, position: Position) {
    pkt.write_float(position.x);
    pkt.write_float(position.y);
    pkt.write_float(position.z);
}

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
