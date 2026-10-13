// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Talent handlers.
//!
//! `#1263 F5 deferred families`: the two registrations moved unchanged to
//! `wow_world_application::register_talent_handlers_like_cpp`; `talent/host.rs`
//! lends that registrar these operations, whose bodies moved to
//! `wow_world_spell` (#1363 slice 9b).

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
use wow_packet::packets::talent::{LearnTalents, SPEC_RESET_TALENTS_LIKE_CPP};

#[cfg(test)]
use crate::session::{
    RepresentedConfirmRespecWipeLikeCpp, RepresentedTalentRespecVisualSpellCastLikeCpp,
    WorldSession,
};
#[cfg(test)]
use wow_world_spell::{AT_LOGIN_RESET_TALENTS_LIKE_CPP, UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP};

mod host;

#[cfg(test)]
const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;

#[cfg(test)]
const AT_LOGIN_RESET_SPELLS_LIKE_CPP: u16 = 0x002;

#[cfg(test)]
#[path = "../../unit_tests/handlers/talent/state/test_only.rs"]
mod test_only;

#[cfg(test)]
#[path = "../../unit_tests/handlers/talent/tests/mod.rs"]
mod tests;
