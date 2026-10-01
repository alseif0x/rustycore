// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn player_logout_like_cpp(&self) -> bool {
        self.lifecycle.player_logout_like_cpp()
    }
    pub(in crate::session) fn complete_logout(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.complete_logout(&mut hub)
    }
}
