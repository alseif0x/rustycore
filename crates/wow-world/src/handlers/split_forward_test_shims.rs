// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the World forwards and bodies retired or
//! moved in #1363 slice 12c: one-call forwards (or the retired split forward
//! verbatim) for the names the scenarios call, so no scenario call site changes.
#![cfg(test)]

use crate::session::WorldSession;

impl WorldSession {
    // Rest flags (moved to `wow_world_instances::instance_session::rest_flags`).
    pub(crate) fn with_owned_player_mut_for_rest_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut wow_entities::Player) -> R,
    ) -> Option<R> {
        wow_world_instances::with_owned_player_mut_for_rest_like_cpp(self, f)
    }
    pub(crate) fn set_represented_rest_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
    ) -> bool {
        wow_world_instances::set_player_rest_flag_like_cpp(self, rest_flag, trigger_id)
    }
    pub(crate) fn remove_represented_rest_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        wow_world_instances::remove_player_rest_flag_like_cpp(self, rest_flag)
    }
    pub(crate) fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool {
        wow_world_instances::take_player_deferred_rest_flag_update_dirty_like_cpp(self)
    }
    pub(crate) fn send_represented_resting_player_flag_update_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_represented_resting_player_flag_update_like_cpp(hub)
    }
}
