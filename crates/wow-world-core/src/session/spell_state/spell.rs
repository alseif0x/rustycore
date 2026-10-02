use crate::session::HubRef;
use crate::session::state::SessionCatalogs;
use std::collections::HashSet;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::ServersideSpellInfoLikeCpp;

impl SessionCatalogs {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_start_all_spells_like_cpp(&mut self, enabled: bool) {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .start_all_spells_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn start_all_spells_like_cpp(&self) -> bool {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .start_all_spells_like_cpp
    }

    pub fn next_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_catalogs
            .spell_chain_store
            .as_ref()
            .map(|store| store.next_spell_in_chain_like_cpp(spell_id))
            .unwrap_or(0)
    }

    pub fn first_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_catalogs
            .spell_chain_store
            .as_ref()
            .map(|store| store.first_spell_in_chain_like_cpp(spell_id))
            .unwrap_or(spell_id)
    }

    pub fn prev_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_catalogs
            .spell_chain_store
            .as_ref()
            .map(|store| store.prev_spell_in_chain_like_cpp(spell_id))
            .unwrap_or(0)
    }

    pub fn spell_custom_attributes_for_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> u32 {
        self.spell_catalogs
            .spell_custom_attribute_store
            .as_ref()
            .map(|store| store.attributes_for_spell_difficulty_like_cpp(spell_id, difficulty))
            .unwrap_or(0)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn serverside_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&ServersideSpellInfoLikeCpp> {
        self.spell_catalogs
            .serverside_spell_store
            .as_ref()
            .and_then(|store| store.get_serverside_spell_like_cpp(spell_id, difficulty))
    }

    pub fn spells_requiring_spell_like_cpp(&self, req_spell: u32) -> &[u32] {
        self.spell_catalogs
            .spell_required_store
            .as_ref()
            .map(|store| store.spells_requiring_spell_like_cpp(req_spell))
            .unwrap_or(&[])
    }

    pub fn represented_mount_capability_mod_spell_like_cpp(
        &self,
        mount_capability_id: i32,
    ) -> Option<i32> {
        u32::try_from(mount_capability_id)
            .ok()
            .and_then(|id| self.mount_capability_store.as_ref()?.get(id))
            .map(|capability| capability.mod_spell_aura_id)
            .filter(|spell_id| *spell_id > 0)
    }
}

impl HubRef<'_> {
    pub fn represented_spell_valid_for_talent_like_cpp(&self, spell_id: i32) -> bool {
        let Some(spell_store) = self.catalogs.spell_store() else {
            return true;
        };
        wow_data::represented_spell_valid_with_seen_like_cpp(
            spell_store,
            spell_id,
            &mut HashSet::new(),
        )
    }
}

impl SessionCatalogs {
    pub fn spell_school_mask_for_difficulty_like_cpp(&self, spell_id: u32, difficulty: u8) -> u32 {
        self.spell_catalogs
            .spell_misc_store()
            .and_then(|store| {
                store.entry_for_spell_difficulty_with_fallback_like_cpp(
                    spell_id,
                    difficulty,
                    self.difficulty_store().map(AsRef::as_ref),
                )
            })
            .map_or(1, |entry| u32::from(entry.school_mask))
    }
}
