// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_start_all_spells_like_cpp(&mut self, enabled: bool) {
        self.catalogs.set_start_all_spells_like_cpp(enabled)
    }
    #[cfg(test)]
    pub(crate) fn start_all_spells_like_cpp(&self) -> bool {
        self.catalogs.start_all_spells_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn serverside_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&ServersideSpellInfoLikeCpp> {
        self.catalogs
            .serverside_spell_like_cpp(spell_id, difficulty)
    }
    pub(in crate::session) fn spell_has_no_unrepresented_runtime_hooks_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.spell_has_no_unrepresented_runtime_hooks_like_cpp(hub, spell_id)
    }
}

impl WorldSession {
    pub(crate) fn spell_linked_like_cpp(
        &self,
        link_type: SpellLinkedTypeLikeCpp,
        spell_id: u32,
    ) -> &[i32] {
        self.catalogs
            .spell_catalogs
            .spell_linked_store
            .as_ref()
            .and_then(|store| store.get_spell_linked_like_cpp(link_type, spell_id))
            .unwrap_or(&[])
    }
    pub(crate) fn spell_area_map_bounds_like_cpp(&self, spell_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_map_bounds_like_cpp(spell_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_area_for_area_map_bounds_like_cpp(
        &self,
        area_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_area_map_bounds_like_cpp(area_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_proc_entry_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&SpellProcEntryLikeCpp> {
        let store = self.catalogs.spell_catalogs.spell_proc_store.as_ref()?;
        store.spell_proc_entry_with_fallback_like_cpp(spell_id, difficulty, |current_difficulty| {
            self.catalogs
                .difficulty_store
                .as_ref()
                .and_then(|difficulties| difficulties.get(current_difficulty))
                .map(|difficulty| u32::from(difficulty.fallback_difficulty_id))
        })
    }
    pub(crate) fn spells_required_for_spell_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.catalogs
            .spell_catalogs
            .spell_required_store
            .as_ref()
            .map(|store| store.spells_required_for_spell_like_cpp(spell_id))
            .unwrap_or(&[])
    }
    pub(crate) fn is_spell_requiring_spell_like_cpp(&self, spell_id: u32, req_spell: u32) -> bool {
        self.catalogs
            .spell_catalogs
            .spell_required_store
            .as_ref()
            .map(|store| store.is_spell_requiring_spell_like_cpp(spell_id, req_spell))
            .unwrap_or(false)
    }
    /// Drop the trait definitions, config headers and entry flags before a
    /// trait-config load replaces them.
    pub(in crate::session) fn begin_represented_trait_authority_load_like_cpp(&mut self) -> bool {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.begin_trait_authority_load_like_cpp();
        })
        .is_some()
    }
    /// Install the authoritative trait-config headers and their entry flags.
    pub(in crate::session) fn install_represented_trait_authority_rows_like_cpp(
        &mut self,
        rows: std::collections::BTreeMap<i32, wow_entities::PlayerTraitConfigState>,
        entries_empty: bool,
    ) -> bool {
        self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.complete_trait_authority_load_like_cpp(rows, entries_empty);
        })
        .is_some()
    }
    /// Settle the saved rows, as `Player::_SaveSpells` completing does.
    pub(in crate::session) fn mark_represented_spell_rows_saved_like_cpp(&mut self) -> bool {
        self.mutate_player_spell_runtime_like_cpp(
            wow_entities::PlayerSpellRuntimeState::mark_spell_rows_saved_like_cpp,
        )
        .is_some()
    }
    /// Rebase every derived set on the rows a save left behind.
    pub(in crate::session) fn rebase_represented_spells_onto_saved_rows_like_cpp(
        &mut self,
    ) -> bool {
        self.mutate_player_spell_runtime_like_cpp(
            wow_entities::PlayerSpellRuntimeState::rebase_onto_saved_rows_like_cpp,
        )
        .is_some()
    }
    pub(crate) fn represented_spell_trait_definition_ids_like_cpp(&self) -> HashMap<i32, i32> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime
                .trait_definition_ids_like_cpp()
                .iter()
                .map(|(&id, &value)| (id, value))
                .collect()
        })
        .unwrap_or_default()
    }
    pub(crate) fn add_represented_self_res_spell_like_cpp(&mut self, spell_id: i32) {
        if spell_id == 0 {
            return;
        }
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .self_res_spells
                    .insert(spell_id)
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.core.player_handle_like_cpp.is_none() {
            self.spell_state
                .add_represented_self_res_spell_for_test_like_cpp(spell_id);
        }
    }
}
