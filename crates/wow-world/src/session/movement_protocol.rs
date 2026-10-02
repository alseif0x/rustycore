// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement protocol: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(crate) use wow_world_core::session::movement_protocol::MovementAckEventLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::MoveSplineDoneTaxiActionLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::movement_protocol::RepresentedTaxiFlightNodeLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::movement_protocol::MoveSplineDoneTaxiEventLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::MoveTeleportAckActionLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::movement_protocol::MoveTeleportAckEventLikeCpp;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedAreaZoneCriteriaLikeCpp {
    EnterArea(u32),
    LeaveArea(u32),
    EnterTopLevelArea(u32),
    LeaveTopLevelArea(u32),
}

pub(crate) use wow_world_core::session::movement_protocol::MovementSpeedAckActionLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::UnitMoveTypeLikeCpp;

pub(crate) use wow_world_core::session::movement_protocol::MovementSpeedAckEventLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::movement_protocol::TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP;

pub(crate) use wow_world_core::session::movement_protocol::movement_speed_ack_move_type_like_cpp;

pub(crate) use wow_world_core::session::movement_protocol::{
    MovementFallDamageEvent, MovementUnderMapDamageEvent,
};

impl WorldSession {}

#[cfg(test)]
#[path = "../../unit_tests/session/movement_protocol/f3_shims.rs"]
mod f3_shims;
