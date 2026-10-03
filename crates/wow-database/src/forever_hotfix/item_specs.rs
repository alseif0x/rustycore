//! 02245dcd HotfixDatabase.cpp:792,1087-1092, source numeric order/types.
use super::{ForeverHotfixRepository, LoadError, PreparedStatement};
use crate::SqlResult;
use wow_persistence::forever::item_specs::{
    GemPropertiesRow, ItemSpecOverlays, ItemSpecOverrideRow, ItemSpecRow, ItemSpecRows,
};
const QUERIES: [&str; 3] = [
    "SELECT ID,MinLevel,MaxLevel,ItemType,PrimaryStat,SecondaryStat,SpecializationID FROM item_spec WHERE (VerifiedBuild>0)=?",
    "SELECT ID,SpecID,ItemID FROM item_spec_override WHERE (VerifiedBuild>0)=?",
    "SELECT ID,EnchantId,Type FROM gem_properties WHERE (VerifiedBuild>0)=?",
];
fn read<T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>>(
    row: &SqlResult,
    index: usize,
) -> Result<T, LoadError> {
    row.try_read(index).ok_or(LoadError::InvalidRow)
}
impl ForeverHotfixRepository {
    pub async fn load_item_spec_overlays(&self) -> Result<ItemSpecOverlays, LoadError> {
        Ok(ItemSpecOverlays {
            official: self.item_spec_batch(false).await?,
            custom: self.item_spec_batch(true).await?,
        })
    }
    async fn item_spec_batch(&self, custom: bool) -> Result<ItemSpecRows, LoadError> {
        let mut rows = ItemSpecRows::default();
        for (table, sql) in QUERIES.iter().enumerate() {
            let mut statement = PreparedStatement::new(*sql);
            statement.set_bool(0, !custom);
            let mut result = self
                .0
                .query(&statement)
                .await
                .map_err(|_| LoadError::Database)?;
            if result.is_empty() {
                continue;
            }
            loop {
                if result.field_count() != [7, 3, 3][table] {
                    return Err(LoadError::InvalidRow);
                }
                match table {
                    0 => rows.specs.push(ItemSpecRow {
                        id: read(&result, 0)?,
                        min_level: read(&result, 1)?,
                        max_level: read(&result, 2)?,
                        item_type: read(&result, 3)?,
                        primary: read(&result, 4)?,
                        secondary: read(&result, 5)?,
                        specialization: read(&result, 6)?,
                    }),
                    1 => rows.overrides.push(ItemSpecOverrideRow {
                        id: read(&result, 0)?,
                        specialization: read(&result, 1)?,
                        item: read(&result, 2)?,
                    }),
                    2 => rows.gems.push(GemPropertiesRow {
                        id: read(&result, 0)?,
                        enchantment: read(&result, 1)?,
                        kind: read(&result, 2)?,
                    }),
                    _ => unreachable!(),
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_columns_and_predicates_do_not_sort_or_filter_locale() {
        for (q, (columns, table)) in QUERIES.iter().zip([
            (
                "ID,MinLevel,MaxLevel,ItemType,PrimaryStat,SecondaryStat,SpecializationID",
                "item_spec",
            ),
            ("ID,SpecID,ItemID", "item_spec_override"),
            ("ID,EnchantId,Type", "gem_properties"),
        ]) {
            assert_eq!(
                *q,
                format!("SELECT {columns} FROM {table} WHERE (VerifiedBuild>0)=?")
            );
        }
    }
}
