//! Explicit admitted test answers, not startup loaders or gameplay recipes.
//! Unknown spell/skill/tier/default queries panic instead of inventing absence.
mod effects;
mod facts;
use super::*;
use crate::{
    SkillLineMetadata, SkillRaceClassMetadata, SkillUpdate, SpellBookDefinition, SpellBookEffect,
    SpellDefinitionSource, SpellLearningAbility, SpellLearningNode, SpellLearningRank,
    SpellLearningSkill, SpellLearningSource,
};
pub(super) use effects::{Effects, Event};
use std::convert::Infallible;

pub(super) type SpellLearningSourceError =
    crate::SpellLearningSourceError<Infallible, Infallible, Infallible>;
pub(super) type SpellLearningError<E> = crate::SpellLearningError<E, SpellLearningSourceError>;

pub(super) struct EffectFacts {
    pub kind: u32,
    pub aura: bool,
    pub aura_kinds: Vec<(u32, bool)>,
    pub misc: i32,
    pub mask: [u32; 4],
}
pub(super) struct DefinitionFacts {
    pub id: u32,
    pub attributes: [u32; 17],
    pub custom: u32,
    pub stances: u64,
    pub caster_state: u32,
    pub equipped_class: i32,
    pub family: u32,
    pub family_flags: [u32; 4],
    pub effects: Vec<EffectFacts>,
    pub has_effect: Vec<(u32, bool)>,
}
impl DefinitionFacts {
    /// One explicitly plain admitted definition; no classifier or DB2 projection.
    pub fn regular(id: u32) -> Self {
        Self {
            id,
            attributes: [0; 17],
            custom: 0,
            stances: 0,
            caster_state: 0,
            equipped_class: -1,
            family: 0,
            family_flags: [0; 4],
            effects: vec![],
            has_effect: vec![(36, false), (44, false), (155, false), (40, false)],
        }
    }
}
pub(super) struct SpellFacts {
    pub id: u32,
    pub definition: Option<DefinitionFacts>,
    pub valid: bool,
    pub rank: Option<SpellLearningRank>,
    pub previous: u32,
    pub next: u32,
    pub first: u32,
    pub required: Vec<u32>,
    pub forward: Vec<SpellLearningNode>,
    pub reverse: Vec<SpellLearningNode>,
    pub abilities: Vec<SpellLearningAbility>,
    pub learned_skill: Option<SpellLearningSkill>,
}
impl SpellFacts {
    /// Fully admitted unranked/plain answer row. `first` is supplied literally;
    /// empty relations and no learn-skill are explicit answers for this scenario.
    pub fn plain(id: u32, first: u32) -> Self {
        Self {
            id,
            definition: Some(DefinitionFacts::regular(id)),
            valid: true,
            rank: None,
            previous: 0,
            next: 0,
            first,
            required: vec![],
            forward: vec![],
            reverse: vec![],
            abilities: vec![],
            learned_skill: None,
        }
    }
}
pub(super) struct Fixture {
    pub spells: Vec<SpellFacts>,
    pub lines: Vec<(u32, Option<SkillLineMetadata>)>,
    pub race_class: Vec<(u32, Option<SkillRaceClassMetadata>)>,
    pub tiers: Vec<((u32, u32), Option<u32>)>,
    pub defaults: Vec<((u32, u8), Option<SkillUpdate>)>,
}
impl Fixture {
    pub fn sources(&self) -> Sources<'_> {
        Sources(self)
    }
}
pub(super) struct Sources<'a>(&'a Fixture);
impl Sources<'_> {
    fn spell(&self, id: u32) -> &SpellFacts {
        self.0
            .spells
            .iter()
            .find(|row| row.id == id)
            .expect("explicit spell answer")
    }
}
// These views deliberately have no Clone/Copy implementation.
pub(super) struct Definition<'a>(&'a DefinitionFacts);
pub(super) struct Effect<'a>(&'a EffectFacts);

pub(super) fn insert(
    book: &mut PlayerSpellBook,
    id: u32,
    state: PlayerSpellState,
    active: bool,
    disabled: bool,
) {
    *book.try_emplace(id).0 = PlayerSpellEntry {
        state,
        active,
        disabled,
        ..Default::default()
    };
}
