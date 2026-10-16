// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the session leftovers moved out of the
//! World `session/**` in #1363 slice 12b7: one-call forwards to the moved
//! functions the scenarios call, so no scenario call site changes.
#![cfg(test)]

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn apply_represented_live_intent_like_cpp(
        &mut self,
        intent: wow_world_core::session::RepresentedLiveIntentLikeCpp,
    ) -> wow_world_core::session::RepresentedLiveIntentApplyOutcomeLikeCpp {
        wow_world_application::apply_represented_live_intent_like_cpp(self, intent)
    }
}
