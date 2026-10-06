#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::BTreeSet;
use std::collections::{BTreeMap, HashMap, HashSet};
use wow_core::{ObjectGuid, Position};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedPlayerSpellStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Removed,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedSpellFocusObjectLikeCpp {
    pub guid: ObjectGuid,
    pub map_key: wow_map::MapKey,
    pub position: Position,
    pub source: wow_entities::SpellFocusUseSource,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedPlayerSpellLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub disabled: bool,
    pub dependent: bool,
    pub favorite: bool,
    pub state: RepresentedPlayerSpellStateLikeCpp,
}

#[derive(Debug, Clone, Default)]
pub struct RepresentedPlayerSpellRuntimeLikeCpp {
    pub known_spells: Vec<i32>,
    pub rows: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub rows_loaded: bool,
    pub rows_complete: bool,
    pub fallback_rows: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    pub dependent_known_spells: HashSet<i32>,
    pub removed_known_spells: HashSet<i32>,
    pub favorite_known_spells: HashSet<i32>,
    pub trait_definition_ids: HashMap<i32, i32>,
    pub trait_definition_ids_complete: bool,
    pub trait_config_rows: BTreeMap<i32, wow_entities::PlayerTraitConfigState>,
    pub trait_config_rows_complete: bool,
    pub trait_entry_rows_complete: bool,
    pub trait_entry_rows_empty: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub override_spells: HashMap<i32, BTreeSet<i32>>,
    pub override_spells_complete: bool,
}

pub fn canonical_player_spell_record_like_cpp(
    row: RepresentedPlayerSpellLikeCpp,
) -> wow_entities::PlayerKnownSpellRecord {
    wow_entities::PlayerKnownSpellRecord {
        spell_id: row.spell_id,
        state: match row.state {
            RepresentedPlayerSpellStateLikeCpp::Unchanged => {
                wow_entities::PlayerSpellLoadState::Unchanged
            }
            RepresentedPlayerSpellStateLikeCpp::Changed => {
                wow_entities::PlayerSpellLoadState::Changed
            }
            RepresentedPlayerSpellStateLikeCpp::New => wow_entities::PlayerSpellLoadState::New,
            RepresentedPlayerSpellStateLikeCpp::Removed => {
                wow_entities::PlayerSpellLoadState::Removed
            }
            RepresentedPlayerSpellStateLikeCpp::Temporary => {
                wow_entities::PlayerSpellLoadState::Temporary
            }
        },
        active: row.active,
        disabled: row.disabled,
        favorite: row.favorite,
        dependent: row.dependent,
    }
}

