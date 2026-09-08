// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Validated application boundary for a deterministic acquisition plan.
//!
//! The planner is the sole semantic owner.  This module only proves that its
//! causal stream is internally coherent, pins it to the exact current player
//! authority, and translates the final snapshot into durable/runtime rows.

use super::*;
use crate::profession::{
    MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP, PrimaryProfessionCapacityPlanLikeCpp,
    PrimaryProfessionEquipmentSlotLikeCpp,
};
use wow_persistence::{
    PlayerSpellAcquisitionAuthorityLikeCpp as DurablePlayerSpellAcquisitionAuthorityLikeCpp,
    PlayerSpellAcquisitionDurableOperationLikeCpp, PlayerSpellAcquisitionPersistencePortLikeCpp,
    PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    PlayerSpellAcquisitionSkillRowLikeCpp as DurablePlayerSkillRowLikeCpp,
    PlayerSpellAcquisitionSpellRowLikeCpp as DurablePlayerSpellRowLikeCpp,
};
#[cfg(test)]
use wow_persistence::{
    PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    classify_player_spell_acquisition_money_reconciliation_like_cpp,
};

mod runtime;
pub(crate) use runtime::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedPlayerSpellAcquisitionLikeCpp {
    pub character_guid: Option<wow_core::ObjectGuid>,
    pub root: SpellAcquisitionRootLikeCpp,
    pub source_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    /// C++ post-`Player::LearnSpell` state before the ordinary
    /// `Player::SaveToDB` lifecycle consumes `_SaveSpells`/`_SaveSkills` dirty
    /// states. Generic `EffectLearnSpell` applies this snapshot immediately;
    /// database-gated consumers use `runtime_snapshot` after their commit.
    pub pending_save_runtime_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    /// C++ post-`_SaveSpells`/`_SaveSkills` in-memory state. Removed spells
    /// disappear; other non-temporary persistence states become unchanged.
    /// Deleted skill tombstones remain in the live slot map but are omitted
    /// from Character DB, matching C++ until the next login rebuild.
    pub runtime_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    pub durable_spells: Vec<DurablePlayerSpellRowLikeCpp>,
    pub durable_favorite_spell_ids: Vec<i32>,
    pub durable_skills: Vec<DurablePlayerSkillRowLikeCpp>,
    /// Skill update-field slots that C++ retains in memory after `_SaveSkills`
    /// deletes their durable rows. The represented full-save path must omit
    /// these normalized tombstones until the slot is reused.
    pub non_durable_skill_tombstone_ids: BTreeSet<u16>,
    pub durable_operations: Vec<PlayerSpellAcquisitionDurableOperationLikeCpp>,
    pub post_commit_actions: Vec<SpellAcquisitionPostCommitActionLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionPublicationFaultPointLikeCpp {
    BeforeAction(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedPlayerSpellAcquisitionOutcomeLikeCpp {
    Ready(PreparedPlayerSpellAcquisitionLikeCpp),
    ActionsOnly(PreparedPlayerSpellAcquisitionActionsLikeCpp),
    AlreadyApplied,
    NoChange,
}

pub(crate) enum PlayerSpellAcquisitionPersistenceOutcomeLikeCpp {
    Applied,
    ReconciledCommit(String),
    DefinitelyRolledBack(String),
    Indeterminate(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedPlayerSpellAcquisitionActionsLikeCpp {
    pub character_guid: Option<wow_core::ObjectGuid>,
    pub runtime_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    pub post_commit_actions: Vec<SpellAcquisitionPostCommitActionLikeCpp>,
    runtime_actions_already_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionPrepareErrorLikeCpp {
    StaleSnapshot,
    InvalidSpellId(u32),
    InvalidSkillId(u32),
    InvalidTraitDefinitionId(i32),
    InvalidProfessionAssociation(i8),
    ConflictingProfessionAssociation(u8),
    DuplicateSpell(u32),
    DuplicateSkill(u32),
    DuplicateOverride(u32, u32),
    TransitionBeforeMismatch { domain: &'static str, id: u32 },
    TransitionIdMismatch { domain: &'static str, id: u32 },
    DuplicateOverrideMutation { overridden: u32, overriding: u32 },
    MissingOverrideMutation { overridden: u32, overriding: u32 },
    TypedProjectionMismatch(&'static str),
    ResultingSnapshotMismatch,
    SnapshotIdentityChanged(&'static str),
    SkillOccupancyMismatch,
    InvalidDeletedSkill(u32),
    InvalidNonDurableSkillTombstone(u32),
    ProfessionInputsMismatch,
    ProfessionPlanMismatch(&'static str),
    InvalidPostCommitAction { domain: &'static str, id: u32 },
    PostCommitActionCausalityMismatch { action: &'static str, id: u32 },
    LearnedActionRowMismatch(u32),
    ProvenanceMismatch(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp {
    InvalidPreparedRuntime,
    PublicationInterrupted,
}

pub(crate) fn snapshot_has_pending_durable_save_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> bool {
    snapshot.spells.iter().any(|spell| {
        !matches!(
            spell.state,
            PlayerSpellPersistenceStateLikeCpp::Unchanged
                | PlayerSpellPersistenceStateLikeCpp::Temporary
        )
    }) || snapshot
        .skills
        .iter()
        .any(|skill| skill.state != PlayerSkillPersistenceStateLikeCpp::Unchanged)
}

pub(crate) fn prepare_player_spell_acquisition_like_cpp(
    plan: &SpellAcquisitionPlanLikeCpp,
    profession_plan: &PrimaryProfessionCapacityPlanLikeCpp,
    current_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<PreparedPlayerSpellAcquisitionOutcomeLikeCpp, PlayerSpellAcquisitionPrepareErrorLikeCpp>
{
    validate_plan_replay_like_cpp(plan)?;
    validate_profession_plan_like_cpp(plan, profession_plan)?;
    let prepared = translate_plan_like_cpp(plan, profession_plan)?;

    if plan.mutations.is_empty() {
        if current_snapshot != &plan.source_snapshot {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::StaleSnapshot);
        }
        return if plan.post_commit_actions.is_empty() {
            Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::NoChange)
        } else {
            Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::ActionsOnly(
                PreparedPlayerSpellAcquisitionActionsLikeCpp {
                    character_guid: plan.source_snapshot.character_guid,
                    // No durable mutation occurred, so do not normalize
                    // persistence states or replace runtime authority.
                    runtime_snapshot: plan.resulting_snapshot.clone(),
                    post_commit_actions: plan.post_commit_actions.clone(),
                    runtime_actions_already_applied: false,
                },
            ))
        };
    }
    if !plan.mutations.is_empty() && current_snapshot == &prepared.runtime_snapshot {
        return Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied);
    }
    if current_snapshot != &plan.source_snapshot {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::StaleSnapshot);
    }

    Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(
        prepared,
    ))
}

fn validate_plan_replay_like_cpp(
    plan: &SpellAcquisitionPlanLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    validate_root_like_cpp(plan.root)?;
    validate_snapshot_like_cpp(&plan.source_snapshot)?;
    validate_snapshot_like_cpp(&plan.resulting_snapshot)?;
    validate_snapshot_identity_like_cpp(&plan.source_snapshot, &plan.resulting_snapshot)?;

    let mut spells = spell_map_like_cpp(&plan.source_snapshot)?;
    let mut skills = skill_map_like_cpp(&plan.source_snapshot)?;
    let mut overrides = override_set_like_cpp(&plan.source_snapshot)?;
    let mut replayed_spells = Vec::new();
    let mut replayed_skills = Vec::new();
    let mut replayed_overrides = Vec::new();

    for mutation in &plan.mutations {
        match mutation {
            PlannedAcquisitionMutationLikeCpp::Spell(transition) => {
                validate_provenance_like_cpp(&transition.provenance, plan.root)?;
                if transition
                    .before
                    .is_some_and(|row| row.spell_id != transition.spell_id)
                    || transition
                        .after
                        .is_some_and(|row| row.spell_id != transition.spell_id)
                {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::TransitionIdMismatch {
                            domain: "spell",
                            id: transition.spell_id,
                        },
                    );
                }
                if spells.get(&transition.spell_id).copied() != transition.before {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::TransitionBeforeMismatch {
                            domain: "spell",
                            id: transition.spell_id,
                        },
                    );
                }
                if let Some(after) = transition.after {
                    spells.insert(transition.spell_id, after);
                } else {
                    spells.remove(&transition.spell_id);
                }
                replayed_spells.push(transition.clone());
            }
            PlannedAcquisitionMutationLikeCpp::Skill(transition) => {
                validate_provenance_like_cpp(&transition.provenance, plan.root)?;
                if transition.after.skill_id != transition.skill_id
                    || transition
                        .before
                        .is_some_and(|row| row.skill_id != transition.skill_id)
                {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::TransitionIdMismatch {
                            domain: "skill",
                            id: transition.skill_id,
                        },
                    );
                }
                if skills.get(&transition.skill_id).copied() != transition.before {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::TransitionBeforeMismatch {
                            domain: "skill",
                            id: transition.skill_id,
                        },
                    );
                }
                skills.insert(transition.skill_id, transition.after);
                replayed_skills.push(transition.clone());
            }
            PlannedAcquisitionMutationLikeCpp::Override(transition) => {
                let pair = (
                    transition.overridden_spell_id,
                    transition.overriding_spell_id,
                );
                if transition.add {
                    if !overrides.insert(pair) {
                        return Err(
                            PlayerSpellAcquisitionPrepareErrorLikeCpp::DuplicateOverrideMutation {
                                overridden: pair.0,
                                overriding: pair.1,
                            },
                        );
                    }
                } else if !overrides.remove(&pair) {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::MissingOverrideMutation {
                            overridden: pair.0,
                            overriding: pair.1,
                        },
                    );
                }
                replayed_overrides.push(*transition);
            }
        }
    }

    if replayed_spells != plan.spell_transitions {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::TypedProjectionMismatch("spell_transitions"),
        );
    }
    if replayed_skills != plan.skill_transitions {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::TypedProjectionMismatch("skill_transitions"),
        );
    }
    if replayed_overrides != plan.override_transitions {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::TypedProjectionMismatch(
                "override_transitions",
            ),
        );
    }

    let resulting_spells = spell_map_like_cpp(&plan.resulting_snapshot)?;
    let resulting_skills = skill_map_like_cpp(&plan.resulting_snapshot)?;
    let resulting_overrides = override_set_like_cpp(&plan.resulting_snapshot)?;
    if spells != resulting_spells || skills != resulting_skills || overrides != resulting_overrides
    {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::ResultingSnapshotMismatch);
    }

    // Action causality is checked only after the complete mutation stream has
    // replayed successfully. It may then consume transition evidence by
    // occurrence without trusting malformed typed projections.
    validate_post_commit_actions_like_cpp(plan)?;

    let mut profession_inputs = plan.profession_association_inputs.clone();
    profession_inputs.sort_by_key(|skill| skill.skill_id);
    let mut resulting_skill_rows = plan.resulting_snapshot.skills.clone();
    resulting_skill_rows.sort_by_key(|skill| skill.skill_id);
    if profession_inputs != resulting_skill_rows {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionInputsMismatch);
    }
    Ok(())
}

