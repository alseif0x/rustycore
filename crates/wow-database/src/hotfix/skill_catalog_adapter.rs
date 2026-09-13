//! MariaDB adapter for the C++ effective skill-catalog Hotfix overlays.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use wow_persistence::{
    PersistenceFutureLikeCpp, SkillCatalogHotfixLoadOutcomeLikeCpp,
    SkillCatalogHotfixPersistencePortLikeCpp, SkillLineAbilityHotfixRowLikeCpp,
    SkillLineAbilityHotfixRowsLikeCpp, SkillLineHotfixRowLikeCpp, SkillLineHotfixRowsLikeCpp,
    SkillLineXTraitTreeHotfixRowLikeCpp, SkillLineXTraitTreeHotfixRowsLikeCpp,
    SkillRaceClassInfoHotfixRowLikeCpp, SkillRaceClassInfoHotfixRowsLikeCpp,
    TraitCatalogHotfixRowLikeCpp, TraitCatalogHotfixRowsLikeCpp, TraitCatalogHotfixTableLikeCpp,
    TraitCatalogHotfixValueLikeCpp, TraitCatalogLocaleHotfixRowLikeCpp,
    TraitCatalogLocaleHotfixRowsLikeCpp,
};

use crate::{HotfixDatabase, HotfixStatements, SqlResult};

const OFFICIAL_THEN_CUSTOM_LIKE_CPP: [bool; 2] = [true, false];

const TRAIT_CATALOG_TABLES_LIKE_CPP: &[(
    TraitCatalogHotfixTableLikeCpp,
    HotfixStatements,
    usize,
)] = &[
    (
        TraitCatalogHotfixTableLikeCpp::SpecSetMember,
        HotfixStatements::SEL_SPEC_SET_MEMBER,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitCond,
        HotfixStatements::SEL_TRAIT_COND,
        15,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitCost,
        HotfixStatements::SEL_TRAIT_COST,
        4,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitCurrency,
        HotfixStatements::SEL_TRAIT_CURRENCY,
        5,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitCurrencySource,
        HotfixStatements::SEL_TRAIT_CURRENCY_SOURCE,
        9,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitDefinition,
        HotfixStatements::SEL_TRAIT_DEFINITION,
        8,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitDefinitionEffectPoints,
        HotfixStatements::SEL_TRAIT_DEFINITION_EFFECT_POINTS,
        5,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitEdge,
        HotfixStatements::SEL_TRAIT_EDGE,
        5,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNode,
        HotfixStatements::SEL_TRAIT_NODE,
        6,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeEntry,
        HotfixStatements::SEL_TRAIT_NODE_ENTRY,
        4,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeEntryXTraitCond,
        HotfixStatements::SEL_TRAIT_NODE_ENTRY_X_TRAIT_COND,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeEntryXTraitCost,
        HotfixStatements::SEL_TRAIT_NODE_ENTRY_X_TRAIT_COST,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeGroup,
        HotfixStatements::SEL_TRAIT_NODE_GROUP,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeGroupXTraitCond,
        HotfixStatements::SEL_TRAIT_NODE_GROUP_X_TRAIT_COND,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeGroupXTraitCost,
        HotfixStatements::SEL_TRAIT_NODE_GROUP_X_TRAIT_COST,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeGroupXTraitNode,
        HotfixStatements::SEL_TRAIT_NODE_GROUP_X_TRAIT_NODE,
        4,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeXTraitCond,
        HotfixStatements::SEL_TRAIT_NODE_X_TRAIT_COND,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeXTraitCost,
        HotfixStatements::SEL_TRAIT_NODE_X_TRAIT_COST,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitNodeXTraitNodeEntry,
        HotfixStatements::SEL_TRAIT_NODE_X_TRAIT_NODE_ENTRY,
        4,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitTree,
        HotfixStatements::SEL_TRAIT_TREE,
        8,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitTreeLoadout,
        HotfixStatements::SEL_TRAIT_TREE_LOADOUT,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitTreeLoadoutEntry,
        HotfixStatements::SEL_TRAIT_TREE_LOADOUT_ENTRY,
        6,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitTreeXTraitCost,
        HotfixStatements::SEL_TRAIT_TREE_X_TRAIT_COST,
        3,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitTreeXTraitCurrency,
        HotfixStatements::SEL_TRAIT_TREE_X_TRAIT_CURRENCY,
        4,
    ),
];

