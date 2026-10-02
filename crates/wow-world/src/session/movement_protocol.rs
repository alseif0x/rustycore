// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement protocol: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(crate) use wow_world_core::session::movement_protocol::MovementAckEventLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::MoveSplineDoneTaxiActionLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_core::session::movement_protocol::RepresentedTaxiFlightNodeLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_core::session::movement_protocol::MoveSplineDoneTaxiEventLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::MoveTeleportAckActionLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_core::session::movement_protocol::MoveTeleportAckEventLikeCpp;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedAreaZoneCriteriaLikeCpp {
    EnterArea(u32),
    LeaveArea(u32),
    EnterTopLevelArea(u32),
    LeaveTopLevelArea(u32),
}

#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use wow_world_core::session::movement_protocol::RepresentedTaxiFlightStateLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use wow_world_core::session::movement_protocol::canonical_taxi_flight_node_like_cpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use wow_world_core::session::movement_protocol::canonical_taxi_flight_state_like_cpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use wow_world_core::session::movement_protocol::represented_taxi_flight_state_like_cpp;

pub(crate) use wow_world_core::session::movement_protocol::MovementSpeedAckActionLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::UnitMoveTypeLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::MovementSpeedAckEventLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP;

pub(crate) use wow_world_core::session::movement_protocol::{
    creature_movement_spline_speed_opcode_like_cpp, movement_speed_ack_move_type_like_cpp,
    player_movement_speed_opcodes_like_cpp,
};

pub(in crate::session) use wow_world_core::session::movement_protocol::PLAYER_BASE_MOVE_SPEED_LIKE_CPP;

pub(crate) use wow_world_core::session::movement_protocol::{
    MovementFallDamageEvent, MovementUnderMapDamageEvent,
};

impl WorldSession {}

#[cfg(test)]
#[path = "../../unit_tests/session/movement_protocol/f3_shims.rs"]
mod f3_shims;
