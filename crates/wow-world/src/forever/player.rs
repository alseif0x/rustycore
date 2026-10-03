//! Target Player-owned domain state, not a legacy Player or Session mirror.
//! Construction/learning/equipment/save/world admission are still incomplete.
mod skills;
mod spell_book;
pub use skills::{
    PLAYER_MAX_SKILLS, PlayerSkills, ProfessionItemMove, SkillBonusError, SkillCriteria,
    SkillFields, SkillSetEffects, SkillSetError, SkillSetInputError, SkillSetOutcome,
    SkillSetSources, SkillUpdate, SkillUpdateState,
};
pub use spell_book::{
    AddPlayerSpell, CastOverrideAura, CastSpellAuras, CastSpellError, CastSpellResult,
    LearnPlayerSpell, PlayerSpellBook, PlayerSpellEntry, PlayerSpellState, PlayerSpellTrait,
    RemovePlayerSpell, SpellBookMessage, SpellBookMutation, SpellBookOrderError,
    SpellLearnCriterion, SpellLearningEffects, SpellLearningError, SpellLearningSourceError,
    SpellLearningSources,
};
