//! Represented aura and spell-state responsibility, separated from the
//! Session root under #601. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

/// Handle-less test fixture for Player spellbook and trait-configuration state.
#[cfg(test)]
pub(super) struct PlayerSpellAndTraitTestFixtureLikeCpp {
    pub(super) known_spells: Vec<i32>,
    pub(super) represented_player_spell_rows_like_cpp: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    pub(super) represented_player_spell_rows_loaded_like_cpp: bool,
    pub(super) represented_player_spell_rows_complete_like_cpp: bool,
    pub(super) represented_fallback_player_spell_rows_like_cpp:
        BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    pub(super) represented_dependent_known_spells_like_cpp: HashSet<i32>,
    pub(super) represented_removed_known_spells_like_cpp: HashSet<i32>,
    pub(super) represented_favorite_known_spells_like_cpp: HashSet<i32>,
    pub(super) represented_spell_trait_definition_ids_like_cpp: HashMap<i32, i32>,
    pub(super) represented_spell_trait_definition_ids_complete_like_cpp: bool,
    pub(super) represented_trait_config_rows_like_cpp:
        BTreeMap<i32, wow_entities::PlayerTraitConfigState>,
    pub(super) represented_trait_config_rows_complete_like_cpp: bool,
    pub(super) represented_trait_entry_rows_complete_like_cpp: bool,
    pub(super) represented_trait_entry_rows_empty_like_cpp: bool,
}

#[cfg(test)]
impl Default for PlayerSpellAndTraitTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            known_spells: Vec::new(),
            represented_player_spell_rows_like_cpp: BTreeMap::new(),
            represented_player_spell_rows_loaded_like_cpp: false,
            represented_player_spell_rows_complete_like_cpp: false,
            represented_fallback_player_spell_rows_like_cpp: BTreeMap::new(),
            represented_dependent_known_spells_like_cpp: HashSet::new(),
            represented_removed_known_spells_like_cpp: HashSet::new(),
            represented_favorite_known_spells_like_cpp: HashSet::new(),
            represented_spell_trait_definition_ids_like_cpp: HashMap::new(),
            represented_spell_trait_definition_ids_complete_like_cpp: false,
            represented_trait_config_rows_like_cpp: BTreeMap::new(),
            represented_trait_config_rows_complete_like_cpp: false,
            represented_trait_entry_rows_complete_like_cpp: false,
            represented_trait_entry_rows_empty_like_cpp: false,
        }
    }
}

mod acquisition;
mod aura;
mod aura_application;
mod aura_publication;
mod cast;
mod catalog;
mod cooldown;
mod effects;
mod mount_aura;
mod shapeshift;

pub(crate) use aura::RepresentedShapeshiftMutationLikeCpp;
pub(crate) use aura_publication::player_aura_info_like_cpp;
mod spell;
mod spell_click;
mod spell_publication;
mod spellbook;
