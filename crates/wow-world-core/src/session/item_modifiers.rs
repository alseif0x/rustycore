// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session item scaling values shared with World.

use std::sync::Arc;
use wow_data::PlayerStatsStore;

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

pub fn player_class_mask_for_transmog_like_cpp(class_id: u8) -> u32 {
    if class_id == 0 || class_id > 32 {
        0
    } else {
        1_u32 << u32::from(class_id - 1)
    }
}

impl crate::session::state::SessionCatalogs {
    /// Get the player stats store reference.
    pub fn player_stats(&self) -> Option<&Arc<PlayerStatsStore>> {
        self.player_stats.as_ref()
    }
}

pub fn player_class_mask_for_talent_like_cpp(class_id: u8) -> Option<u32> {
    if class_id == 0 || class_id > 32 {
        None
    } else {
        Some(1_u32 << u32::from(class_id - 1))
    }
}
