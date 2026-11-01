use std::sync::Arc;

use wow_data::{ItemModifiedAppearanceStore, TransmogSetEntry};

impl crate::session::state::SessionCatalogs {
    /// Get the item modified appearance store reference.
    pub fn item_modified_appearance_store(&self) -> Option<&Arc<ItemModifiedAppearanceStore>> {
        self.items.modified_appearance_store.as_ref()
    }

    /// C++ `DB2Manager::GetTransmogSetItems`.
    pub fn transmog_set_items_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Option<&[wow_data::TransmogSetItemEntry]> {
        self.transmog_set_item_store
            .as_ref()
            .and_then(|store| store.get_transmog_set_items_like_cpp(transmog_set_id))
    }

    /// C++ `DB2Manager::GetTransmogSetsForItemModifiedAppearance`.
    pub fn transmog_sets_for_item_modified_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> Option<&[TransmogSetEntry]> {
        self.transmog_set_item_store.as_ref().and_then(|store| {
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

impl crate::session::HubRef<'_> {
    /// C++ `CollectionMgr::LoadAccountItemAppearances` active-player create data order.
    pub fn account_transmog_active_player_rows_like_cpp(&self) -> Vec<u32> {
        let Some(collections) = self.player_collection_state_snapshot_like_cpp() else {
            return Vec::new();
        };
        if !collections.item_appearance_blocks_like_cpp().is_empty() {
            return collections.item_appearance_blocks_snapshot_like_cpp();
        }

        if let Some(blocks) = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.transmog_blocks_like_cpp().to_vec())
        {
            return blocks;
        }

        let Some(highest_appearance) = collections.item_appearances_like_cpp().iter().max() else {
            return Vec::new();
        };

        let mut blocks = vec![0_u32; (highest_appearance / 32 + 1) as usize];
        for &item_modified_appearance_id in collections.item_appearances_like_cpp() {
            let block_index = (item_modified_appearance_id / 32) as usize;
            let bit_index = item_modified_appearance_id % 32;
            if let Some(flag) = 1_u32.checked_shl(bit_index) {
                blocks[block_index] |= flag;
            }
        }

        blocks
    }
}
