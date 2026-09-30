//! Failures in acquisition replay, snapshot shape and action causality.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquisitionPlanError {
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