fn validate_root_like_cpp(
    root: SpellAcquisitionRootLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let spell_id = match root {
        SpellAcquisitionRootLikeCpp::DirectLearn(spell_id)
        | SpellAcquisitionRootLikeCpp::TrainerWrapperCast(spell_id) => spell_id,
    };
    if spell_id == 0 || i32::try_from(spell_id).is_err() {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(
            spell_id,
        ));
    }
    Ok(())
}

fn validate_profession_plan_like_cpp(
    acquisition_plan: &SpellAcquisitionPlanLikeCpp,
    profession_plan: &PrimaryProfessionCapacityPlanLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    if profession_plan.configured_max > MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP
        || profession_plan.used_before != profession_plan.existing_professions.len()
        || profession_plan.free_before
            != usize::from(profession_plan.configured_max)
                .saturating_sub(profession_plan.used_before)
        || profession_plan.new_professions.len() > profession_plan.free_before
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "capacity arithmetic",
            ),
        );
    }

    let new_ids = profession_plan
        .new_professions
        .iter()
        .map(|profession| profession.skill_id)
        .collect::<Vec<_>>();
    if new_ids != acquisition_plan.root_primary_profession_skill_ids {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "new profession order",
            ),
        );
    }

    let source_skills = skill_map_like_cpp(&acquisition_plan.source_snapshot)?;
    let resulting_skills = skill_map_like_cpp(&acquisition_plan.resulting_snapshot)?;
    let expected_existing_ids = acquisition_plan
        .source_snapshot
        .primary_profession_skill_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let expected_resulting_primary_ids = expected_existing_ids
        .iter()
        .copied()
        .chain(
            acquisition_plan
                .root_primary_profession_skill_ids
                .iter()
                .copied(),
        )
        .collect::<BTreeSet<_>>();
    if acquisition_plan
        .resulting_snapshot
        .primary_profession_skill_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        != expected_resulting_primary_ids
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "resulting primary profession authority",
            ),
        );
    }
    let actual_existing_ids = profession_plan
        .existing_professions
        .iter()
        .map(|profession| profession.skill_id)
        .collect::<BTreeSet<_>>();
    if actual_existing_ids != expected_existing_ids
        || actual_existing_ids.len() != profession_plan.existing_professions.len()
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "complete existing profession membership",
            ),
        );
    }
    let mut assigned_skill_ids = BTreeSet::new();
    let mut assigned_slots = BTreeSet::new();
    for (profession, existing) in profession_plan
        .existing_professions
        .iter()
        .map(|profession| (profession, true))
        .chain(
            profession_plan
                .new_professions
                .iter()
                .map(|profession| (profession, false)),
        )
    {
        if !assigned_skill_ids.insert(profession.skill_id)
            || !resulting_skills
                .get(&profession.skill_id)
                .is_some_and(|skill| skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted)
            || (existing
                && !source_skills
                    .get(&profession.skill_id)
                    .is_some_and(|skill| skill.value != 0))
            || profession
                .equipment_slot
                .is_some_and(|slot| !assigned_slots.insert(slot))
        {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                    "profession membership or slot assignment",
                ),
            );
        }
    }

    let mut normalized_skill_ids = BTreeSet::new();
    for normalization in &profession_plan.slot_normalizations {
        let Some(source) = source_skills.get(&normalization.skill_id) else {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                    "normalization source skill",
                ),
            );
        };
        let assigned_slot = profession_plan
            .existing_professions
            .iter()
            .chain(&profession_plan.new_professions)
            .find(|profession| profession.skill_id == normalization.skill_id)
            .map(|profession| profession.equipment_slot);
        if !normalized_skill_ids.insert(normalization.skill_id)
            || source.profession_association.database_value_like_cpp()
                != normalization.original_slot
            || !resulting_skills.contains_key(&normalization.skill_id)
            || match assigned_slot {
                Some(slot) => slot != normalization.normalized_slot,
                None => normalization.normalized_slot.is_some(),
            }
        {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                    "slot normalization",
                ),
            );
        }
    }
    Ok(())
}

