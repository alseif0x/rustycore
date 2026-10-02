// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only inputs for constructing Player bootstrap catalogs.

use std::sync::Arc;
use wow_data::{
    PlayerCreateInfoCastSpellStoreLikeCpp, PlayerCreateInfoCustomSpellStoreLikeCpp,
    PlayerCreateInfoStoreLikeCpp,
};

/// Configuration and catalog inputs used by detached Session tests.
pub struct PlayerBootstrapCatalogTestFixtureLikeCpp {
    pub start_all_explored_like_cpp: bool,
    pub start_all_reputation_like_cpp: bool,
    pub start_all_spells_like_cpp: bool,
    pub player_create_info_store_like_cpp: Option<Arc<PlayerCreateInfoStoreLikeCpp>>,
    pub player_create_cast_spell_store_like_cpp:
        Option<Arc<PlayerCreateInfoCastSpellStoreLikeCpp>>,
    pub player_create_custom_spell_store_like_cpp:
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