const TRAIT_CATALOG_LOCALE_TABLES_LIKE_CPP: &[(
    TraitCatalogHotfixTableLikeCpp,
    HotfixStatements,
    usize,
)] = &[
    (
        TraitCatalogHotfixTableLikeCpp::TraitCurrencySourceLocale,
        HotfixStatements::SEL_TRAIT_CURRENCY_SOURCE_LOCALE,
        2,
    ),
    (
        TraitCatalogHotfixTableLikeCpp::TraitDefinitionLocale,
        HotfixStatements::SEL_TRAIT_DEFINITION_LOCALE,
        4,
    ),
];

fn read_integer_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<i128> {
    result
        .try_read::<i64>(column)
        .map(i128::from)
        .or_else(|| result.try_read::<u64>(column).map(i128::from))
        .or_else(|| result.try_read::<i32>(column).map(i128::from))
        .or_else(|| result.try_read::<u32>(column).map(i128::from))
        .or_else(|| result.try_read::<i16>(column).map(i128::from))
        .or_else(|| result.try_read::<u16>(column).map(i128::from))
        .or_else(|| result.try_read::<i8>(column).map(i128::from))
        .or_else(|| result.try_read::<u8>(column).map(i128::from))
        .with_context(|| format!("missing or non-integer {field} SQL column {column}"))
}

fn id_like_cpp(value: i128, field: &'static str) -> Result<u32> {
    u32::try_from(value).with_context(|| format!("{field} SQL value {value} is not u32"))
}

fn trait_catalog_value_like_cpp(
    result: &SqlResult,
    column: usize,
) -> Result<TraitCatalogHotfixValueLikeCpp> {
    if result.is_null(column) {
        return Ok(TraitCatalogHotfixValueLikeCpp::Null);
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return Ok(TraitCatalogHotfixValueLikeCpp::Integer(i128::from(value)));
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return Ok(TraitCatalogHotfixValueLikeCpp::Integer(i128::from(value)));
    }
    if let Some(value) = result.try_read::<f64>(column) {
        return Ok(TraitCatalogHotfixValueLikeCpp::Real(value));
    }
    if let Some(value) = result.try_read::<String>(column) {
        return Ok(TraitCatalogHotfixValueLikeCpp::Text(value));
    }
    if let Some(value) = result.try_read::<Vec<u8>>(column) {
        return Ok(TraitCatalogHotfixValueLikeCpp::Text(
            String::from_utf8_lossy(&value).into_owned(),
        ));
    }
    bail!("unsupported Trait catalog SQL value at column {column}")
}

fn skill_line_values_like_cpp(values: [i128; 4]) -> Result<SkillLineHotfixRowLikeCpp> {
    Ok(SkillLineHotfixRowLikeCpp {
        id: id_like_cpp(values[0], "SkillLine.ID")?,
        category_id: values[1],
        parent_skill_line_id: values[2],
        parent_tier_index: values[3],
    })
}

fn skill_line_row_like_cpp(result: &SqlResult) -> Result<SkillLineHotfixRowLikeCpp> {
    skill_line_values_like_cpp([
        read_integer_checked_like_cpp(result, 0, "SkillLine.ID")?,
        read_integer_checked_like_cpp(result, 1, "SkillLine.CategoryID")?,
        read_integer_checked_like_cpp(result, 2, "SkillLine.ParentSkillLineID")?,
        read_integer_checked_like_cpp(result, 3, "SkillLine.ParentTierIndex")?,
    ])
}