fn validate_provenance_like_cpp(
    provenance: &SpellAcquisitionProvenanceLikeCpp,
    root: SpellAcquisitionRootLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let valid_spell = |spell_id: u32| spell_id != 0 && i32::try_from(spell_id).is_ok();
    let valid_skill = |skill_id: u32| skill_id != 0 && u16::try_from(skill_id).is_ok();
    let valid = match provenance {
        SpellAcquisitionProvenanceLikeCpp::Root {
            root: provenance_root,
        } => *provenance_root == root,
        SpellAcquisitionProvenanceLikeCpp::PreviousRank { requested_spell_id } => {
            valid_spell(*requested_spell_id)
        }
        SpellAcquisitionProvenanceLikeCpp::LearnDependency { source_spell_id }
        | SpellAcquisitionProvenanceLikeCpp::HigherDisabledRank { source_spell_id }
        | SpellAcquisitionProvenanceLikeCpp::DirectLearnSkill { source_spell_id } => {
            valid_spell(*source_spell_id)
        }
        SpellAcquisitionProvenanceLikeCpp::RequiredDisabledSpell { required_spell_id } => {
            valid_spell(*required_spell_id)
        }
        SpellAcquisitionProvenanceLikeCpp::SkillLineAbilityFallback {
            source_spell_id,
            record_id,
        } => valid_spell(*source_spell_id) && *record_id != 0,
        SpellAcquisitionProvenanceLikeCpp::ParentSkill { child_skill_id } => {
            valid_skill(*child_skill_id)
        }
        SpellAcquisitionProvenanceLikeCpp::RootChildSkill { parent_skill_id } => {
            valid_skill(*parent_skill_id)
        }
        SpellAcquisitionProvenanceLikeCpp::SkillReward {
            skill_id,
            record_id,
        } => valid_skill(*skill_id) && *record_id != 0,
        SpellAcquisitionProvenanceLikeCpp::WrapperEffect {
            wrapper_spell_id,
            record_id,
            ..
        } => valid_spell(*wrapper_spell_id) && *record_id != 0,
        SpellAcquisitionProvenanceLikeCpp::AutocastEffect {
            source_spell_id,
            record_id,
            ..
        } => valid_spell(*source_spell_id) && *record_id != 0,
    };
    if !valid {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProvenanceMismatch(
                "invalid or root-mismatched transition provenance",
            ),
        );
    }
    Ok(())
}

