//! MariaDB adapter for the C++ item random-enchantment World catalog.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp,
    ItemRandomEnchantmentCatalogPersistencePortLikeCpp, ItemRandomEnchantmentPersistenceRowLikeCpp,
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

fn decode_row_like_cpp(result: &SqlResult) -> Result<ItemRandomEnchantmentPersistenceRowLikeCpp> {
    Ok(ItemRandomEnchantmentPersistenceRowLikeCpp {
        group_id: integer_like_cpp(result, 0, "item_random_enchantment_template.Id")?,
        enchantment_id: integer_like_cpp(
            result,
            1,
            "item_random_enchantment_template.EnchantmentId",
        )?,
        chance: result
            .try_read::<f32>(2)
            .context("missing or non-float item_random_enchantment_template.Chance SQL column 2")?,
    })
}

pub struct MariaDbItemRandomEnchantmentCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbItemRandomEnchantmentCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl ItemRandomEnchantmentCatalogPersistencePortLikeCpp
    for MariaDbItemRandomEnchantmentCatalogPersistenceAdapterLikeCpp
{
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let mut result = self
                    .world_db
                    .query(
                        &self
                            .world_db
                            .prepare(WorldStatements::SEL_ITEM_RANDOM_ENCHANTMENT_TEMPLATE),
                    )
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
                Ok(rows) => ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp::Failed {
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
    fn statement_matches_cpp_columns() {
        assert_eq!(
            WorldStatements::SEL_ITEM_RANDOM_ENCHANTMENT_TEMPLATE.sql(),
            "SELECT Id, EnchantmentId, Chance FROM item_random_enchantment_template"
        );
    }

    #[test]
    fn checked_ids_reject_values_that_old_loader_fabricated_as_zero() {
        assert_eq!(integer_field_like_cpp::<u32>(42, "id").unwrap(), 42);
        assert!(integer_field_like_cpp::<u32>(-1, "id").is_err());
        assert!(integer_field_like_cpp::<u32>(i128::from(u32::MAX) + 1, "id").is_err());
    }
}
