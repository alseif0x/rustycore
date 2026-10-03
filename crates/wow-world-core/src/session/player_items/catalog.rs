use std::sync::Arc;

use wow_constants::ItemFlags;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::{ItemClassStore, ItemPriceBaseStore};
use wow_data::{
    ItemExtendedCostStore, ItemRandomPropertiesStore, ItemRandomPropertyTemplateEntry,
    ItemRandomSuffixStore, ItemStatsStore, ItemStore,
};
use wow_entities::ItemStorageTemplate;

impl crate::session::state::SessionCatalogs {
    /// Set the C++ ItemPriceBase.db2 store for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_item_price_base_store(&mut self, store: Arc<ItemPriceBaseStore>) {
        self.item_price_base_store = Some(store);
    }

    /// Set the item class store for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_item_class_store(&mut self, store: Arc<ItemClassStore>) {
        self.item_class_store = Some(store);
    }

    /// Get the item extended cost store reference.
    pub fn item_extended_cost_store(&self) -> Option<&Arc<ItemExtendedCostStore>> {
        self.items.extended_cost_store.as_ref()
    }

    /// Resolve C++ `ItemTemplate::GetRandomSelect()`.
    pub fn item_template_random_select(&self, item_id: u32) -> u16 {
        self.items
            .store
            .as_ref()
            .map(|store| store.random_select(item_id))
            .unwrap_or(0)
    }

    /// Resolve C++ `ItemTemplate::GetRandomSuffixGroupID()`.
    pub fn item_template_random_suffix_group_id(&self, item_id: u32) -> u16 {
        self.items
            .store
            .as_ref()
            .map(|store| store.random_suffix_group_id(item_id))
            .unwrap_or(0)
    }

    /// Get the item store reference.
    pub fn item_store(&self) -> Option<&Arc<ItemStore>> {
        self.items.store.as_ref()
    }

    /// Resolve cached C++ `ItemTemplate::QuestLogItemId` from `item_template_addon`.
    pub fn item_template_addon_quest_log_item_id_like_cpp(&self, item_id: u32) -> Option<u32> {
        self.item_template_addon_quest_log_item_ids_like_cpp
            .get(&item_id)
            .copied()
    }

    /// Cache C++ `ItemTemplate::QuestLogItemId` from `item_template_addon`.
    pub fn cache_item_template_addon_quest_log_item_id_like_cpp(
        &mut self,
        item_id: u32,
        quest_log_item_id: u32,
    ) {
        self.item_template_addon_quest_log_item_ids_like_cpp
            .insert(item_id, quest_log_item_id);
    }

    /// Resolve C++ `ItemTemplate::ExtendedData->Flags[0]`.
    pub fn item_template_flags(&self, item_id: u32) -> Option<ItemFlags> {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.item_flags(item_id))
    }

    /// Resolve C++ `ItemTemplate::ExtendedData->Flags[1]`.
    pub fn item_template_flags2(&self, item_id: u32) -> Option<u32> {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.flags[1])
    }

    /// Resolve C++ `ItemTemplate::ExtendedData->Flags[2]`.
    pub fn item_template_flags3(&self, item_id: u32) -> Option<u32> {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.flags[2])
    }

    /// Resolve C++ `ItemTemplate::GetLockID()` (`ItemSparseEntry::LockID`).
    pub fn item_template_lock_id(&self, item_id: u32) -> Option<u16> {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.lock_id)
    }

    pub fn item_template_quality(&self, item_id: u32) -> Option<i8> {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.random_property_template(item_id))
            .map(|template| template.quality)
    }

    /// Resolve C++ `ItemSparseEntry` data used by random-property generation.
    pub fn item_random_property_template(
        &self,
        item_id: u32,
    ) -> Option<ItemRandomPropertyTemplateEntry> {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.random_property_template(item_id))
            .copied()
    }

    /// Get the item random suffix store reference.
    pub fn item_random_suffix_store(&self) -> Option<&Arc<ItemRandomSuffixStore>> {
        self.items.random_suffix_store.as_ref()
    }

    /// Get the item random properties store reference.
    pub fn item_random_properties_store(&self) -> Option<&Arc<ItemRandomPropertiesStore>> {
        self.items.random_properties_store.as_ref()
    }

    pub fn item_template_name_like_cpp(&self, item_id: u32) -> &str {
        self.items
            .search_name_store
            .as_ref()
            .and_then(|store| store.get(item_id))
            .map(|entry| entry.display.as_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("<error>")
    }
}

impl crate::session::state::SessionCatalogs {
    /// Get the item stats store reference.
    pub fn item_stats_store(&self) -> Option<&Arc<ItemStatsStore>> {
        self.items.stats_store.as_ref()
    }

    pub fn item_template_start_quest_id(&self, item_id: u32) -> Option<i32> {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.start_quest_id)
    }

    /// Resolve the C++ `ItemTemplate` subset used by storage validation.
    pub fn item_storage_template(&self, item_id: u32) -> Option<ItemStorageTemplate> {
        crate::catalogs::item::item_storage_template_like_cpp(
            self.items.store.as_ref(),
            self.items.stats_store.as_ref(),
            item_id,
        )
    }
}
