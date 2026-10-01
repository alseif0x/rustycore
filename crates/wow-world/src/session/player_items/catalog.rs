//! Item template and catalog lookups used by the represented inventory.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the item extended cost store for this session.
    pub fn set_item_extended_cost_store(&mut self, store: Arc<ItemExtendedCostStore>) {
        self.catalogs.items.extended_cost_store = Some(store);
    }
    pub fn item_extended_cost_store(&self) -> Option<&Arc<ItemExtendedCostStore>> {
        self.catalogs.item_extended_cost_store()
    }
    /// Set the item store for this session.
    pub fn set_item_store(&mut self, store: Arc<ItemStore>) {
        self.catalogs.items.store = Some(store);
    }
    pub fn item_store(&self) -> Option<&Arc<ItemStore>> {
        self.catalogs.item_store()
    }
    /// Set the item search-name store for this session.
    pub fn set_item_search_name_store(&mut self, store: Arc<ItemSearchNameStore>) {
        self.catalogs.items.search_name_store = Some(store);
    }
    /// Get the item search-name store reference.
    pub fn item_search_name_store(&self) -> Option<&Arc<ItemSearchNameStore>> {
        self.catalogs.items.search_name_store.as_ref()
    }
    pub fn set_pvp_item_store(&mut self, store: Arc<PvpItemStore>) {
        self.catalogs.pvp_item_store = Some(store);
    }
    pub fn set_item_stats_store(&mut self, store: Arc<ItemStatsStore>) {
        self.catalogs.items.stats_store = Some(store);
    }
    pub(in crate::session) fn restore_represented_health_pct_after_item_mod_scaling_like_cpp(
        &mut self,
        health_before: u32,
        max_health_before: u32,
    ) {
        let Some((_, max_health_after, _)) = self.resolved_player_vitals_like_cpp() else {
            return;
        };
        let restored = (u64::from(max_health_after) * u64::from(health_before)
            / u64::from(max_health_before.max(1))) as u32;
        self.set_player_health_like_cpp(restored, max_health_after);
    }
    pub fn set_item_spec_override_store(&mut self, store: Arc<ItemSpecOverrideStore>) {
        self.catalogs.items.spec_override_store = Some(store);
    }
    pub fn item_spec_override_store(&self) -> Option<&Arc<ItemSpecOverrideStore>> {
        self.catalogs.items.spec_override_store.as_ref()
    }
    pub fn set_item_effect_store(&mut self, store: Arc<ItemEffectStore>) {
        self.catalogs.items.effect_store = Some(store);
    }
    pub fn item_stats_store(&self) -> Option<&Arc<ItemStatsStore>> {
        self.catalogs.item_stats_store()
    }
    pub fn item_effect_store(&self) -> Option<&Arc<ItemEffectStore>> {
        self.catalogs.items.effect_store.as_ref()
    }
    pub(crate) fn item_template_addon_quest_log_item_id_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<u32> {
        self.catalogs
            .item_template_addon_quest_log_item_id_like_cpp(item_id)
    }
    pub(crate) fn cache_item_template_addon_quest_log_item_id_like_cpp(
        &mut self,
        item_id: u32,
        quest_log_item_id: u32,
    ) {
        self.catalogs
            .cache_item_template_addon_quest_log_item_id_like_cpp(item_id, quest_log_item_id)
    }
    pub fn item_template_flags(&self, item_id: u32) -> Option<ItemFlags> {
        self.catalogs.item_template_flags(item_id)
    }
    pub fn item_template_flags2(&self, item_id: u32) -> Option<u32> {
        self.catalogs.item_template_flags2(item_id)
    }
    pub fn item_template_flags3(&self, item_id: u32) -> Option<u32> {
        self.catalogs.item_template_flags3(item_id)
    }
    pub fn item_template_lock_id(&self, item_id: u32) -> Option<u16> {
        self.catalogs.item_template_lock_id(item_id)
    }
    pub fn item_template_start_quest_id(&self, item_id: u32) -> Option<i32> {
        self.catalogs.item_template_start_quest_id(item_id)
    }
    pub fn item_template_quality(&self, item_id: u32) -> Option<i8> {
        self.catalogs.item_template_quality(item_id)
    }
    pub fn item_storage_template(&self, item_id: u32) -> Option<ItemStorageTemplate> {
        self.catalogs.item_storage_template(item_id)
    }
    /// Set the item random suffix store for this session.
    pub fn set_item_random_suffix_store(&mut self, store: Arc<ItemRandomSuffixStore>) {
        self.catalogs.items.random_suffix_store = Some(store);
    }
    pub fn item_random_suffix_store(&self) -> Option<&Arc<ItemRandomSuffixStore>> {
        self.catalogs.item_random_suffix_store()
    }
    /// Set the item random properties store for this session.
    pub fn set_item_random_properties_store(&mut self, store: Arc<ItemRandomPropertiesStore>) {
        self.catalogs.items.random_properties_store = Some(store);
    }
    pub fn item_random_properties_store(&self) -> Option<&Arc<ItemRandomPropertiesStore>> {
        self.catalogs.item_random_properties_store()
    }
    /// Set the QuestPackageItem store used by C++ quest package reward selection.
    pub fn set_quest_package_item_store(&mut self, store: Arc<QuestPackageItemStore>) {
        self.catalogs.quests.package_item_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn set_loot_item_store_test_seam_like_cpp(
        &mut self,
        grants: Arc<AtomicUsize>,
        success: bool,
    ) {
        self.loot.loot_item_store_test_grants_like_cpp = Some(grants);
        self.loot.loot_item_store_test_success_like_cpp = success;
    }
    pub(in crate::session) fn item_template_name_like_cpp(&self, item_id: u32) -> &str {
        self.catalogs.item_template_name_like_cpp(item_id)
    }
}

