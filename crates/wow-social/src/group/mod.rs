// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared registry of active groups for cross-session party management.
//!
//! This is the single atomic Group authority. Its state lives in private
//! submodules and is reachable only through the named operations re-exported
//! here; packet encoding, session addressing/delivery and the database adapter
//! stay with the caller.

mod invites;
mod membership;
mod model;
mod outcome;
mod ready_check;
mod settings;

pub use invites::*;
pub use membership::*;
pub use model::*;
pub use outcome::*;
pub use ready_check::*;
pub use settings::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
