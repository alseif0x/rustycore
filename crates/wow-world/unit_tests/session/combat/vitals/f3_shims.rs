// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn set_represented_player_power_slot_like_cpp(
        &mut self,
        slot: usize,
        current: i32,
        max: Option<i32>,
    ) {
        crate::session::hub_mut(self).set_represented_player_power_slot_like_cpp(slot, current, max)
    }
}
