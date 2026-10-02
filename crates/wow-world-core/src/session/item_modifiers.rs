// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session item scaling values shared with World.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedScalingStatContextLikeCpp {
    pub stat_id: [i32; 10],
    pub bonus: [i32; 10],
    pub ssd_multiplier: i32,
    pub spell_bonus: i32,
    pub armor_mod: i32,
    pub dps_mod: i32,
    pub is_two_hand: bool,
}
