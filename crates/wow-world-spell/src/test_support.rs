use std::collections::{BTreeMap, HashMap, HashSet};

use crate::RepresentedPlayerSpellLikeCpp;

/// Handle-less test fixture for Player spellbook and trait-configuration state.
pub struct PlayerSpellAndTraitTestFixtureLikeCpp {
    pub known_spells: Vec<i32>,
    pub represented_player_spell_rows_like_cpp: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    pub represented_player_spell_rows_loaded_like_cpp: bool,
    pub represented_player_spell_rows_complete_like_cpp: bool,
    pub represented_fallback_player_spell_rows_like_cpp:
        BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    pub represented_dependent_known_spells_like_cpp: HashSet<i32>,
    pub represented_removed_known_spells_like_cpp: HashSet<i32>,
    pub represented_favorite_known_spells_like_cpp: HashSet<i32>,
    pub represented_spell_trait_definition_ids_like_cpp: HashMap<i32, i32>,
    pub represented_spell_trait_definition_ids_complete_like_cpp: bool,
    pub represented_trait_config_rows_like_cpp: BTreeMap<i32, wow_entities::PlayerTraitConfigState>,
    pub represented_trait_config_rows_complete_like_cpp: bool,
    pub represented_trait_entry_rows_complete_like_cpp: bool,
    pub represented_trait_entry_rows_empty_like_cpp: bool,
}

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
