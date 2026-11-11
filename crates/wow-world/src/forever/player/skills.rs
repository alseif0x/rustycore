//! Concrete World/Data skill sources; canonical skill state and algorithms live in Entities.
mod sources;
pub use sources::{SkillSetInputError, SkillSetSources};
pub use wow_entities::{
    PLAYER_MAX_SKILLS, PlayerSkills, ProfessionItemMove, SkillAbilityMetadata, SkillBonusError,
    SkillCatalog, SkillCriteria, SkillFields, SkillLineMetadata, SkillRaceClassMetadata,
    SkillRewardSpellMetadata, SkillSetEffects, SkillSetError as SkillSetDomainError,
    SkillSetOutcome, SkillSetSource, SkillStatusEntry, SkillStatusOrderError, SkillUpdate,
    SkillUpdateState,
};
/// Compatibility error for the concrete World/Data source adapter.
pub type SkillSetError<E> = SkillSetDomainError<E, SkillSetInputError>;

#[cfg(test)]
#[path = "../../../unit_tests/forever/player/skills/mod.rs"]
mod tests;
