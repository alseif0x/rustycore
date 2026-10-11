// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Action bar adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(in crate::session) use wow_world_core::session::{
    action_button_action_like_cpp, action_button_type_like_cpp, make_action_button_like_cpp,
};

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn set_active_player_local_flags_like_cpp(&mut self, flags: u32) {
        let _ = crate::session::hub_mut(self).mutate_active_player_update_state_like_cpp(|state| {
            state.active_local_flags = flags;
        });
        self.sync_current_player_session_visibility_detection_like_cpp();
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/action_bar_adapter/f3_shims.rs"]
mod f3_shims;
