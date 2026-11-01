// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical regeneration accessors shared with World.

impl crate::session::HubRef<'_> {
    /// Test accessor for the canonical five-second-rule state after a cast.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_player_mp5_regen_interrupted_like_cpp(&self) -> bool {
        self.core
            .with_owned_player_like_cpp(|player| {
                player
                    .unit()
                    .is_power_regen_interrupted_by_mp5_rule_like_cpp(
                        crate::session::game_time_ms_like_cpp(),
                    )
            })
            .unwrap_or(false)
    }
}
