// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Action bar adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(in crate::session) use wow_world_core::session::{
    action_button_action_like_cpp, action_button_type_like_cpp, make_action_button_like_cpp,
    set_active_player_update_bit_like_cpp,
};

impl WorldSession {
    pub(crate) fn set_tutorial_int_like_cpp(&mut self, index: usize, value: u32) -> bool {
        self.lifecycle.set_tutorial_int_like_cpp(index, value)
    }

    pub(crate) fn apply_tutorial_action_like_cpp(
        &mut self,
        action: u8,
        tutorial_bit: Option<u32>,
    ) -> bool {
        match action {
            wow_packet::packets::misc::TUTORIAL_ACTION_UPDATE_LIKE_CPP => {
                let Some(tutorial_bit) = tutorial_bit else {
                    return false;
                };
                let index = (tutorial_bit >> 5) as usize;
                if index >= self.lifecycle.tutorial_values_like_cpp().len() {
                    return false;
                }
                let flag = self.lifecycle.tutorial_values_like_cpp()[index]
                    | (1u32 << (tutorial_bit & 0x1F));
                self.set_tutorial_int_like_cpp(index, flag)
            }
            wow_packet::packets::misc::TUTORIAL_ACTION_CLEAR_LIKE_CPP => {
                for index in 0..self.lifecycle.tutorial_values_like_cpp().len() {
                    self.set_tutorial_int_like_cpp(index, u32::MAX);
                }
                true
            }
            wow_packet::packets::misc::TUTORIAL_ACTION_RESET_LIKE_CPP => {
                for index in 0..self.lifecycle.tutorial_values_like_cpp().len() {
                    self.set_tutorial_int_like_cpp(index, 0);
                }
                true
            }
            _ => false,
        }
    }

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
