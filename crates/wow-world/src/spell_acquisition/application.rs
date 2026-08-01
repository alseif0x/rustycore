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
use sqlx::MySql;
use sqlx::Transaction;
use wow_database::{
    CharacterDatabase, DatabaseError, SqlTransactionCommitError, is_database_deadlock_like_cpp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DurablePlayerSpellRowLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DurablePlayerSkillRowLikeCpp {
    pub skill_id: u16,
    pub value: u16,
    pub maximum: u16,
    pub profession_slot: i8,
}

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
pub(crate) enum PlayerSpellAcquisitionDurableOperationLikeCpp {
    LockCharacter,
    DeleteSpells,
    DeleteFavoriteSpells,
    DeleteSkills,
    InsertSpell(DurablePlayerSpellRowLikeCpp),
    InsertFavoriteSpell(i32),
    InsertSkill(DurablePlayerSkillRowLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionPersistenceFaultPointLikeCpp {
    BeforeOperation(usize),
    BeforeCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionPublicationFaultPointLikeCpp {
    BeforeAction(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerSpellAcquisitionReconciliationLikeCpp {
    Committed,
    NotCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedPlayerSpellAcquisitionOutcomeLikeCpp {
    Ready(PreparedPlayerSpellAcquisitionLikeCpp),
    ActionsOnly(PreparedPlayerSpellAcquisitionActionsLikeCpp),
    AlreadyApplied,
    NoChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedPlayerSpellAcquisitionActionsLikeCpp {
    pub character_guid: Option<wow_core::ObjectGuid>,
    pub runtime_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp,
    pub post_commit_actions: Vec<SpellAcquisitionPostCommitActionLikeCpp>,
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
                if *skill_id != u32::from(crate::session::SKILL_RIDING_LIKE_CPP)
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
    let mut non_durable_skill_tombstone_ids = BTreeSet::new();
    for skill in &runtime_snapshot.skills {
        if skill.state == PlayerSkillPersistenceStateLikeCpp::Deleted {
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

pub(crate) async fn persist_prepared_player_spell_acquisition_like_cpp(
    character_db: &CharacterDatabase,
    guid_counter: u64,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<(), SqlTransactionCommitError> {
    validate_prepared_character_counter_like_cpp(guid_counter, prepared)
        .map_err(SqlTransactionCommitError::DefinitelyRolledBack)?;
    persist_prepared_player_spell_acquisition_with_fault_like_cpp(
        character_db,
        guid_counter,
        prepared,
        |_| Ok(()),
    )
    .await
}

/// Re-reads the complete durable authority after a lost COMMIT response.
/// Exact equality proves that publishing the prepared result is safe; any
/// other complete state is treated as not committed and is never guessed.
pub(crate) async fn reconcile_prepared_player_spell_acquisition_like_cpp(
    character_db: &CharacterDatabase,
    guid_counter: u64,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<PlayerSpellAcquisitionReconciliationLikeCpp, DatabaseError> {
    validate_prepared_character_counter_like_cpp(guid_counter, prepared)?;
    let mut transaction = character_db
        .pool()
        .begin()
        .await
        .map_err(DatabaseError::from)?;
    let locked_guid =
        sqlx::query_scalar::<_, u64>("SELECT guid FROM characters WHERE guid = ? FOR UPDATE")
            .bind(guid_counter)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(DatabaseError::from)?;
    if locked_guid != Some(guid_counter) {
        rollback_player_spell_acquisition_like_cpp(transaction).await;
        return Err(DatabaseError::Transaction(
            "prepared player spell acquisition character vanished during reconciliation"
                .to_string(),
        ));
    }

    let spell_rows = sqlx::query_as::<_, (i32, bool, bool)>(
        "SELECT spell, active, disabled FROM character_spell WHERE guid = ? ORDER BY spell",
    )
    .bind(guid_counter)
    .fetch_all(&mut *transaction)
    .await
    .map_err(DatabaseError::from)?
    .into_iter()
    .map(
        |(spell_id, active, disabled)| DurablePlayerSpellRowLikeCpp {
            spell_id,
            active,
            disabled,
        },
    )
    .collect::<Vec<_>>();
    let favorite_spell_ids = sqlx::query_scalar::<_, i32>(
        "SELECT spell FROM character_spell_favorite WHERE guid = ? ORDER BY spell",
    )
    .bind(guid_counter)
    .fetch_all(&mut *transaction)
    .await
    .map_err(DatabaseError::from)?;
    let skill_rows = sqlx::query_as::<_, (u16, u16, u16, i8)>(
        "SELECT skill, value, max, professionSlot FROM character_skills WHERE guid = ? ORDER BY skill",
    )
    .bind(guid_counter)
    .fetch_all(&mut *transaction)
    .await
    .map_err(DatabaseError::from)?
    .into_iter()
    .map(
        |(skill_id, value, maximum, profession_slot)| DurablePlayerSkillRowLikeCpp {
            skill_id,
            value,
            maximum,
            profession_slot,
        },
    )
    .collect::<Vec<_>>();

    transaction.commit().await.map_err(DatabaseError::from)?;
    Ok(
        if spell_rows == prepared.durable_spells
            && favorite_spell_ids == prepared.durable_favorite_spell_ids
            && skill_rows == prepared.durable_skills
        {
            PlayerSpellAcquisitionReconciliationLikeCpp::Committed
        } else {
            PlayerSpellAcquisitionReconciliationLikeCpp::NotCommitted
        },
    )
}

async fn persist_prepared_player_spell_acquisition_with_fault_like_cpp<F>(
    character_db: &CharacterDatabase,
    guid_counter: u64,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
    mut fault: F,
) -> Result<(), SqlTransactionCommitError>
where
    F: FnMut(PlayerSpellAcquisitionPersistenceFaultPointLikeCpp) -> Result<(), DatabaseError>,
{
    validate_prepared_character_counter_like_cpp(guid_counter, prepared)
        .map_err(SqlTransactionCommitError::DefinitelyRolledBack)?;
    let mut transaction = character_db
        .pool()
        .begin()
        .await
        .map_err(DatabaseError::from)
        .map_err(SqlTransactionCommitError::DefinitelyRolledBack)?;

    for (index, operation) in prepared.durable_operations.iter().copied().enumerate() {
        if let Err(error) =
            fault(PlayerSpellAcquisitionPersistenceFaultPointLikeCpp::BeforeOperation(index))
        {
            rollback_player_spell_acquisition_like_cpp(transaction).await;
            return Err(SqlTransactionCommitError::DefinitelyRolledBack(error));
        }
        if let Err(error) = execute_player_spell_acquisition_operation_like_cpp(
            &mut transaction,
            guid_counter,
            operation,
        )
        .await
        {
            rollback_player_spell_acquisition_like_cpp(transaction).await;
            return Err(SqlTransactionCommitError::DefinitelyRolledBack(error));
        }
    }

    if let Err(error) = fault(PlayerSpellAcquisitionPersistenceFaultPointLikeCpp::BeforeCommit) {
        rollback_player_spell_acquisition_like_cpp(transaction).await;
        return Err(SqlTransactionCommitError::DefinitelyRolledBack(error));
    }

    transaction.commit().await.map_err(|error| {
        let error = DatabaseError::from(error);
        if is_database_deadlock_like_cpp(&error) {
            SqlTransactionCommitError::DefinitelyRolledBack(error)
        } else {
            SqlTransactionCommitError::CommitOutcomeUnknown(error)
        }
    })
}

fn validate_prepared_character_counter_like_cpp(
    guid_counter: u64,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<(), DatabaseError> {
    let prepared_counter = prepared
        .character_guid
        .and_then(|guid| u64::try_from(guid.counter()).ok());
    if prepared_counter != Some(guid_counter) {
        return Err(DatabaseError::Transaction(
            "prepared player spell acquisition character GUID mismatch".to_string(),
        ));
    }
    Ok(())
}

async fn rollback_player_spell_acquisition_like_cpp(transaction: Transaction<'_, MySql>) {
    if let Err(error) = transaction.rollback().await {
        tracing::error!(
            error = %error,
            "Failed to roll back prepared player spell acquisition transaction"
        );
    }
}

async fn execute_player_spell_acquisition_operation_like_cpp(
    transaction: &mut Transaction<'_, MySql>,
    guid_counter: u64,
    operation: PlayerSpellAcquisitionDurableOperationLikeCpp,
) -> Result<(), DatabaseError> {
    let result = match operation {
        PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter => {
            let row = sqlx::query_scalar::<_, u64>(
                "SELECT guid FROM characters WHERE guid = ? FOR UPDATE",
            )
            .bind(guid_counter)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(DatabaseError::from)?;
            if row != Some(guid_counter) {
                return Err(DatabaseError::Transaction(
                    "prepared player spell acquisition character vanished".to_string(),
                ));
            }
            return Ok(());
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells => {
            sqlx::query("DELETE FROM character_spell WHERE guid = ?")
                .bind(guid_counter)
                .execute(&mut **transaction)
                .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells => {
            sqlx::query("DELETE FROM character_spell_favorite WHERE guid = ?")
                .bind(guid_counter)
                .execute(&mut **transaction)
                .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills => {
            sqlx::query("DELETE FROM character_skills WHERE guid = ?")
                .bind(guid_counter)
                .execute(&mut **transaction)
                .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(spell) => {
            sqlx::query(
                "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
            )
            .bind(guid_counter)
            .bind(spell.spell_id)
            .bind(spell.active)
            .bind(spell.disabled)
            .execute(&mut **transaction)
            .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(spell_id) => {
            sqlx::query("INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)")
                .bind(guid_counter)
                .bind(spell_id)
                .execute(&mut **transaction)
                .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill(skill) => {
            sqlx::query(
                "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(guid_counter)
            .bind(skill.skill_id)
            .bind(skill.value)
            .bind(skill.maximum)
            .bind(skill.profession_slot)
            .execute(&mut **transaction)
            .await
        }
    };

    let result = result.map_err(DatabaseError::from)?;
    if matches!(
        operation,
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(_)
            | PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(_)
            | PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill(_)
    ) && result.rows_affected() != 1
    {
        return Err(DatabaseError::Transaction(format!(
            "prepared player spell acquisition insert affected {} rows; expected exactly 1",
            result.rows_affected()
        )));
    }
    Ok(())
}

/// Applies and publishes an already committed plan. This function contains no
/// await point: the live mirrors are replaced first and the ordered C++ packet
/// and side-effect intents are observed only afterwards.
pub(crate) fn apply_prepared_player_spell_acquisition_like_cpp(
    session: &mut crate::session::WorldSession,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    require_prepared_session_character_like_cpp(session, prepared.character_guid)?;
    apply_prepared_player_spell_acquisition_with_fault_like_cpp(session, prepared, |_| Ok(()))
}

/// Applies the exact prepared plan with C++ `Player::LearnSpell` timing. The
/// runtime and packets change synchronously while the plan's persistence states
/// remain dirty for the ordinary `Player::SaveToDB` lifecycle.
pub(crate) fn apply_prepared_player_spell_acquisition_before_save_like_cpp(
    session: &mut crate::session::WorldSession,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    require_prepared_session_character_like_cpp(session, prepared.character_guid)?;
    apply_player_spell_acquisition_runtime_snapshot_with_fault_like_cpp(
        session,
        &prepared.pending_save_runtime_snapshot,
        &prepared.non_durable_skill_tombstone_ids,
        &prepared.post_commit_actions,
        |_| Ok(()),
    )
}

fn apply_prepared_player_spell_acquisition_with_fault_like_cpp<F>(
    session: &mut crate::session::WorldSession,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
    fault: F,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>
where
    F: FnMut(PlayerSpellAcquisitionPublicationFaultPointLikeCpp) -> Result<(), ()>,
{
    require_prepared_session_character_like_cpp(session, prepared.character_guid)?;
    apply_player_spell_acquisition_runtime_snapshot_with_fault_like_cpp(
        session,
        &prepared.runtime_snapshot,
        &prepared.non_durable_skill_tombstone_ids,
        &prepared.post_commit_actions,
        fault,
    )
}

fn apply_player_spell_acquisition_runtime_snapshot_with_fault_like_cpp<F>(
    session: &mut crate::session::WorldSession,
    runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    post_commit_actions: &[SpellAcquisitionPostCommitActionLikeCpp],
    fault: F,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>
where
    F: FnMut(PlayerSpellAcquisitionPublicationFaultPointLikeCpp) -> Result<(), ()>,
{
    preflight_player_spell_acquisition_runtime_owners_like_cpp(session, post_commit_actions)?;
    let spell_rows = runtime_snapshot
        .spells
        .iter()
        .map(|spell| crate::session::RepresentedPlayerSpellLikeCpp {
            spell_id: i32::try_from(spell.spell_id)
                .expect("prepared acquisition validated every spell ID"),
            active: spell.active,
            disabled: spell.disabled,
            dependent: spell.dependent,
            favorite: spell.favorite,
            state: match spell.state {
                PlayerSpellPersistenceStateLikeCpp::Unchanged => {
                    crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged
                }
                PlayerSpellPersistenceStateLikeCpp::Changed => {
                    crate::session::RepresentedPlayerSpellStateLikeCpp::Changed
                }
                PlayerSpellPersistenceStateLikeCpp::New => {
                    crate::session::RepresentedPlayerSpellStateLikeCpp::New
                }
                PlayerSpellPersistenceStateLikeCpp::Removed => {
                    crate::session::RepresentedPlayerSpellStateLikeCpp::Removed
                }
                PlayerSpellPersistenceStateLikeCpp::Temporary => {
                    crate::session::RepresentedPlayerSpellStateLikeCpp::Temporary
                }
            },
        })
        .collect::<Vec<_>>();
    let traits = runtime_snapshot
        .spells
        .iter()
        .filter_map(|spell| {
            if spell.state == PlayerSpellPersistenceStateLikeCpp::Removed {
                return None;
            }
            Some((
                i32::try_from(spell.spell_id).ok()?,
                spell.trait_definition_id?,
            ))
        })
        .collect::<Vec<_>>();
    let overrides = runtime_snapshot
        .overrides
        .iter()
        .map(|&(overridden, overriding)| {
            (
                i32::try_from(overridden).expect("validated overridden spell ID"),
                i32::try_from(overriding).expect("validated overriding spell ID"),
            )
        })
        .collect::<Vec<_>>();
    let skill_records = runtime_snapshot
        .skills
        .iter()
        .map(|skill| {
            let skill_id = u16::try_from(skill.skill_id)
                .expect("prepared acquisition validated every skill ID");
            (
                skill_id,
                crate::session::RepresentedPlayerSkillLikeCpp {
                    skill_id,
                    step: skill.step,
                    value: skill.value,
                    max: skill.maximum,
                    profession_slot: skill.profession_association.database_value_like_cpp(),
                    state: match skill.state {
                        PlayerSkillPersistenceStateLikeCpp::Unchanged => {
                            crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged
                        }
                        PlayerSkillPersistenceStateLikeCpp::Changed => {
                            crate::session::RepresentedPlayerSkillStateLikeCpp::Changed
                        }
                        PlayerSkillPersistenceStateLikeCpp::New => {
                            crate::session::RepresentedPlayerSkillStateLikeCpp::New
                        }
                        PlayerSkillPersistenceStateLikeCpp::Deleted => {
                            crate::session::RepresentedPlayerSkillStateLikeCpp::Deleted
                        }
                    },
                },
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    let mut non_durable_skill_tombstone_ids = session
        .player_skill_non_durable_tombstones_like_cpp()
        .clone();
    // C++ `Player::SetSkill` reactivates a `SKILL_DELETED` entry as
    // `SKILL_CHANGED`. A saved, non-durable tombstone therefore survives only
    // while the resulting row is still the zero-valued deleted shape.
    non_durable_skill_tombstone_ids.retain(|skill_id| {
        skill_records.get(skill_id).is_some_and(|skill| {
            skill.step == 0
                && skill.value == 0
                && skill.max == 0
                && skill.profession_slot == -1
                && matches!(
                    skill.state,
                    crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged
                        | crate::session::RepresentedPlayerSkillStateLikeCpp::Deleted
                )
        })
    });
    non_durable_skill_tombstone_ids.extend(new_non_durable_skill_tombstone_ids.iter().copied());
    if !session.replace_complete_spell_acquisition_runtime_like_cpp(
        spell_rows,
        traits,
        overrides,
        skill_records,
        runtime_snapshot.occupied_skill_slots,
        non_durable_skill_tombstone_ids,
    ) {
        return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
    }
    publish_player_spell_acquisition_actions_with_fault_like_cpp(
        session,
        runtime_snapshot,
        post_commit_actions,
        fault,
    )
}

pub(crate) fn apply_prepared_player_spell_acquisition_actions_like_cpp(
    session: &mut crate::session::WorldSession,
    prepared: &PreparedPlayerSpellAcquisitionActionsLikeCpp,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    require_prepared_session_character_like_cpp(session, prepared.character_guid)?;
    preflight_player_spell_acquisition_runtime_owners_like_cpp(
        session,
        &prepared.post_commit_actions,
    )?;
    publish_player_spell_acquisition_actions_with_fault_like_cpp(
        session,
        &prepared.runtime_snapshot,
        &prepared.post_commit_actions,
        |_| Ok(()),
    )
}

fn require_prepared_session_character_like_cpp(
    session: &crate::session::WorldSession,
    character_guid: Option<wow_core::ObjectGuid>,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    if character_guid.is_none() || session.player_guid() != character_guid {
        return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
    }
    Ok(())
}

fn preflight_player_spell_acquisition_runtime_owners_like_cpp(
    session: &crate::session::WorldSession,
    post_commit_actions: &[SpellAcquisitionPostCommitActionLikeCpp],
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    if post_commit_actions.iter().any(|action| {
        matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. }
        )
    }) && !session.has_canonical_player_for_spell_acquisition_like_cpp()
    {
        return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
    }
    Ok(())
}

fn publish_player_spell_acquisition_actions_with_fault_like_cpp<F>(
    session: &mut crate::session::WorldSession,
    runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    post_commit_actions: &[SpellAcquisitionPostCommitActionLikeCpp],
    mut fault: F,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>
where
    F: FnMut(PlayerSpellAcquisitionPublicationFaultPointLikeCpp) -> Result<(), ()>,
{
    session.begin_spell_acquisition_post_commit_action_batch_like_cpp();
    for (index, action) in post_commit_actions.iter().cloned().enumerate() {
        fault(PlayerSpellAcquisitionPublicationFaultPointLikeCpp::BeforeAction(index))
            .map_err(|()| PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::PublicationInterrupted)?;
        session.record_spell_acquisition_post_commit_action_like_cpp(action.clone());
        match action {
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id,
                favorite,
                suppress_messaging,
            } => {
                let trait_definition_id = runtime_snapshot
                    .spells
                    .iter()
                    .find(|spell| spell.spell_id == spell_id)
                    .and_then(|spell| spell.trait_definition_id);
                session.send_packet(&wow_packet::packets::trainer::LearnedSpells {
                    spells: vec![wow_packet::packets::trainer::LearnedSpellEntry {
                        spell_id: i32::try_from(spell_id).expect("validated learned spell ID"),
                        is_favorite: favorite,
                        field_8: None,
                        superceded: None,
                        trait_definition_id,
                    }],
                    suppress_messaging,
                });
            }
            SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                old_spell_id,
                new_spell_id,
            } => session.send_packet(&wow_packet::packets::trainer::SupercededSpells::single(
                i32::try_from(old_spell_id).expect("validated old spell ID"),
                i32::try_from(new_spell_id).expect("validated new spell ID"),
            )),
            SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id } => session
                .send_packet(&wow_packet::packets::trainer::UnlearnedSpells::single(
                    spell_id, false,
                )),
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. } => {
                if !session.grant_dual_wield_after_spell_acquisition_like_cpp() {
                    return Err(
                        PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime,
                    );
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                ..
            }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                ..
            }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { .. }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> (crate::session::WorldSession, flume::Receiver<Vec<u8>>) {
        let (_packet_tx, packet_rx) = flume::bounded(8);
        let (send_tx, send_rx) = flume::unbounded();
        let mut session = crate::session::WorldSession::new(
            1,
            "AcquisitionTest".to_string(),
            0,
            2,
            2,
            54261,
            vec![0; 40],
            "enUS".to_string(),
            packet_rx,
            send_tx,
        );
        session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
            wow_core::ObjectGuid::create_player(1, 42),
            "AcquisitionPlayer".to_string(),
            wow_core::Position::ZERO,
            0,
            1,
            1,
            80,
            0,
        ));
        (session, send_rx)
    }

    fn snapshot(
        spells: Vec<PlayerSpellAcquisitionRowLikeCpp>,
    ) -> PlayerSpellAcquisitionSnapshotLikeCpp {
        PlayerSpellAcquisitionSnapshotLikeCpp {
            character_guid: Some(wow_core::ObjectGuid::create_player(1, 42)),
            spells,
            skills: Vec::new(),
            occupied_skill_slots: 0,
            overrides: Vec::new(),
            primary_profession_skill_ids: Vec::new(),
            race: 1,
            class: 1,
            level: 80,
            lifecycle: PlayerAcquisitionLifecycleLikeCpp::InWorld,
            future_player_condition_resolutions: Vec::new(),
            cast_resolutions: BTreeMap::new(),
        }
    }

    fn spell(
        spell_id: u32,
        state: PlayerSpellPersistenceStateLikeCpp,
    ) -> PlayerSpellAcquisitionRowLikeCpp {
        PlayerSpellAcquisitionRowLikeCpp {
            spell_id,
            active: true,
            disabled: false,
            dependent: false,
            favorite: false,
            trait_definition_id: None,
            state,
        }
    }

    fn no_profession_changes() -> PrimaryProfessionCapacityPlanLikeCpp {
        PrimaryProfessionCapacityPlanLikeCpp {
            configured_max: 2,
            used_before: 0,
            free_before: 2,
            existing_professions: Vec::new(),
            new_professions: Vec::new(),
            slot_normalizations: Vec::new(),
        }
    }

    fn direct_learn_actions(
        spell_id: u32,
        favorite: bool,
        suppress_messaging: bool,
    ) -> Vec<SpellAcquisitionPostCommitActionLikeCpp> {
        vec![
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id },
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id,
                favorite,
                suppress_messaging,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { spell_id },
        ]
    }

    fn direct_learn_requirements(
        spell_id: u32,
        favorite: bool,
        suppress_messaging: bool,
    ) -> Vec<SpellAcquisitionPublicationRequirementLikeCpp> {
        vec![
            SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id,
            },
            SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                spell_id,
                favorite,
                suppress_messaging,
            },
            SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id,
            },
        ]
    }

    fn direct_learn_plan() -> (
        PlayerSpellAcquisitionSnapshotLikeCpp,
        SpellAcquisitionPlanLikeCpp,
    ) {
        let source = snapshot(Vec::new());
        let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: direct_learn_requirements(100, false, false),
            profession_association_inputs: Vec::new(),
            post_commit_actions: direct_learn_actions(100, false, false),
            diagnostics: Vec::new(),
            resulting_snapshot: snapshot(vec![learned]),
        };
        (source, plan)
    }

    #[test]
    fn required_learning_publications_cannot_be_omitted() {
        let (source, plan) = direct_learn_plan();
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("complete publication tape is valid");

        for omitted_index in 0..plan.post_commit_actions.len() {
            let mut omitted = plan.clone();
            omitted.post_commit_actions.remove(omitted_index);
            assert!(matches!(
                prepare_player_spell_acquisition_like_cpp(
                    &omitted,
                    &no_profession_changes(),
                    &source,
                ),
                Err(
                    PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch { .. }
                )
            ));
        }
    }

    #[test]
    fn skill_line_criteria_require_exact_occurrence_identity_and_cardinality() {
        let (source, mut plan) = direct_learn_plan();
        plan.publication_requirements.splice(
            0..0,
            [
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                    source_spell_id: 100,
                    skill_id: 164,
                },
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                    source_spell_id: 100,
                    skill_id: 164,
                },
            ],
        );
        plan.post_commit_actions.splice(
            0..0,
            [
                SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                    source_spell_id: 100,
                    skill_id: 164,
                },
                SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                    source_spell_id: 100,
                    skill_id: 164,
                },
            ],
        );
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("the exact C++ SkillLineAbility occurrence is valid");

        let mut wrong_skill = plan.clone();
        for action in &mut wrong_skill.post_commit_actions[..2] {
            match action {
                SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                    skill_id,
                    ..
                }
                | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                    skill_id,
                    ..
                } => *skill_id = 165,
                _ => unreachable!(),
            }
        }
        assert!(
            prepare_player_spell_acquisition_like_cpp(
                &wrong_skill,
                &no_profession_changes(),
                &source,
            )
            .is_err()
        );

        let mut duplicated = plan.clone();
        duplicated
            .post_commit_actions
            .splice(2..2, plan.post_commit_actions[..2].iter().cloned());
        assert!(
            prepare_player_spell_acquisition_like_cpp(
                &duplicated,
                &no_profession_changes(),
                &source,
            )
            .is_err()
        );

        let mut omitted = plan.clone();
        omitted.post_commit_actions.drain(..2);
        assert!(
            prepare_player_spell_acquisition_like_cpp(&omitted, &no_profession_changes(), &source,)
                .is_err()
        );
    }

    #[test]
    fn profession_plan_cannot_omit_an_existing_primary_profession() {
        const EXISTING: u32 = 164;
        const NEW: u32 = 165;
        let existing = PlayerSkillAcquisitionRowLikeCpp {
            skill_id: EXISTING,
            step: 1,
            value: 75,
            maximum: 75,
            profession_association: ProfessionAssociationInputLikeCpp::Slot(0),
            state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
        };
        let learned = PlayerSkillAcquisitionRowLikeCpp {
            skill_id: NEW,
            step: 1,
            value: 1,
            maximum: 75,
            profession_association: ProfessionAssociationInputLikeCpp::Slot(1),
            state: PlayerSkillPersistenceStateLikeCpp::New,
        };
        let mut source = snapshot(Vec::new());
        source.skills = vec![existing];
        source.occupied_skill_slots = 1;
        source.primary_profession_skill_ids = vec![EXISTING];
        let mut resulting = source.clone();
        resulting.skills.push(learned);
        resulting.occupied_skill_slots = 2;
        resulting.primary_profession_skill_ids = vec![EXISTING, NEW];
        let transition = PlannedSkillTransitionLikeCpp {
            skill_id: NEW,
            before: None,
            after: learned,
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(transition.clone())],
            spell_transitions: Vec::new(),
            skill_transitions: vec![transition],
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: vec![NEW],
            publication_requirements: Vec::new(),
            profession_association_inputs: vec![existing, learned],
            post_commit_actions: Vec::new(),
            diagnostics: Vec::new(),
            resulting_snapshot: resulting,
        };
        let omitted_existing = PrimaryProfessionCapacityPlanLikeCpp {
            configured_max: 2,
            used_before: 0,
            free_before: 2,
            existing_professions: Vec::new(),
            new_professions: vec![crate::profession::PlannedPrimaryProfessionLikeCpp {
                skill_id: NEW,
                equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
            }],
            slot_normalizations: Vec::new(),
        };

        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(&plan, &omitted_existing, &source),
            Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                    "complete existing profession membership"
                )
            )
        );
    }

    #[test]
    fn prepared_acquisition_cannot_cross_character_boundaries() {
        let (source, plan) = direct_learn_plan();
        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
                .expect("valid prepared acquisition")
        else {
            panic!("expected durable acquisition")
        };
        assert!(validate_prepared_character_counter_like_cpp(43, &prepared).is_err());

        let (mut other_session, send_rx) = make_session();
        other_session.attach_player_controller_like_cpp(
            crate::session::SessionPlayerController::new(
                wow_core::ObjectGuid::create_player(1, 43),
                "OtherAcquisitionPlayer".to_string(),
                wow_core::Position::ZERO,
                0,
                1,
                1,
                80,
                0,
            ),
        );
        assert_eq!(
            apply_prepared_player_spell_acquisition_like_cpp(&mut other_session, &prepared),
            Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime)
        );
        assert!(
            other_session
                .complete_represented_player_spell_rows_like_cpp()
                .is_none(),
            "cross-character rejection occurs before installing runtime state"
        );
        assert!(send_rx.is_empty());
    }

    #[test]
    fn prepared_plan_replays_causal_stream_and_normalizes_post_save_state() {
        let source = snapshot(Vec::new());
        let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let resulting = snapshot(vec![learned]);
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: direct_learn_requirements(100, false, false),
            profession_association_inputs: Vec::new(),
            post_commit_actions: direct_learn_actions(100, false, false),
            diagnostics: Vec::new(),
            resulting_snapshot: resulting,
        };

        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
                .expect("valid plan")
        else {
            panic!("expected a prepared mutation")
        };
        assert_eq!(prepared.durable_spells.len(), 1);
        assert_eq!(
            prepared.pending_save_runtime_snapshot.spells[0].state,
            PlayerSpellPersistenceStateLikeCpp::New,
            "C++ LearnSpell keeps the new row dirty until Player::SaveToDB"
        );
        assert_eq!(
            prepared.runtime_snapshot.spells[0].state,
            PlayerSpellPersistenceStateLikeCpp::Unchanged
        );
        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(
                &plan,
                &no_profession_changes(),
                &prepared.runtime_snapshot,
            ),
            Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied)
        );
    }

    #[test]
    fn prepared_plan_rejects_stale_or_tampered_authority_before_sql() {
        let source = snapshot(Vec::new());
        let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let mut plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: direct_learn_requirements(100, false, false),
            profession_association_inputs: Vec::new(),
            post_commit_actions: direct_learn_actions(100, false, false),
            diagnostics: Vec::new(),
            resulting_snapshot: snapshot(vec![learned]),
        };
        let stale = snapshot(vec![spell(
            99,
            PlayerSpellPersistenceStateLikeCpp::Unchanged,
        )]);
        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &stale),
            Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::StaleSnapshot)
        );

        plan.spell_transitions.clear();
        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source),
            Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::TypedProjectionMismatch(
                    "spell_transitions"
                )
            )
        );
    }

    #[test]
    fn prepared_plan_builds_one_deterministic_full_replacement_transaction() {
        let source = snapshot(Vec::new());
        let mut learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        learned.favorite = true;
        learned.trait_definition_id = Some(7);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: direct_learn_requirements(100, true, true),
            profession_association_inputs: Vec::new(),
            post_commit_actions: direct_learn_actions(100, true, true),
            diagnostics: Vec::new(),
            resulting_snapshot: snapshot(vec![learned]),
        };
        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
                .expect("valid plan")
        else {
            panic!("expected ready plan")
        };

        assert_eq!(
            prepared.durable_operations,
            vec![
                PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(
                    DurablePlayerSpellRowLikeCpp {
                        spell_id: 100,
                        active: true,
                        disabled: false,
                    }
                ),
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(100),
            ]
        );
    }

    #[test]
    fn runtime_state_is_complete_before_ordered_publication() {
        let source = snapshot(Vec::new());
        let mut learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        learned.favorite = true;
        learned.trait_definition_id = Some(7);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let actions = direct_learn_actions(100, true, false);
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: direct_learn_requirements(100, true, false),
            profession_association_inputs: Vec::new(),
            post_commit_actions: actions.clone(),
            diagnostics: Vec::new(),
            resulting_snapshot: snapshot(vec![learned]),
        };
        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
                .expect("valid plan")
        else {
            panic!("expected ready plan")
        };
        let (mut interrupted_session, interrupted_send_rx) = make_session();
        assert_eq!(
            apply_prepared_player_spell_acquisition_with_fault_like_cpp(
                &mut interrupted_session,
                &prepared,
                |point| (point
                    != PlayerSpellAcquisitionPublicationFaultPointLikeCpp::BeforeAction(0))
                .then_some(())
                .ok_or(()),
            ),
            Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::PublicationInterrupted)
        );
        assert!(
            interrupted_session
                .complete_represented_player_spell_rows_like_cpp()
                .is_some(),
            "committed runtime state is installed before publication can be interrupted"
        );
        assert_eq!(interrupted_send_rx.len(), 0);
        assert!(
            interrupted_session
                .represented_spell_acquisition_post_commit_actions_like_cpp()
                .is_empty()
        );
        let (mut pre_save_session, pre_save_send_rx) = make_session();
        apply_prepared_player_spell_acquisition_before_save_like_cpp(
            &mut pre_save_session,
            &prepared,
        )
        .expect("validated C++-timed snapshot applies");
        assert_eq!(
            pre_save_session
                .complete_represented_player_spell_rows_like_cpp()
                .and_then(|rows| rows.get(&100))
                .map(|row| row.state),
            Some(crate::session::RepresentedPlayerSpellStateLikeCpp::New),
            "EffectLearnSpell must publish immediately while leaving _SaveSpells dirty"
        );
        assert_eq!(pre_save_send_rx.len(), 1);
        let (mut session, send_rx) = make_session();

        apply_prepared_player_spell_acquisition_like_cpp(&mut session, &prepared)
            .expect("validated snapshot applies");

        let rows = session
            .complete_represented_player_spell_rows_like_cpp()
            .expect("complete rows installed");
        assert_eq!(rows.get(&100).map(|row| row.favorite), Some(true));
        assert_eq!(
            session
                .complete_represented_spell_trait_definition_ids_like_cpp()
                .and_then(|traits| traits.get(&100)),
            Some(&7)
        );
        assert_eq!(
            session.represented_spell_acquisition_post_commit_actions_like_cpp(),
            actions
        );
        assert_eq!(
            send_rx.len(),
            1,
            "the learned result publishes exactly once"
        );
        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(
                &plan,
                &no_profession_changes(),
                &session
                    .spell_acquisition_snapshot_like_cpp(
                        PlayerAcquisitionLifecycleLikeCpp::InWorld,
                        Vec::new(),
                        BTreeMap::new(),
                    )
                    .expect("installed state remains snapshot-complete"),
            ),
            Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied)
        );
    }

    #[test]
    fn learned_action_must_match_the_final_favorite_row() {
        let source = snapshot(Vec::new());
        let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: vec![
                SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                    spell_id: 100,
                    favorite: true,
                    suppress_messaging: false,
                },
            ],
            profession_association_inputs: Vec::new(),
            post_commit_actions: vec![SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: 100,
                favorite: true,
                suppress_messaging: false,
            }],
            diagnostics: Vec::new(),
            resulting_snapshot: snapshot(vec![learned]),
        };
        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source),
            Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::LearnedActionRowMismatch(100))
        );
    }

    #[test]
    fn learned_action_requires_a_causal_learning_transition() {
        let unchanged = spell(100, PlayerSpellPersistenceStateLikeCpp::Unchanged);
        let source = snapshot(vec![unchanged]);
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: Vec::new(),
            spell_transitions: Vec::new(),
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: vec![
                SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                    spell_id: 100,
                    favorite: false,
                    suppress_messaging: false,
                },
            ],
            profession_association_inputs: Vec::new(),
            post_commit_actions: vec![SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: 100,
                favorite: false,
                suppress_messaging: false,
            }],
            diagnostics: Vec::new(),
            resulting_snapshot: source.clone(),
        };

        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source,),
            Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::LearnedActionRowMismatch(100))
        );
    }

    #[test]
    fn learned_transition_evidence_is_consumed_once() {
        let source = snapshot(Vec::new());
        let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let duplicate = SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id: 100,
            favorite: false,
            suppress_messaging: false,
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: vec![
                SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                    spell_id: 100,
                    favorite: false,
                    suppress_messaging: false,
                },
                SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                    spell_id: 100,
                    favorite: false,
                    suppress_messaging: false,
                },
            ],
            profession_association_inputs: Vec::new(),
            post_commit_actions: vec![duplicate.clone(), duplicate],
            diagnostics: Vec::new(),
            resulting_snapshot: snapshot(vec![learned]),
        };

        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source),
            Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::LearnedActionRowMismatch(100))
        );
    }

    #[test]
    fn dual_wield_diagnostic_cannot_replace_live_cast_effect_authority() {
        let source = snapshot(Vec::new());
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: Vec::new(),
            spell_transitions: Vec::new(),
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: Vec::new(),
            profession_association_inputs: Vec::new(),
            post_commit_actions: vec![SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
                source_spell_id: 100,
                effect_record_id: 7,
                effect_index: 0,
            }],
            diagnostics: vec![
                SpellAcquisitionDiagnosticLikeCpp::DualWieldEffectProjected {
                    spell_id: 100,
                    effect_record_id: 7,
                    effect_index: 0,
                },
            ],
            resulting_snapshot: source.clone(),
        };

        assert_eq!(
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source),
            Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
                    action: "GrantDualWield",
                    id: 100,
                }
            )
        );
    }

    #[test]
    fn action_only_plan_publishes_without_preparing_a_durable_rewrite() {
        let unchanged = spell(100, PlayerSpellPersistenceStateLikeCpp::Unchanged);
        let source = snapshot(vec![unchanged]);
        let action = SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
            spell_id: 100,
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: Vec::new(),
            spell_transitions: Vec::new(),
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: vec![
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective {
                    spell_id: 100,
                },
            ],
            profession_association_inputs: Vec::new(),
            post_commit_actions: vec![action.clone()],
            diagnostics: Vec::new(),
            resulting_snapshot: source.clone(),
        };

        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::ActionsOnly(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
                .expect("valid action-only plan")
        else {
            panic!("an action-only learn must not prepare durable replacement operations")
        };
        assert_eq!(prepared.runtime_snapshot, source);

        let (mut session, send_rx) = make_session();
        session.record_spell_acquisition_post_commit_action_like_cpp(
            SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability {
                skill_id: u32::from(crate::session::SKILL_RIDING_LIKE_CPP),
            },
        );
        apply_prepared_player_spell_acquisition_actions_like_cpp(&mut session, &prepared)
            .expect("publish action-only plan");
        assert_eq!(
            session.represented_spell_acquisition_post_commit_actions_like_cpp(),
            &[action],
            "the current acquisition batch replaces earlier retained intent instead of growing forever"
        );
        assert!(send_rx.is_empty());
    }

    #[test]
    fn unrelated_publication_actions_are_rejected_before_application() {
        let source = snapshot(Vec::new());
        let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let base_plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: Vec::new(),
            profession_association_inputs: Vec::new(),
            post_commit_actions: Vec::new(),
            diagnostics: Vec::new(),
            resulting_snapshot: snapshot(vec![learned]),
        };
        let unrelated_actions = [
            SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id: 999 },
            SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                old_spell_id: 999,
                new_spell_id: 100,
            },
            SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id: 999 },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id: 999,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: 999,
            },
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
                source_spell_id: 999,
                effect_record_id: 1,
                effect_index: 0,
            },
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
                source_spell_id: 100,
                effect_record_id: 1,
                effect_index: 0,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id: 999,
                skill_id: 164,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { skill_id: 164 },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id: 164 },
        ];

        for action in unrelated_actions {
            let mut plan = base_plan.clone();
            plan.post_commit_actions = vec![action.clone()];
            assert!(
                matches!(
                    prepare_player_spell_acquisition_like_cpp(
                        &plan,
                        &no_profession_changes(),
                        &source,
                    ),
                    Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
                            ..
                        }
                    )
                ),
                "unrelated action reached the prepared boundary: {action:?}"
            );
        }
    }

    #[test]
    fn profession_capacity_assignment_is_applied_to_durable_and_runtime_rows() {
        const PROFESSION: u32 = 164;
        let source = snapshot(Vec::new());
        let learned_skill = PlayerSkillAcquisitionRowLikeCpp {
            skill_id: PROFESSION,
            step: 1,
            value: 1,
            maximum: 75,
            profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
            state: PlayerSkillPersistenceStateLikeCpp::New,
        };
        let transition = PlannedSkillTransitionLikeCpp {
            skill_id: PROFESSION,
            before: None,
            after: learned_skill,
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let mut resulting = source.clone();
        resulting.skills.push(learned_skill);
        resulting.occupied_skill_slots = 1;
        resulting.primary_profession_skill_ids = vec![PROFESSION];
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(transition.clone())],
            spell_transitions: Vec::new(),
            skill_transitions: vec![transition],
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: vec![PROFESSION],
            publication_requirements: Vec::new(),
            profession_association_inputs: vec![learned_skill],
            post_commit_actions: Vec::new(),
            diagnostics: Vec::new(),
            resulting_snapshot: resulting,
        };
        let profession_plan = PrimaryProfessionCapacityPlanLikeCpp {
            configured_max: 2,
            used_before: 0,
            free_before: 2,
            existing_professions: Vec::new(),
            new_professions: vec![crate::profession::PlannedPrimaryProfessionLikeCpp {
                skill_id: PROFESSION,
                equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
            }],
            slot_normalizations: Vec::new(),
        };

        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &profession_plan, &source)
                .expect("valid profession assignment")
        else {
            panic!("expected ready profession plan")
        };
        assert_eq!(prepared.durable_skills[0].profession_slot, 0);
        assert_eq!(
            prepared.runtime_snapshot.skills[0].profession_association,
            ProfessionAssociationInputLikeCpp::Slot(0)
        );
    }

    #[test]
    fn deleted_skill_tombstone_is_retained_only_in_runtime_after_save() {
        let existing = PlayerSkillAcquisitionRowLikeCpp {
            skill_id: 95,
            step: 1,
            value: 75,
            maximum: 75,
            profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
            state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
        };
        let deleted = PlayerSkillAcquisitionRowLikeCpp {
            skill_id: 95,
            step: 0,
            value: 0,
            maximum: 0,
            profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
            state: PlayerSkillPersistenceStateLikeCpp::Deleted,
        };
        let mut source = snapshot(Vec::new());
        source.skills.push(existing);
        source.occupied_skill_slots = 1;
        let mut resulting = source.clone();
        resulting.skills[0] = deleted;
        let transition = PlannedSkillTransitionLikeCpp {
            skill_id: 95,
            before: Some(existing),
            after: deleted,
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(transition.clone())],
            spell_transitions: Vec::new(),
            skill_transitions: vec![transition],
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: Vec::new(),
            profession_association_inputs: vec![deleted],
            post_commit_actions: Vec::new(),
            diagnostics: Vec::new(),
            resulting_snapshot: resulting,
        };

        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
                .expect("valid deleted skill")
        else {
            panic!("expected ready deleted-skill plan")
        };
        assert!(prepared.durable_skills.is_empty());
        assert_eq!(
            prepared.non_durable_skill_tombstone_ids,
            BTreeSet::from([95])
        );
        assert_eq!(
            prepared.runtime_snapshot.skills[0].state,
            PlayerSkillPersistenceStateLikeCpp::Unchanged
        );
        assert_eq!(prepared.runtime_snapshot.skills[0].value, 0);

        let (mut session, _) = make_session();
        apply_prepared_player_spell_acquisition_like_cpp(&mut session, &prepared)
            .expect("apply committed deleted-skill snapshot");
        assert_eq!(
            session.character_skill_save_statements_like_cpp(42).len(),
            1,
            "a later full save retains only DELETE ALL and cannot reinsert the normalized tombstone"
        );

        let relearned = PlayerSkillAcquisitionRowLikeCpp {
            skill_id: 95,
            step: 1,
            value: 1,
            maximum: 75,
            profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
            state: PlayerSkillPersistenceStateLikeCpp::Changed,
        };
        let relearn_source = prepared.runtime_snapshot.clone();
        let mut relearn_resulting = relearn_source.clone();
        relearn_resulting.skills[0] = relearned;
        let relearn_transition = PlannedSkillTransitionLikeCpp {
            skill_id: 95,
            before: Some(relearn_source.skills[0]),
            after: relearned,
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(101),
            },
        };
        let relearn_plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(101),
            source_snapshot: relearn_source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(
                relearn_transition.clone(),
            )],
            spell_transitions: Vec::new(),
            skill_transitions: vec![relearn_transition],
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: Vec::new(),
            profession_association_inputs: vec![relearned],
            post_commit_actions: Vec::new(),
            diagnostics: Vec::new(),
            resulting_snapshot: relearn_resulting,
        };
        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(relearn_prepared) =
            prepare_player_spell_acquisition_like_cpp(
                &relearn_plan,
                &no_profession_changes(),
                &relearn_source,
            )
            .expect("C++ SetSkill reactivates a saved deleted skill")
        else {
            panic!("expected ready relearn plan")
        };

        apply_prepared_player_spell_acquisition_before_save_like_cpp(
            &mut session,
            &relearn_prepared,
        )
        .expect("saved tombstone must not block relearning the skill");
        assert!(
            !session
                .player_skill_non_durable_tombstones_like_cpp()
                .contains(&95)
        );
        assert_eq!(session.player_skill_records_like_cpp()[&95].value, 1);
        assert_eq!(
            session.player_skill_records_like_cpp()[&95].state,
            crate::session::RepresentedPlayerSkillStateLikeCpp::Changed
        );
    }

    #[test]
    fn committed_skill_snapshot_refreshes_enchanting_runtime_projection() {
        let enchanting = PlayerSkillAcquisitionRowLikeCpp {
            skill_id: crate::session::SKILL_ENCHANTING_LIKE_CPP.into(),
            step: 2,
            value: 150,
            maximum: 225,
            profession_association: ProfessionAssociationInputLikeCpp::Slot(0),
            state: PlayerSkillPersistenceStateLikeCpp::New,
        };
        let source = snapshot(Vec::new());
        let mut resulting = source.clone();
        resulting.skills.push(enchanting);
        resulting.occupied_skill_slots = 1;
        resulting.primary_profession_skill_ids = vec![enchanting.skill_id];
        let transition = PlannedSkillTransitionLikeCpp {
            skill_id: enchanting.skill_id,
            before: None,
            after: enchanting,
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(transition.clone())],
            spell_transitions: Vec::new(),
            skill_transitions: vec![transition],
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: vec![enchanting.skill_id],
            publication_requirements: Vec::new(),
            profession_association_inputs: vec![enchanting],
            post_commit_actions: Vec::new(),
            diagnostics: Vec::new(),
            resulting_snapshot: resulting,
        };
        let profession_plan = PrimaryProfessionCapacityPlanLikeCpp {
            configured_max: 2,
            used_before: 0,
            free_before: 2,
            existing_professions: Vec::new(),
            new_professions: vec![crate::profession::PlannedPrimaryProfessionLikeCpp {
                skill_id: enchanting.skill_id,
                equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
            }],
            slot_normalizations: Vec::new(),
        };
        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &profession_plan, &source)
                .expect("valid enchanting acquisition")
        else {
            panic!("expected ready enchanting plan")
        };

        let (mut session, _) = make_session();
        apply_prepared_player_spell_acquisition_like_cpp(&mut session, &prepared)
            .expect("apply committed enchanting snapshot");
        assert_eq!(session.represented_enchanting_skill, 150);
    }

    #[test]
    fn dual_wield_missing_owner_stops_before_any_publication() {
        let mut source = snapshot(Vec::new());
        source.cast_resolutions.insert(
            100,
            PlayerCastAcquisitionResolutionLikeCpp {
                reached_immediate_phase: true,
                executed_hit_target_effect_mask: 1,
                executed_dual_wield_effects: vec![PlayerExecutedDualWieldEffectLikeCpp {
                    effect_record_id: 7,
                    effect_index: 0,
                }],
            },
        );
        let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
        let transition = PlannedSpellTransitionLikeCpp {
            spell_id: 100,
            before: None,
            after: Some(learned),
            provenance: SpellAcquisitionProvenanceLikeCpp::Root {
                root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            },
        };
        let plan = SpellAcquisitionPlanLikeCpp {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
            source_snapshot: source.clone(),
            mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
            spell_transitions: vec![transition],
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: Vec::new(),
            publication_requirements: direct_learn_requirements(100, false, false),
            profession_association_inputs: Vec::new(),
            post_commit_actions: vec![
                SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                    spell_id: 100,
                },
                SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                    spell_id: 100,
                    favorite: false,
                    suppress_messaging: false,
                },
                SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                    spell_id: 100,
                },
                SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
                    source_spell_id: 100,
                    effect_record_id: 7,
                    effect_index: 0,
                },
            ],
            diagnostics: vec![
                SpellAcquisitionDiagnosticLikeCpp::DualWieldEffectProjected {
                    spell_id: 100,
                    effect_record_id: 7,
                    effect_index: 0,
                },
            ],
            resulting_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp {
                spells: vec![learned],
                ..source.clone()
            },
        };
        let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
            prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
                .expect("valid dual-wield plan")
        else {
            panic!("expected ready plan")
        };
        let (mut session, send_rx) = make_session();

        assert_eq!(
            apply_prepared_player_spell_acquisition_like_cpp(&mut session, &prepared),
            Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime)
        );
        assert!(
            session
                .complete_represented_player_spell_rows_like_cpp()
                .is_none_or(|rows| !rows.contains_key(&100)),
            "a missing required runtime owner is rejected before replacing spell authority"
        );
        assert!(
            send_rx.is_empty(),
            "a missing required runtime owner is rejected before any success packet"
        );
        assert!(
            session
                .represented_spell_acquisition_post_commit_actions_like_cpp()
                .is_empty(),
            "a missing required runtime owner is rejected before recording any action"
        );
    }

    #[test]
    fn every_durable_fault_boundary_discards_the_whole_operation_prefix() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct DurableState {
            spells: Vec<DurablePlayerSpellRowLikeCpp>,
            favorites: Vec<i32>,
            skills: Vec<DurablePlayerSkillRowLikeCpp>,
        }

        fn execute_atomically(
            original: &DurableState,
            operations: &[PlayerSpellAcquisitionDurableOperationLikeCpp],
            fail_before_operation: Option<usize>,
            fail_before_commit: bool,
        ) -> DurableState {
            let mut transaction = original.clone();
            for (index, operation) in operations.iter().copied().enumerate() {
                if fail_before_operation == Some(index) {
                    return original.clone();
                }
                match operation {
                    PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter => {}
                    PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells => {
                        transaction.spells.clear()
                    }
                    PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells => {
                        transaction.favorites.clear()
                    }
                    PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills => {
                        transaction.skills.clear()
                    }
                    PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(row) => {
                        transaction.spells.push(row)
                    }
                    PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(id) => {
                        transaction.favorites.push(id)
                    }
                    PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill(row) => {
                        transaction.skills.push(row)
                    }
                }
            }
            if fail_before_commit {
                original.clone()
            } else {
                transaction
            }
        }

        let original = DurableState {
            spells: vec![DurablePlayerSpellRowLikeCpp {
                spell_id: 99,
                active: true,
                disabled: false,
            }],
            favorites: vec![99],
            skills: vec![DurablePlayerSkillRowLikeCpp {
                skill_id: 10,
                value: 1,
                maximum: 75,
                profession_slot: -1,
            }],
        };
        let operations = vec![
            PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
            PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
            PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
            PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
            PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(
                DurablePlayerSpellRowLikeCpp {
                    spell_id: 100,
                    active: true,
                    disabled: false,
                },
            ),
            PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(100),
        ];

        for index in 0..operations.len() {
            assert_eq!(
                execute_atomically(&original, &operations, Some(index), false),
                original,
                "fault before operation {index} must roll back the entire prefix"
            );
        }
        assert_eq!(
            execute_atomically(&original, &operations, None, true),
            original,
            "fault at commit must not expose the transactional prefix"
        );
        assert_eq!(
            execute_atomically(&original, &operations, None, false),
            DurableState {
                spells: vec![DurablePlayerSpellRowLikeCpp {
                    spell_id: 100,
                    active: true,
                    disabled: false,
                }],
                favorites: vec![100],
                skills: Vec::new(),
            }
        );
    }
}
