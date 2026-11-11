//! Target spell-book domain lives in Entities; these aliases retain World views.
pub use super::spell_learning_sources::{
    SpellLearningError, SpellLearningSourceError, SpellLearningSources,
};
pub use wow_entities::{
    AddPlayerSpell, CastOverrideAura as GenericCastOverrideAura,
    CastSpellAuras as GenericCastSpellAuras, CastSpellError as GenericCastSpellError,
    CastSpellResult as GenericCastSpellResult, LearnPlayerSpell, PlayerSpellBook, PlayerSpellEntry,
    PlayerSpellState, PlayerSpellTrait, RemovePlayerSpell, SpellBookMessage, SpellBookMutation,
    SpellBookOrderError, SpellLearnCriterion, SpellLearningEffects,
};
pub type CastOverrideAura<'a> =
    GenericCastOverrideAura<'a, crate::forever::spells::SpellDefinitionView<'a>>;
pub type CastSpellAuras<'a> =
    GenericCastSpellAuras<'a, crate::forever::spells::SpellDefinitionView<'a>>;
pub type CastSpellResult<'a> =
    GenericCastSpellResult<crate::forever::spells::SpellDefinitionView<'a>>;
pub type CastSpellError<E> = GenericCastSpellError<E, crate::forever::spells::SpellDefinitionError>;

#[cfg(test)]
#[path = "../../../unit_tests/forever/player/spell_book/mod.rs"]
mod tests;
