// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet handlers — CMSG_MOVE_*.
//!
//! All movement opcodes map to the same handler logic:
//!   1. Parse MovementInfo from the packet
//!   2. Sanitize movement flags like `Player::ValidateMovementInfo`
//!   3. Validate: GUID must match `Player::GetUnitBeingMoved()`, position must be finite
//!   4. Update server-side mover position when represented
//!   5. Broadcast SMSG_MOVE_UPDATE to nearby visible sessions
//!
//! Reference: C++ `WorldSession::HandleMovementOpcode`.

use tracing::{info, trace, warn};
use wow_packet::ClientPacket;

use wow_constants::ClientOpcodes;
use wow_constants::movement::MovementFlag;
use wow_constants::unit::UnitStandStateType;

use crate::map_manager::zone_and_area_for_position_like_cpp;
use crate::session::{
    AreaTriggerCatalogsLikeCpp, MovementTransportMembershipLikeCpp, ProgressionCatalogsLikeCpp,
    SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP,
    SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP, SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP,
    WorldSession,
};
use wow_packet::ServerPacket;
use wow_packet::packets::movement::{
    ClientPlayerMovement, MoveSplineDone, MoveUpdate, MovementAckMessage, MovementInfo,
    MovementSpeedAck,
};

mod ops_1;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
mod test_shims;

#[cfg(test)]
#[path = "../../unit_tests/handlers/movement/tests/mod.rs"]
mod tests;
