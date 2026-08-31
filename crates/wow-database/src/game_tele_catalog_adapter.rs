//! MariaDB adapter for the C++ GameTele World catalog.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    GameTeleCatalogLoadOutcomeLikeCpp, GameTeleCatalogPersistencePortLikeCpp,
    GameTelePersistenceRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

fn integer_field_like_cpp<T>(value: i128, field: &'static str) -> Result<T>
where
    T: TryFrom<i128>,
{
    T::try_from(value).map_err(|_| anyhow::anyhow!("{field} SQL value {value} is out of range"))
}

fn integer_like_cpp<T>(result: &SqlResult, column: usize, field: &'static str) -> Result<T>
where
    T: TryFrom<i128>,
{
    let value = result
        .try_read::<i64>(column)
        .map(i128::from)
        .or_else(|| result.try_read::<u64>(column).map(i128::from))
        .or_else(|| result.try_read::<i32>(column).map(i128::from))
        .or_else(|| result.try_read::<u32>(column).map(i128::from))
        .or_else(|| result.try_read::<i16>(column).map(i128::from))
        .or_else(|| result.try_read::<u16>(column).map(i128::from))
        .or_else(|| result.try_read::<i8>(column).map(i128::from))
        .or_else(|| result.try_read::<u8>(column).map(i128::from))
        .with_context(|| format!("missing or non-integer {field} SQL column {column}"))?;
    integer_field_like_cpp(value, field)
}

fn float_like_cpp(result: &SqlResult, column: usize, field: &'static str) -> Result<f32> {
    result
        .try_read::<f32>(column)
        .with_context(|| format!("missing or non-float {field} SQL column {column}"))
}

fn decode_row_like_cpp(result: &SqlResult) -> Result<GameTelePersistenceRowLikeCpp> {
    Ok(GameTelePersistenceRowLikeCpp {
        id: integer_like_cpp(result, 0, "game_tele.id")?,
        position_x: float_like_cpp(result, 1, "game_tele.position_x")?,
        position_y: float_like_cpp(result, 2, "game_tele.position_y")?,
        position_z: float_like_cpp(result, 3, "game_tele.position_z")?,
        orientation: float_like_cpp(result, 4, "game_tele.orientation")?,
        map_id: integer_like_cpp(result, 5, "game_tele.map")?,
        name: result
            .try_read::<String>(6)
            .context("missing or non-string game_tele.name SQL column 6")?,
    })
}

pub struct MariaDbGameTeleCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbGameTeleCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl GameTeleCatalogPersistencePortLikeCpp for MariaDbGameTeleCatalogPersistenceAdapterLikeCpp {
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, GameTeleCatalogLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let mut result = self
                    .world_db
                    .query(&self.world_db.prepare(WorldStatements::SEL_GAME_TELE))
                    .await?;
                let mut rows = Vec::with_capacity(result.count());
                if result.is_empty() {
                    return Ok(rows);
                }

                loop {
                    rows.push(decode_row_like_cpp(&result)?);
                    if !result.next_row() {
                        break;
                    }
                }
                Ok::<_, anyhow::Error>(rows)
            }
            .await;

            match result {
                Ok(rows) => GameTeleCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => GameTeleCatalogLoadOutcomeLikeCpp::Failed {
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
    fn statement_matches_cpp_columns_and_order() {
        assert_eq!(
            WorldStatements::SEL_GAME_TELE.sql(),
            "SELECT id, position_x, position_y, position_z, orientation, map, name FROM game_tele"
        );
    }

    #[test]
    fn checked_integer_widths_reject_out_of_range_values() {
        assert_eq!(integer_field_like_cpp::<u32>(42, "id").unwrap(), 42);
        assert!(integer_field_like_cpp::<u32>(-1, "id").is_err());
        assert_eq!(integer_field_like_cpp::<u16>(571, "map").unwrap(), 571);
        assert!(integer_field_like_cpp::<u16>(i128::from(u16::MAX) + 1, "map").is_err());
    }
}
