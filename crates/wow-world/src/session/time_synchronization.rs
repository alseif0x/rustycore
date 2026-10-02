// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Time synchronization: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;
pub(crate) use wow_world_core::session::game_time_ms_like_cpp;

impl WorldSession {}

#[cfg(test)]
#[path = "../../unit_tests/session/time_synchronization/f3_shims.rs"]
mod f3_shims;
