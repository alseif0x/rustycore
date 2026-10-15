// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cast handlers — CMSG_CAST_SPELL, CMSG_CANCEL_CAST, CMSG_CANCEL_CHANNELLING.
//!
//! Normal requests decode/adapt into `player_cast`; canonical state and
//! publication adapters live under `session/player_cast`. Immediate and queued
//! requests share preparation, while the existing driver consumes timed casts.
//! Other represented spell/item handlers retain their explicit operation paths.
//! Reference: Classic Game/Handlers/SpellHandler.cpp, Player.cpp and Spell.cpp.
//!
//! `#1263 F5 remaining families`: the four registrations that lived here
//! (`CMSG_CAST_SPELL`, `CMSG_OPEN_ITEM`, `CMSG_SELF_RES`, `CMSG_SPELL_CLICK`)
//! moved unchanged to the application crate's `crate::spell_handlers` area
//! registrar, which already owns the `SpellHandler.cpp` cancellation surface.
//! `crates/wow-world/src/session/spell_handler_contexts.rs` lends that registrar
//! the same `WorldSession` operations and catalog destructuring the legacy
//! closures used.

#[cfg(test)]
use wow_packet::ClientPacket;
#[cfg(test)]
use wow_packet::packets::spell::CancelModSpeedNoControlAuras;

#[cfg(test)]
use crate::session::WorldSession;

mod state;
#[cfg(test)]
mod test_shims;
#[cfg(test)]
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "../../unit_tests/handlers/spell/ops_1/f3_shims.rs"]
mod f3_shims;
#[cfg(test)]
#[path = "../../unit_tests/handlers/spell/ops_2/test_only.rs"]
mod test_only;

// ── Handler implementations ───────────────────────────────────────

#[cfg(test)]
#[path = "../../unit_tests/handlers/spell/tests/mod.rs"]
mod tests;
