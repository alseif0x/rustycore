// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::session) fn active_player_update_state_like_cpp(&self) -> Option<(u32, i32, u8)> {
        crate::session::hub_ref(self).active_player_update_state_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn active_player_local_flags_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self).active_player_local_flags_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn active_player_multi_action_bars_like_cpp(&self) -> u8 {
        crate::session::hub_ref(self).active_player_multi_action_bars_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_action_button_like_cpp(&self, index: u8) -> Option<u32> {
        crate::session::hub_ref(self).represented_action_button_like_cpp(index)
    }
    pub(in crate::session) fn mutate_active_player_update_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerGameplayState) -> R,
    ) -> Option<R> {
        crate::session::hub_mut(self).mutate_active_player_update_state_like_cpp(mutate)
    }
}

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
}
