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
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.restore_represented_character_spell_charge_like_cpp(&mut hub, category_id)
    }
    /// Set the spell store for this session.
    pub fn set_spell_store(&mut self, store: Arc<SpellStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.spell_catalogs.spell_store = Some(store);
    }
    pub fn set_spell_chain_store(&mut self, store: Arc<SpellChainStoreLikeCpp>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.spell_catalogs.spell_chain_store = Some(store);
    }
    pub fn set_spell_linked_store(&mut self, store: Arc<SpellLinkedStoreLikeCpp>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.spell_catalogs.spell_linked_store = Some(store);
    }
    pub fn set_spell_area_store(&mut self, store: Arc<SpellAreaStoreLikeCpp>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.spell_catalogs.spell_area_store = Some(store);
    }
    #[cfg(test)]
    pub(in crate::session) fn store_player_spell_runtime_fixture_like_cpp(
        &mut self,
        runtime: RepresentedPlayerSpellRuntimeLikeCpp,
    ) -> bool {
        if self.core.player_handle_like_cpp.is_none() {
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .known_spells = runtime.known_spells;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_player_spell_rows_like_cpp = runtime.rows;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_player_spell_rows_loaded_like_cpp = runtime.rows_loaded;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_player_spell_rows_complete_like_cpp = runtime.rows_complete;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_fallback_player_spell_rows_like_cpp = runtime.fallback_rows;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_dependent_known_spells_like_cpp = runtime.dependent_known_spells;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_removed_known_spells_like_cpp = runtime.removed_known_spells;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_favorite_known_spells_like_cpp = runtime.favorite_known_spells;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_spell_trait_definition_ids_like_cpp = runtime.trait_definition_ids;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_spell_trait_definition_ids_complete_like_cpp =
                runtime.trait_definition_ids_complete;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_trait_config_rows_like_cpp = runtime.trait_config_rows;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_trait_config_rows_complete_like_cpp =
                runtime.trait_config_rows_complete;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_trait_entry_rows_complete_like_cpp = runtime.trait_entry_rows_complete;
            self.spell_state
                .player_spell_test_fixture_mut_like_cpp()
                .represented_trait_entry_rows_empty_like_cpp = runtime.trait_entry_rows_empty;
            self.spell_state
                .replace_represented_override_spell_fixture_like_cpp(
                    runtime.override_spells,
                    runtime.override_spells_complete,
                );
            return true;
        }
        false
    }
}


impl WorldSession {
    pub fn set_spell_aura_restrictions_store(&mut self, store: Arc<SpellAuraRestrictionsStore>) {
        self.catalogs
            .spell_catalogs
            .set_spell_aura_restrictions_store(store);
    }
    pub fn set_spell_aura_options_store(&mut self, store: Arc<SpellAuraOptionsStore>) {
        self.catalogs
            .spell_catalogs
            .set_spell_aura_options_store(store);
    }
    pub fn set_spell_target_position_store(&mut self, store: Arc<SpellTargetPositionStoreLikeCpp>) {
        self.catalogs
            .spell_catalogs
            .set_spell_target_position_store(store);
    }
    pub fn set_spell_range_store(&mut self, store: Arc<SpellRangeStore>) {
        self.catalogs.spell_catalogs.set_spell_range_store(store);
    }
    pub fn set_spell_radius_store(&mut self, store: Arc<SpellRadiusStore>) {
        self.catalogs.spell_catalogs.set_spell_radius_store(store);
    }
    pub fn set_spell_duration_store(&mut self, store: Arc<SpellDurationStore>) {
        self.catalogs.spell_catalogs.set_spell_duration_store(store);
    }
    pub fn set_spell_required_store(&mut self, store: Arc<SpellRequiredStoreLikeCpp>) {
        self.catalogs.spell_catalogs.set_spell_required_store(store);
    }
    pub fn set_spell_proc_store(&mut self, store: Arc<SpellProcStoreLikeCpp>) {
        self.catalogs.spell_catalogs.set_spell_proc_store(store);
    }
    pub fn set_spell_custom_attribute_store(
        &mut self,
        store: Arc<SpellCustomAttributeStoreLikeCpp>,
    ) {
        self.catalogs
            .spell_catalogs
            .set_spell_custom_attribute_store(store);
    }
    pub fn set_spell_misc_store(&mut self, store: Arc<SpellMiscStore>) {
        self.catalogs.spell_catalogs.set_spell_misc_store(store);
    }
    pub fn set_spell_target_restrictions_store(
        &mut self,
        store: Arc<SpellTargetRestrictionsStore>,
    ) {
        self.catalogs
            .spell_catalogs
            .set_spell_target_restrictions_store(store);
    }
    pub fn set_npc_spell_click_store(&mut self, store: Arc<NpcSpellClickStoreLikeCpp>) {
        self.catalogs
            .spell_catalogs
            .set_npc_spell_click_store(store);
    }
    pub fn set_spell_category_store(&mut self, store: Arc<SpellCategoryStore>) {
        self.catalogs.spell_catalogs.set_spell_category_store(store);
    }
    pub fn set_spell_levels_store(&mut self, store: Arc<SpellLevelsStore>) {
        self.catalogs.spell_catalogs.set_spell_levels_store(store);
    }
    pub fn set_spell_acquisition_catalog(&mut self, catalog: Arc<SpellAcquisitionCatalogLikeCpp>) {
        self.catalogs
            .spell_catalogs
            .set_spell_acquisition_catalog(catalog);
    }
    pub fn spell_store(&self) -> Option<&Arc<SpellStore>> {
        self.catalogs.spell_store()
    }
    pub fn set_spell_shapeshift_form_store(&mut self, store: Arc<SpellShapeshiftFormStore>) {
        self.catalogs
            .spell_catalogs
            .set_spell_shapeshift_form_store(store);
    }
    pub fn set_spell_class_options_store(&mut self, store: Arc<wow_data::SpellClassOptionsStore>) {
        self.catalogs
            .spell_catalogs
            .set_spell_class_options_store(store);
    }
    /// C++ `sSpellMgr` label authority, read by `SpellInfo::HasLabel`.
    pub fn set_spell_label_store(&mut self, store: Arc<wow_data::SpellLabelStore>) {
        self.catalogs.spell_catalogs.set_spell_label_store(store);
    }
    pub fn spell_label_store(&self) -> Option<&Arc<wow_data::SpellLabelStore>> {
        self.catalogs.spell_catalogs.spell_label_store()
    }
    pub fn set_spell_learn_spell_store(&mut self, store: Arc<SpellLearnSpellStoreLikeCpp>) {
        self.catalogs
            .spell_catalogs
            .set_spell_learn_spell_store(store);
    }
    pub fn set_spell_learn_skill_store(&mut self, store: Arc<SpellLearnSkillStoreLikeCpp>) {
        self.catalogs
            .spell_catalogs
            .set_spell_learn_skill_store(store);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/catalog/f3_shims.rs"]
mod f3_shims;