fn skill_line_ability_values_like_cpp(
    values: [i128; 13],
) -> Result<SkillLineAbilityHotfixRowLikeCpp> {
    Ok(SkillLineAbilityHotfixRowLikeCpp {
        race_mask: values[0],
        id: id_like_cpp(values[1], "SkillLineAbility.ID")?,
        skill_line: values[2],
        spell: values[3],
        min_skill_line_rank: values[4],
        class_mask: values[5],
        supercedes_spell: values[6],
        acquire_method: values[7],
        trivial_rank_high: values[8],
        trivial_rank_low: values[9],
        flags: values[10],
        num_skill_ups: values[11],
        skillup_skill_line_id: values[12],
    })
}

fn skill_line_ability_row_like_cpp(result: &SqlResult) -> Result<SkillLineAbilityHotfixRowLikeCpp> {
    skill_line_ability_values_like_cpp([
        read_integer_checked_like_cpp(result, 0, "SkillLineAbility.RaceMask")?,
        read_integer_checked_like_cpp(result, 1, "SkillLineAbility.ID")?,
        read_integer_checked_like_cpp(result, 2, "SkillLineAbility.SkillLine")?,
        read_integer_checked_like_cpp(result, 3, "SkillLineAbility.Spell")?,
        read_integer_checked_like_cpp(result, 4, "SkillLineAbility.MinSkillLineRank")?,
        read_integer_checked_like_cpp(result, 5, "SkillLineAbility.ClassMask")?,
        read_integer_checked_like_cpp(result, 6, "SkillLineAbility.SupercedesSpell")?,
        read_integer_checked_like_cpp(result, 7, "SkillLineAbility.AcquireMethod")?,
        read_integer_checked_like_cpp(result, 8, "SkillLineAbility.TrivialSkillLineRankHigh")?,
        read_integer_checked_like_cpp(result, 9, "SkillLineAbility.TrivialSkillLineRankLow")?,
        read_integer_checked_like_cpp(result, 10, "SkillLineAbility.Flags")?,
        read_integer_checked_like_cpp(result, 11, "SkillLineAbility.NumSkillUps")?,
        read_integer_checked_like_cpp(result, 14, "SkillLineAbility.SkillupSkillLineID")?,
    ])
}

fn skill_race_class_info_values_like_cpp(
    values: [i128; 8],
) -> Result<SkillRaceClassInfoHotfixRowLikeCpp> {
    Ok(SkillRaceClassInfoHotfixRowLikeCpp {
        id: id_like_cpp(values[0], "SkillRaceClassInfo.ID")?,
        race_mask: values[1],
        skill_id: values[2],
        class_mask: values[3],
        flags: values[4],
        availability: values[5],
        min_level: values[6],
        skill_tier_id: values[7],
    })
}

fn skill_line_x_trait_tree_values_like_cpp(
    values: [i128; 4],
) -> Result<SkillLineXTraitTreeHotfixRowLikeCpp> {
    Ok(SkillLineXTraitTreeHotfixRowLikeCpp {
        id: id_like_cpp(values[0], "SkillLineXTraitTree.ID")?,
        skill_line_id: id_like_cpp(values[1], "SkillLineXTraitTree.SkillLineID")?,
        trait_tree_id: values[2],
        order_index: values[3],
    })
}

pub struct MariaDbSkillCatalogHotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbSkillCatalogHotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl SkillCatalogHotfixPersistencePortLikeCpp
    for MariaDbSkillCatalogHotfixPersistenceAdapterLikeCpp
{
    fn load_trait_catalog_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<TraitCatalogHotfixRowsLikeCpp>,
    > {
        Box::pin(async move {
            let loaded = async {
                let mut batches = [Vec::new(), Vec::new()];
                for (table, statement_kind, expected_columns) in TRAIT_CATALOG_TABLES_LIKE_CPP {
                    for (batch_index, official) in OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate() {
                        let mut statement = self.hotfix_db.prepare(*statement_kind);
                        statement.set_bool(0, official);
                        let mut rows = self.hotfix_db.query(&statement).await?;
                        if rows.is_empty() {
                            continue;
                        }
                        loop {
                            if rows.field_count() != *expected_columns {
                                bail!("Trait catalog statement for {table:?} returned {} columns, expected {expected_columns}", rows.field_count());
                            }
                            let values = (0..*expected_columns)
                                .map(|column| trait_catalog_value_like_cpp(&rows, column))
                                .collect::<Result<Vec<_>>>()?;
                            batches[batch_index].push(TraitCatalogHotfixRowLikeCpp {
                                table: *table,
                                values,
                            });
                            if !rows.next_row() {
                                break;
                            }
                        }
                    }
                }
                let [official, custom] = batches;
                Ok::<_, anyhow::Error>(TraitCatalogHotfixRowsLikeCpp { official, custom })
            }
            .await;
            match loaded {
                Ok(rows) => SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => SkillCatalogHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_trait_catalog_locale_hotfix_rows_like_cpp(
        &self,
        locale: &str,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<TraitCatalogLocaleHotfixRowsLikeCpp>,
    > {
        let locale = locale.to_owned();
        Box::pin(async move {
            let loaded = async {
                let mut batches = [Vec::new(), Vec::new()];
                for (table, statement_kind, expected_columns) in
                    TRAIT_CATALOG_LOCALE_TABLES_LIKE_CPP
                {
                    for (batch_index, official) in
                        OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate()
                    {
                        let mut statement = self.hotfix_db.prepare(*statement_kind);
                        statement.set_bool(0, official);
                        statement.set_string(1, &locale);
                        let mut rows = self.hotfix_db.query(&statement).await?;
                        if rows.is_empty() {
                            continue;
                        }
                        loop {
                            if rows.field_count() != *expected_columns {
                                bail!(
                                    "Trait locale statement for {table:?} returned {} columns, expected {expected_columns}",
                                    rows.field_count()
                                );
                            }
                            let values = (0..*expected_columns)
                                .map(|column| trait_catalog_value_like_cpp(&rows, column))
                                .collect::<Result<Vec<_>>>()?;
                            batches[batch_index].push(TraitCatalogLocaleHotfixRowLikeCpp {
                                table: *table,
                                locale: locale.clone(),
                                values,
                            });
                            if !rows.next_row() {
                                break;
                            }
                        }
                    }
                }
                let [official, custom] = batches;
                Ok::<_, anyhow::Error>(TraitCatalogLocaleHotfixRowsLikeCpp { official, custom })
            }
            .await;
            match loaded {
                Ok(rows) => SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => SkillCatalogHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_skill_line_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillLineHotfixRowsLikeCpp>,
    > {
        Box::pin(async move {
            let loaded = async {
                let mut batches = [Vec::new(), Vec::new()];
                for (batch_index, official) in OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate()
                {
                    let mut statement = self.hotfix_db.prepare(HotfixStatements::SEL_SKILL_LINE);
                    statement.set_bool(0, official);
                    let mut rows = self.hotfix_db.query(&statement).await?;
                    if rows.is_empty() {
                        continue;
                    }
                    loop {
                        batches[batch_index].push(skill_line_row_like_cpp(&rows)?);
                        if !rows.next_row() {
                            break;
                        }
                    }
                }
                let [official, custom] = batches;
                Ok::<_, anyhow::Error>(SkillLineHotfixRowsLikeCpp { official, custom })
            }
            .await;
            match loaded {
                Ok(rows) => SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => SkillCatalogHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_skill_line_ability_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillLineAbilityHotfixRowsLikeCpp>,
    > {
        Box::pin(async move {
            let loaded = async {
                let mut ability_batches = [Vec::new(), Vec::new()];
                for (batch_index, official) in OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate()
                {
                    let mut statement = self
                        .hotfix_db
                        .prepare(HotfixStatements::SEL_SKILL_LINE_ABILITY);
                    statement.set_bool(0, official);
                    let mut rows = self.hotfix_db.query(&statement).await?;
                    if !rows.is_empty() {
                        loop {
                            ability_batches[batch_index]
                                .push(skill_line_ability_row_like_cpp(&rows)?);
                            if !rows.next_row() {
                                break;
                            }
                        }
                    }
                }

                let [official, custom] = ability_batches;
                Ok::<_, anyhow::Error>(SkillLineAbilityHotfixRowsLikeCpp { official, custom })
            }
            .await;
            match loaded {
                Ok(rows) => SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => SkillCatalogHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_skill_line_x_trait_tree_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillLineXTraitTreeHotfixRowsLikeCpp>,
    > {
        Box::pin(async move {
            let loaded = async {
                let mut batches = [Vec::new(), Vec::new()];
                for (batch_index, official) in OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate()
                {
                    let mut statement = self
                        .hotfix_db
                        .prepare(HotfixStatements::SEL_SKILL_LINE_X_TRAIT_TREE);
                    statement.set_bool(0, official);
                    let mut rows = self.hotfix_db.query(&statement).await?;
                    if !rows.is_empty() {
                        loop {
                            batches[batch_index].push(skill_line_x_trait_tree_values_like_cpp([
                                read_integer_checked_like_cpp(&rows, 0, "SkillLineXTraitTree.ID")?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    1,
                                    "SkillLineXTraitTree.SkillLineID",
                                )?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    2,
                                    "SkillLineXTraitTree.TraitTreeID",
                                )?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    3,
                                    "SkillLineXTraitTree.OrderIndex",
                                )?,
                            ])?);
                            if !rows.next_row() {
                                break;
                            }
                        }
                    }
                }
                let [official, custom] = batches;
                Ok::<_, anyhow::Error>(SkillLineXTraitTreeHotfixRowsLikeCpp { official, custom })
            }
            .await;
            match loaded {
                Ok(rows) => SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => SkillCatalogHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_skill_race_class_info_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillCatalogHotfixLoadOutcomeLikeCpp<SkillRaceClassInfoHotfixRowsLikeCpp>,
    > {
        Box::pin(async move {
            let loaded = async {
                let mut race_class_batches = [Vec::new(), Vec::new()];
                for (batch_index, official) in OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate()
                {
                    let mut statement = self
                        .hotfix_db
                        .prepare(HotfixStatements::SEL_SKILL_RACE_CLASS_INFO);
                    statement.set_bool(0, official);
                    let mut rows = self.hotfix_db.query(&statement).await?;
                    if !rows.is_empty() {
                        loop {
                            let values = [
                                read_integer_checked_like_cpp(&rows, 0, "SkillRaceClassInfo.ID")?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    1,
                                    "SkillRaceClassInfo.RaceMask",
                                )?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    2,
                                    "SkillRaceClassInfo.SkillID",
                                )?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    3,
                                    "SkillRaceClassInfo.ClassMask",
                                )?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    4,
                                    "SkillRaceClassInfo.Flags",
                                )?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    5,
                                    "SkillRaceClassInfo.Availability",
                                )?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    6,
                                    "SkillRaceClassInfo.MinLevel",
                                )?,
                                read_integer_checked_like_cpp(
                                    &rows,
                                    7,
                                    "SkillRaceClassInfo.SkillTierID",
                                )?,
                            ];
                            race_class_batches[batch_index]
                                .push(skill_race_class_info_values_like_cpp(values)?);
                            if !rows.next_row() {
                                break;
                            }
                        }
                    }
                }
                let [official, custom] = race_class_batches;
                Ok::<_, anyhow::Error>(SkillRaceClassInfoHotfixRowsLikeCpp { official, custom })
            }
            .await;
            match loaded {
                Ok(rows) => SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => SkillCatalogHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn statements_and_official_custom_polarity_match_cpp() {
        assert_eq!(OFFICIAL_THEN_CUSTOM_LIKE_CPP, [true, false]);
        assert_eq!(
            HotfixStatements::SEL_SKILL_LINE.sql(),
            concat!(
                "SELECT ID, CategoryID, ParentSkillLineID, ParentTierIndex FROM skill_line ",
                "WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
        assert_eq!(
            HotfixStatements::SEL_SKILL_LINE_ABILITY.sql(),
            concat!(
                "SELECT RaceMask, ID, SkillLine, Spell, MinSkillLineRank, ClassMask, ",
                "SupercedesSpell, AcquireMethod, TrivialSkillLineRankHigh, ",
                "TrivialSkillLineRankLow, Flags, NumSkillUps, UniqueBit, ",
                "TradeSkillCategoryID, SkillupSkillLineID, CharacterPoints1, ",
                "CharacterPoints2 FROM skill_line_ability WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
        assert_eq!(
            HotfixStatements::SEL_SKILL_LINE_X_TRAIT_TREE.sql(),
            concat!(
                "SELECT ID, SkillLineID, TraitTreeID, OrderIndex FROM skill_line_x_trait_tree ",
                "WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
        assert_eq!(
            HotfixStatements::SEL_SKILL_RACE_CLASS_INFO.sql(),
            concat!(
                "SELECT ID, RaceMask, SkillID, ClassMask, Flags, Availability, ",
                "MinLevel, SkillTierID FROM skill_race_class_info ",
                "WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
    }

    #[test]
    fn ability_loader_queries_official_then_custom_like_cpp() {
        let source = include_str!("skill_catalog_adapter.rs");
        let loader = source
            .find("fn load_skill_line_ability_hotfix_rows_like_cpp")
            .expect("SkillLineAbility loader must remain present");
        let body_end = source[loader..]
            .find("\n    fn load_skill_race_class_info_hotfix_rows_like_cpp")
            .map(|offset| loader + offset)
            .expect("ability loader must precede race-class loader");
        let body = &source[loader..body_end];
        assert!(body.contains("SEL_SKILL_LINE_ABILITY"));
        assert!(!body.contains("SEL_SKILL_RACE_CLASS_INFO"));
    }

    #[test]
    fn race_class_loader_is_a_distinct_official_then_custom_stage_like_cpp() {
        let source = include_str!("skill_catalog_adapter.rs");
        let loader = source
            .find("fn load_skill_race_class_info_hotfix_rows_like_cpp")
            .expect("SkillRaceClassInfo loader must remain present");
        let body_end = source[loader..]
            .find("\n}\n\n#[cfg(test)]")
            .map(|offset| loader + offset);
        let body = &source[loader..body_end.unwrap_or(source.len())];
        assert!(body.contains("SEL_SKILL_RACE_CLASS_INFO"));
        assert!(!body.contains("SEL_SKILL_LINE_ABILITY"));
    }

    #[test]
    fn checked_rows_preserve_raw_domains_and_reject_invalid_ids() {
        assert_eq!(
            skill_line_values_like_cpp([7, -8, -9, 10]).unwrap(),
            SkillLineHotfixRowLikeCpp {
                id: 7,
                category_id: -8,
                parent_skill_line_id: -9,
                parent_tier_index: 10,
            }
        );
        assert_eq!(
            skill_line_values_like_cpp([-1, 2, 3, 4])
                .unwrap_err()
                .to_string(),
            "SkillLine.ID SQL value -1 is not u32"
        );
        let ability =
            skill_line_ability_values_like_cpp([-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13])
                .unwrap();
        assert_eq!(ability.id, 2);
        assert_eq!(ability.race_mask, -1);
        assert_eq!(ability.skillup_skill_line_id, -13);
        let race_class =
            skill_race_class_info_values_like_cpp([1, -2, -3, 4, 5, -6, 7, -8]).unwrap();
        assert_eq!(race_class.id, 1);
        assert_eq!(race_class.skill_id, -3);
        assert_eq!(race_class.skill_tier_id, -8);
        let trait_link = skill_line_x_trait_tree_values_like_cpp([9, 10, -11, 12]).unwrap();
        assert_eq!(trait_link.id, 9);
        assert_eq!(trait_link.skill_line_id, 10);
        assert_eq!(trait_link.trait_tree_id, -11);
        assert_eq!(trait_link.order_index, 12);
    }
}