fn validate_post_commit_actions_like_cpp(
    plan: &SpellAcquisitionPlanLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let actual_publication_requirements = plan
        .post_commit_actions
        .iter()
        .filter_map(|action| match action {
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id,
                favorite,
                suppress_messaging,
            } => Some(SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                spell_id: *spell_id,
                favorite: *favorite,
                suppress_messaging: *suppress_messaging,
            }),
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id,
            } => Some(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective {
                    spell_id: *spell_id,
                },
            ),
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id,
                skill_id,
            } => Some(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                    source_spell_id: *source_spell_id,
                    skill_id: *skill_id,
                },
            ),
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                source_spell_id,
                skill_id,
            } => Some(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                    source_spell_id: *source_spell_id,
                    skill_id: *skill_id,
                },
            ),
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id,
            } => Some(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnOrKnowSpellCriteria {
                    spell_id: *spell_id,
                },
            ),
            _ => None,
        })
        .collect::<Vec<_>>();
    if actual_publication_requirements != plan.publication_requirements {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
                action: "required publication tape",
                id: 0,
            },
        );
    }
    let resulting_spells = spell_map_like_cpp(&plan.resulting_snapshot)?;
    let resulting_skills = skill_map_like_cpp(&plan.resulting_snapshot)?;
    let mut learned_evidence = BTreeMap::<(u32, bool), usize>::new();
    let mut refresh_evidence = BTreeMap::<u32, usize>::new();
    let mut spell_transition_evidence = BTreeMap::<u32, usize>::new();
    let mut skill_transition_evidence = BTreeMap::<u32, usize>::new();
    let mut deactivation_evidence = Vec::<(usize, u32)>::new();
    let mut supersede_evidence = BTreeMap::<(u32, u32), Vec<usize>>::new();
    let mut replayed_spells = spell_map_like_cpp(&plan.source_snapshot)?;
    let mut prior_activations = Vec::<(usize, u32, SpellAcquisitionProvenanceLikeCpp)>::new();
    let mut superseding_activations = BTreeSet::<usize>::new();
    for (transition_index, transition) in plan.spell_transitions.iter().enumerate() {
        *spell_transition_evidence
            .entry(transition.spell_id)
            .or_default() += 1;
        let before_active = transition.before.is_some_and(|before| {
            before.state != PlayerSpellPersistenceStateLikeCpp::Removed
                && before.active
                && !before.disabled
        });
        let after_active = transition.after.is_some_and(|after| {
            after.state != PlayerSpellPersistenceStateLikeCpp::Removed
                && after.active
                && !after.disabled
        });
        if let Some(after) = transition.after {
            replayed_spells.insert(transition.spell_id, after);
        } else {
            replayed_spells.remove(&transition.spell_id);
        }
        if !before_active && after_active {
            let favorite = transition.after.is_some_and(|after| after.favorite);
            *learned_evidence
                .entry((transition.spell_id, favorite))
                .or_default() += 1;
            *refresh_evidence.entry(transition.spell_id).or_default() += 1;
            prior_activations.push((
                transition_index,
                transition.spell_id,
                transition.provenance.clone(),
            ));
        }
        if before_active && !after_active {
            deactivation_evidence.push((transition_index, transition.spell_id));
            for (candidate_index, candidate_spell_id, candidate_provenance) in &prior_activations {
                if *candidate_spell_id != transition.spell_id
                    && *candidate_provenance == transition.provenance
                    && replayed_spells.get(candidate_spell_id).is_some_and(|row| {
                        row.state != PlayerSpellPersistenceStateLikeCpp::Removed
                            && row.active
                            && !row.disabled
                    })
                {
                    superseding_activations.insert(*candidate_index);
                    supersede_evidence
                        .entry((transition.spell_id, *candidate_spell_id))
                        .or_default()
                        .push(transition_index);
                }
            }
        }
    }
    // C++ AddSpell activates the new rank before it discovers and publishes
    // the supersede relationship. That activation is causal evidence for the
    // SupersededSpell packet, not for an additional LearnedSpell packet:
    // AddSpell returns false to LearnSpell when it replaced an active rank.
    for transition_index in superseding_activations {
        let transition = &plan.spell_transitions[transition_index];
        let favorite = transition.after.is_some_and(|after| after.favorite);
        let _ = consume_action_evidence_like_cpp(
            &mut learned_evidence,
            (transition.spell_id, favorite),
        );
    }
    for transition in &plan.skill_transitions {
        *skill_transition_evidence
            .entry(transition.skill_id)
            .or_default() += 1;
    }
    let mut mount_skill_evidence = skill_transition_evidence.clone();
    let mut raised_skill_evidence = skill_transition_evidence.clone();
    let mut achieve_skill_evidence = skill_transition_evidence.clone();
    let mut consumed_deactivations = BTreeSet::<usize>::new();
    let resulting_spell_is_present = |spell_id| {
        resulting_spells
            .get(&spell_id)
            .is_some_and(|spell| spell.state != PlayerSpellPersistenceStateLikeCpp::Removed)
    };
    let resulting_spell_is_inactive = |spell_id| {
        !resulting_spells.get(&spell_id).is_some_and(|spell| {
            spell.state != PlayerSpellPersistenceStateLikeCpp::Removed
                && spell.active
                && !spell.disabled
        })
    };
    let mut dual_wield_effect_evidence = BTreeMap::<(u32, u32, u8), usize>::new();
    for (&spell_id, resolution) in &plan.source_snapshot.cast_resolutions {
        if !resolution.reached_immediate_phase {
            continue;
        }
        for effect in &resolution.executed_dual_wield_effects {
            *dual_wield_effect_evidence
                .entry((spell_id, effect.effect_record_id, effect.effect_index))
                .or_default() += 1;
        }
    }

    for (action_index, action) in plan.post_commit_actions.iter().enumerate() {
        let mismatch = |action, id| {
            Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
                    action,
                    id,
                },
            )
        };
        match action {
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id, favorite, ..
            } => {
                let row_matches = resulting_spells
                    .get(spell_id)
                    .is_some_and(|row| row.favorite == *favorite);
                if !row_matches
                    || !consume_action_evidence_like_cpp(
                        &mut learned_evidence,
                        (*spell_id, *favorite),
                    )
                {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::LearnedActionRowMismatch(
                            *spell_id,
                        ),
                    );
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                old_spell_id,
                new_spell_id,
            } => {
                for spell_id in [*old_spell_id, *new_spell_id] {
                    if spell_id == 0 || i32::try_from(spell_id).is_err() {
                        return Err(
                            PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidPostCommitAction {
                                domain: "spell",
                                id: spell_id,
                            },
                        );
                    }
                }
                let evidence_index = supersede_evidence
                    .get_mut(&(*old_spell_id, *new_spell_id))
                    .and_then(|indices| {
                        indices
                            .iter()
                            .copied()
                            .find(|index| !consumed_deactivations.contains(index))
                    });
                if old_spell_id == new_spell_id || evidence_index.is_none() {
                    return mismatch("SupersededSpell", *old_spell_id);
                }
                consumed_deactivations.insert(evidence_index.expect("checked above"));
            }
            SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id } => {
                validate_post_commit_id_like_cpp("spell", *spell_id, false)?;
                let evidence_index =
                    deactivation_evidence
                        .iter()
                        .find_map(|(index, evidence_spell_id)| {
                            (*evidence_spell_id == *spell_id
                                && !consumed_deactivations.contains(index))
                            .then_some(*index)
                        });
                if evidence_index.is_none() || !resulting_spell_is_inactive(*spell_id) {
                    return mismatch("UnlearnedSpell", *spell_id);
                }
                consumed_deactivations.insert(evidence_index.expect("checked above"));
            }
            SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id } => {
                validate_post_commit_id_like_cpp("spell", *spell_id, false)?;
                if !consume_action_evidence_like_cpp(&mut refresh_evidence, *spell_id)
                    || !resulting_spell_is_present(*spell_id)
                {
                    return mismatch("RefreshPassive", *spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id,
            } => {
                validate_post_commit_id_like_cpp("spell", *spell_id, false)?;
                if !resulting_spell_is_present(*spell_id) {
                    return mismatch("UpdateLearnSpellQuestObjective", *spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id,
            } => {
                validate_post_commit_id_like_cpp("spell", *spell_id, false)?;
                if !resulting_spell_is_present(*spell_id) {
                    return mismatch("UpdateLearnOrKnowSpellCriteria", *spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
                source_spell_id,
                effect_record_id,
                effect_index,
            } => {
                validate_post_commit_id_like_cpp("spell", *source_spell_id, false)?;
                let evidence_count = dual_wield_effect_evidence.get_mut(&(
                    *source_spell_id,
                    *effect_record_id,
                    *effect_index,
                ));
                if *effect_record_id == 0
                    || !evidence_count.is_some_and(|count| {
                        if *count == 0 {
                            false
                        } else {
                            *count -= 1;
                            true
                        }
                    })
                {
                    return mismatch("GrantDualWield", *source_spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id,
                skill_id,
            } => {
                validate_post_commit_id_like_cpp("spell", *source_spell_id, false)?;
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                let paired = plan.post_commit_actions.get(action_index + 1).is_some_and(
                    |next| {
                        matches!(
                            next,
                            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                                source_spell_id: next_source_spell_id,
                                skill_id: next_skill_id,
                            } if next_source_spell_id == source_spell_id && next_skill_id == skill_id
                        )
                    },
                );
                if !paired
                    || !spell_transition_evidence.contains_key(source_spell_id)
                    || !resulting_spell_is_present(*source_spell_id)
                {
                    return mismatch("tradeskill skill-line criteria", *source_spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                source_spell_id,
                skill_id,
            } => {
                validate_post_commit_id_like_cpp("spell", *source_spell_id, false)?;
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                let paired = action_index.checked_sub(1).is_some_and(|previous_index| {
                    matches!(
                        &plan.post_commit_actions[previous_index],
                        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                            source_spell_id: previous_source_spell_id,
                            skill_id: previous_skill_id,
                        } if previous_source_spell_id == source_spell_id && previous_skill_id == skill_id
                    )
                });
                let continues_same_spell_block = plan
                    .post_commit_actions
                    .get(action_index + 1)
                    .is_some_and(|next| {
                        matches!(
                            next,
                            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                                source_spell_id: next_source_spell_id,
                                ..
                            } | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                                spell_id: next_source_spell_id,
                            } if next_source_spell_id == source_spell_id
                        )
                    });
                if !paired
                    || !continues_same_spell_block
                    || !spell_transition_evidence.contains_key(source_spell_id)
                    || !resulting_spell_is_present(*source_spell_id)
                {
                    return mismatch("learn-spell skill-line criteria", *source_spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { skill_id } => {
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                if *skill_id != u32::from(SKILL_RIDING_LIKE_CPP)
                    || !consume_action_evidence_like_cpp(&mut mount_skill_evidence, *skill_id)
                    || !resulting_skills.get(skill_id).is_some_and(|skill| {
                        skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                    })
                {
                    return mismatch("UpdateMountCapability", *skill_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id } => {
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                if !consume_action_evidence_like_cpp(&mut raised_skill_evidence, *skill_id)
                    || !resulting_skills.get(skill_id).is_some_and(|skill| {
                        skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                    })
                {
                    return mismatch("skill action", *skill_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria {
                skill_id,
            } => {
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                if !consume_action_evidence_like_cpp(&mut achieve_skill_evidence, *skill_id)
                    || !resulting_skills.get(skill_id).is_some_and(|skill| {
                        skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                    })
                {
                    return mismatch("skill action", *skill_id);
                }
            }
        }
    }
    if let Some((&(spell_id, _, _), _)) = dual_wield_effect_evidence
        .iter()
        .find(|(_, count)| **count != 0)
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
                action: "DualWieldEffectProjected",
                id: spell_id,
            },
        );
    }
    Ok(())
}

fn consume_action_evidence_like_cpp<K: Ord>(evidence: &mut BTreeMap<K, usize>, key: K) -> bool {
    evidence.get_mut(&key).is_some_and(|count| {
        if *count == 0 {
            false
        } else {
            *count -= 1;
            true
        }
    })
}

fn validate_post_commit_id_like_cpp(
    domain: &'static str,
    id: u32,
    u16_required: bool,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    if id == 0 || i32::try_from(id).is_err() || (u16_required && u16::try_from(id).is_err()) {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidPostCommitAction { domain, id },
        );
    }
    Ok(())
}

fn validate_snapshot_identity_like_cpp(
    source: &PlayerSpellAcquisitionSnapshotLikeCpp,
    resulting: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    for (same, field) in [
        (
            source.character_guid == resulting.character_guid,
            "character_guid",
        ),
        (source.race == resulting.race, "race"),
        (source.class == resulting.class, "class"),
        (source.level == resulting.level, "level"),
        (source.lifecycle == resulting.lifecycle, "lifecycle"),
        (
            source.future_player_condition_resolutions
                == resulting.future_player_condition_resolutions,
            "future_player_condition_resolutions",
        ),
        (
            source.cast_resolutions == resulting.cast_resolutions,
            "cast_resolutions",
        ),
    ] {
        if !same {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::SnapshotIdentityChanged(field));
        }
    }
    Ok(())
}

fn validate_snapshot_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    if snapshot
        .character_guid
        .is_some_and(|guid| !guid.is_player() || guid.counter() == 0)
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::SnapshotIdentityChanged("character_guid"),
        );
    }
    let spells = spell_map_like_cpp(snapshot)?;
    let skills = skill_map_like_cpp(snapshot)?;
    let _ = override_set_like_cpp(snapshot)?;
    if snapshot
        .primary_profession_skill_ids
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
        || snapshot
            .primary_profession_skill_ids
            .iter()
            .any(|skill_id| {
                !skills.get(skill_id).is_some_and(|skill| {
                    skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted && skill.value != 0
                })
            })
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "snapshot primary profession authority",
            ),
        );
    }
    if let Some(invalid_skill_id) = snapshot
        .non_durable_skill_tombstone_ids
        .windows(2)
        .find_map(|pair| (pair[0] >= pair[1]).then_some(pair[1]))
        .or_else(|| {
            snapshot
                .non_durable_skill_tombstone_ids
                .iter()
                .copied()
                .find(|skill_id| {
                    !skills.get(skill_id).is_some_and(|skill| {
                        skill.step == 0
                            && skill.value == 0
                            && skill.maximum == 0
                            && skill.profession_association
                                == ProfessionAssociationInputLikeCpp::Unassigned
                            && matches!(
                                skill.state,
                                PlayerSkillPersistenceStateLikeCpp::Unchanged
                                    | PlayerSkillPersistenceStateLikeCpp::Deleted
                            )
                    })
                })
        })
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidNonDurableSkillTombstone(
                invalid_skill_id,
            ),
        );
    }
    if usize::from(snapshot.occupied_skill_slots) != skills.len()
        || snapshot.occupied_skill_slots > 256
    {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::SkillOccupancyMismatch);
    }

    let mut profession_slots = BTreeMap::<u8, u32>::new();
    for skill in skills.values() {
        if skill.state == PlayerSkillPersistenceStateLikeCpp::Deleted
            && (skill.step != 0
                || skill.value != 0
                || skill.maximum != 0
                || skill.profession_association != ProfessionAssociationInputLikeCpp::Unassigned)
        {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidDeletedSkill(skill.skill_id),
            );
        }
        match skill.profession_association {
            ProfessionAssociationInputLikeCpp::Unassigned => {}
            ProfessionAssociationInputLikeCpp::Slot(slot @ 0..=1) => {
                if profession_slots.insert(slot, skill.skill_id).is_some() {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::ConflictingProfessionAssociation(
                            slot,
                        ),
                    );
                }
            }
            ProfessionAssociationInputLikeCpp::Slot(slot) => {
                return Err(
                    PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidProfessionAssociation(
                        slot as i8,
                    ),
                );
            }
            ProfessionAssociationInputLikeCpp::Invalid(value) => {
                return Err(
                    PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidProfessionAssociation(value),
                );
            }
        }
    }
    for spell in spells.values() {
        if let Some(trait_definition_id) = spell.trait_definition_id
            && trait_definition_id <= 0
        {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidTraitDefinitionId(
                    trait_definition_id,
                ),
            );
        }
    }
    Ok(())
}

