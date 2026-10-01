// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn can_gain_represented_xp_rest_bonus_like_cpp(&self) -> Option<bool> {
        crate::session::hub_ref(self).can_gain_represented_xp_rest_bonus_like_cpp()
    }
    #[cfg(test)]
    pub(in crate::session) fn represented_xp_rest_bonus_cap_like_cpp(&self) -> Option<f32> {
        crate::session::hub_ref(self).represented_xp_rest_bonus_cap_like_cpp()
    }
    #[cfg(test)]
    pub(in crate::session) fn calc_represented_xp_rest_extra_per_sec_like_cpp(
        &self,
        bubble: f32,
    ) -> Option<f32> {
        crate::session::hub_ref(self).calc_represented_xp_rest_extra_per_sec_like_cpp(bubble)
    }
    #[cfg(test)]
    pub(crate) fn represented_xp_rest_bonus_like_cpp(&self) -> f32 {
        crate::session::hub_ref(self).represented_xp_rest_bonus_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_xp_rest_state_like_cpp(&self) -> u8 {
        crate::session::hub_ref(self).represented_xp_rest_state_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_xp_rest_threshold_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self).represented_xp_rest_threshold_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_is_resting_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_is_resting_like_cpp()
    }
    pub(crate) fn represented_action_button_db_context_like_cpp(&self) -> Option<(u8, i32)> {
        crate::session::hub_ref(self).represented_action_button_db_context_like_cpp()
    }
}
