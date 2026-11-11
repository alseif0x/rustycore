//! Target Player-owned domain state, not a legacy Player or Session mirror.
//! Construction/learning/equipment/save/world admission are still incomplete.
mod skills;
mod spell_book;
mod spell_learning_sources;
pub use skills::{
    PLAYER_MAX_SKILLS, PlayerSkills, ProfessionItemMove, SkillAbilityMetadata, SkillBonusError,
    SkillCatalog, SkillCriteria, SkillFields, SkillLineMetadata, SkillRaceClassMetadata,
    SkillRewardSpellMetadata, SkillSetDomainError, SkillSetEffects, SkillSetError,
    SkillSetInputError, SkillSetOutcome, SkillSetSource, SkillSetSources, SkillStatusEntry,
    SkillStatusOrderError, SkillUpdate, SkillUpdateState,
};
pub use spell_book::{
    AddPlayerSpell, CastOverrideAura, CastSpellAuras, CastSpellError, CastSpellResult,
    GenericCastOverrideAura, GenericCastSpellAuras, GenericCastSpellError, GenericCastSpellResult,
    LearnPlayerSpell, PlayerSpellBook, PlayerSpellEntry, PlayerSpellState, PlayerSpellTrait,
    RemovePlayerSpell, SpellBookMessage, SpellBookMutation, SpellBookOrderError,
    SpellLearnCriterion, SpellLearningEffects, SpellLearningError, SpellLearningSourceError,
    SpellLearningSources,
};
