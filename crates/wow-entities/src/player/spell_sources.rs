//! Borrowed immutable spell definitions and separately admitted learning facts.
//! The basic definition capability imposes no learning/rank/birth readiness gate.
use super::skill_sources::{SkillLineMetadata, SkillRaceClassMetadata, SkillUpdate};

pub trait SpellBookEffect {
    fn effect_kind(&self) -> u32;
    fn is_aura(&self) -> bool;
    /// Preserve the real effect-kind and aura-kind predicate, including gaps.
    fn is_aura_kind(&self, kind: u32) -> bool;
    fn misc_value_0(&self) -> i32;
    fn class_mask(&self) -> [u32; 4];
}

pub trait SpellBookDefinition {
    type Effect: SpellBookEffect;
    fn spell_id(&self) -> u32;
    fn attributes(&self) -> &[u32; 17];
    fn custom_attributes(&self) -> u32;
    fn stances(&self) -> u64;
    fn caster_aura_state(&self) -> u32;
    fn equipped_item_class(&self) -> i32;
    fn family_name(&self) -> u32;
    fn family_flags(&self) -> [u32; 4];
    /// Physical slot lookup; missing slots and non-aura effects stay distinct.
    fn effect(&self, index: usize) -> Option<Self::Effect>;
    fn effect_count(&self) -> usize;
    /// Physical order including gaps; never an iterator filtered to active effects.
    fn effects(&self) -> impl Iterator<Item = Self::Effect> + '_;
    fn has_effect(&self, kind: u32) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellLearningRank {
    pub first: (u32, i16),
    pub rank: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellLearningSkill {
    pub skill: u16,
    pub step: u16,
    pub value: u16,
    pub max_value: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellLearningNode {
    pub source: u32,
    pub spell: u32,
    pub overrides_spell: u32,
    pub active: bool,
    pub auto_learned: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellLearningAbility {
    pub skill_line: u16,
    pub acquire_method: i32,
    pub trivial_rank_high: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellLearningSourceError<W, D, V> {
    World(W),
    MissingLearningPhases,
    MismatchedBirthCatalog,
    Definition(D),
    Validity(V),
    InvalidBookOrder,
    UndefinedShapeshiftMask,
    UnauthorizedGlobalCleanup,
    MissingAssertedDefinition,
    RecursiveRemovalCycle,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpellLearningError<E, S> {
    Source(S),
    Effect(E),
}

/// All facts come from the same frozen admitted owners. None on collection
/// queries means an unavailable phase; Some(empty) means an absent key after
/// admission. Keep required, forward/reverse learn and ability orders distinct.
pub trait SpellLearningSource: SpellDefinitionSource {
    type WorldError;
    type ValidityError;
    fn spell_is_valid(&self, spell: u32) -> Result<bool, Self::ValidityError>;
    fn spell_rank_node(&self, spell: u32) -> Option<SpellLearningRank>;
    fn previous_spell_in_chain(&self, spell: u32) -> u32;
    fn next_spell_in_chain(&self, spell: u32) -> u32;
    fn first_spell_in_chain(&self, spell: u32) -> u32;
    fn spells_requiring(&self, spell: u32) -> Option<impl Iterator<Item = u32> + '_>;
    fn spell_learn_nodes(&self, spell: u32)
    -> Option<impl Iterator<Item = SpellLearningNode> + '_>;
    fn spell_learned_by(&self, spell: u32) -> Option<impl Iterator<Item = SpellLearningNode> + '_>;
    fn skill_line_abilities(
        &self,
        spell: u32,
    ) -> Option<impl Iterator<Item = SpellLearningAbility> + '_>;
    fn spell_learn_skill(&self, spell: u32) -> Option<SpellLearningSkill>;
    fn skill_line(&self, skill: u32) -> Option<SkillLineMetadata>;
    fn skill_race_class_info(
        &self,
        skill: u32,
    ) -> Result<Option<SkillRaceClassMetadata>, Self::WorldError>;
    fn skill_tier_value(&self, tier: u32, index: u32) -> Option<u32>;
    /// Reuse the admitted recipe by RC identity at the caller's live level.
    fn default_skill_request(&self, rc: SkillRaceClassMetadata, level: u8) -> Option<SkillUpdate>;
}

pub trait SpellDefinitionSource {
    type Error;
    type Definition<'a>: SpellBookDefinition
    where
        Self: 'a;

    /// Actual difficulty fallback, absence and errors belong to this authority.
    fn definition(
        &self,
        spell: u32,
        difficulty: i16,
    ) -> Result<Option<Self::Definition<'_>>, Self::Error>;
    /// Same derived-definition authority, not merely equal IDs or shared raw data.
    fn owns<'a>(&self, definition: &Self::Definition<'a>) -> bool
    where
        Self: 'a;
}
