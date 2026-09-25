// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player spell records: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedPlayerSkillStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPlayerSkillLikeCpp {
    pub skill_id: u16,
    pub step: u16,
    pub value: u16,
    pub max: u16,
    pub profession_slot: i8,
    pub state: RepresentedPlayerSkillStateLikeCpp,
}

#[cfg(test)]
pub(crate) fn is_non_durable_skill_tombstone_like_cpp(
    skill: &RepresentedPlayerSkillLikeCpp,
) -> bool {
    skill.step == 0
        && skill.value == 0
        && skill.max == 0
        && skill.profession_slot == -1
        && matches!(
            skill.state,
            RepresentedPlayerSkillStateLikeCpp::Unchanged
                | RepresentedPlayerSkillStateLikeCpp::Deleted
        )
}

pub(in crate::session) fn canonical_player_skill_record_like_cpp(
    skill: RepresentedPlayerSkillLikeCpp,
) -> wow_entities::PlayerSkillRecord {
    wow_entities::PlayerSkillRecord {
        skill_line_id: u32::from(skill.skill_id),
        current_value: skill.value,
        max_value: skill.max,
        step: skill.step,
        profession_slot: skill.profession_slot,
        state: match skill.state {
            RepresentedPlayerSkillStateLikeCpp::Unchanged => {
                wow_entities::PlayerSkillLoadState::Unchanged
            }
            RepresentedPlayerSkillStateLikeCpp::Changed => {
                wow_entities::PlayerSkillLoadState::Changed
            }
            RepresentedPlayerSkillStateLikeCpp::New => wow_entities::PlayerSkillLoadState::New,
            RepresentedPlayerSkillStateLikeCpp::Deleted => {
                wow_entities::PlayerSkillLoadState::Deleted
            }
        },
    }
}

pub(in crate::session) fn represented_player_skill_record_like_cpp(
    skill: &wow_entities::PlayerSkillRecord,
) -> Option<RepresentedPlayerSkillLikeCpp> {
    Some(RepresentedPlayerSkillLikeCpp {
        skill_id: u16::try_from(skill.skill_line_id).ok()?,
        step: skill.step,
        value: skill.current_value,
        max: skill.max_value,
        profession_slot: skill.profession_slot,
        state: match skill.state {
            wow_entities::PlayerSkillLoadState::Unchanged => {
                RepresentedPlayerSkillStateLikeCpp::Unchanged
            }
            wow_entities::PlayerSkillLoadState::Changed => {
                RepresentedPlayerSkillStateLikeCpp::Changed
            }
            wow_entities::PlayerSkillLoadState::New => RepresentedPlayerSkillStateLikeCpp::New,
            wow_entities::PlayerSkillLoadState::Deleted => {
                RepresentedPlayerSkillStateLikeCpp::Deleted
            }
        },
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedPlayerSpellStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Removed,
    Temporary,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPlayerSpellLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub disabled: bool,
    pub dependent: bool,
    pub favorite: bool,
    pub state: RepresentedPlayerSpellStateLikeCpp,
}

#[derive(Debug, Clone, Default)]
pub(in crate::session) struct RepresentedPlayerSpellRuntimeLikeCpp {
    pub(in crate::session) known_spells: Vec<i32>,
    pub(in crate::session) rows: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) rows_loaded: bool,
    pub(in crate::session) rows_complete: bool,
    pub(in crate::session) fallback_rows: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    pub(in crate::session) dependent_known_spells: HashSet<i32>,
    pub(in crate::session) removed_known_spells: HashSet<i32>,
    pub(in crate::session) favorite_known_spells: HashSet<i32>,
    pub(in crate::session) trait_definition_ids: HashMap<i32, i32>,
    pub(in crate::session) trait_definition_ids_complete: bool,
    pub(in crate::session) trait_config_rows: BTreeMap<i32, wow_entities::PlayerTraitConfigState>,
    pub(in crate::session) trait_config_rows_complete: bool,
    pub(in crate::session) trait_entry_rows_complete: bool,
    pub(in crate::session) trait_entry_rows_empty: bool,
    #[cfg(test)]
    pub(in crate::session) override_spells: HashMap<i32, BTreeSet<i32>>,
    pub(in crate::session) override_spells_complete: bool,
}

pub(in crate::session) fn canonical_player_spell_record_like_cpp(
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

pub(in crate::session) fn represented_player_spell_record_like_cpp(
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

#[cfg(test)]
pub(in crate::session) fn canonical_player_spell_runtime_like_cpp(
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

pub(in crate::session) fn represented_player_spell_runtime_like_cpp(
    runtime: &wow_entities::PlayerSpellRuntimeState,
) -> RepresentedPlayerSpellRuntimeLikeCpp {
    RepresentedPlayerSpellRuntimeLikeCpp {
        known_spells: runtime.known_spells_like_cpp().to_vec(),
        rows: runtime
            .rows_like_cpp()
            .iter()
            .map(|(&spell_id, row)| (spell_id, represented_player_spell_record_like_cpp(row)))
            .collect(),
        #[cfg(test)]
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
        #[cfg(test)]
        override_spells: runtime
            .override_spells_like_cpp()
            .iter()
            .map(|(&spell_id, overrides)| (spell_id, overrides.clone()))
            .collect(),
        override_spells_complete: runtime.override_spells_complete_like_cpp(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedCharacterSpellCooldownLikeCpp {
    pub spell_id: u32,
    pub item_id: u32,
    pub cooldown_end_unix_secs: i64,
    pub category_id: u32,
    pub category_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedCharacterSpellChargeLikeCpp {
    pub category_id: u32,
    pub recharge_start_unix_secs: i64,
    pub recharge_end_unix_secs: i64,
}

#[allow(dead_code)]
pub(in crate::session) fn represented_skill_records_from_values_like_cpp(
    skill_values: &HashMap<u16, u16>,
) -> HashMap<u16, RepresentedPlayerSkillLikeCpp> {
    skill_values
        .iter()
        .map(|(&skill_id, &value)| {
            (
                skill_id,
                RepresentedPlayerSkillLikeCpp {
                    skill_id,
                    step: 0,
                    value,
                    max: value,
                    profession_slot: -1,
                    state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
                },
            )
        })
        .collect()
}

pub(in crate::session) fn represented_skill_values_from_records_like_cpp(
    skill_records: &HashMap<u16, RepresentedPlayerSkillLikeCpp>,
) -> HashMap<u16, u16> {
    skill_records
        .iter()
        .map(|(&skill_id, record)| (skill_id, record.value))
        .collect()
}
