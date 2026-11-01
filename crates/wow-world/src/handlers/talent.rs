// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Talent handlers.
//!
//! `#1263 F5 deferred families`: the two registrations moved unchanged to
//! `wow_world_application::register_talent_handlers_like_cpp`; `talent/host.rs`
//! lends that registrar these operations. The CMSG_LEARN_TALENT body moved to
//! `wow_world_spell` (#1363 slice 9b); the respec wipe with the talent-reset
//! commit and publication it awaits to `wow_world_application` (slice 13b).

#[cfg(test)]
use tracing::warn;

#[cfg(test)]
use wow_constants::ClientOpcodes;
#[cfg(test)]
use wow_constants::unit::NPCFlags1;
#[cfg(test)]
use wow_handler::{PacketProcessing, SessionStatus};

#[cfg(test)]
use wow_packet::WorldPacket;
#[cfg(test)]
use wow_packet::packets::talent::LearnTalents;
#[cfg(test)]
use wow_packet::packets::talent::SPEC_RESET_TALENTS_LIKE_CPP;

#[cfg(test)]
use crate::session::RepresentedTalentRespecVisualSpellCastLikeCpp;
#[cfg(test)]
use crate::session::{RepresentedConfirmRespecWipeLikeCpp, WorldSession};

mod host;
mod state;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "../../unit_tests/handlers/talent/tests/mod.rs"]
mod tests;
