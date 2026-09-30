//! Causal validation of the planner's immutable acquisition stream.
//!
//! These checks preserve the represented Rust replay and publication contract;
//! they do not grant authority to apply, persist or publish the plan.

use std::collections::{BTreeMap, BTreeSet};

use crate::SKILL_RIDING_LIKE_CPP;
use crate::model::*;

mod actions;
mod errors;
mod provenance;
mod snapshots;

use actions::validate_acquisition_actions;
pub use errors::AcquisitionPlanError;
use provenance::{validate_provenance, validate_root};
pub use snapshots::acquisition_skill_rows;
use snapshots::{
    acquisition_override_pairs, acquisition_spell_rows, validate_snapshot,
    validate_snapshot_identity,
};

pub fn validate_acquisition_plan(
    plan: &SpellAcquisitionPlanLikeCpp,
) -> Result<(), AcquisitionPlanError> {
    validate_root(plan.root)?;
    validate_snapshot(&plan.source_snapshot)?;
    validate_snapshot(&plan.resulting_snapshot)?;
    validate_snapshot_identity(&plan.source_snapshot, &plan.resulting_snapshot)?;

    let mut spells = acquisition_spell_rows(&plan.source_snapshot)?;
    let mut skills = acquisition_skill_rows(&plan.source_snapshot)?;
    let mut overrides = acquisition_override_pairs(&plan.source_snapshot)?;
    let mut replayed_spells = Vec::new();
    let mut replayed_skills = Vec::new();
    let mut replayed_overrides = Vec::new();

    for mutation in &plan.mutations {
        match mutation {
            PlannedAcquisitionMutationLikeCpp::Spell(transition) => {
                validate_provenance(&transition.provenance, plan.root)?;
                if transition
                    .before
                    .is_some_and(|row| row.spell_id != transition.spell_id)
                    || transition
                        .after
                        .is_some_and(|row| row.spell_id != transition.spell_id)
                {
                    return Err(AcquisitionPlanError::TransitionIdMismatch {
                        domain: "spell",
                        id: transition.spell_id,
                    });
                }
                if spells.get(&transition.spell_id).copied() != transition.before {
                    return Err(AcquisitionPlanError::TransitionBeforeMismatch {
                        domain: "spell",
                        id: transition.spell_id,
                    });
                }
                if let Some(after) = transition.after {
                    spells.insert(transition.spell_id, after);
                } else {
                    spells.remove(&transition.spell_id);
                }
                replayed_spells.push(transition.clone());
            }
            PlannedAcquisitionMutationLikeCpp::Skill(transition) => {
                validate_provenance(&transition.provenance, plan.root)?;
                if transition.after.skill_id != transition.skill_id
                    || transition
                        .before
                        .is_some_and(|row| row.skill_id != transition.skill_id)
                {
                    return Err(AcquisitionPlanError::TransitionIdMismatch {
                        domain: "skill",
                        id: transition.skill_id,
                    });
                }
                if skills.get(&transition.skill_id).copied() != transition.before {
                    return Err(AcquisitionPlanError::TransitionBeforeMismatch {
                        domain: "skill",
                        id: transition.skill_id,
                    });
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
                        return Err(AcquisitionPlanError::DuplicateOverrideMutation {
                            overridden: pair.0,
                            overriding: pair.1,
                        });
                    }
                } else if !overrides.remove(&pair) {
                    return Err(AcquisitionPlanError::MissingOverrideMutation {
                        overridden: pair.0,
                        overriding: pair.1,
                    });
                }
                replayed_overrides.push(*transition);
            }
        }
    }

    if replayed_spells != plan.spell_transitions {
        return Err(AcquisitionPlanError::TypedProjectionMismatch(
            "spell_transitions",
        ));
    }
    if replayed_skills != plan.skill_transitions {
        return Err(AcquisitionPlanError::TypedProjectionMismatch(
            "skill_transitions",
        ));
    }
    if replayed_overrides != plan.override_transitions {
        return Err(AcquisitionPlanError::TypedProjectionMismatch(
            "override_transitions",
        ));
    }

    let resulting_spells = acquisition_spell_rows(&plan.resulting_snapshot)?;
    let resulting_skills = acquisition_skill_rows(&plan.resulting_snapshot)?;
    let resulting_overrides = acquisition_override_pairs(&plan.resulting_snapshot)?;
    if spells != resulting_spells || skills != resulting_skills || overrides != resulting_overrides
    {
        return Err(AcquisitionPlanError::ResultingSnapshotMismatch);
    }

    // Action causality is checked only after the complete mutation stream has
    // replayed successfully. It may then consume transition evidence by
    // occurrence without trusting malformed typed projections.
    validate_acquisition_actions(plan)?;

    let mut profession_inputs = plan.profession_association_inputs.clone();
    profession_inputs.sort_by_key(|skill| skill.skill_id);
    let mut resulting_skill_rows = plan.resulting_snapshot.skills.clone();
    resulting_skill_rows.sort_by_key(|skill| skill.skill_id);
    if profession_inputs != resulting_skill_rows {
        return Err(AcquisitionPlanError::ProfessionInputsMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
