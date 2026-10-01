// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn set_player_normal_damage_immune_like_cpp(&mut self, immune: bool) {
        crate::session::hub_mut(self).set_player_normal_damage_immune_like_cpp(immune)
    }
    #[cfg(test)]
    pub(crate) fn set_player_environmental_damage_immune_like_cpp(&mut self, immune: bool) {
        crate::session::hub_mut(self).set_player_environmental_damage_immune_like_cpp(immune)
    }
    pub(in crate::session) fn resolved_player_damage_control_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerDamageControlStateLikeCpp> {
        crate::session::hub_ref(self).resolved_player_damage_control_like_cpp()
    }
}