fn spell_map_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<
    BTreeMap<u32, PlayerSpellAcquisitionRowLikeCpp>,
    PlayerSpellAcquisitionPrepareErrorLikeCpp,
> {
    let mut rows = BTreeMap::new();
    for row in &snapshot.spells {
        if i32::try_from(row.spell_id).is_err() || row.spell_id == 0 {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(
                row.spell_id,
            ));
        }
        if rows.insert(row.spell_id, *row).is_some() {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::DuplicateSpell(
                row.spell_id,
            ));
        }
    }
    Ok(rows)
}

fn skill_map_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<
    BTreeMap<u32, PlayerSkillAcquisitionRowLikeCpp>,
    PlayerSpellAcquisitionPrepareErrorLikeCpp,
> {
    let mut rows = BTreeMap::new();
    for row in &snapshot.skills {
        if u16::try_from(row.skill_id).is_err() || row.skill_id == 0 {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSkillId(
                row.skill_id,
            ));
        }
        if rows.insert(row.skill_id, *row).is_some() {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::DuplicateSkill(
                row.skill_id,
            ));
        }
    }
    Ok(rows)
}

fn override_set_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<BTreeSet<(u32, u32)>, PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let mut overrides = BTreeSet::new();
    for &(overridden, overriding) in &snapshot.overrides {
        if i32::try_from(overridden).is_err() || overridden == 0 {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(
                overridden,
            ));
        }
        if i32::try_from(overriding).is_err() || overriding == 0 {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(
                overriding,
            ));
        }
        if !overrides.insert((overridden, overriding)) {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::DuplicateOverride(
                    overridden, overriding,
                ),
            );
        }
    }
    Ok(overrides)
}

