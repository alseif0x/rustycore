// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void storage adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.
//! The void-storage reads and writes the handlers used moved to
//! `wow_world_application::void_storage_session` with them (#1363 slice 4c).

use super::WorldSession;

impl WorldSession {
    pub(crate) fn clear_represented_void_storage_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.clear_represented_void_storage_like_cpp(&mut hub)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/void_storage_adapter/f3_shims.rs"]
mod f3_shims;
