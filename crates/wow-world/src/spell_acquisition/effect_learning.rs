// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player hit-target EffectLearnSpell. This operation never requests a database
//! write: both planned and fallback learning retain ordinary SaveToDB dirty state.
use super::*;
use crate::profession::{
    PrimaryProfessionCapacityPlanErrorLikeCpp, PrimaryProfessionCapacityPlanLikeCpp,
};
use wow_entities::{PlayerKnownSpellRecord, PlayerSpellLoadState, PlayerSpellRuntimeState};

/// Synchronous capabilities for the represented player-learning operation.
/// Snapshots are read projections, never a second mutable Player authority.
pub(crate) trait EffectLearningRuntimeLikeCpp: PlayerSpellAcquisitionRuntimeLikeCpp {
    fn admits_learning_target(&self, spell: i32, target: wow_core::ObjectGuid) -> bool;
    fn project_learning(&self, spell: u32) -> SpellAcquisitionOutcomeLikeCpp;
    fn acquisition_snapshot(
        &self,
    ) -> Result<PlayerSpellAcquisitionSnapshotLikeCpp, SpellAcquisitionSnapshotAdapterErrorLikeCpp>;
    fn profession_capacity(
        &self,
        skills: &[u32],
    ) -> Result<PrimaryProfessionCapacityPlanLikeCpp, PrimaryProfessionCapacityPlanErrorLikeCpp>;
    fn fallback_snapshot(&self) -> Option<PlayerSpellRuntimeState>;
    fn fallback_chains(&self) -> Option<&SpellChainStoreLikeCpp>;
    fn fallback_requirements(&self) -> Option<&SpellRequiredStoreLikeCpp>;
    fn fallback_traits(&self) -> Option<&TraitDefinitionStore>;
    fn install_fallback_row(
        &mut self,
        row: PlayerKnownSpellRecord,
        source: &PlayerSpellRuntimeState,
    );
}

pub(crate) fn execute_effect_learning_like_cpp<R: EffectLearningRuntimeLikeCpp>(
    runtime: &mut R,
    spell: i32,
    target: wow_core::ObjectGuid,
) -> bool {
    if !runtime.admits_learning_target(spell, target) {
        return false;
    }
    let plan = match runtime.project_learning(spell as u32) {
        SpellAcquisitionOutcomeLikeCpp::Deterministic(plan) => plan,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(_) => {
            return apply_base_learning_like_cpp(runtime, spell);
        }
    };
    let Ok(snapshot) = runtime.acquisition_snapshot() else {
        return apply_base_learning_like_cpp(runtime, spell);
    };
    let capacity = match runtime.profession_capacity(&plan.root_primary_profession_skill_ids) {
        Ok(capacity) => capacity,
        Err(error) if may_shallow_fallback_after_profession_plan_error_like_cpp(error) => {
            return apply_base_learning_like_cpp(runtime, spell);
        }
        Err(_) => return false,
    };
    let applied = match prepare_player_spell_acquisition_like_cpp(&plan, &capacity, &snapshot) {
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared)) => {
            apply_prepared_player_spell_acquisition_before_save_like_cpp(runtime, &prepared).is_ok()
        }
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::ActionsOnly(actions)) => {
            apply_prepared_player_spell_acquisition_actions_like_cpp(runtime, &actions).is_ok()
        }
        Ok(
            PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied
            | PreparedPlayerSpellAcquisitionOutcomeLikeCpp::NoChange,
        )
        | Err(_) => false,
    };
    applied || apply_base_learning_like_cpp(runtime, spell)
}

pub(crate) const fn may_shallow_fallback_after_profession_plan_error_like_cpp(
    error: PrimaryProfessionCapacityPlanErrorLikeCpp,
) -> bool {
    matches!(
        error,
        PrimaryProfessionCapacityPlanErrorLikeCpp::MissingSkillLineStore
            | PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot
    )
}

/// Retained bounded fallback of Player::AddSpell/LearnSpell (Player.cpp:2741,3192).
/// Validate the disabled-spell closure before its first mutation/publication.
pub(crate) fn apply_base_learning_like_cpp<R: EffectLearningRuntimeLikeCpp>(
    runtime: &mut R,
    spell: i32,
) -> bool {
    let Ok(spell_id) = u32::try_from(spell) else {
        return false;
    };
    let Some(snapshot) = runtime.fallback_snapshot() else {
        return false;
    };
    if !snapshot.rows_complete {
        return false;
    }
    let Some(chains) = runtime.fallback_chains() else {
        return false;
    };
    let previous = snapshot
        .rows
        .get(&spell)
        .or_else(|| snapshot.fallback_rows.get(&spell));
    match chains.spell_chain_lookup_like_cpp(spell_id) {
        SpellChainLookupLikeCpp::Indeterminate(_) => return false,
        SpellChainLookupLikeCpp::Node(node)
            if node.prev_spell_id.is_some()
                && previous.is_none_or(|row| {
                    matches!(
                        row.state,
                        PlayerSpellLoadState::Removed | PlayerSpellLoadState::Temporary
                    )
                }) =>
        {
            return false;
        }
        SpellChainLookupLikeCpp::Unranked | SpellChainLookupLikeCpp::Node(_) => {}
    }
    if !validate_fallback(runtime, spell, &mut BTreeSet::new()) {
        return false;
    }
    runtime.begin_action_batch();
    apply_fallback(runtime, spell, &mut BTreeSet::new())
}

