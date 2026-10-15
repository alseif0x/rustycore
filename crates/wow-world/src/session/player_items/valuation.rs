//! Represented item level and price valuation.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// C++ `Player::GetAverageItemLevel`.
    pub(crate) fn represented_average_item_level_like_cpp(&self) -> Option<f32> {
        self.inventory
            .represented_average_item_level_with_access_like_cpp(
                &self.core.owned_inventory_access_like_cpp(),
                &self.core.inventory_valuation_access_like_cpp(),
                &self.core.owned_item_modifiers_access_like_cpp(),
                &self.catalogs.inventory_valuation_catalog_view_like_cpp(),
                Self::MIN_ITEM_LEVEL_LIKE_CPP,
                Self::MAX_ITEM_LEVEL_LIKE_CPP,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_level,
            )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/valuation/f3_shims.rs"]
mod f3_shims;
