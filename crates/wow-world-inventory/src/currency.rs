// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;

use wow_entities::PlayerCurrency;
use wow_world_core::session::{HubMut, HubRef};

impl crate::InventoryState {
    pub fn set_player_currencies_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        currencies: HashMap<u32, PlayerCurrency>,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.install_currencies_like_cpp(currencies.clone());
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_currencies = currencies;
            return true;
        }
        canonical
    }

    pub fn player_currencies_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<HashMap<u32, PlayerCurrency>> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().currencies.clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.player_currencies.clone());
        }
        canonical
    }
}
