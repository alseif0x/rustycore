// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `operations` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn send_update_world_state_like_cpp(
        &self,
        variable_id: u32,
        value: i32,
        hidden: bool,
    ) {
        self.core
            .send_update_world_state_like_cpp(variable_id, value, hidden)
    }
}