fn translate_plan_like_cpp(
    plan: &SpellAcquisitionPlanLikeCpp,
    profession_plan: &PrimaryProfessionCapacityPlanLikeCpp,
) -> Result<PreparedPlayerSpellAcquisitionLikeCpp, PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let mut runtime_snapshot = plan.resulting_snapshot.clone();
    for normalization in &profession_plan.slot_normalizations {
        let skill = runtime_snapshot
            .skills
            .iter_mut()
            .find(|skill| skill.skill_id == normalization.skill_id)
            .expect("validated profession normalization skill");
        skill.profession_association =
            profession_association_like_cpp(normalization.normalized_slot);
    }
    for profession in profession_plan
        .existing_professions
        .iter()
        .chain(&profession_plan.new_professions)
    {
        let skill = runtime_snapshot
            .skills
            .iter_mut()
            .find(|skill| skill.skill_id == profession.skill_id)
            .expect("validated profession assignment skill");
        skill.profession_association = profession_association_like_cpp(profession.equipment_slot);
    }
    let pending_save_runtime_snapshot = runtime_snapshot.clone();
    let mut durable_spells = Vec::new();
    let mut durable_favorite_spell_ids = Vec::new();
    for spell in &plan.resulting_snapshot.spells {
        if spell.state == PlayerSpellPersistenceStateLikeCpp::Removed
            || spell.state == PlayerSpellPersistenceStateLikeCpp::Temporary
        {
            continue;
        }
        // C++ `_SaveSpells` suppresses the `character_spell` insert for a
        // dependent row, but favorite maintenance remains outside that
        // dependent check and therefore still persists independently.
        if spell.favorite {
            durable_favorite_spell_ids.push(i32::try_from(spell.spell_id).map_err(|_| {
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(spell.spell_id)
            })?);
        }
        if spell.dependent {
            continue;
        }
        durable_spells.push(DurablePlayerSpellRowLikeCpp {
            spell_id: i32::try_from(spell.spell_id).map_err(|_| {
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(spell.spell_id)
            })?,
            active: spell.active,
            disabled: spell.disabled,
        });
    }
    durable_spells.sort_by_key(|spell| spell.spell_id);
    durable_favorite_spell_ids.sort_unstable();

    let mut durable_skills = Vec::new();
    let source_non_durable_skill_tombstone_ids = plan
        .source_snapshot
        .non_durable_skill_tombstone_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut non_durable_skill_tombstone_ids = BTreeSet::new();
    for skill in &runtime_snapshot.skills {
        let remains_saved_tombstone = source_non_durable_skill_tombstone_ids
            .contains(&skill.skill_id)
            && skill.step == 0
            && skill.value == 0
            && skill.maximum == 0
            && skill.profession_association == ProfessionAssociationInputLikeCpp::Unassigned
            && matches!(
                skill.state,
                PlayerSkillPersistenceStateLikeCpp::Unchanged
                    | PlayerSkillPersistenceStateLikeCpp::Deleted
            );
        if skill.state == PlayerSkillPersistenceStateLikeCpp::Deleted || remains_saved_tombstone {
            non_durable_skill_tombstone_ids.insert(u16::try_from(skill.skill_id).map_err(
                |_| PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSkillId(skill.skill_id),
            )?);
            continue;
        }
        durable_skills.push(DurablePlayerSkillRowLikeCpp {
            skill_id: u16::try_from(skill.skill_id).map_err(|_| {
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSkillId(skill.skill_id)
            })?,
            value: skill.value,
            maximum: skill.maximum,
            profession_slot: skill.profession_association.database_value_like_cpp(),
        });
    }
    durable_skills.sort_by_key(|skill| skill.skill_id);
    runtime_snapshot.non_durable_skill_tombstone_ids = non_durable_skill_tombstone_ids
        .iter()
        .map(|skill_id| u32::from(*skill_id))
        .collect();

    runtime_snapshot
        .spells
        .retain(|spell| spell.state != PlayerSpellPersistenceStateLikeCpp::Removed);
    for spell in &mut runtime_snapshot.spells {
        if spell.state != PlayerSpellPersistenceStateLikeCpp::Temporary {
            spell.state = PlayerSpellPersistenceStateLikeCpp::Unchanged;
        }
    }
    for skill in &mut runtime_snapshot.skills {
        skill.state = PlayerSkillPersistenceStateLikeCpp::Unchanged;
    }

    let mut durable_operations = vec![
        PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
    ];
    durable_operations.extend(
        durable_spells
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell),
    );
    durable_operations.extend(
        durable_favorite_spell_ids
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell),
    );
    durable_operations.extend(
        durable_skills
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill),
    );

    Ok(PreparedPlayerSpellAcquisitionLikeCpp {
        character_guid: plan.source_snapshot.character_guid,
        root: plan.root,
        source_snapshot: plan.source_snapshot.clone(),
        pending_save_runtime_snapshot,
        runtime_snapshot,
        durable_spells,
        durable_favorite_spell_ids,
        durable_skills,
        non_durable_skill_tombstone_ids,
        durable_operations,
        post_commit_actions: plan.post_commit_actions.clone(),
    })
}

