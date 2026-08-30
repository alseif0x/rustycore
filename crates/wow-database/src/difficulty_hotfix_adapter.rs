//! MariaDB adapter for C++ Difficulty Hotfix overlays.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    DifficultyHotfixLoadOutcomeLikeCpp, DifficultyHotfixPersistencePortLikeCpp,
    DifficultyHotfixRowLikeCpp, DifficultyHotfixRowsLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{HotfixDatabase, HotfixStatements, SqlResult};

const OFFICIAL_THEN_CUSTOM_LIKE_CPP: [bool; 2] = [true, false];

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

fn u32_field_like_cpp(value: i128, field: &'static str) -> Result<u32> {
    if let Ok(value) = u32::try_from(value) {
        return Ok(value);
    }
    i32::try_from(value)
        .map(|value| value as u32)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ uint32 field range"))
}

fn u8_field_like_cpp(value: i128, field: &'static str) -> Result<u8> {
    if let Ok(value) = u8::try_from(value) {
        return Ok(value);
    }
    i8::try_from(value)
        .map(|value| value as u8)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ uint8 field range"))
}

fn difficulty_values_like_cpp(
    values: (i128, i128, i128, i128, i128),
) -> Result<DifficultyHotfixRowLikeCpp> {
    Ok(DifficultyHotfixRowLikeCpp {
        id: u32_field_like_cpp(values.0, "Difficulty.ID")?,
        instance_type: u8_field_like_cpp(values.1, "Difficulty.InstanceType")?,
        fallback_difficulty_id: u8_field_like_cpp(values.2, "Difficulty.FallbackDifficultyID")?,
        flags: u8_field_like_cpp(values.3, "Difficulty.Flags")?,
        toggle_difficulty_id: u8_field_like_cpp(values.4, "Difficulty.ToggleDifficultyID")?,
    })
}

fn difficulty_row_like_cpp(result: &SqlResult) -> Result<DifficultyHotfixRowLikeCpp> {
    difficulty_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "Difficulty.ID")?,
        read_integer_checked_like_cpp(result, 2, "Difficulty.InstanceType")?,
        read_integer_checked_like_cpp(result, 5, "Difficulty.FallbackDifficultyID")?,
        read_integer_checked_like_cpp(result, 8, "Difficulty.Flags")?,
        read_integer_checked_like_cpp(result, 10, "Difficulty.ToggleDifficultyID")?,
    ))
}

pub struct MariaDbDifficultyHotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbDifficultyHotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl DifficultyHotfixPersistencePortLikeCpp for MariaDbDifficultyHotfixPersistenceAdapterLikeCpp {
    fn load_difficulty_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, DifficultyHotfixLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let mut batches = [Vec::new(), Vec::new()];
                for (batch_index, official) in OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate()
                {
                    let mut statement = self.hotfix_db.prepare(HotfixStatements::SEL_DIFFICULTY);
                    statement.set_bool(0, official);
                    let mut rows = self.hotfix_db.query(&statement).await?;
                    if rows.is_empty() {
                        continue;
                    }
                    loop {
                        batches[batch_index].push(difficulty_row_like_cpp(&rows)?);
                        if !rows.next_row() {
                            break;
                        }
                    }
                }
                let [official, custom] = batches;
                Ok::<_, anyhow::Error>(DifficultyHotfixRowsLikeCpp { official, custom })
            }
            .await;

            match result {
                Ok(rows) => DifficultyHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => DifficultyHotfixLoadOutcomeLikeCpp::Failed {
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
    fn difficulty_statement_and_bind_order_match_cpp() {
        assert_eq!(OFFICIAL_THEN_CUSTOM_LIKE_CPP, [true, false]);
        assert_eq!(
            HotfixStatements::SEL_DIFFICULTY.sql(),
            concat!(
                "SELECT ID, Name, InstanceType, OrderIndex, OldEnumValue, FallbackDifficultyID, ",
                "MinPlayers, MaxPlayers, Flags, ItemContext, ToggleDifficultyID, ",
                "GroupSizeHealthCurveID, GroupSizeDmgCurveID, GroupSizeSpellPointsCurveID ",
                "FROM difficulty WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
    }

    #[test]
    fn boundary_mapping_preserves_all_fields_and_cpp_signed_width_conversion() {
        assert_eq!(
            difficulty_values_like_cpp((1, 2, 3, 4, 5)).unwrap(),
            DifficultyHotfixRowLikeCpp {
                id: 1,
                instance_type: 2,
                fallback_difficulty_id: 3,
                flags: 4,
                toggle_difficulty_id: 5,
            }
        );
        assert_eq!(u32_field_like_cpp(-1, "field").unwrap(), u32::MAX);
        assert_eq!(u8_field_like_cpp(-1, "field").unwrap(), u8::MAX);
        assert!(u8_field_like_cpp(256, "field").is_err());
        assert!(u32_field_like_cpp(i128::from(i64::MAX), "field").is_err());
    }
}
