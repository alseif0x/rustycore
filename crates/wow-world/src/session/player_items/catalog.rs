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
        let Some((_, max_health_after, _)) =
            crate::session::hub_ref(self).resolved_player_vitals_like_cpp()
        else {
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
        self.loot
            .set_loot_item_store_test_seam_like_cpp(grants, success);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/catalog/f3_shims.rs"]
mod f3_shims;
