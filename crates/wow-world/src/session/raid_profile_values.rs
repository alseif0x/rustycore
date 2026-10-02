// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Raid profile values: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(in crate::session) use wow_world_core::session::{
    player_cuf_profile_from_packet_like_cpp, player_cuf_profile_to_packet_like_cpp,
};

impl WorldSession {}

#[cfg(test)]
#[path = "../../unit_tests/session/raid_profile_values/f3_shims.rs"]
mod f3_shims;
