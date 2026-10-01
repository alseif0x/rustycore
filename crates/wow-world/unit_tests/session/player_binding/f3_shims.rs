// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn set_player_character_points_like_cpp(&mut self, points: i32) -> bool {
        crate::session::hub_mut(self).set_player_character_points_like_cpp(points)
    }
    #[cfg(test)]
    pub(crate) fn represented_can_swim_to_fly_transition_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_can_swim_to_fly_transition_like_cpp()
    }
    pub(in crate::session) fn resolved_can_swim_to_fly_transition_like_cpp(&self) -> Option<bool> {
        crate::session::hub_ref(self).resolved_can_swim_to_fly_transition_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_player_faction_template_like_cpp(&mut self, faction_template: u32) {
        crate::session::hub_mut(self).set_player_faction_template_like_cpp(faction_template)
    }
    pub(in crate::session) fn resolved_player_scale_duration_like_cpp(&self) -> Option<i32> {
        crate::session::hub_ref(self).resolved_player_scale_duration_like_cpp()
    }
}
