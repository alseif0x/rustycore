//! MariaDB adapter for C++ Phase and PhaseXPhaseGroup Hotfix overlays.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    PersistenceFutureLikeCpp, PhaseGroupHotfixRowLikeCpp, PhaseHotfixLoadOutcomeLikeCpp,
    PhaseHotfixPersistencePortLikeCpp, PhaseHotfixRowLikeCpp,
};

use crate::{HotfixDatabase, HotfixStatements, SqlResult};

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

fn u16_field_like_cpp(value: i128, field: &'static str) -> Result<u16> {
    if let Ok(value) = u16::try_from(value) {
        return Ok(value);
    }
    i16::try_from(value)
        .map(|value| value as u16)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ uint16 field range"))
}

fn phase_values_like_cpp(values: (i128, i128)) -> Result<PhaseHotfixRowLikeCpp> {
    Ok(PhaseHotfixRowLikeCpp {
        id: u32_field_like_cpp(values.0, "Phase.ID")?,
        flags: u16_field_like_cpp(values.1, "Phase.Flags")?,
    })
}

fn phase_group_values_like_cpp(values: (i128, i128, i128)) -> Result<PhaseGroupHotfixRowLikeCpp> {
    Ok(PhaseGroupHotfixRowLikeCpp {
        id: u32_field_like_cpp(values.0, "PhaseXPhaseGroup.ID")?,
        phase_id: u16_field_like_cpp(values.1, "PhaseXPhaseGroup.PhaseID")?,
        phase_group_id: u32_field_like_cpp(values.2, "PhaseXPhaseGroup.PhaseGroupID")?,
    })
}

fn phase_row_like_cpp(result: &SqlResult) -> Result<PhaseHotfixRowLikeCpp> {
    phase_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "Phase.ID")?,
        read_integer_checked_like_cpp(result, 1, "Phase.Flags")?,
    ))
}

fn phase_group_row_like_cpp(result: &SqlResult) -> Result<PhaseGroupHotfixRowLikeCpp> {
    phase_group_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "PhaseXPhaseGroup.ID")?,
        read_integer_checked_like_cpp(result, 1, "PhaseXPhaseGroup.PhaseID")?,
        read_integer_checked_like_cpp(result, 2, "PhaseXPhaseGroup.PhaseGroupID")?,
    ))
}

async fn query_rows_like_cpp<T>(
    db: &HotfixDatabase,
    statement: HotfixStatements,
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

fn classify_rows_like_cpp<T>(result: Result<Vec<T>>) -> PhaseHotfixLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => PhaseHotfixLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => PhaseHotfixLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbPhaseHotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbPhaseHotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl PhaseHotfixPersistencePortLikeCpp for MariaDbPhaseHotfixPersistenceAdapterLikeCpp {
    fn load_phase_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, PhaseHotfixLoadOutcomeLikeCpp<PhaseHotfixRowLikeCpp>> {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::SEL_PHASE,
                    phase_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_phase_group_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, PhaseHotfixLoadOutcomeLikeCpp<PhaseGroupHotfixRowLikeCpp>>
    {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::SEL_PHASE_X_PHASE_GROUP,
                    phase_group_row_like_cpp,
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
    fn phase_hotfix_statements_and_field_order_match_cpp_load_info() {
        assert_eq!(
            HotfixStatements::SEL_PHASE.sql(),
            "SELECT ID, Flags FROM phase WHERE VerifiedBuild > 0"
        );
        assert_eq!(
            HotfixStatements::SEL_PHASE_X_PHASE_GROUP.sql(),
            "SELECT ID, PhaseID, PhaseGroupID FROM phase_x_phase_group WHERE VerifiedBuild > 0"
        );
        assert_eq!(
            phase_values_like_cpp((1, 2)).unwrap(),
            PhaseHotfixRowLikeCpp { id: 1, flags: 2 }
        );
        assert_eq!(
            phase_group_values_like_cpp((3, 4, 5)).unwrap(),
            PhaseGroupHotfixRowLikeCpp {
                id: 3,
                phase_id: 4,
                phase_group_id: 5,
            }
        );
    }

    #[test]
    fn boundary_conversion_preserves_cpp_unsigned_widths() {
        assert_eq!(u32_field_like_cpp(-1, "field").unwrap(), u32::MAX);
        assert_eq!(u16_field_like_cpp(-1, "field").unwrap(), u16::MAX);
        assert!(u16_field_like_cpp(65_536, "field").is_err());
        assert!(u32_field_like_cpp(i128::from(i64::MAX), "field").is_err());
    }
}
