//! Root and ordered transition provenance checks.

use super::*;

pub(super) fn validate_root(
    root: SpellAcquisitionRootLikeCpp,
) -> Result<(), AcquisitionPlanError> {
    let spell_id = match root {
        SpellAcquisitionRootLikeCpp::DirectLearn(spell_id)
        | SpellAcquisitionRootLikeCpp::TrainerWrapperCast(spell_id) => spell_id,
    };
    if spell_id == 0 || i32::try_from(spell_id).is_err() {
        return Err(AcquisitionPlanError::InvalidSpellId(
            spell_id,
        ));
    }
    Ok(())
}

pub(super) fn validate_provenance(
    provenance: &SpellAcquisitionProvenanceLikeCpp,
    root: SpellAcquisitionRootLikeCpp,
) -> Result<(), AcquisitionPlanError> {
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
            AcquisitionPlanError::ProvenanceMismatch(
                "invalid or root-mismatched transition provenance",
            ),
        );
    }
    Ok(())
}
