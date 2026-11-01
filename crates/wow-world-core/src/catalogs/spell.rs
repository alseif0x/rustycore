// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The spell and aura catalog authority a session reads rules from.
//!
//! C++ keeps these stores in `SpellMgr`/`ObjectMgr` and reaches them from
//! `WorldSession` through global accessors. Rust cannot use a global, so the
//! slots were session fields until #668 gave them their own owner: this type
//! owns every spell/aura catalog slot, and `WorldSession` holds exactly one of
//! it. No store slot remains on the session, so there is no second mirror to
//! keep in sync.
//!
//! The owner depends on `wow-data` stores only - no session, no map, no packet
//! and no database. It performs no async work and holds no lock; every slot is
//! filled once at startup and read afterwards.

use std::sync::Arc;

/// Every spell and aura catalog slot a session reads, owned in one place.
#[derive(Default)]
pub struct SpellCatalogsLikeCpp {
    pub item_set_spell_store: Option<Arc<wow_data::ItemSetSpellStore>>,
    pub npc_spell_click_store: Option<Arc<wow_data::NpcSpellClickStoreLikeCpp>>,
    pub pet_default_spell_store: Option<Arc<wow_data::PetDefaultSpellStoreLikeCpp>>,
    pub pet_family_spell_store: Option<Arc<wow_data::PetFamilySpellStoreLikeCpp>>,
    pub pet_levelup_spell_store: Option<Arc<wow_data::PetLevelupSpellStoreLikeCpp>>,
    pub serverside_spell_store: Option<Arc<wow_data::ServersideSpellStoreLikeCpp>>,
    pub(crate) spell_acquisition_catalog: Option<Arc<wow_data::SpellAcquisitionCatalogLikeCpp>>,
    pub spell_area_store: Option<Arc<wow_data::SpellAreaStoreLikeCpp>>,
    pub spell_aura_options_store: Option<Arc<wow_data::SpellAuraOptionsStore>>,
    pub spell_aura_restrictions_store: Option<Arc<wow_data::SpellAuraRestrictionsStore>>,
    pub spell_category_store: Option<Arc<wow_data::SpellCategoryStore>>,
    pub spell_chain_store: Option<Arc<wow_data::SpellChainStoreLikeCpp>>,
    /// C++ `SpellInfo::SpellFamilyName`/`SpellFamilyFlags` source
    /// (`SpellClassOptions.db2`), consumed by spell-specific classification
    /// such as `Player::IsPolymorphed`.
    pub(crate) spell_class_options_store: Option<Arc<wow_data::SpellClassOptionsStore>>,
    pub(crate) spell_label_store: Option<Arc<wow_data::SpellLabelStore>>,
    pub spell_custom_attribute_store: Option<Arc<wow_data::SpellCustomAttributeStoreLikeCpp>>,
    pub spell_duration_store: Option<Arc<wow_data::SpellDurationStore>>,
    pub spell_enchant_proc_store: Option<Arc<wow_data::SpellEnchantProcStoreLikeCpp>>,
    pub spell_equipped_items_store: Option<Arc<wow_data::SpellEquippedItemsStore>>,
    pub spell_group_stack_rule_store: Option<Arc<wow_data::SpellGroupStackRuleStoreLikeCpp>>,
    pub spell_group_store: Option<Arc<wow_data::SpellGroupStoreLikeCpp>>,
    pub spell_item_enchantment_condition_store:
        Option<Arc<wow_data::SpellItemEnchantmentConditionStore>>,
    pub spell_item_enchantment_store: Option<Arc<wow_data::SpellItemEnchantmentStore>>,
    pub spell_learn_skill_store: Option<Arc<wow_data::SpellLearnSkillStoreLikeCpp>>,
    pub spell_learn_spell_store: Option<Arc<wow_data::SpellLearnSpellStoreLikeCpp>>,
    pub spell_levels_store: Option<Arc<wow_data::SpellLevelsStore>>,
    pub spell_linked_store: Option<Arc<wow_data::SpellLinkedStoreLikeCpp>>,
    pub spell_misc_store: Option<Arc<wow_data::SpellMiscStore>>,
    pub spell_pet_aura_store: Option<Arc<wow_data::SpellPetAuraStoreLikeCpp>>,
    pub spell_proc_store: Option<Arc<wow_data::SpellProcStoreLikeCpp>>,
    pub spell_radius_store: Option<Arc<wow_data::SpellRadiusStore>>,
    pub spell_range_store: Option<Arc<wow_data::SpellRangeStore>>,
    pub spell_required_store: Option<Arc<wow_data::SpellRequiredStoreLikeCpp>>,
    pub spell_shapeshift_form_store: Option<Arc<wow_data::SpellShapeshiftFormStore>>,
    pub spell_store: Option<Arc<wow_data::SpellStore>>,
    pub spell_target_position_store: Option<Arc<wow_data::SpellTargetPositionStoreLikeCpp>>,
    pub spell_target_restrictions_store: Option<Arc<wow_data::SpellTargetRestrictionsStore>>,
    pub spell_threat_store: Option<Arc<wow_data::SpellThreatStoreLikeCpp>>,
    pub spell_totem_model_store: Option<Arc<wow_data::SpellTotemModelStoreLikeCpp>>,
}