fn next_spell<R: EffectLearningRuntimeLikeCpp>(runtime: &R, spell: i32) -> Option<i32> {
    u32::try_from(spell)
        .ok()
        .map(|id| {
            runtime
                .fallback_chains()
                .map_or(0, |chains| chains.next_spell_in_chain_like_cpp(id))
        })
        .filter(|id| *id != 0)
        .and_then(|id| i32::try_from(id).ok())
}

fn requiring_spells<R: EffectLearningRuntimeLikeCpp>(runtime: &R, spell: i32) -> Vec<i32> {
    u32::try_from(spell)
        .ok()
        .and_then(|id| {
            runtime.fallback_requirements().map(|store| {
                store
                    .spells_requiring_spell_like_cpp(id)
                    .iter()
                    .filter_map(|id| i32::try_from(*id).ok())
                    .collect()
            })
        })
        .unwrap_or_default()
}

fn validate_fallback<R: EffectLearningRuntimeLikeCpp>(
    runtime: &R,
    spell: i32,
    visiting: &mut BTreeSet<i32>,
) -> bool {
    if !visiting.insert(spell) {
        return true;
    }
    let valid = (|| {
        let Some(snapshot) = runtime.fallback_snapshot() else {
            return false;
        };
        let Some(previous) = snapshot.rows.get(&spell) else {
            return true;
        };
        if !snapshot.trait_definition_ids_complete || !snapshot.override_spells_complete {
            return false;
        }
        if let Some(&trait_id) = snapshot.trait_definition_ids.get(&spell) {
            let Some(definition) = u32::try_from(trait_id)
                .ok()
                .and_then(|id| runtime.fallback_traits().and_then(|store| store.get(id)))
            else {
                return false;
            };
            if definition.overrides_spell_id < 0 {
                return false;
            }
        }
        if previous.disabled {
            if runtime.fallback_chains().is_none() || runtime.fallback_requirements().is_none() {
                return false;
            }
            if let Some(next) = next_spell(runtime, spell)
                && snapshot.rows.get(&next).is_some_and(|row| row.disabled)
                && !validate_fallback(runtime, next, visiting)
            {
                return false;
            }
            for required in requiring_spells(runtime, spell) {
                if snapshot.rows.get(&required).is_some_and(|row| row.disabled)
                    && !validate_fallback(runtime, required, visiting)
                {
                    return false;
                }
            }
        }
        true
    })();
    visiting.remove(&spell);
    valid
}

fn apply_fallback<R: EffectLearningRuntimeLikeCpp>(
    runtime: &mut R,
    spell: i32,
    visiting: &mut BTreeSet<i32>,
) -> bool {
    if !visiting.insert(spell) {
        return true;
    }
    let applied = (|| {
        let Some(snapshot) = runtime.fallback_snapshot() else {
            return false;
        };
        let previous = snapshot
            .rows
            .get(&spell)
            .or_else(|| snapshot.fallback_rows.get(&spell));
        let was_disabled = previous.is_some_and(|row| row.disabled);
        if was_disabled
            && (!snapshot.rows_complete
                || runtime.fallback_chains().is_none()
                || runtime.fallback_requirements().is_none())
        {
            return false;
        }
        let next = next_spell(runtime, spell);
        let active = previous.filter(|row| row.disabled).map_or_else(
            || !next.is_some_and(|id| snapshot.known_spells.contains(&id)),
            |row| row.active,
        );
        let requires_learn = previous.map_or_else(
            || !snapshot.known_spells.contains(&spell),
            |row| {
                matches!(
                    row.state,
                    PlayerSpellLoadState::Removed | PlayerSpellLoadState::Temporary
                ) || row.disabled
                    || row.active != active
            },
        );
        let reaches_tail = previous.map_or(requires_learn, |row| {
            row.disabled
                || matches!(
                    row.state,
                    PlayerSpellLoadState::Removed | PlayerSpellLoadState::Temporary
                )
        });
        if reaches_tail {
            runtime.record_action(
                SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                    spell_id: spell as u32,
                },
            );
        }
        if requires_learn {
            let favorite = previous.is_some_and(|row| row.favorite);
            let row = PlayerKnownSpellRecord {
                spell_id: spell,
                active,
                disabled: false,
                favorite,
                dependent: previous.is_some_and(|row| row.dependent),
                state: match previous.map(|row| row.state) {
                    None | Some(PlayerSpellLoadState::Temporary | PlayerSpellLoadState::New) => {
                        PlayerSpellLoadState::New
                    }
                    Some(_) => PlayerSpellLoadState::Changed,
                },
            };
            runtime.install_fallback_row(row, &snapshot);
            if active {
                let action = SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                    spell_id: spell as u32,
                    favorite,
                    suppress_messaging: false,
                };
                runtime.record_action(action.clone());
                runtime.publish_action(&action, None);
            }
        }
        if was_disabled {
            let current = runtime.fallback_snapshot();
            let disabled = |id: &i32| {
                current
                    .as_ref()
                    .and_then(|state| state.rows.get(id).or_else(|| state.fallback_rows.get(id)))
                    .is_some_and(|row| row.disabled)
            };
            let mut dependents = Vec::new();
            if let Some(next) = next
                && disabled(&next)
            {
                dependents.push(next);
            }
            for required in requiring_spells(runtime, spell) {
                if disabled(&required) && !dependents.contains(&required) {
                    dependents.push(required);
                }
            }
            for dependent in dependents {
                if !apply_fallback(runtime, dependent, visiting) {
                    return false;
                }
            }
        } else {
            runtime.record_action(
                SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                    spell_id: spell as u32,
                },
            );
        }
        true
    })();
    visiting.remove(&spell);
    applied
}
