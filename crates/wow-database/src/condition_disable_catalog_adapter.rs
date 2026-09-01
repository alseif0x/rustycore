//! MariaDB adapter for ConditionMgr and DisableMgr startup rows.

use std::sync::Arc;

use anyhow::Result;
use wow_persistence::{
    ConditionDisableCatalogPersistencePortLikeCpp, ConditionDisableRowsLoadOutcomeLikeCpp,
    ConditionPersistenceRowLikeCpp, DisablePersistenceRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

pub struct MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn condition_rows(&self) -> Result<Vec<ConditionPersistenceRowLikeCpp>> {
        let stmt = self.world_db.prepare(WorldStatements::SEL_CONDITIONS);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(ConditionPersistenceRowLikeCpp {
                    source_type_or_reference_id: read_i32(&result, 0),
                    source_group: read_u32(&result, 1),
                    source_entry: read_i32(&result, 2),
                    source_id: read_i32(&result, 3) as u32,
                    else_group: read_u32(&result, 4),
                    condition_type_or_reference: read_i32(&result, 5),
                    condition_target: read_u8(&result, 6),
                    condition_value1: read_u32(&result, 7),
                    condition_value2: read_u32(&result, 8),
                    condition_value3: read_u32(&result, 9),
                    condition_string_value1: result.read_string(10),
                    negative_condition: read_u8(&result, 11) == 1,
                    error_type: read_u32(&result, 12),
                    error_text_id: read_u32(&result, 13),
                    script_name: result.read_string(14),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn disable_rows(&self) -> Result<Vec<DisablePersistenceRowLikeCpp>> {
        let mut result = self
            .world_db
            .direct_query("SELECT sourceType, entry, flags, params_0, params_1 FROM disables")
            .await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(DisablePersistenceRowLikeCpp {
                    source_type: read_u32(&result, 0),
                    entry: read_u32(&result, 1),
                    flags: read_u16(&result, 2),
                    params_0: result.read_string(3),
                    params_1: result.read_string(4),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

fn read_i32(result: &SqlResult, column: usize) -> i32 {
    if let Some(value) = result.try_read::<i32>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return i32::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return i32::from(value);
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return i32::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return i32::from(value);
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return i32::from(value);
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return i32::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return i32::try_from(value).unwrap_or(0);
    }
    0
}

fn read_u32(result: &SqlResult, column: usize) -> u32 {
    if let Some(value) = result.try_read::<u32>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return value as u32;
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return u32::from(value);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return (i32::from(value)) as u32;
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return u32::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return (i32::from(value)) as u32;
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return u32::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return if (0..=i64::from(u32::MAX)).contains(&value) {
            value as u32
        } else {
            0
        };
    }
    0
}

fn read_u8(result: &SqlResult, column: usize) -> u8 {
    if let Some(value) = result.try_read::<u8>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_u8(i32::from(value));
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return u8::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_u8(i32::from(value));
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return u8::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_u8(value);
    }
    0
}

fn normalize_signed_u8(value: i32) -> u8 {
    let converted = value as u8;
    if i32::from(converted) == value || (converted as i8) as i32 == value {
        converted
    } else {
        0
    }
}

fn read_u16(result: &SqlResult, column: usize) -> u16 {
    if let Some(value) = result.try_read::<u16>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_u16(i32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return u16::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_u16(i32::from(value));
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return u16::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_u16(value);
    }
    0
}

fn normalize_signed_u16(value: i32) -> u16 {
    let converted = value as u16;
    if i32::from(converted) == value || (converted as i16) as i32 == value {
        converted
    } else {
        0
    }
}

fn outcome<T>(result: Result<T>) -> ConditionDisableRowsLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => ConditionDisableRowsLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => ConditionDisableRowsLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

impl ConditionDisableCatalogPersistencePortLikeCpp
    for MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp
{
    fn load_condition_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ConditionDisableRowsLoadOutcomeLikeCpp<Vec<ConditionPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.condition_rows().await) })
    }
    fn load_disable_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ConditionDisableRowsLoadOutcomeLikeCpp<Vec<DisablePersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.disable_rows().await) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn condition_statement_and_signed_normalization_remain_exact() {
        assert!(
            WorldStatements::SEL_CONDITIONS
                .sql()
                .contains("FROM conditions")
        );
        assert_eq!((-1_i32) as u32, u32::MAX);
        assert_eq!(normalize_signed_u8(-1), u8::MAX);
        assert_eq!(normalize_signed_u8(0x100), 0);
        assert_eq!(normalize_signed_u16(0x0200), 0x0200);
        assert_eq!(normalize_signed_u16(-1), u16::MAX);
        assert_eq!(normalize_signed_u16(0x1_0000), 0);
    }
}
