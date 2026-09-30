// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement update blocks.

use super::*;
use crate::packets::movement::{
    MonsterMoveFace, MonsterSplineAnimTierTransition, MonsterSplineJumpExtraData,
    MonsterSplineSpellEffectExtraData,
};

/// Packet-ready spline data embedded in a creature CreateObject movement block.
/// Runtime spline selection and conversion belong to the world boundary.
#[derive(Debug, Clone)]
pub struct CreateObjectSplineDataLikeCpp {
    pub id: u32,
    pub destination: Position,
    pub has_spline_move: bool,
    pub flags: u32,
    pub elapsed_ms: i32,
    pub duration_ms: u32,
    pub face: MonsterMoveFace,
    pub fade_object_time: Option<u32>,
    pub points: Vec<Position>,
    pub spell_effect_extra: Option<MonsterSplineSpellEffectExtraData>,
    pub jump_extra: Option<MonsterSplineJumpExtraData>,
    pub anim_tier_transition: Option<MonsterSplineAnimTierTransition>,
}

/// Movement data included in a CreateObject block.
#[derive(Debug, Clone)]
pub struct MovementBlock {
    pub position: Position,
    pub movement_flags: u32,
    pub movement_flags2: u32,
    pub movement_flags3: u32,
    /// C++ `MovementInfo::transport`, written inside `MovementUpdate` when
    /// `HasTransport` is set. This is distinct from the top-level
    /// `CreateObjectBits::MovementTransport` fallback used by world objects
    /// without a normal movement block.
    pub transport: Option<Box<TransportInfo>>,
    pub create_object_spline: Option<CreateObjectSplineDataLikeCpp>,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub run_back_speed: f32,
    pub swim_speed: f32,
    pub swim_back_speed: f32,
    pub fly_speed: f32,
    pub fly_back_speed: f32,
    pub turn_rate: f32,
    pub pitch_rate: f32,
}

impl Default for MovementBlock {
    fn default() -> Self {
        Self {
            position: Position::ZERO,
            movement_flags: 0,
            movement_flags2: 0,
            movement_flags3: 0,
            transport: None,
            create_object_spline: None,
            walk_speed: 2.5,
            run_speed: 7.0,
            run_back_speed: 4.5,
            swim_speed: 4.72222,
            swim_back_speed: 2.5,
            fly_speed: 7.0,
            fly_back_speed: 4.5,
            // C++ Unit.cpp `baseMoveSpeed[MOVE_TURN_RATE]` is the literal
            // 3.141594f, not std::f32::consts::PI.
            turn_rate: 3.141594,
            // C++ Unit.cpp `playerBaseMoveSpeed[MOVE_PITCH_RATE]`.
            pitch_rate: 3.14,
        }
    }
}

// ── ObjectData VALUES delta ─────────────────────────────────────────

/// Write the movement update block (when bit 3 = true).
pub(super) fn write_movement_update(buf: &mut WorldPacket, guid: &ObjectGuid, mv: &MovementBlock) {
    let active_create_spline = mv.create_object_spline.as_ref();

    // MoverGUID
    buf.write_packed_guid(guid);

    // MovementFlags, MovementFlags2, ExtraMovementFlags2.
    // C++ `Object::BuildMovementUpdate` serializes Unit::m_movementInfo; creature
    // addons can set MOVEMENTFLAG_HOVER during `Creature::LoadCreaturesAddon`.
    buf.write_uint32(mv.movement_flags);
    buf.write_uint32(mv.movement_flags2);
    buf.write_uint32(mv.movement_flags3);

    // MoveTime
    buf.write_uint32(0);

    // Position
    buf.write_float(mv.position.x);
    buf.write_float(mv.position.y);
    buf.write_float(mv.position.z);
    buf.write_float(mv.position.orientation);

    // Pitch
    buf.write_float(0.0);

    // StepUpStartElevation (f32, NOT u32!)
    buf.write_float(0.0);

    // RemoveForcesIDs.Count
    buf.write_uint32(0);

    // MoveIndex
    buf.write_uint32(0);

    // C++ 3.4.3 `Object::BuildMovementUpdate` writes eight conditional
    // sub-bits here, ending at HasAdvFlying. A previous Rust-only ninth
    // HasDriveStatus bit crashed the 54261 client while parsing player CREATE.
    buf.write_bit(false); // HasStandingOnGameObjectGUID
    buf.write_bit(mv.transport.is_some()); // HasTransport
    buf.write_bit(false); // HasFall
    buf.write_bit(active_create_spline.is_some()); // HasSpline
    buf.write_bit(false); // HeightChangeFailed
    buf.write_bit(false); // RemoteTimeValid
    buf.write_bit(false); // HasInertia
    buf.write_bit(false); // HasAdvFlying
    buf.flush_bits();

    if let Some(transport) = &mv.transport {
        let prev_time = transport.prev_time.filter(|time| *time != 0);
        let vehicle_id = transport.vehicle_id.filter(|id| *id != 0);

        buf.write_packed_guid(&transport.guid);
        buf.write_float(transport.x);
        buf.write_float(transport.y);
        buf.write_float(transport.z);
        buf.write_float(transport.o);
        buf.write_int8(transport.seat);
        buf.write_uint32(transport.time);
        buf.write_bit(prev_time.is_some());
        buf.write_bit(vehicle_id.is_some());
        buf.flush_bits();
        if let Some(prev_time) = prev_time {
            buf.write_uint32(prev_time);
        }
        if let Some(vehicle_id) = vehicle_id {
            buf.write_int32(vehicle_id);
        }
    }

    // No standing, inertia, advFlying, or fall blocks.

    // 9 movement speeds
    buf.write_float(mv.walk_speed);
    buf.write_float(mv.run_speed);
    buf.write_float(mv.run_back_speed);
    buf.write_float(mv.swim_speed);
    buf.write_float(mv.swim_back_speed);
    buf.write_float(mv.fly_speed);
    buf.write_float(mv.fly_back_speed);
    buf.write_float(mv.turn_rate);
    buf.write_float(mv.pitch_rate);

    // MovementForces count + modMagnitude
    buf.write_int32(0);
    buf.write_float(1.0);

    // 17 AdvancedFlying parameters (default zero-state values for C++ create movement)
    buf.write_float(2.0); // airFriction
    buf.write_float(65.0); // maxVel
    buf.write_float(1.0); // liftCoefficient
    buf.write_float(3.0); // doubleJumpVelMod
    buf.write_float(10.0); // glideStartMinHeight
    buf.write_float(100.0); // addImpulseMaxSpeed
    buf.write_float(90.0); // minBankingRate
    buf.write_float(140.0); // maxBankingRate
    buf.write_float(180.0); // minPitchingRateDown
    buf.write_float(360.0); // maxPitchingRateDown
    buf.write_float(90.0); // minPitchingRateUp
    buf.write_float(270.0); // maxPitchingRateUp
    buf.write_float(30.0); // minTurnVelThreshold
    buf.write_float(80.0); // maxTurnVelThreshold
    buf.write_float(2.75); // surfaceFriction
    buf.write_float(7.0); // overMaxDeceleration
    buf.write_float(0.4); // launchSpeedCoefficient

    // HasSplineData bit
    buf.write_bit(active_create_spline.is_some());
    buf.flush_bits();

    // No movement forces.
    if let Some(spline) = active_create_spline {
        write_create_object_spline_data_block_like_cpp(buf, spline);
    }
}

