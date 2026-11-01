// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical death and resurrection adapters shared with World.

impl crate::session::HubMut<'_> {
    pub fn apply_represented_resurrection_health_like_cpp(&mut self, health: u32) {
        let Some((_, max_health, _)) = self.shared().resolved_player_vitals_like_cpp() else {
            return;
        };
        let _ = self.sync_canonical_player_health_like_cpp(health, max_health);
    }

    pub fn apply_represented_resurrection_percent_like_cpp(&mut self, restore_percent: f32) {
        let Some((_, max_health, _)) = self.shared().resolved_player_vitals_like_cpp() else {
            return;
        };
        let health =
            ((f64::from(max_health) * f64::from(restore_percent)).floor() as u32).min(max_health);
        self.apply_represented_resurrection_health_like_cpp(health);
    }
}
