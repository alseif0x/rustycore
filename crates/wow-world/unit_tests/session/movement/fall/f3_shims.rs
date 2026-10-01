// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn fall_damage_events_like_cpp(&self) -> &[MovementFallDamageEvent] {
        self.fixtures.movement.fall_damage_events_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn fall_information_like_cpp(&self) -> (u32, f32) {
        crate::session::hub_ref(self).fall_information_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn player_movement_jump_like_cpp(&self) -> &wow_packet::packets::movement::JumpInfo {
        self.fixtures.movement.player_movement_jump_like_cpp()
    }
}