fn profession_association_like_cpp(
    slot: Option<PrimaryProfessionEquipmentSlotLikeCpp>,
) -> ProfessionAssociationInputLikeCpp {
    slot.map(|slot| ProfessionAssociationInputLikeCpp::Slot(slot.db_value_like_cpp() as u8))
        .unwrap_or(ProfessionAssociationInputLikeCpp::Unassigned)
}

/// Project the exact Character DB rows represented by a clean runtime source.
/// Temporary spells have no durable row; any other dirty state is ambiguous
/// until the ordinary C++ save lifecycle consumes it, so trainer persistence
/// must fail closed instead of guessing the database pre-state.
fn stable_source_durable_authority_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Option<DurablePlayerSpellAcquisitionAuthorityLikeCpp> {
    let mut spells = Vec::new();
    let mut favorite_spell_ids = Vec::new();
    for spell in &snapshot.spells {
        if spell.state == PlayerSpellPersistenceStateLikeCpp::Temporary {
            continue;
        }
        if spell.state != PlayerSpellPersistenceStateLikeCpp::Unchanged {
            return None;
        }
        let spell_id = i32::try_from(spell.spell_id).ok()?;
        if spell.favorite {
            favorite_spell_ids.push(spell_id);
        }
        if !spell.dependent {
            spells.push(DurablePlayerSpellRowLikeCpp {
                spell_id,
                active: spell.active,
                disabled: spell.disabled,
            });
        }
    }
    spells.sort_by_key(|spell| spell.spell_id);
    favorite_spell_ids.sort_unstable();

    let non_durable_tombstones = snapshot
        .non_durable_skill_tombstone_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut skills = Vec::new();
    for skill in &snapshot.skills {
        if skill.state != PlayerSkillPersistenceStateLikeCpp::Unchanged {
            return None;
        }
        let skill_id = u16::try_from(skill.skill_id).ok()?;
        if non_durable_tombstones.contains(&skill.skill_id) {
            continue;
        }
        skills.push(DurablePlayerSkillRowLikeCpp {
            skill_id,
            value: skill.value,
            maximum: skill.maximum,
            profession_slot: skill.profession_association.database_value_like_cpp(),
        });
    }
    skills.sort_by_key(|skill| skill.skill_id);

    Some(DurablePlayerSpellAcquisitionAuthorityLikeCpp {
        spells,
        favorite_spell_ids,
        skills,
    })
}

