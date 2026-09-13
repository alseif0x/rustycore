//! SQLx-free Hotfix source contract for the effective skill catalog.

use crate::PersistenceFutureLikeCpp;

/// Trait DB2 tables mirrored by the C++ hotfix loader.  The table identity is
/// carried with every row so composition cannot accidentally apply a row to a
/// different store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitCatalogHotfixTableLikeCpp {
    SpecSetMember,
    TraitCurrencySourceLocale,
    TraitCond,
    TraitCost,
    TraitCurrency,
    TraitCurrencySource,
    TraitDefinition,
    TraitDefinitionLocale,
    TraitDefinitionEffectPoints,
    TraitEdge,
    TraitNode,
    TraitNodeEntry,
    TraitNodeEntryXTraitCond,
    TraitNodeEntryXTraitCost,
    TraitNodeGroup,
    TraitNodeGroupXTraitCond,
    TraitNodeGroupXTraitCost,
    TraitNodeGroupXTraitNode,
    TraitNodeXTraitCond,
    TraitNodeXTraitCost,
    TraitNodeXTraitNodeEntry,
    TraitTree,
    TraitTreeLoadout,
    TraitTreeLoadoutEntry,
    TraitTreeXTraitCost,
    TraitTreeXTraitCurrency,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraitCatalogHotfixValueLikeCpp {
    Integer(i128),
    Real(f64),
    Text(String),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitCatalogHotfixRowLikeCpp {
    pub table: TraitCatalogHotfixTableLikeCpp,
    pub values: Vec<TraitCatalogHotfixValueLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraitCatalogHotfixRowsLikeCpp {
    pub official: Vec<TraitCatalogHotfixRowLikeCpp>,
    pub custom: Vec<TraitCatalogHotfixRowLikeCpp>,
}

/// A locale-specific C++ `PREPARE_LOCALE_STMT` row. The table identity and
/// locale remain explicit so a localized row cannot be applied to a base store
/// or a different client locale by accident.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitCatalogLocaleHotfixRowLikeCpp {
    pub table: TraitCatalogHotfixTableLikeCpp,
    pub locale: String,
    pub values: Vec<TraitCatalogHotfixValueLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraitCatalogLocaleHotfixRowsLikeCpp {
    pub official: Vec<TraitCatalogLocaleHotfixRowLikeCpp>,
    pub custom: Vec<TraitCatalogLocaleHotfixRowLikeCpp>,
}

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
pub struct SkillLineAbilityHotfixRowsLikeCpp {
    pub official: Vec<SkillLineAbilityHotfixRowLikeCpp>,
    pub custom: Vec<SkillLineAbilityHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillRaceClassInfoHotfixRowsLikeCpp {
    pub official: Vec<SkillRaceClassInfoHotfixRowLikeCpp>,
    pub custom: Vec<SkillRaceClassInfoHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLineXTraitTreeHotfixRowLikeCpp {
    pub id: u32,
    pub skill_line_id: u32,
    pub trait_tree_id: i128,
    pub order_index: i128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillLineXTraitTreeHotfixRowsLikeCpp {
    pub official: Vec<SkillLineXTraitTreeHotfixRowLikeCpp>,
    pub custom: Vec<SkillLineXTraitTreeHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillCatalogHotfixLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// Hotfix capabilities for the cross-indexed C++ skill catalog.
///
/// These operations are startup stages rather than table CRUD: the final
/// `SkillLine` identity set must exist before the relation authorities can be
/// validated and published, and each relation table remains independently
/// observable so its C++ order and failure boundary are preserved.
pub trait SkillCatalogHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_trait_catalog_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<TraitCatalogHotfixRowsLikeCpp>,
    >;

    /// Load the locale statements emitted by C++ `PREPARE_LOCALE_STMT` for the
    /// requested database locale. Implementors without locale SQL support may
    /// return an empty set; production MariaDB supplies the concrete rows.
    fn load_trait_catalog_locale_hotfix_rows_like_cpp(
        &self,
        _locale: &str,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<TraitCatalogLocaleHotfixRowsLikeCpp>,
    > {
        Box::pin(async {
            SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(
                TraitCatalogLocaleHotfixRowsLikeCpp::default(),
            )
        })
    }

    fn load_skill_line_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillLineHotfixRowsLikeCpp>,
    >;

    fn load_skill_line_ability_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillLineAbilityHotfixRowsLikeCpp>,
    >;

    fn load_skill_line_x_trait_tree_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillLineXTraitTreeHotfixRowsLikeCpp>,
    >;

    fn load_skill_race_class_info_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillRaceClassInfoHotfixRowsLikeCpp>,
    >;
}
