// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `SessionFixtures::movement` sub-state (#1241 F2): moved fields, no logic.

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::movement_protocol::{
    MovementAckEventLikeCpp, MovementFallDamageEvent, MovementSpeedAckEventLikeCpp,
    MovementUnderMapDamageEvent, UnitMoveTypeLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::{
    RepresentedVehicleBaseMovementLikeCpp, RepresentedVehicleDismissMovementLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_constants::MovementFlag;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;

/// Player movement fixtures: position, flags, jump and fall, acks, speeds and force mods, and
/// vehicle movement sinks.
pub struct MovementState {
    // ── Dual-connection (realm + instance) ───────────────────────
    // After ConnectTo completes, the session uses the instance socket for
    // game packets but MUST keep the realm socket alive — the WoW client
    // disconnects if either connection drops.

    // ── Movement & World position ─────────────────────────────────
    /// Server-side position of the player (updated from CMSG_MOVE_*).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_position: Option<wow_core::Position>,
    /// Last accepted player movement flags, mirroring C++ `Unit::m_movementInfo`.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_movement_flags_like_cpp: MovementFlag,
    /// Represented C++ `MOVEMENTFLAG2_CAN_SWIM_TO_FLY_TRANS` server-controlled state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_can_swim_to_fly_transition_like_cpp: bool,
    /// Represented `m_unitMovedByMe->GetVehicle()->GetVehicleInfo()->Flags & VEHICLE_FLAG_FIXED_POSITION`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_mover_fixed_position_vehicle_like_cpp: bool,
    /// Represented `Unit::m_movementInfo.time` for client movement ACK side effects.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_movement_time_like_cpp: u32,
    /// Represented `Unit::m_movementInfo.jump`, reset by `Player::TeleportTo`.
    /// Test-only evidence: production consumes the typed movement status and
    /// does not retain a second packet-shaped jump mirror.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_movement_jump_like_cpp: wow_packet::packets::movement::JumpInfo,
    /// C++ `Player::m_lastFallTime`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub last_fall_time_like_cpp: u32,
    /// C++ `Player::m_lastFallZ`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub last_fall_z_like_cpp: f32,
    /// Recorded fall damage events until combat log/update packet runtime is complete.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fall_damage_events_like_cpp: Vec<MovementFallDamageEvent>,
    /// C++ `PLAYER_FLAGS_IS_OUT_OF_BOUNDS` represented state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_out_of_bounds_like_cpp: bool,
    /// Recorded `DAMAGE_FALL_TO_VOID` events until environmental damage packets are complete.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub under_map_damage_events_like_cpp: Vec<MovementUnderMapDamageEvent>,
    /// Count of C++ jump proc side effects requested by movement.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub movement_jump_proc_requests_like_cpp: u32,

    /// C++ `Player::GetUnitBeingMoved()` represented GUID.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_moved_unit_guid_like_cpp: ObjectGuid,

    /// ACKs accepted by represented movement handling until full Unit movement runtime/broadcasts exist.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub movement_ack_events_like_cpp: Vec<MovementAckEventLikeCpp>,
    /// Represented `m_movementInfo = MoveDismissVehicle.Status` before live `ExitVehicle`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_vehicle_dismiss_movements_like_cpp:
        Vec<RepresentedVehicleDismissMovementLikeCpp>,
    /// Represented `vehicle_base->m_movementInfo = MoveChangeVehicleSeats.Status`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_vehicle_base_movements_like_cpp: Vec<RepresentedVehicleBaseMovementLikeCpp>,
    /// C++ `Unit::m_movementCounter`: one per-player counter shared by ALL movement-control
    /// packets (vehicle-rec, collision height, near-teleport, speed/flag changes) and read
    /// for `SMSG_RESUME_TOKEN` SequenceIndex on far teleport. Reset to 0 in
    /// `send_initial_packets_before_add_to_map` (non-seamless). #NEXT.R8.ENTITIES.1229.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub movement_counter_like_cpp: u32,
    /// Represented `Unit::GetCollisionHeight()` until model-display collision data owns it.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_collision_height_like_cpp: f32,
    /// Count of C++ `ProcessDelayedOperations` calls after successful near teleport ACK.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub delayed_operations_processed_like_cpp: u32,
    /// C++ `Player::m_forced_speed_changes[MAX_MOVE_TYPE]` represented state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub forced_speed_changes_like_cpp: [u8; UnitMoveTypeLikeCpp::COUNT],
    /// C++ `Unit::m_speed_rate[MAX_MOVE_TYPE]` represented state for player-controlled movers.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub movement_speed_rates_like_cpp: [f32; UnitMoveTypeLikeCpp::COUNT],
    /// C++ `Player::m_movementForceModMagnitudeChanges` represented state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub movement_force_mod_magnitude_changes_like_cpp: u8,
    /// C++ `MovementForces::GetModMagnitude()` represented value; default is 1.0 when no force container exists.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub movement_force_mod_magnitude_like_cpp: f32,
    /// Speed ACK outcomes recorded until full Unit speed runtime owns this state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub movement_speed_ack_events_like_cpp: Vec<MovementSpeedAckEventLikeCpp>,
}
