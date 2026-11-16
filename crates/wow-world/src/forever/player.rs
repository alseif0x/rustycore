//! Target Player-owned domain state, not a legacy Player or Session mirror.
//! Construction/learning/equipment/save/world admission are still incomplete.
mod enchantment_sources;
mod skills;
mod spell_book;
mod spell_learning_sources;
pub use enchantment_sources::{
    EnchantmentConditionSources, SkillEnchantmentSources, SkillEnchantmentSourcesError,
};
pub use skills::{
    PLAYER_MAX_SKILLS, PlayerSkills, ProfessionItemMove, SkillAbilityMetadata, SkillBonusError,
    SkillCatalog, SkillCriteria, SkillFields, SkillLineMetadata, SkillLoadError, SkillLoadOutcome,
    SkillLoadRow, SkillLoadSource, SkillRaceClassMetadata, SkillRewardSpellMetadata,
    SkillSaveBatch, SkillSaveRow, SkillSetDomainError, SkillSetEffects, SkillSetError,
    SkillSetInputError, SkillSetOutcome, SkillSetSource, SkillSetSources, SkillStatusEntry,
    SkillStatusOrderError, SkillUpdate, SkillUpdateState, SkillValuesSnapshot,
};
pub use spell_book::{
    AddPlayerSpell, CastOverrideAura, CastSpellAuras, CastSpellError, CastSpellResult,
    GenericCastOverrideAura, GenericCastSpellAuras, GenericCastSpellError, GenericCastSpellResult,
    LearnPlayerSpell, PlayerSpellBook, PlayerSpellEntry, PlayerSpellState, PlayerSpellTrait,
    RemovePlayerSpell, SpellBookMessage, SpellBookMutation, SpellBookOrderError,
    SpellLearnCriterion, SpellLearningEffects, SpellLearningError, SpellLearningSourceError,
    SpellLearningSources, SpellLoadRow, SpellSaveAction, SpellSaveBatch, SpellSaveFinishError,
    SpellSavePrepareError,
};
