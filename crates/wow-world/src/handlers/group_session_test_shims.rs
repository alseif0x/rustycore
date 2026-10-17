// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the group bodies moved to
//! `wow_world_instances::group_session` (#1363 slice 4d): one-call forwards
//! with this session as the host, so no scenario call site changes.
//! `next_group_update_sequence_number_like_cpp`, whose last production caller
//! was the moved party-update receiver (which now calls the same social-state
//! operation over the same split), moves here verbatim for its scenarios.

#![cfg(test)]

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn next_group_update_sequence_number_like_cpp(
        &mut self,
        category: u8,
    ) -> Option<i32> {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.next_group_update_sequence_number_like_cpp(&mut hub, category)
    }
}
