// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn resolved_player_inventory_slot_count_like_cpp(&self) -> Option<u8> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_player_inventory_slot_count_like_cpp(hub)
    }
}
