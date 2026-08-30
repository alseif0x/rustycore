//! SQLx-free World source contract for immutable player-skill rules.

use crate::PersistenceFutureLikeCpp;

pub const SKILL_TIER_VALUE_COUNT_LIKE_CPP: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FishingBaseSkillPersistenceRowLikeCpp {
    pub area_id: u32,
    pub skill: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTierPersistenceRowLikeCpp {
    pub id: u32,
    pub value: [u32; SKILL_TIER_VALUE_COUNT_LIKE_CPP],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillWorldRulesLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// C++ `ObjectMgr` World-table source for immutable skill rules.
///
/// These are two independent startup reads rather than table CRUD: fishing
/// publication requires the already loaded AreaTable authority, while skill
/// tiers are composed later with the effective DB2/Hotfix skill catalog.
pub trait SkillWorldRulesPersistencePortLikeCpp: Send + Sync {
    fn load_fishing_base_skill_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillWorldRulesLoadOutcomeLikeCpp<FishingBaseSkillPersistenceRowLikeCpp>,
    >;

    fn load_skill_tier_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillWorldRulesLoadOutcomeLikeCpp<SkillTierPersistenceRowLikeCpp>,
    >;
}
