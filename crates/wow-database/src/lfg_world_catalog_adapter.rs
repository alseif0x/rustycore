//! MariaDB adapter for C++ late LFG World catalogs.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    LfgDungeonRewardPersistenceRowLikeCpp, LfgDungeonTemplatePersistenceRowLikeCpp,
    LfgWorldCatalogLoadOutcomeLikeCpp, LfgWorldCatalogPersistencePortLikeCpp,
    PersistenceFutureLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

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

fn integer_field_like_cpp<T>(value: i128, field: &'static str) -> Result<T>
where
    T: TryFrom<i128>,
{
    T::try_from(value).map_err(|_| anyhow::anyhow!("{field} SQL value {value} is out of range"))
}

fn float_field_like_cpp(result: &SqlResult, column: usize, field: &'static str) -> Result<f32> {
    result
        .try_read::<f32>(column)
        .with_context(|| format!("missing or non-float {field} SQL column {column}"))
}

fn template_row_like_cpp(result: &SqlResult) -> Result<LfgDungeonTemplatePersistenceRowLikeCpp> {
    Ok(LfgDungeonTemplatePersistenceRowLikeCpp {
        dungeon_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 0, "lfg_dungeon_template.dungeonId")?,
            "lfg_dungeon_template.dungeonId",
        )?,
        position_x: float_field_like_cpp(result, 1, "lfg_dungeon_template.position_x")?,
        position_y: float_field_like_cpp(result, 2, "lfg_dungeon_template.position_y")?,
        position_z: float_field_like_cpp(result, 3, "lfg_dungeon_template.position_z")?,
        orientation: float_field_like_cpp(result, 4, "lfg_dungeon_template.orientation")?,
        required_item_level: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 5, "lfg_dungeon_template.requiredItemLevel")?,
            "lfg_dungeon_template.requiredItemLevel",
        )?,
    })
}

fn reward_row_like_cpp(result: &SqlResult) -> Result<LfgDungeonRewardPersistenceRowLikeCpp> {
    Ok(LfgDungeonRewardPersistenceRowLikeCpp {
        dungeon_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 0, "lfg_dungeon_rewards.dungeonId")?,
            "lfg_dungeon_rewards.dungeonId",
        )?,
        max_level: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 1, "lfg_dungeon_rewards.maxLevel")?,
            "lfg_dungeon_rewards.maxLevel",
        )?,
        first_quest_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 2, "lfg_dungeon_rewards.firstQuestId")?,
            "lfg_dungeon_rewards.firstQuestId",
        )?,
        other_quest_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 3, "lfg_dungeon_rewards.otherQuestId")?,
            "lfg_dungeon_rewards.otherQuestId",
        )?,
    })
}

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> Result<T>,
) -> Result<Vec<T>> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::with_capacity(result.count());
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

fn classify_rows_like_cpp<T>(result: Result<Vec<T>>) -> LfgWorldCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => LfgWorldCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => LfgWorldCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbLfgWorldCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbLfgWorldCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl LfgWorldCatalogPersistencePortLikeCpp for MariaDbLfgWorldCatalogPersistenceAdapterLikeCpp {
    fn load_lfg_dungeon_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        LfgWorldCatalogLoadOutcomeLikeCpp<LfgDungeonTemplatePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_LFG_DUNGEON_TEMPLATES,
                    template_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_lfg_dungeon_reward_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        LfgWorldCatalogLoadOutcomeLikeCpp<LfgDungeonRewardPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_LFG_DUNGEON_REWARDS,
                    reward_row_like_cpp,
                )
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
    fn statements_preserve_cpp_columns_and_reward_order() {
        assert_eq!(
            WorldStatements::SEL_LFG_DUNGEON_TEMPLATES.sql(),
            "SELECT dungeonId, position_x, position_y, position_z, orientation, requiredItemLevel FROM lfg_dungeon_template"
        );
        assert_eq!(
            WorldStatements::SEL_LFG_DUNGEON_REWARDS.sql(),
            "SELECT dungeonId, maxLevel, firstQuestId, otherQuestId FROM lfg_dungeon_rewards ORDER BY dungeonId, maxLevel ASC"
        );
    }

    #[test]
    fn checked_widths_reject_fabricated_unsigned_defaults() {
        assert_eq!(integer_field_like_cpp::<u32>(42, "id").unwrap(), 42);
        assert_eq!(
            integer_field_like_cpp::<u16>(65_535, "ilevel").unwrap(),
            65_535
        );
        assert_eq!(integer_field_like_cpp::<u8>(80, "level").unwrap(), 80);
        assert!(integer_field_like_cpp::<u32>(-1, "id").is_err());
        assert!(integer_field_like_cpp::<u16>(65_536, "ilevel").is_err());
        assert!(integer_field_like_cpp::<u8>(256, "level").is_err());
    }
}