impl crate::session::state::InventoryState {
    /// C++ `Player::GetItemByEntry(entry, ItemSearchLocation::Default)`.
    pub(in crate::session) fn represented_player_has_default_item_entry_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        item_id: u32,
    ) -> bool {
        let Some(player) = self.direct_inventory_player_snapshot(hub) else {
            return false;
        };
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp(hub) else {
            return false;
        };
        let mut found = false;
        player.for_each_item_guid(wow_entities::ItemSearchLocation::DEFAULT, |item_guid| {
            if item_objects
                .get(&item_guid)
                .is_some_and(|item| item.object().entry() == item_id)
            {
                found = true;
                wow_entities::ItemSearchCallbackResult::Stop
            } else {
                wow_entities::ItemSearchCallbackResult::Continue
            }
        });
        found
    }
}

impl crate::session::state::SessionCatalogs {
    /// Set the C++ ItemPriceBase.db2 store for this session.
    #[cfg(test)]
    pub fn set_item_price_base_store(&mut self, store: Arc<ItemPriceBaseStore>) {
        self.item_price_base_store = Some(store);
    }

    /// Set the item class store for this session.
    #[cfg(test)]
    pub fn set_item_class_store(&mut self, store: Arc<ItemClassStore>) {
        self.item_class_store = Some(store);
    }

    /// Get the item extended cost store reference.
    pub fn item_extended_cost_store(&self) -> Option<&Arc<ItemExtendedCostStore>> {
        self.items.extended_cost_store.as_ref()
    }

    /// Resolve C++ `ItemTemplate::GetRandomSelect()`.
    pub(crate) fn item_template_random_select(&self, item_id: u32) -> u16 {
        self.items
            .store
            .as_ref()
            .map(|store| store.random_select(item_id))
            .unwrap_or(0)
    }

    /// Resolve C++ `ItemTemplate::GetRandomSuffixGroupID()`.
    pub(crate) fn item_template_random_suffix_group_id(&self, item_id: u32) -> u16 {
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
    pub(crate) fn item_template_addon_quest_log_item_id_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<u32> {
        self.item_template_addon_quest_log_item_ids_like_cpp
            .get(&item_id)
            .copied()
    }

    /// Cache C++ `ItemTemplate::QuestLogItemId` from `item_template_addon`.
    pub(crate) fn cache_item_template_addon_quest_log_item_id_like_cpp(
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
    pub(crate) fn item_random_property_template(
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

    pub(in crate::session) fn item_template_name_like_cpp(&self, item_id: u32) -> &str {
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
        let basic = self.items.store.as_ref()?.get(item_id)?;
        let sparse = self.items.stats_store.as_ref()?.sparse_template(item_id)?;
        let class_id = <ItemClass as num_traits::FromPrimitive>::from_u8(basic.class_id)?;
        let inventory_type =
            <InventoryType as num_traits::FromPrimitive>::from_i8(sparse.inventory_type)?;
        let bonding = <ItemBondingType as num_traits::FromPrimitive>::from_u8(sparse.bonding)?;

        Some(ItemStorageTemplate {
            entry: item_id,
            class_id,
            subclass_id: u32::from(basic.subclass_id),
            inventory_type,
            bonding,
            bag_family: BagFamilyMask::from_bits_retain(sparse.bag_family),
            max_stack_size: sparse.max_stack_size(),
            max_count: sparse.max_count,
            item_limit_category: u32::from(sparse.limit_category),
            container_slots: sparse.container_slots,
            sell_price: sparse.sell_price,
            is_crafting_reagent: (sparse.flags[1] & ItemFlags2::UsedInATradeskill as u32) != 0,
            flags: sparse.item_flags(),
        })
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/catalog/f3_shims.rs"]
mod f3_shims;
