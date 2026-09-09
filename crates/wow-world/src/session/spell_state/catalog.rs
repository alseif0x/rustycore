//! Spell catalog and template lookups used by the represented Session.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn restore_represented_character_spell_charge_like_cpp(
        &mut self,
        category_id: u32,
    ) -> bool {
        self.mutate_player_spell_history_like_cpp(|history| {
            if !history.charges_loaded {
                return false;
            }
            let Some(charges) = history.charges.get_mut(&category_id) else {
                return false;
            };
            if charges.pop_back().is_none() {
                return false;
            }
            if charges.is_empty() {
                history.charges.remove(&category_id);
            }
            true
        })
        .unwrap_or(false)
    }
    #[cfg(test)]
    pub fn set_player_create_custom_spell_store_like_cpp(
        &mut self,
        store: Arc<PlayerCreateInfoCustomSpellStoreLikeCpp>,
    ) {
        self.player_create_custom_spell_store_like_cpp = Some(store);
    }
    pub fn set_spell_shapeshift_form_store(&mut self, store: Arc<SpellShapeshiftFormStore>) {
        self.spell_shapeshift_form_store = Some(store);
    }
    #[allow(dead_code)]
    pub(crate) fn spell_shapeshift_form_store(&self) -> Option<&Arc<SpellShapeshiftFormStore>> {
        self.spell_shapeshift_form_store.as_ref()
    }
    /// Set the spell store for this session.
    pub fn set_spell_store(&mut self, store: Arc<SpellStore>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_store = Some(store);
    }
    /// Get the spell store reference.
    pub fn spell_store(&self) -> Option<&Arc<SpellStore>> {
        self.spell_store.as_ref()
    }
    pub fn set_spell_acquisition_catalog(&mut self, catalog: Arc<SpellAcquisitionCatalogLikeCpp>) {
        self.spell_acquisition_catalog = Some(catalog);
    }
    pub(crate) fn spell_acquisition_catalog(&self) -> Option<&Arc<SpellAcquisitionCatalogLikeCpp>> {
        self.spell_acquisition_catalog.as_ref()
    }
    pub fn set_spell_levels_store(&mut self, store: Arc<SpellLevelsStore>) {
        self.spell_levels_store = Some(store);
    }
    pub(crate) fn spell_levels_store(&self) -> Option<&Arc<SpellLevelsStore>> {
        self.spell_levels_store.as_ref()
    }
    pub fn set_spell_chain_store(&mut self, store: Arc<SpellChainStoreLikeCpp>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_chain_store = Some(store);
    }
    pub(crate) fn spell_chain_store(&self) -> Option<&Arc<SpellChainStoreLikeCpp>> {
        self.spell_chain_store.as_ref()
    }
    pub fn set_spell_category_store(&mut self, store: Arc<SpellCategoryStore>) {
        self.spell_category_store = Some(store);
    }
    pub(crate) fn spell_category_store(&self) -> Option<&Arc<SpellCategoryStore>> {
        self.spell_category_store.as_ref()
    }
    pub fn set_npc_spell_click_store(&mut self, store: Arc<NpcSpellClickStoreLikeCpp>) {
        self.npc_spell_click_store = Some(store);
    }
    #[allow(dead_code)]
    pub(crate) fn npc_spell_click_store(&self) -> Option<&Arc<NpcSpellClickStoreLikeCpp>> {
        self.npc_spell_click_store.as_ref()
    }
    pub fn set_spell_target_restrictions_store(
        &mut self,
        store: Arc<SpellTargetRestrictionsStore>,
    ) {
        self.spell_target_restrictions_store = Some(store);
    }
    pub(crate) fn spell_target_restrictions_store(
        &self,
    ) -> Option<&Arc<SpellTargetRestrictionsStore>> {
        self.spell_target_restrictions_store.as_ref()
    }
    pub fn set_spell_misc_store(&mut self, store: Arc<SpellMiscStore>) {
        self.spell_misc_store = Some(store);
    }
    pub fn set_spell_linked_store(&mut self, store: Arc<SpellLinkedStoreLikeCpp>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_linked_store = Some(store);
    }
    pub(crate) fn spell_linked_store_like_cpp(&self) -> Option<&SpellLinkedStoreLikeCpp> {
        self.spell_linked_store.as_deref()
    }
    pub fn set_spell_area_store(&mut self, store: Arc<SpellAreaStoreLikeCpp>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_area_store = Some(store);
    }
    pub fn set_spell_custom_attribute_store(
        &mut self,
        store: Arc<SpellCustomAttributeStoreLikeCpp>,
    ) {
        self.spell_custom_attribute_store = Some(store);
    }
    pub(crate) fn spell_custom_attribute_store_like_cpp(
        &self,
    ) -> Option<&Arc<SpellCustomAttributeStoreLikeCpp>> {
        self.spell_custom_attribute_store.as_ref()
    }
    #[cfg(test)]
    pub fn set_serverside_spell_store(&mut self, store: Arc<ServersideSpellStoreLikeCpp>) {
        self.serverside_spell_store = Some(store);
    }
    pub fn set_spell_proc_store(&mut self, store: Arc<SpellProcStoreLikeCpp>) {
        self.spell_proc_store = Some(store);
    }
    #[allow(dead_code)]
    pub(crate) fn spell_proc_store(&self) -> Option<&Arc<SpellProcStoreLikeCpp>> {
        self.spell_proc_store.as_ref()
    }
    pub fn set_spell_required_store(&mut self, store: Arc<SpellRequiredStoreLikeCpp>) {
        self.spell_required_store = Some(store);
    }
    pub(crate) fn spell_required_store_like_cpp(&self) -> Option<&Arc<SpellRequiredStoreLikeCpp>> {
        self.spell_required_store.as_ref()
    }
    #[cfg(test)]
    pub fn set_spell_totem_model_store(&mut self, store: Arc<SpellTotemModelStoreLikeCpp>) {
        self.spell_totem_model_store = Some(store);
    }
    pub fn set_spell_duration_store(&mut self, store: Arc<SpellDurationStore>) {
        self.spell_duration_store = Some(store);
    }
    pub fn set_spell_radius_store(&mut self, store: Arc<SpellRadiusStore>) {
        self.spell_radius_store = Some(store);
    }
    pub(crate) fn spell_misc_store(&self) -> Option<&Arc<SpellMiscStore>> {
        self.spell_misc_store.as_ref()
    }
    pub fn set_spell_range_store(&mut self, store: Arc<SpellRangeStore>) {
        self.spell_range_store = Some(store);
    }
    pub(crate) fn spell_range_store(&self) -> Option<&Arc<SpellRangeStore>> {
        self.spell_range_store.as_ref()
    }
    pub fn set_spell_target_position_store(&mut self, store: Arc<SpellTargetPositionStoreLikeCpp>) {
        self.spell_target_position_store = Some(store);
    }
    #[cfg(test)]
    pub(in crate::session) fn store_player_spell_runtime_fixture_like_cpp(
        &mut self,
        runtime: RepresentedPlayerSpellRuntimeLikeCpp,
    ) -> bool {
        if self.player_handle_like_cpp.is_none() {
            self.known_spells = runtime.known_spells;
            self.represented_player_spell_rows_like_cpp = runtime.rows;
            self.represented_player_spell_rows_loaded_like_cpp = runtime.rows_loaded;
            self.represented_player_spell_rows_complete_like_cpp = runtime.rows_complete;
            self.represented_fallback_player_spell_rows_like_cpp = runtime.fallback_rows;
            self.represented_dependent_known_spells_like_cpp = runtime.dependent_known_spells;
            self.represented_removed_known_spells_like_cpp = runtime.removed_known_spells;
            self.represented_favorite_known_spells_like_cpp = runtime.favorite_known_spells;
            self.represented_spell_trait_definition_ids_like_cpp = runtime.trait_definition_ids;
            self.represented_spell_trait_definition_ids_complete_like_cpp =
                runtime.trait_definition_ids_complete;
            self.represented_trait_config_rows_like_cpp = runtime.trait_config_rows;
            self.represented_trait_config_rows_complete_like_cpp =
                runtime.trait_config_rows_complete;
            self.represented_trait_entry_rows_complete_like_cpp = runtime.trait_entry_rows_complete;
            self.represented_trait_entry_rows_empty_like_cpp = runtime.trait_entry_rows_empty;
            self.represented_override_spells_like_cpp = runtime.override_spells;
            self.represented_override_spells_complete_like_cpp = runtime.override_spells_complete;
            return true;
        }
        false
    }
}
