//! Represented item appearance: enchantment, transmogrification and illusion state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the item appearance store for this session.
    pub fn set_item_appearance_store(&mut self, store: Arc<ItemAppearanceStore>) {
        self.catalogs.items.appearance_store = Some(store);
    }
    /// Set the item modified appearance store for this session.
    pub fn set_item_modified_appearance_store(&mut self, store: Arc<ItemModifiedAppearanceStore>) {
        self.catalogs.items.modified_appearance_store = Some(store);
    }
    /// Set the transmog set item store for this session.
    pub fn set_transmog_set_item_store(&mut self, store: Arc<TransmogSetItemStore>) {
        self.catalogs.transmog_set_item_store = Some(store);
    }
    pub(crate) fn add_transmog_set_like_cpp(
        &mut self,
        transmog_set_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_transmog_set_like_cpp(&mut hub, transmog_set_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_alter_appearance_requests_like_cpp(
        &self,
    ) -> &[RepresentedAlterAppearanceLikeCpp] {
        &self
            .fixtures
            .presentation
            .represented_alter_appearance_requests_like_cpp
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/appearance/f3_shims.rs"]
mod f3_shims;
