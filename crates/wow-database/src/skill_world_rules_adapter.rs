//! MariaDB adapter for C++ `ObjectMgr` immutable World skill rules.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    FishingBaseSkillPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
    SKILL_TIER_VALUE_COUNT_LIKE_CPP, SkillTierPersistenceRowLikeCpp,
    SkillWorldRulesLoadOutcomeLikeCpp, SkillWorldRulesPersistencePortLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

const STARTUP_STATEMENTS_LIKE_CPP: [WorldStatements; 2] = [
    WorldStatements::SEL_FISHING_BASE_SKILL_LEVELS,
    WorldStatements::SEL_SKILL_TIERS,
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

fn u32_checked_like_cpp(value: i128, field: &'static str) -> Result<u32> {
    u32::try_from(value).with_context(|| format!("{field} SQL value {value} is not u32"))
}

fn i16_checked_like_cpp(value: i128, field: &'static str) -> Result<i16> {
    i16::try_from(value).with_context(|| format!("{field} SQL value {value} is not i16"))
}

fn fishing_values_like_cpp(values: [i128; 2]) -> Result<FishingBaseSkillPersistenceRowLikeCpp> {
    Ok(FishingBaseSkillPersistenceRowLikeCpp {
        area_id: u32_checked_like_cpp(values[0], "FishingBaseSkill.AreaID")?,
        skill: i16_checked_like_cpp(values[1], "FishingBaseSkill.Skill")?,
    })
}

fn fishing_row_like_cpp(result: &SqlResult) -> Result<FishingBaseSkillPersistenceRowLikeCpp> {
    fishing_values_like_cpp([
        read_integer_checked_like_cpp(result, 0, "FishingBaseSkill.AreaID")?,
        read_integer_checked_like_cpp(result, 1, "FishingBaseSkill.Skill")?,
    ])
}

fn skill_tier_values_like_cpp(
    id: i128,
    values: [i128; SKILL_TIER_VALUE_COUNT_LIKE_CPP],
) -> Result<SkillTierPersistenceRowLikeCpp> {
    let mut decoded = [0; SKILL_TIER_VALUE_COUNT_LIKE_CPP];
    for (index, value) in values.into_iter().enumerate() {
        decoded[index] = u32_checked_like_cpp(value, "SkillTier.Value")?;
    }
    Ok(SkillTierPersistenceRowLikeCpp {
        id: u32_checked_like_cpp(id, "SkillTier.ID")?,
        value: decoded,
    })
}

fn skill_tier_row_like_cpp(result: &SqlResult) -> Result<SkillTierPersistenceRowLikeCpp> {
    let mut values = [0; SKILL_TIER_VALUE_COUNT_LIKE_CPP];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_integer_checked_like_cpp(result, 1 + index, "SkillTier.Value")?;
    }
    skill_tier_values_like_cpp(
        read_integer_checked_like_cpp(result, 0, "SkillTier.ID")?,
        values,
    )
}

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> Result<T>,
) -> Result<Vec<T>> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::new();
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode(&result)?);
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn classify_rows_like_cpp<T>(result: Result<Vec<T>>) -> SkillWorldRulesLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => SkillWorldRulesLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => SkillWorldRulesLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbSkillWorldRulesPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbSkillWorldRulesPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl SkillWorldRulesPersistencePortLikeCpp for MariaDbSkillWorldRulesPersistenceAdapterLikeCpp {
    fn load_fishing_base_skill_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillWorldRulesLoadOutcomeLikeCpp<FishingBaseSkillPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(&self.world_db, STARTUP_STATEMENTS_LIKE_CPP[0], |row| {
                    fishing_row_like_cpp(row)
                })
                .await,
            )
        })
    }

    fn load_skill_tier_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SkillWorldRulesLoadOutcomeLikeCpp<SkillTierPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(&self.world_db, STARTUP_STATEMENTS_LIKE_CPP[1], |row| {
                    skill_tier_row_like_cpp(row)
                })
                .await,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn skill_world_rule_statements_match_cpp_startup_order_and_sql() {
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_FISHING_BASE_SKILL_LEVELS,
                WorldStatements::SEL_SKILL_TIERS,
            ]
        );
        assert_eq!(
            WorldStatements::SEL_FISHING_BASE_SKILL_LEVELS.sql(),
            "SELECT entry, skill FROM skill_fishing_base_level"
        );
        assert_eq!(
            WorldStatements::SEL_SKILL_TIERS.sql(),
            concat!(
                "SELECT ID, Value1, Value2, Value3, Value4, Value5, Value6, Value7, Value8, ",
                "Value9, Value10, Value11, Value12, Value13, Value14, Value15, Value16 ",
                "FROM skill_tiers"
            )
        );
    }

    #[test]
    fn checked_rows_preserve_signed_fishing_and_all_tier_values() {
        assert_eq!(
            fishing_values_like_cpp([7, -8]).unwrap(),
            FishingBaseSkillPersistenceRowLikeCpp {
                area_id: 7,
                skill: -8,
            }
        );
        assert!(fishing_values_like_cpp([-1, 2]).is_err());
        assert!(fishing_values_like_cpp([1, i128::from(i16::MAX) + 1]).is_err());

        let values = std::array::from_fn(|index| index as i128 + 1);
        let tier = skill_tier_values_like_cpp(9, values).unwrap();
        assert_eq!(tier.id, 9);
        assert_eq!(tier.value, std::array::from_fn(|index| index as u32 + 1));
        assert!(skill_tier_values_like_cpp(-1, [0; 16]).is_err());
        assert!(skill_tier_values_like_cpp(1, [-1; 16]).is_err());
    }
}
