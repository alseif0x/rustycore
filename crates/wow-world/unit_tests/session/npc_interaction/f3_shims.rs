// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn player_interaction_trainer_id_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_interaction_ref(self);
        state.player_interaction_trainer_id_like_cpp(hub)
    }
}
