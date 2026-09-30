//! catalog for the existing appearance owner.

use super::*;

impl WorldSession {
    /// Set the item appearance store for this session.
    pub fn set_item_appearance_store(&mut self, store: Arc<ItemAppearanceStore>) {
        self.items.appearance_store = Some(store);
    }
    /// Get the item appearance store reference.
    pub fn item_appearance_store(&self) -> Option<&Arc<ItemAppearanceStore>> {
        self.items.appearance_store.as_ref()
    }
    /// Set the item modified appearance store for this session.
    pub fn set_item_modified_appearance_store(&mut self, store: Arc<ItemModifiedAppearanceStore>) {
        self.items.modified_appearance_store = Some(store);
    }
    /// Get the item modified appearance store reference.
    pub fn item_modified_appearance_store(&self) -> Option<&Arc<ItemModifiedAppearanceStore>> {
        self.items.modified_appearance_store.as_ref()
    }
    /// Set the transmog set item store for this session.
    pub fn set_transmog_set_item_store(&mut self, store: Arc<TransmogSetItemStore>) {
        self.items.install_transmog_set_item_store(store);
    }
    /// Get the transmog set item store reference.
    pub fn transmog_set_item_store(&self) -> Option<&Arc<TransmogSetItemStore>> {
        self.items.transmog_set_item_store()
    }
    /// C++ `DB2Manager::GetTransmogSetItems`.
    pub fn transmog_set_items_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Option<&[wow_data::TransmogSetItemEntry]> {
        self.items
            .transmog_set_item_store()
            .and_then(|store| store.get_transmog_set_items_like_cpp(transmog_set_id))
    }
    /// C++ `DB2Manager::GetTransmogSetsForItemModifiedAppearance`.
    pub fn transmog_sets_for_item_modified_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> Option<&[TransmogSetEntry]> {
        self.items.transmog_set_item_store().and_then(|store| {
            store.get_transmog_sets_for_item_modified_appearance_like_cpp(
                item_modified_appearance_id,
            )
        })
    }
    /// C++ `CollectionMgr::AddTransmogSet` expansion before `AddItemAppearance`.
    pub fn transmog_set_item_modified_appearances_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Vec<&wow_data::ItemModifiedAppearanceEntry> {
        let Some(items) = self.transmog_set_items_like_cpp(transmog_set_id) else {
            return Vec::new();
        };
        let Some(item_modified_appearance_store) = self.items.modified_appearance_store.as_ref()
        else {
            return Vec::new();
        };

        items
            .iter()
            .filter_map(|item| item_modified_appearance_store.get(item.item_modified_appearance_id))
            .collect()
    }
    /// Build the closure result expected by `Item::visible_entry` and
    /// `Item::visible_appearance_mod_id` from `ItemModifiedAppearance.db2`.
    pub fn item_modified_appearance_ref(&self, id: u32) -> Option<(u32, u16)> {
        self.items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get(id))
            .and_then(|entry| {
                Some((
                    u32::try_from(entry.item_id).ok()?,
                    u16::try_from(entry.item_appearance_modifier_id).ok()?,
                ))
            })
    }
    /// C++ `DB2Manager::GetItemModifiedAppearance`.
    pub fn item_modified_appearance_for_item(
        &self,
        item_id: u32,
        appearance_mod_id: u32,
    ) -> Option<u32> {
        self.items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get_for_item(item_id, appearance_mod_id))
            .map(|entry| entry.id)
    }
}
