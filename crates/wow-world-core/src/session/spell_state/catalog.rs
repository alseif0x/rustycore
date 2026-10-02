use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::{
    PlayerCreateInfoCustomSpellStoreLikeCpp, ServersideSpellStoreLikeCpp,
    SpellTotemModelStoreLikeCpp,
};
use wow_data::SpellStore;

impl crate::session::state::SessionCatalogs {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_create_custom_spell_store_like_cpp(
        &mut self,
        store: Arc<PlayerCreateInfoCustomSpellStoreLikeCpp>,
    ) {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .player_create_custom_spell_store_like_cpp = Some(store);
    }
}

impl crate::session::state::SessionCatalogs {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_spell_totem_model_store(&mut self, store: Arc<SpellTotemModelStoreLikeCpp>) {
        self.spell_catalogs.set_spell_totem_model_store(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_serverside_spell_store(&mut self, store: Arc<ServersideSpellStoreLikeCpp>) {
        self.spell_catalogs.set_serverside_spell_store(store);
    }

    /// C++ `SpellInfo::SpellFamilyName`/`SpellFamilyFlags` source
    /// (`SpellClassOptions.db2`).
    pub fn spell_class_options_store(&self) -> Option<&Arc<wow_data::SpellClassOptionsStore>> {
        self.spell_catalogs.spell_class_options_store()
    }
}

impl crate::session::state::SessionCatalogs {
    /// Get the spell store reference.
    pub fn spell_store(&self) -> Option<&Arc<SpellStore>> {
        self.spell_catalogs.spell_store()
    }
}
