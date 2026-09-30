//! Preserve the application boundary's existing error variants and payloads.

use super::*;

impl From<AcquisitionPlanError> for PlayerSpellAcquisitionPrepareErrorLikeCpp {
    fn from(error: AcquisitionPlanError) -> Self {
        match error {
            AcquisitionPlanError::InvalidSpellId(id) => Self::InvalidSpellId(id),
            AcquisitionPlanError::InvalidSkillId(id) => Self::InvalidSkillId(id),
            AcquisitionPlanError::InvalidTraitDefinitionId(id) => Self::InvalidTraitDefinitionId(id),
            AcquisitionPlanError::InvalidProfessionAssociation(value) => Self::InvalidProfessionAssociation(value),
            AcquisitionPlanError::ConflictingProfessionAssociation(slot) => Self::ConflictingProfessionAssociation(slot),
            AcquisitionPlanError::DuplicateSpell(id) => Self::DuplicateSpell(id),
            AcquisitionPlanError::DuplicateSkill(id) => Self::DuplicateSkill(id),
            AcquisitionPlanError::DuplicateOverride(overridden, overriding) => Self::DuplicateOverride(overridden, overriding),
            AcquisitionPlanError::TransitionBeforeMismatch { domain, id } => Self::TransitionBeforeMismatch { domain, id },
            AcquisitionPlanError::TransitionIdMismatch { domain, id } => Self::TransitionIdMismatch { domain, id },
            AcquisitionPlanError::DuplicateOverrideMutation { overridden, overriding } => Self::DuplicateOverrideMutation { overridden, overriding },
            AcquisitionPlanError::MissingOverrideMutation { overridden, overriding } => Self::MissingOverrideMutation { overridden, overriding },
            AcquisitionPlanError::TypedProjectionMismatch(domain) => Self::TypedProjectionMismatch(domain),
            AcquisitionPlanError::ResultingSnapshotMismatch => Self::ResultingSnapshotMismatch,
            AcquisitionPlanError::SnapshotIdentityChanged(field) => Self::SnapshotIdentityChanged(field),
            AcquisitionPlanError::SkillOccupancyMismatch => Self::SkillOccupancyMismatch,
            AcquisitionPlanError::InvalidDeletedSkill(id) => Self::InvalidDeletedSkill(id),
            AcquisitionPlanError::InvalidNonDurableSkillTombstone(id) => Self::InvalidNonDurableSkillTombstone(id),
            AcquisitionPlanError::ProfessionInputsMismatch => Self::ProfessionInputsMismatch,
            AcquisitionPlanError::ProfessionPlanMismatch(reason) => Self::ProfessionPlanMismatch(reason),
            AcquisitionPlanError::InvalidPostCommitAction { domain, id } => Self::InvalidPostCommitAction { domain, id },
            AcquisitionPlanError::PostCommitActionCausalityMismatch { action, id } => Self::PostCommitActionCausalityMismatch { action, id },
            AcquisitionPlanError::LearnedActionRowMismatch(id) => Self::LearnedActionRowMismatch(id),
            AcquisitionPlanError::ProvenanceMismatch(reason) => Self::ProvenanceMismatch(reason),
        }
    }
}
