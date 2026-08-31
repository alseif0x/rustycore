//! MariaDB adapter for C++ battle-pet breed and quality World catalogs.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    BattlePetBreedPersistenceRowLikeCpp, BattlePetQualityPersistenceRowLikeCpp,
    BattlePetSelectionCatalogLoadOutcomeLikeCpp, BattlePetSelectionCatalogPersistencePortLikeCpp,
    PersistenceFutureLikeCpp,
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

fn classify_like_cpp<T>(result: Result<Vec<T>>) -> BattlePetSelectionCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => BattlePetSelectionCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => BattlePetSelectionCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl BattlePetSelectionCatalogPersistencePortLikeCpp
    for MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp
{
    fn load_breed_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        BattlePetSelectionCatalogLoadOutcomeLikeCpp<BattlePetBreedPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_BATTLE_PET_BREEDS,
                    |row| {
                        Ok(BattlePetBreedPersistenceRowLikeCpp {
                            species_id: integer_like_cpp(row, 0, "battle_pet_breeds.speciesId")?,
                            breed_id: integer_like_cpp(row, 1, "battle_pet_breeds.breedId")?,
                        })
                    },
                )
                .await,
            )
        })
    }

    fn load_quality_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        BattlePetSelectionCatalogLoadOutcomeLikeCpp<BattlePetQualityPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_BATTLE_PET_QUALITY,
                    |row| {
                        Ok(BattlePetQualityPersistenceRowLikeCpp {
                            species_id: integer_like_cpp(row, 0, "battle_pet_quality.speciesId")?,
                            quality: integer_like_cpp(row, 1, "battle_pet_quality.quality")?,
                        })
                    },
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
    fn statements_match_cpp_sources() {
        assert_eq!(
            WorldStatements::SEL_BATTLE_PET_BREEDS.sql(),
            "SELECT speciesId, breedId FROM battle_pet_breeds"
        );
        assert_eq!(
            WorldStatements::SEL_BATTLE_PET_QUALITY.sql(),
            "SELECT speciesId, quality FROM battle_pet_quality"
        );
    }

    #[test]
    fn checked_widths_reject_values_that_old_loader_fabricated_as_zero() {
        assert_eq!(integer_field_like_cpp::<u32>(42, "species").unwrap(), 42);
        assert_eq!(
            integer_field_like_cpp::<u16>(65_535, "breed").unwrap(),
            65_535
        );
        assert_eq!(integer_field_like_cpp::<u8>(5, "quality").unwrap(), 5);
        assert!(integer_field_like_cpp::<u32>(-1, "species").is_err());
        assert!(integer_field_like_cpp::<u16>(65_536, "breed").is_err());
        assert!(integer_field_like_cpp::<u8>(256, "quality").is_err());
    }
}