pub(super) fn write_position_xyz_like_cpp(buf: &mut WorldPacket, position: Position) {
    buf.write_float(position.x);
    buf.write_float(position.y);
    buf.write_float(position.z);
}

fn write_create_object_spline_data_block_like_cpp(
    buf: &mut WorldPacket,
    spline: &CreateObjectSplineDataLikeCpp,
) {
    buf.write_uint32(spline.id);
    write_position_xyz_like_cpp(buf, spline.destination);

    buf.write_bit(spline.has_spline_move);
    buf.flush_bits();

    if !spline.has_spline_move {
        return;
    }

    buf.write_uint32(spline.flags);
    buf.write_int32(spline.elapsed_ms);
    buf.write_uint32(spline.duration_ms);
    buf.write_float(1.0);
    buf.write_float(1.0);
    buf.write_bits(u32::from(monster_move_face_kind_like_cpp(spline.face)), 2);
    buf.write_bit(spline.fade_object_time.is_some());
    buf.write_bits(spline.points.len() as u32, 16);
    buf.write_bit(false); // HasSplineFilter
    buf.write_bit(spline.spell_effect_extra.is_some());
    buf.write_bit(spline.jump_extra.is_some());
    buf.write_bit(spline.anim_tier_transition.is_some());
    buf.write_bit(false); // HasUnknown901
    buf.flush_bits();

    match spline.face {
        MonsterMoveFace::FacingSpot(position) => write_position_xyz_like_cpp(buf, position),
        MonsterMoveFace::FacingTarget { target_guid, .. } => {
            buf.write_packed_guid(&target_guid);
        }
        MonsterMoveFace::FacingAngle(angle) => buf.write_float(angle),
        MonsterMoveFace::Normal => {}
    }

    if let Some(fade_object_time) = spline.fade_object_time {
        buf.write_uint32(fade_object_time);
    }

    for point in &spline.points {
        write_position_xyz_like_cpp(buf, *point);
    }

    if let Some(extra) = spline.spell_effect_extra {
        buf.write_packed_guid(&extra.target_guid);
        buf.write_uint32(extra.spell_visual_id);
        buf.write_uint32(extra.progress_curve_id);
        buf.write_uint32(extra.parabolic_curve_id);
        buf.write_float(extra.jump_gravity);
    }

    if let Some(extra) = spline.jump_extra {
        buf.write_float(extra.jump_gravity);
        buf.write_uint32(extra.start_time);
        buf.write_uint32(extra.duration);
    }

    if let Some(anim_tier) = spline.anim_tier_transition {
        buf.write_int32(anim_tier.tier_transition_id);
        buf.write_uint32(anim_tier.start_time);
        buf.write_uint32(anim_tier.end_time);
        buf.write_uint8(anim_tier.anim_tier);
    }
}

fn monster_move_face_kind_like_cpp(face: MonsterMoveFace) -> u8 {
    match face {
        MonsterMoveFace::Normal => 0,
        MonsterMoveFace::FacingSpot(_) => 1,
        MonsterMoveFace::FacingTarget { .. } => 2,
        MonsterMoveFace::FacingAngle(_) => 3,
    }
}
