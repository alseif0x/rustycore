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
    /// Set the spell store for this session.
    pub fn set_spell_store(&mut self, store: Arc<SpellStore>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_catalogs.spell_store = Some(store);
    }
    pub fn set_spell_chain_store(&mut self, store: Arc<SpellChainStoreLikeCpp>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_catalogs.spell_chain_store = Some(store);
    }
    pub fn set_spell_linked_store(&mut self, store: Arc<SpellLinkedStoreLikeCpp>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_catalogs.spell_linked_store = Some(store);
    }
    pub fn set_spell_area_store(&mut self, store: Arc<SpellAreaStoreLikeCpp>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.spell_catalogs.spell_area_store = Some(store);
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

impl WorldSession {
    pub fn set_spell_aura_restrictions_store(&mut self, store: Arc<SpellAuraRestrictionsStore>) {
        self.spell_catalogs.set_spell_aura_restrictions_store(store);
    }
    pub fn set_spell_aura_options_store(&mut self, store: Arc<SpellAuraOptionsStore>) {
        self.spell_catalogs.set_spell_aura_options_store(store);
    }
    pub fn set_spell_target_position_store(&mut self, store: Arc<SpellTargetPositionStoreLikeCpp>) {
        self.spell_catalogs.set_spell_target_position_store(store);
    }
    pub fn set_spell_range_store(&mut self, store: Arc<SpellRangeStore>) {
        self.spell_catalogs.set_spell_range_store(store);
    }
    pub fn set_spell_radius_store(&mut self, store: Arc<SpellRadiusStore>) {
        self.spell_catalogs.set_spell_radius_store(store);
    }
    pub fn set_spell_duration_store(&mut self, store: Arc<SpellDurationStore>) {
        self.spell_catalogs.set_spell_duration_store(store);
    }
    #[cfg(test)]
    pub fn set_spell_totem_model_store(&mut self, store: Arc<SpellTotemModelStoreLikeCpp>) {
        self.spell_catalogs.set_spell_totem_model_store(store);
    }
    pub fn set_spell_required_store(&mut self, store: Arc<SpellRequiredStoreLikeCpp>) {
        self.spell_catalogs.set_spell_required_store(store);
    }
    pub fn set_spell_proc_store(&mut self, store: Arc<SpellProcStoreLikeCpp>) {
        self.spell_catalogs.set_spell_proc_store(store);
    }
    #[cfg(test)]
    pub fn set_serverside_spell_store(&mut self, store: Arc<ServersideSpellStoreLikeCpp>) {
        self.spell_catalogs.set_serverside_spell_store(store);
    }
    pub fn set_spell_custom_attribute_store(
        &mut self,
        store: Arc<SpellCustomAttributeStoreLikeCpp>,
    ) {
        self.spell_catalogs.set_spell_custom_attribute_store(store);
    }
    pub fn set_spell_misc_store(&mut self, store: Arc<SpellMiscStore>) {
        self.spell_catalogs.set_spell_misc_store(store);
    }
    pub fn set_spell_target_restrictions_store(
        &mut self,
        store: Arc<SpellTargetRestrictionsStore>,
    ) {
        self.spell_catalogs
            .set_spell_target_restrictions_store(store);
    }
    pub fn set_npc_spell_click_store(&mut self, store: Arc<NpcSpellClickStoreLikeCpp>) {
        self.spell_catalogs.set_npc_spell_click_store(store);
    }
    pub fn set_spell_category_store(&mut self, store: Arc<SpellCategoryStore>) {
        self.spell_catalogs.set_spell_category_store(store);
    }
    pub fn set_spell_levels_store(&mut self, store: Arc<SpellLevelsStore>) {
        self.spell_catalogs.set_spell_levels_store(store);
    }
    pub fn set_spell_acquisition_catalog(&mut self, catalog: Arc<SpellAcquisitionCatalogLikeCpp>) {
        self.spell_catalogs.set_spell_acquisition_catalog(catalog);
    }
    /// Get the spell store reference.
    pub fn spell_store(&self) -> Option<&Arc<SpellStore>> {
        self.spell_catalogs.spell_store()
    }
    pub fn set_spell_shapeshift_form_store(&mut self, store: Arc<SpellShapeshiftFormStore>) {
        self.spell_catalogs.set_spell_shapeshift_form_store(store);
    }
    pub fn set_spell_learn_spell_store(&mut self, store: Arc<SpellLearnSpellStoreLikeCpp>) {
        self.spell_catalogs.set_spell_learn_spell_store(store);
    }
    pub fn set_spell_learn_skill_store(&mut self, store: Arc<SpellLearnSkillStoreLikeCpp>) {
        self.spell_catalogs.set_spell_learn_skill_store(store);
    }
}
