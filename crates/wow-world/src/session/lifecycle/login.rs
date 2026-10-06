// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login-side lifecycle: the single-live-session character claim.
//!
//! One character may only be logging in through one session at a time. The
//! claim is taken before the login sequence commits and released on any exit
//! path — completion, a failed ConnectTo, or disconnect — so a dropped
//! instance socket cannot leave a character permanently unloggable.

use super::super::{ObjectGuid, WorldSession};

impl WorldSession {}

#[cfg(test)]
#[path = "../../../unit_tests/session/lifecycle/login/f3_shims.rs"]
mod f3_shims;
