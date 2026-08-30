//! SQLx-free Hotfix source contract for the effective skill catalog.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLineHotfixRowLikeCpp {
    pub id: u32,
    pub category_id: i128,
    pub parent_skill_line_id: i128,
    pub parent_tier_index: i128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillLineHotfixRowsLikeCpp {
    pub official: Vec<SkillLineHotfixRowLikeCpp>,
    pub custom: Vec<SkillLineHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLineAbilityHotfixRowLikeCpp {
    pub id: u32,
    pub race_mask: i128,
    pub skill_line: i128,
    pub spell: i128,
    pub min_skill_line_rank: i128,
    pub class_mask: i128,
    pub supercedes_spell: i128,
    pub acquire_method: i128,
    pub trivial_rank_high: i128,
    pub trivial_rank_low: i128,
    pub flags: i128,
    pub num_skill_ups: i128,
    pub skillup_skill_line_id: i128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillRaceClassInfoHotfixRowLikeCpp {
    pub id: u32,
    pub race_mask: i128,
    pub skill_id: i128,
    pub class_mask: i128,
    pub flags: i128,
    pub availability: i128,
    pub min_level: i128,
    pub skill_tier_id: i128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillRelationHotfixRowsLikeCpp {
    pub official_abilities: Vec<SkillLineAbilityHotfixRowLikeCpp>,
    pub official_race_class_infos: Vec<SkillRaceClassInfoHotfixRowLikeCpp>,
    pub custom_abilities: Vec<SkillLineAbilityHotfixRowLikeCpp>,
    pub custom_race_class_infos: Vec<SkillRaceClassInfoHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillCatalogHotfixLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// One Hotfix capability for the cross-indexed C++ skill catalog.
///
/// The two operations are startup stages rather than table CRUD: the final
/// `SkillLine` identity set must exist before the ability/race-class authority
/// can be validated and published.
pub trait SkillCatalogHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_skill_line_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillLineHotfixRowsLikeCpp>,
    >;

    fn load_skill_relation_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillRelationHotfixRowsLikeCpp>,
    >;
}
