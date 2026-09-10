//! MariaDB adapter for creature display/model Hotfix overlays.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    CreatureDisplayHotfixLoadOutcomeLikeCpp, CreatureDisplayHotfixPersistencePortLikeCpp,
    CreatureDisplayInfoHotfixRowLikeCpp, CreatureModelDataHotfixRowLikeCpp,
    PersistenceFutureLikeCpp,
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

fn i32_field_like_cpp(value: i128, field: &'static str) -> Result<i32> {
    i32::try_from(value)
        .or_else(|_| u32::try_from(value).map(|value| value as i32))
        .with_context(|| format!("{field} SQL value {value} is outside the C++ int32 field range"))
}

fn read_f32_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<f32> {
    result
        .try_read::<f32>(column)
        .or_else(|| result.try_read::<f64>(column).map(|value| value as f32))
        .with_context(|| format!("missing or non-floating {field} SQL column {column}"))
}

fn display_values_like_cpp(
    values: (i128, i128, i128, f32),
) -> Result<CreatureDisplayInfoHotfixRowLikeCpp> {
    Ok(CreatureDisplayInfoHotfixRowLikeCpp {
        id: u32_field_like_cpp(values.0, "CreatureDisplayInfo.ID")?,
        model_id: u16_field_like_cpp(values.1, "CreatureDisplayInfo.ModelID")?,
        extended_display_info_id: i32_field_like_cpp(
            values.2,
            "CreatureDisplayInfo.ExtendedDisplayInfoID",
        )?,
        creature_model_scale: values.3,
    })
}

fn display_row_like_cpp(result: &SqlResult) -> Result<CreatureDisplayInfoHotfixRowLikeCpp> {
    display_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "CreatureDisplayInfo.ID")?,
        read_integer_checked_like_cpp(result, 1, "CreatureDisplayInfo.ModelID")?,
        read_integer_checked_like_cpp(result, 7, "CreatureDisplayInfo.ExtendedDisplayInfoID")?,
        read_f32_checked_like_cpp(result, 4, "CreatureDisplayInfo.CreatureModelScale")?,
    ))
}

fn model_values_like_cpp(
    values: (i128, i128, i128, f32, f32, f32, f32),
) -> Result<CreatureModelDataHotfixRowLikeCpp> {
    Ok(CreatureModelDataHotfixRowLikeCpp {
        id: u32_field_like_cpp(values.0, "CreatureModelData.ID")?,
        flags: u32_field_like_cpp(values.1, "CreatureModelData.Flags")?,
        file_data_id: u32_field_like_cpp(values.2, "CreatureModelData.FileDataID")?,
        collision_height: values.3,
        hover_height: values.4,
        model_scale: values.5,
        mount_height: values.6,
    })
}

fn model_row_like_cpp(result: &SqlResult) -> Result<CreatureModelDataHotfixRowLikeCpp> {
    model_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "CreatureModelData.ID")?,
        read_integer_checked_like_cpp(result, 7, "CreatureModelData.Flags")?,
        read_integer_checked_like_cpp(result, 8, "CreatureModelData.FileDataID")?,
        read_f32_checked_like_cpp(result, 20, "CreatureModelData.CollisionHeight")?,
        read_f32_checked_like_cpp(result, 23, "CreatureModelData.HoverHeight")?,
        read_f32_checked_like_cpp(result, 25, "CreatureModelData.ModelScale")?,
        read_f32_checked_like_cpp(result, 29, "CreatureModelData.MountHeight")?,
    ))
}

pub struct MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl CreatureDisplayHotfixPersistencePortLikeCpp
    for MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp
{
    fn load_creature_display_info_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CreatureDisplayHotfixLoadOutcomeLikeCpp<CreatureDisplayInfoHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            let result = async {
                let statement = self
                    .hotfix_db
                    .prepare(HotfixStatements::SEL_CREATURE_DISPLAY_INFO);
                let mut result = self.hotfix_db.query(&statement).await?;
                let mut rows = Vec::with_capacity(result.count());
                if !result.is_empty() {
                    loop {
                        rows.push(display_row_like_cpp(&result)?);
                        if !result.next_row() {
                            break;
                        }
                    }
                }
                Ok::<_, anyhow::Error>(rows)
            }
            .await;
            match result {
                Ok(rows) => CreatureDisplayHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => CreatureDisplayHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_creature_model_data_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CreatureDisplayHotfixLoadOutcomeLikeCpp<CreatureModelDataHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            let result = async {
                let statement = self
                    .hotfix_db
                    .prepare(HotfixStatements::SEL_CREATURE_MODEL_DATA);
                let mut result = self.hotfix_db.query(&statement).await?;
                let mut rows = Vec::with_capacity(result.count());
                if !result.is_empty() {
                    loop {
                        rows.push(model_row_like_cpp(&result)?);
                        if !result.next_row() {
                            break;
                        }
                    }
                }
                Ok::<_, anyhow::Error>(rows)
            }
            .await;
            match result {
                Ok(rows) => CreatureDisplayHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => CreatureDisplayHotfixLoadOutcomeLikeCpp::Failed {
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
    fn statements_keep_the_existing_independent_hotfix_shapes() {
        assert!(
            HotfixStatements::SEL_CREATURE_DISPLAY_INFO
                .sql()
                .ends_with("FROM creature_display_info WHERE VerifiedBuild > 0")
        );
        assert!(
            HotfixStatements::SEL_CREATURE_MODEL_DATA
                .sql()
                .ends_with("FROM creature_model_data WHERE VerifiedBuild > 0")
        );
    }

    #[test]
    fn boundary_mapping_preserves_consumed_fields_and_cpp_widths() {
        assert_eq!(
            display_values_like_cpp((1, 2, -3, 1.25)).unwrap(),
            CreatureDisplayInfoHotfixRowLikeCpp {
                id: 1,
                model_id: 2,
                extended_display_info_id: -3,
                creature_model_scale: 1.25,
            }
        );
        assert_eq!(
            model_values_like_cpp((4, 5, 6, 1.0, 2.0, 3.0, 4.0)).unwrap(),
            CreatureModelDataHotfixRowLikeCpp {
                id: 4,
                flags: 5,
                file_data_id: 6,
                collision_height: 1.0,
                hover_height: 2.0,
                model_scale: 3.0,
                mount_height: 4.0,
            }
        );
        assert!(display_values_like_cpp((1, 65_536, 0, 1.0)).is_err());
        assert!(model_values_like_cpp((i128::from(i64::MAX), 0, 0, 0.0, 0.0, 0.0, 0.0)).is_err());
    }
}