/// Converts an already validated application plan into the complete SQLx-free
/// transaction request consumed by the Character-database adapter.
pub(crate) fn player_spell_acquisition_persistence_request_like_cpp(
    guid_counter: u64,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
    money_before: u64,
    money_after: u64,
    operation_token: [u8; 16],
) -> Result<PlayerSpellAcquisitionPersistenceRequestLikeCpp, String> {
    let prepared_counter = prepared
        .character_guid
        .and_then(|guid| u64::try_from(guid.counter()).ok());
    if prepared_counter != Some(guid_counter) {
        return Err("prepared player spell acquisition character GUID mismatch".to_owned());
    }
    let source_authority = stable_source_durable_authority_like_cpp(&prepared.source_snapshot)
        .ok_or_else(|| {
            "trainer acquisition source contains unsaved persistence state".to_owned()
        })?;
    Ok(PlayerSpellAcquisitionPersistenceRequestLikeCpp {
        player_guid: guid_counter,
        money_before,
        money_after,
        operation_token,
        source_authority,
        resulting_authority: DurablePlayerSpellAcquisitionAuthorityLikeCpp {
            spells: prepared.durable_spells.clone(),
            favorite_spell_ids: prepared.durable_favorite_spell_ids.clone(),
            skills: prepared.durable_skills.clone(),
        },
        operations: prepared.durable_operations.clone(),
    })
}

pub(crate) async fn persist_player_spell_acquisition_through_port_like_cpp(
    port: &dyn PlayerSpellAcquisitionPersistencePortLikeCpp,
    request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
) -> PlayerSpellAcquisitionPersistenceOutcomeLikeCpp {
    use wow_persistence::{
        PlayerSpellAcquisitionMoneyReconciliationLikeCpp as Reconciliation,
        PlayerSpellAcquisitionPersistenceAttemptLikeCpp as Attempt,
    };

    match port
        .attempt_player_spell_acquisition_like_cpp(request.clone())
        .await
    {
        Attempt::Applied => PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::Applied,
        Attempt::DefinitelyRolledBack { reason, .. } => {
            PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::DefinitelyRolledBack(reason)
        }
        Attempt::CommitOutcomeUnknown { reason } => match port
            .reconcile_player_spell_acquisition_like_cpp(request)
            .await
        {
            Reconciliation::Committed => {
                PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::ReconciledCommit(reason)
            }
            Reconciliation::Indeterminate => {
                PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::Indeterminate(reason)
            }
        },
    }
}

#[cfg(test)]
mod tests;
