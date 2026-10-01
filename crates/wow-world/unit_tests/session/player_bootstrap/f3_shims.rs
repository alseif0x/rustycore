// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn apply_represented_player_powers_to_canonical_like_cpp(
        &self,
        player: &mut Player,
    ) {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.apply_represented_player_powers_to_canonical_like_cpp(hub, player)
    }
}
