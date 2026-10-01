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
}
