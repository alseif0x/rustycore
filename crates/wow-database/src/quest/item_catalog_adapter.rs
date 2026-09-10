//! MariaDB adapter for C++ `ObjectMgr` immutable World quest-item metadata.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    CreatureQuestItemPersistenceRowLikeCpp, GameObjectQuestItemPersistenceRowLikeCpp,
    PersistenceFutureLikeCpp, QuestItemCatalogLoadOutcomeLikeCpp,
    QuestItemCatalogPersistencePortLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

const GAMEOBJECT_QUEST_ITEM_STATEMENT_LIKE_CPP: WorldStatements =
    WorldStatements::SEL_GAMEOBJECT_QUEST_ITEM_ROWS;
const CREATURE_QUEST_ITEM_STATEMENT_LIKE_CPP: WorldStatements =
    WorldStatements::SEL_CREATURE_QUEST_ITEM_ROWS;

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

fn u8_checked_like_cpp(value: i128, field: &'static str) -> Result<u8> {
    u8::try_from(value).with_context(|| format!("{field} SQL value {value} is not u8"))
}

fn gameobject_quest_item_values_like_cpp(
    values: [i128; 3],
) -> Result<GameObjectQuestItemPersistenceRowLikeCpp> {
    Ok(GameObjectQuestItemPersistenceRowLikeCpp {
        gameobject_entry: u32_checked_like_cpp(values[0], "GameObjectQuestItem.GameObjectEntry")?,
        item_id: u32_checked_like_cpp(values[1], "GameObjectQuestItem.ItemId")?,
        idx: u32_checked_like_cpp(values[2], "GameObjectQuestItem.Idx")?,
    })
}

fn gameobject_quest_item_row_like_cpp(
    result: &SqlResult,
) -> Result<GameObjectQuestItemPersistenceRowLikeCpp> {
    gameobject_quest_item_values_like_cpp([
        read_integer_checked_like_cpp(result, 0, "GameObjectQuestItem.GameObjectEntry")?,
        read_integer_checked_like_cpp(result, 1, "GameObjectQuestItem.ItemId")?,
        read_integer_checked_like_cpp(result, 2, "GameObjectQuestItem.Idx")?,
    ])
}

fn creature_quest_item_values_like_cpp(
    values: [i128; 4],
) -> Result<CreatureQuestItemPersistenceRowLikeCpp> {
    Ok(CreatureQuestItemPersistenceRowLikeCpp {
        creature_entry: u32_checked_like_cpp(values[0], "CreatureQuestItem.CreatureEntry")?,
        difficulty_id: u8_checked_like_cpp(values[1], "CreatureQuestItem.DifficultyID")?,
        item_id: u32_checked_like_cpp(values[2], "CreatureQuestItem.ItemId")?,
        idx: u32_checked_like_cpp(values[3], "CreatureQuestItem.Idx")?,
    })
}

fn creature_quest_item_row_like_cpp(
    result: &SqlResult,
) -> Result<CreatureQuestItemPersistenceRowLikeCpp> {
    creature_quest_item_values_like_cpp([
        read_integer_checked_like_cpp(result, 0, "CreatureQuestItem.CreatureEntry")?,
        read_integer_checked_like_cpp(result, 1, "CreatureQuestItem.DifficultyID")?,
        read_integer_checked_like_cpp(result, 2, "CreatureQuestItem.ItemId")?,
        read_integer_checked_like_cpp(result, 3, "CreatureQuestItem.Idx")?,
    ])
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

fn classify_rows_like_cpp<T>(result: Result<Vec<T>>) -> QuestItemCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => QuestItemCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => QuestItemCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbQuestItemCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbQuestItemCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl QuestItemCatalogPersistencePortLikeCpp for MariaDbQuestItemCatalogPersistenceAdapterLikeCpp {
    fn load_gameobject_quest_item_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        QuestItemCatalogLoadOutcomeLikeCpp<GameObjectQuestItemPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    GAMEOBJECT_QUEST_ITEM_STATEMENT_LIKE_CPP,
                    gameobject_quest_item_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_creature_quest_item_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        QuestItemCatalogLoadOutcomeLikeCpp<CreatureQuestItemPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    CREATURE_QUEST_ITEM_STATEMENT_LIKE_CPP,
                    creature_quest_item_row_like_cpp,
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
    fn quest_item_statements_match_cpp_exactly() {
        assert_eq!(
            GAMEOBJECT_QUEST_ITEM_STATEMENT_LIKE_CPP.sql(),
            "SELECT GameObjectEntry, ItemId, Idx FROM gameobject_questitem ORDER BY Idx ASC"
        );
        assert_eq!(
            CREATURE_QUEST_ITEM_STATEMENT_LIKE_CPP.sql(),
            "SELECT CreatureEntry, DifficultyID, ItemId, Idx FROM creature_questitem ORDER BY Idx ASC"
        );
    }

    #[test]
    fn checked_rows_preserve_every_field_and_reject_invalid_unsigned_values() {
        assert_eq!(
            gameobject_quest_item_values_like_cpp([11, 22, 3]).unwrap(),
            GameObjectQuestItemPersistenceRowLikeCpp {
                gameobject_entry: 11,
                item_id: 22,
                idx: 3,
            }
        );
        assert_eq!(
            creature_quest_item_values_like_cpp([44, 5, 66, 7]).unwrap(),
            CreatureQuestItemPersistenceRowLikeCpp {
                creature_entry: 44,
                difficulty_id: 5,
                item_id: 66,
                idx: 7,
            }
        );

        assert!(gameobject_quest_item_values_like_cpp([-1, 22, 3]).is_err());
        assert!(gameobject_quest_item_values_like_cpp([11, i128::from(u32::MAX) + 1, 3]).is_err());
        assert!(creature_quest_item_values_like_cpp([44, 256, 66, 7]).is_err());
        assert!(creature_quest_item_values_like_cpp([44, 5, 66, -1]).is_err());
    }
}
