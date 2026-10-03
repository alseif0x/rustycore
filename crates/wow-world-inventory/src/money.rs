// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::Player;
use wow_world_core::session::{HubMut, HubRef};

impl crate::InventoryState {
    pub fn set_player_gold_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        gold: u64,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_money(gold))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_gold = gold;
        }
        canonical || cfg!(any(test, feature = "test-fixtures")) && hub.core.player_handle_like_cpp.is_none()
    }

}

impl crate::InventoryState {
    pub fn resolved_player_money_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u64> {
        let canonical = hub.core.with_owned_player_like_cpp(Player::money);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.player_gold);
        }
        canonical
    }
}
