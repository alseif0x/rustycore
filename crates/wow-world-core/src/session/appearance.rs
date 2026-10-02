// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read the current incarnation's appearance without repeating login hydration.

impl crate::session::HubRef<'_> {
    pub fn owned_player_customizations_like_cpp(
        &self,
    ) -> Option<Vec<wow_entities::PlayerCustomizationChoice>> {
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().customizations.clone())
    }
}
