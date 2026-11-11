//! Pure immutable skill facts and source capabilities for one canonical skill owner.
//! Catalogs are frozen for each operation. Child/ability/default iteration orders
//! are distinct source contracts, not interchangeable sorted collections.
//! None means a genuinely absent admitted row; unavailable full sources return errors.
//! Real adapters validate class 1..=15 before complete operations and supply full
//! sources/default-request rows from the same immutable owner. Basic queries impose
//! no rank, race/class lookup or default-readiness requirement.

/// A source SetSkill input, not proof of an applied or learned skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillUpdate {
    pub skill: u32,
    pub step: u16,
    pub value: u16,
    pub maximum: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkillSetError<E, S> {
    Effect(E),
    Source(S),
    RecursiveSkillCycle,
}
#[derive(Clone, Copy)]
pub struct SkillLineMetadata {
    pub id: u32,
    pub category: i8,
    pub parent_skill: u32,
    pub parent_tier_index: i32,
}

#[derive(Clone, Copy)]
pub struct SkillAbilityMetadata {
    pub id: u32,
    pub skill_line: u16,
    pub spell: i32,
    pub min_skill_rank: i16,
    pub class_mask: i32,
    pub acquire_method: i32,
    pub race_mask: u64,
}

#[derive(Clone, Copy)]
pub struct SkillRaceClassMetadata {
    pub id: u32,
    pub skill: u16,
    pub flags: i32,
    pub min_level: i8,
    pub tier: i16,
}

#[derive(Clone, Copy)]
pub struct SkillRewardSpellMetadata {
    pub future_condition_id: u32,
    pub spell_level: u32,
    pub base_level: u32,
}

/// Admitted immutable catalog only: basic operations require no rank/RC readiness.
pub trait SkillCatalog {
    fn skill_line(&self, id: u32) -> Option<SkillLineMetadata>;
    /// Borrowed lazy traversal, preserving the catalog's child order.
    fn child_lines(&self, parent: u32) -> impl Iterator<Item = SkillLineMetadata> + '_;
    /// Borrowed lazy traversal, preserving the catalog's ability order.
    fn abilities_for_skill(&self, skill: u32) -> impl Iterator<Item = SkillAbilityMetadata> + '_;
}

/// Full SetSkill/default/reward source, admitted separately from the basic catalog.
pub trait SkillSetSource: SkillCatalog {
    type Error;
    fn race(&self) -> u8;
    fn class(&self) -> u8;
    fn default_skill_records(
        &self,
    ) -> Result<impl Iterator<Item = SkillRaceClassMetadata> + '_, Self::Error>;
    fn skill_race_class_info(
        &self,
        skill: u32,
    ) -> Result<Option<SkillRaceClassMetadata>, Self::Error>;
    fn skill_tier_value(&self, tier: u32, index: u32) -> Option<u32>;
    /// Query at the live level; do not precompute/deduplicate default requests.
    fn default_skill_request(&self, rc: SkillRaceClassMetadata, level: u8) -> Option<SkillUpdate>;
    fn reward_spell(&self, id: u32) -> Result<Option<SkillRewardSpellMetadata>, Self::Error>;
    fn first_spell_in_chain(&self, id: u32) -> u32;
    fn race_in_mask(&self, mask: u64, race: u32) -> bool;
}
