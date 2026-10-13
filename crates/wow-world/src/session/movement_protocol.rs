// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement protocol: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(crate) use wow_world_core::session::movement_protocol::MoveTeleportAckActionLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::movement_protocol::UnitMoveTypeLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::movement_protocol::{
    MoveSplineDoneTaxiActionLikeCpp, MoveSplineDoneTaxiEventLikeCpp, MoveTeleportAckEventLikeCpp,
    MovementAckEventLikeCpp, MovementFallDamageEvent, MovementSpeedAckActionLikeCpp,
    MovementSpeedAckEventLikeCpp, MovementUnderMapDamageEvent, RepresentedTaxiFlightNodeLikeCpp,
    TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_instances::RepresentedAreaZoneCriteriaLikeCpp;

impl WorldSession {}

#[cfg(test)]
#[path = "../../unit_tests/session/movement_protocol/f3_shims.rs"]
mod f3_shims;
