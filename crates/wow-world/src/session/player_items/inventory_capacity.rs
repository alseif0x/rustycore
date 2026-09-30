//! Canonical inventory capacities and their explicit ownerless fixture fallback.

use super::*;

impl WorldSession {
    pub(crate) fn set_player_bank_bag_slot_count_like_cpp(&mut self, count: u8) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_bank_bag_slot_count(count))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if cfg!(test) && canonical || self.ownerless_inventory_fallback_enabled_for_test() {
            self.player_item_test_fixture_like_cpp
                .player_bank_bag_slot_count_like_cpp = count;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        return canonical || self.ownerless_inventory_fallback_enabled_for_test();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        canonical
    }

    pub(crate) fn resolved_player_bank_bag_slot_count_like_cpp(&self) -> Option<u8> {
        let canonical = self.with_owned_player_like_cpp(Player::bank_bag_slot_count);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.ownerless_inventory_fallback_enabled_for_test() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .player_bank_bag_slot_count_like_cpp,
            );
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn player_bank_bag_slot_count_like_cpp(&self) -> u8 {
        self.resolved_player_bank_bag_slot_count_like_cpp()
            .expect("test Player bank-bag-slot owner must resolve")
    }

    pub(crate) fn set_player_inventory_slot_count_like_cpp(&mut self, count: u8) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(count))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if cfg!(test) && canonical || self.ownerless_inventory_fallback_enabled_for_test() {
            self.player_item_test_fixture_like_cpp
                .player_inventory_slot_count_like_cpp = count;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        return canonical || self.ownerless_inventory_fallback_enabled_for_test();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        canonical
    }

    pub(crate) fn resolved_player_inventory_slot_count_like_cpp(&self) -> Option<u8> {
        let canonical = self.with_owned_player_like_cpp(Player::inventory_slot_count);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.ownerless_inventory_fallback_enabled_for_test() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .player_inventory_slot_count_like_cpp,
            );
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn player_inventory_slot_count_like_cpp(&self) -> u8 {
        self.resolved_player_inventory_slot_count_like_cpp()
            .expect("test Player inventory-slot owner must resolve")
    }
}
