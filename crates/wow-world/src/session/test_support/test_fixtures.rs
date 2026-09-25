//! Test-only inputs for constructing Player bootstrap catalogs.

use super::super::{
    Arc, PlayerCreateInfoCastSpellStoreLikeCpp, PlayerCreateInfoCustomSpellStoreLikeCpp,
    PlayerCreateInfoStoreLikeCpp,
};

/// Configuration and catalog inputs used by detached Session tests.
pub(crate) struct PlayerBootstrapCatalogTestFixtureLikeCpp {
    pub(in crate::session) start_all_explored_like_cpp: bool,
    pub(in crate::session) start_all_reputation_like_cpp: bool,
    pub(in crate::session) start_all_spells_like_cpp: bool,
    pub(in crate::session) player_create_info_store_like_cpp:
        Option<Arc<PlayerCreateInfoStoreLikeCpp>>,
    pub(in crate::session) player_create_cast_spell_store_like_cpp:
        Option<Arc<PlayerCreateInfoCastSpellStoreLikeCpp>>,
    pub(in crate::session) player_create_custom_spell_store_like_cpp:
        Option<Arc<PlayerCreateInfoCustomSpellStoreLikeCpp>>,
}

impl Default for PlayerBootstrapCatalogTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            start_all_explored_like_cpp: false,
            start_all_reputation_like_cpp: false,
            start_all_spells_like_cpp: false,
            player_create_info_store_like_cpp: None,
            player_create_cast_spell_store_like_cpp: None,
            player_create_custom_spell_store_like_cpp: None,
        }
    }
}