pub fn represented_player_spell_record_like_cpp(
    row: &wow_entities::PlayerKnownSpellRecord,
) -> RepresentedPlayerSpellLikeCpp {
    RepresentedPlayerSpellLikeCpp {
        spell_id: row.spell_id,
        active: row.active,
        disabled: row.disabled,
        dependent: row.dependent,
        favorite: row.favorite,
        state: match row.state {
            wow_entities::PlayerSpellLoadState::Unchanged => {
                RepresentedPlayerSpellStateLikeCpp::Unchanged
            }
            wow_entities::PlayerSpellLoadState::Changed => {
                RepresentedPlayerSpellStateLikeCpp::Changed
            }
            wow_entities::PlayerSpellLoadState::New => RepresentedPlayerSpellStateLikeCpp::New,
            wow_entities::PlayerSpellLoadState::Removed => {
                RepresentedPlayerSpellStateLikeCpp::Removed
            }
            wow_entities::PlayerSpellLoadState::Temporary => {
                RepresentedPlayerSpellStateLikeCpp::Temporary
            }
        },
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn canonical_player_spell_runtime_like_cpp(
    runtime: RepresentedPlayerSpellRuntimeLikeCpp,
) -> wow_entities::PlayerSpellRuntimeState {
    let mut state = wow_entities::PlayerSpellRuntimeState::default();
    state.install_acquisition_snapshot_like_cpp(
        wow_entities::PlayerSpellAcquisitionSnapshotLikeCpp {
            known_spells: runtime.known_spells,
            rows: runtime
                .rows
                .into_iter()
                .map(|(spell_id, row)| (spell_id, canonical_player_spell_record_like_cpp(row)))
                .collect(),
            dependent_known_spells: runtime.dependent_known_spells.into_iter().collect(),
            removed_known_spells: runtime.removed_known_spells.into_iter().collect(),
            favorite_known_spells: runtime.favorite_known_spells.into_iter().collect(),
            trait_definition_ids: runtime.trait_definition_ids.into_iter().collect(),
            override_spells: runtime.override_spells.into_iter().collect(),
        },
    );
    state.set_row_authority_like_cpp(runtime.rows_loaded, runtime.rows_complete);
    state.set_acquisition_snapshot_completeness_like_cpp(
        runtime.trait_definition_ids_complete,
        runtime.override_spells_complete,
    );
    state.replace_fallback_rows_like_cpp(
        runtime
            .fallback_rows
            .into_iter()
            .map(|(spell_id, row)| (spell_id, canonical_player_spell_record_like_cpp(row)))
            .collect(),
    );
    state.complete_trait_authority_load_like_cpp(
        runtime.trait_config_rows,
        runtime.trait_entry_rows_empty,
    );
    state.set_trait_config_authority_for_fixture_like_cpp(
        runtime.trait_config_rows_complete,
        runtime.trait_entry_rows_complete,
        runtime.trait_entry_rows_empty,
    );
    state
}

pub fn represented_player_spell_runtime_like_cpp(
    runtime: &wow_entities::PlayerSpellRuntimeState,
) -> RepresentedPlayerSpellRuntimeLikeCpp {
    RepresentedPlayerSpellRuntimeLikeCpp {
        known_spells: runtime.known_spells_like_cpp().to_vec(),
        rows: runtime
            .rows_like_cpp()
            .iter()
            .map(|(&spell_id, row)| (spell_id, represented_player_spell_record_like_cpp(row)))
            .collect(),
        #[cfg(any(test, feature = "test-fixtures"))]
        rows_loaded: runtime.rows_loaded_like_cpp(),
        rows_complete: runtime.rows_complete_like_cpp(),
        fallback_rows: runtime
            .fallback_rows_like_cpp()
            .iter()
            .map(|(&spell_id, row)| (spell_id, represented_player_spell_record_like_cpp(row)))
            .collect(),
        dependent_known_spells: runtime
            .dependent_known_spells_like_cpp()
            .iter()
            .copied()
            .collect(),
        removed_known_spells: runtime
            .removed_known_spells_like_cpp()
            .iter()
            .copied()
            .collect(),
        favorite_known_spells: runtime
            .favorite_known_spells_like_cpp()
            .iter()
            .copied()
            .collect(),
        trait_definition_ids: runtime
            .trait_definition_ids_like_cpp()
            .iter()
            .map(|(&spell_id, &trait_definition_id)| (spell_id, trait_definition_id))
            .collect(),
        trait_definition_ids_complete: runtime.trait_definition_ids_complete_like_cpp(),
        trait_config_rows: runtime.trait_config_rows_like_cpp().clone(),
        trait_config_rows_complete: runtime.trait_config_rows_complete_like_cpp(),
        trait_entry_rows_complete: runtime.trait_entry_rows_complete_like_cpp(),
        trait_entry_rows_empty: runtime.trait_entry_rows_empty_like_cpp(),
        #[cfg(any(test, feature = "test-fixtures"))]
        override_spells: runtime
            .override_spells_like_cpp()
            .iter()
            .map(|(&spell_id, overrides)| (spell_id, overrides.clone()))
            .collect(),
        override_spells_complete: runtime.override_spells_complete_like_cpp(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedCharacterSpellCooldownLikeCpp {
    pub spell_id: u32,
    pub item_id: u32,
    pub cooldown_end_unix_secs: i64,
    pub category_id: u32,
    pub category_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedCharacterSpellChargeLikeCpp {
    pub category_id: u32,
    pub recharge_start_unix_secs: i64,
    pub recharge_end_unix_secs: i64,
}
