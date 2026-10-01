// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_total_stat_multipliers_like_cpp(&self) -> [f32; 5] {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_total_stat_multipliers_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn represented_total_stat_buff_multipliers_like_cpp(&self) -> [f32; 5] {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_total_stat_buff_multipliers_like_cpp(hub)
    }
}
