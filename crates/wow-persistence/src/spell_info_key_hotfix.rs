//! SQLx-free Hotfix contract for the exact regular `SpellInfo` key authority.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellInfoKeyContributorLikeCpp {
    SpellEffect,
    SpellAuraOptions,
    SpellAuraRestrictions,
    SpellCastingRequirements,
    SpellCategories,
    SpellClassOptions,
    SpellCooldowns,
    SpellEquippedItems,
    SpellInterrupts,
    SpellLabel,
    SpellLevels,
    SpellMisc,
    SpellPower,
    SpellReagents,
    SpellReagentsCurrency,
    SpellScaling,
    SpellShapeshift,
    SpellTargetRestrictions,
    SpellTotems,
    SpellXSpellVisual,
}

pub const SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP: [SpellInfoKeyContributorLikeCpp; 20] = [
    SpellInfoKeyContributorLikeCpp::SpellEffect,
    SpellInfoKeyContributorLikeCpp::SpellAuraOptions,
    SpellInfoKeyContributorLikeCpp::SpellAuraRestrictions,
    SpellInfoKeyContributorLikeCpp::SpellCastingRequirements,
    SpellInfoKeyContributorLikeCpp::SpellCategories,
    SpellInfoKeyContributorLikeCpp::SpellClassOptions,
    SpellInfoKeyContributorLikeCpp::SpellCooldowns,
    SpellInfoKeyContributorLikeCpp::SpellEquippedItems,
    SpellInfoKeyContributorLikeCpp::SpellInterrupts,
    SpellInfoKeyContributorLikeCpp::SpellLabel,
    SpellInfoKeyContributorLikeCpp::SpellLevels,
    SpellInfoKeyContributorLikeCpp::SpellMisc,
    SpellInfoKeyContributorLikeCpp::SpellPower,
    SpellInfoKeyContributorLikeCpp::SpellReagents,
    SpellInfoKeyContributorLikeCpp::SpellReagentsCurrency,
    SpellInfoKeyContributorLikeCpp::SpellScaling,
    SpellInfoKeyContributorLikeCpp::SpellShapeshift,
    SpellInfoKeyContributorLikeCpp::SpellTargetRestrictions,
    SpellInfoKeyContributorLikeCpp::SpellTotems,
    SpellInfoKeyContributorLikeCpp::SpellXSpellVisual,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellInfoKeyContributorHotfixRowLikeCpp {
    pub record_id: u32,
    pub spell_id: u32,
    pub difficulty_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellInfoKeyContributorHotfixBatchLikeCpp {
    pub contributor: SpellInfoKeyContributorLikeCpp,
    /// Official rows followed by custom rows, preserving C++ replacement order.
    pub rows: Vec<SpellInfoKeyContributorHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellInfoPowerDifficultyHotfixRowLikeCpp {
    pub power_record_id: u32,
    pub difficulty_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellInfoKeyHotfixRowsLikeCpp {
    /// Exactly the twenty C++ contributors in `LoadSpellInfoStore` order.
    pub contributor_batches: Vec<SpellInfoKeyContributorHotfixBatchLikeCpp>,
    /// Official rows followed by custom rows for the SpellPower difficulty join.
    pub power_difficulty_rows: Vec<SpellInfoPowerDifficultyHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellInfoKeyHotfixLoadOutcomeLikeCpp {
    Loaded(SpellInfoKeyHotfixRowsLikeCpp),
    Failed { reason: String },
}

/// Hotfix capability used to compose the exact regular `SpellInfo` key set.
/// It exposes no statement, result-row, pool, transaction, or generic query API.
pub trait SpellInfoKeyHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_spell_info_key_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, SpellInfoKeyHotfixLoadOutcomeLikeCpp>;
}
