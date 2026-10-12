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

use tracing::trace;

use wow_constants::ClientOpcodes;
#[cfg(test)]
use wow_constants::unit::UnitStandStateType;

use crate::session::WorldSession;
#[cfg(test)]
use crate::session::{
    SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP, SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP,
};
#[cfg(test)]
use wow_packet::packets::movement::MovementInfo;
use wow_packet::packets::movement::MovementSpeedAck;

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