impl SpellCatalogsLikeCpp {
    #[allow(dead_code)]
    pub(crate) fn npc_spell_click_store(
        &self,
    ) -> Option<&Arc<wow_data::NpcSpellClickStoreLikeCpp>> {
        self.npc_spell_click_store.as_ref()
    }
    pub fn set_npc_spell_click_store(&mut self, store: Arc<wow_data::NpcSpellClickStoreLikeCpp>) {
        self.npc_spell_click_store = Some(store);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_serverside_spell_store(
        &mut self,
        store: Arc<wow_data::ServersideSpellStoreLikeCpp>,
    ) {
        self.serverside_spell_store = Some(store);
    }
    pub fn set_spell_acquisition_catalog(
        &mut self,
        catalog: Arc<wow_data::SpellAcquisitionCatalogLikeCpp>,
    ) {
        self.spell_acquisition_catalog = Some(catalog);
    }
    pub fn set_spell_aura_options_store(&mut self, store: Arc<wow_data::SpellAuraOptionsStore>) {
        self.spell_aura_options_store = Some(store);
    }
    pub fn set_spell_aura_restrictions_store(
        &mut self,
        store: Arc<wow_data::SpellAuraRestrictionsStore>,
    ) {
        self.spell_aura_restrictions_store = Some(store);
    }
    pub fn set_spell_category_store(&mut self, store: Arc<wow_data::SpellCategoryStore>) {
        self.spell_category_store = Some(store);
    }
    pub fn set_spell_class_options_store(&mut self, store: Arc<wow_data::SpellClassOptionsStore>) {
        self.spell_class_options_store = Some(store);
    }
    pub fn set_spell_label_store(&mut self, store: Arc<wow_data::SpellLabelStore>) {
        self.spell_label_store = Some(store);
    }
    pub fn spell_label_store(&self) -> Option<&Arc<wow_data::SpellLabelStore>> {
        self.spell_label_store.as_ref()
    }
    pub fn spell_class_options_store(&self) -> Option<&Arc<wow_data::SpellClassOptionsStore>> {
        self.spell_class_options_store.as_ref()
    }
    pub fn set_spell_custom_attribute_store(
        &mut self,
        store: Arc<wow_data::SpellCustomAttributeStoreLikeCpp>,
    ) {
        self.spell_custom_attribute_store = Some(store);
    }
    pub fn set_spell_duration_store(&mut self, store: Arc<wow_data::SpellDurationStore>) {
        self.spell_duration_store = Some(store);
    }
    pub fn set_spell_learn_skill_store(
        &mut self,
        store: Arc<wow_data::SpellLearnSkillStoreLikeCpp>,
    ) {
        self.spell_learn_skill_store = Some(store);
    }
    pub fn set_spell_learn_spell_store(
        &mut self,
        store: Arc<wow_data::SpellLearnSpellStoreLikeCpp>,
    ) {
        self.spell_learn_spell_store = Some(store);
    }
    pub fn set_spell_levels_store(&mut self, store: Arc<wow_data::SpellLevelsStore>) {
        self.spell_levels_store = Some(store);
    }
    pub fn set_spell_misc_store(&mut self, store: Arc<wow_data::SpellMiscStore>) {
        self.spell_misc_store = Some(store);
    }
    pub fn set_spell_proc_store(&mut self, store: Arc<wow_data::SpellProcStoreLikeCpp>) {
        self.spell_proc_store = Some(store);
    }
    pub fn set_spell_radius_store(&mut self, store: Arc<wow_data::SpellRadiusStore>) {
        self.spell_radius_store = Some(store);
    }
    pub fn set_spell_range_store(&mut self, store: Arc<wow_data::SpellRangeStore>) {
        self.spell_range_store = Some(store);
    }
    pub fn set_spell_required_store(&mut self, store: Arc<wow_data::SpellRequiredStoreLikeCpp>) {
        self.spell_required_store = Some(store);
    }
    pub fn set_spell_shapeshift_form_store(
        &mut self,
        store: Arc<wow_data::SpellShapeshiftFormStore>,
    ) {
        self.spell_shapeshift_form_store = Some(store);
    }
    pub fn set_spell_target_position_store(
        &mut self,
        store: Arc<wow_data::SpellTargetPositionStoreLikeCpp>,
    ) {
        self.spell_target_position_store = Some(store);
    }
    pub fn set_spell_target_restrictions_store(
        &mut self,
        store: Arc<wow_data::SpellTargetRestrictionsStore>,
    ) {
        self.spell_target_restrictions_store = Some(store);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_spell_totem_model_store(
        &mut self,
        store: Arc<wow_data::SpellTotemModelStoreLikeCpp>,
    ) {
        self.spell_totem_model_store = Some(store);
    }
    pub fn spell_acquisition_catalog(
        &self,
    ) -> Option<&Arc<wow_data::SpellAcquisitionCatalogLikeCpp>> {
        self.spell_acquisition_catalog.as_ref()
    }
    pub fn spell_aura_restrictions_store(
        &self,
    ) -> Option<&Arc<wow_data::SpellAuraRestrictionsStore>> {
        self.spell_aura_restrictions_store.as_ref()
    }
    pub fn spell_category_store(&self) -> Option<&Arc<wow_data::SpellCategoryStore>> {
        self.spell_category_store.as_ref()
    }
    pub fn spell_chain_store(&self) -> Option<&Arc<wow_data::SpellChainStoreLikeCpp>> {
        self.spell_chain_store.as_ref()
    }
    pub fn spell_custom_attribute_store_like_cpp(
        &self,
    ) -> Option<&Arc<wow_data::SpellCustomAttributeStoreLikeCpp>> {
        self.spell_custom_attribute_store.as_ref()
    }
    pub fn spell_learn_skill_store_like_cpp(
        &self,
    ) -> Option<&Arc<wow_data::SpellLearnSkillStoreLikeCpp>> {
        self.spell_learn_skill_store.as_ref()
    }
    pub fn spell_learn_spell_store_like_cpp(
        &self,
    ) -> Option<&Arc<wow_data::SpellLearnSpellStoreLikeCpp>> {
        self.spell_learn_spell_store.as_ref()
    }
    pub fn spell_levels_store(&self) -> Option<&Arc<wow_data::SpellLevelsStore>> {
        self.spell_levels_store.as_ref()
    }
    pub fn spell_linked_store_like_cpp(&self) -> Option<&wow_data::SpellLinkedStoreLikeCpp> {
        self.spell_linked_store.as_deref()
    }
    pub fn spell_misc_store(&self) -> Option<&Arc<wow_data::SpellMiscStore>> {
        self.spell_misc_store.as_ref()
    }
    #[allow(dead_code)]
    pub(crate) fn spell_proc_store(&self) -> Option<&Arc<wow_data::SpellProcStoreLikeCpp>> {
        self.spell_proc_store.as_ref()
    }
    pub fn spell_range_store(&self) -> Option<&Arc<wow_data::SpellRangeStore>> {
        self.spell_range_store.as_ref()
    }
    pub fn spell_required_store_like_cpp(
        &self,
    ) -> Option<&Arc<wow_data::SpellRequiredStoreLikeCpp>> {
        self.spell_required_store.as_ref()
    }
    #[allow(dead_code)]
    pub fn spell_shapeshift_form_store(&self) -> Option<&Arc<wow_data::SpellShapeshiftFormStore>> {
        self.spell_shapeshift_form_store.as_ref()
    }
    /// Get the spell store reference.
    pub fn spell_store(&self) -> Option<&Arc<wow_data::SpellStore>> {
        self.spell_store.as_ref()
    }
    pub fn spell_target_restrictions_store(
        &self,
    ) -> Option<&Arc<wow_data::SpellTargetRestrictionsStore>> {
        self.spell_target_restrictions_store.as_ref()
    }
}
