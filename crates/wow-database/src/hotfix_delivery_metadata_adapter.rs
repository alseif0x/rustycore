//! MariaDB adapter for C++ Hotfix client-delivery metadata.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    HotfixBlobPersistenceRowLikeCpp, HotfixDataPersistenceRowLikeCpp,
    HotfixDeliveryMetadataLoadOutcomeLikeCpp, HotfixDeliveryMetadataPersistencePortLikeCpp,
    HotfixOptionalDataPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{HotfixDatabase, HotfixStatements, SqlResult};

const STARTUP_STATEMENTS_LIKE_CPP: [HotfixStatements; 3] = [
    HotfixStatements::SEL_HOTFIX_BLOB,
    HotfixStatements::SEL_HOTFIX_DATA,
    HotfixStatements::SEL_HOTFIX_OPTIONAL_DATA,
];

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

fn i32_field_like_cpp(value: i128, field: &'static str) -> Result<i32> {
    if let Ok(value) = i32::try_from(value) {
        return Ok(value);
    }
    u32::try_from(value)
        .map(|value| value as i32)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ int32 field range"))
}

fn u8_field_like_cpp(value: i128, field: &'static str) -> Result<u8> {
    if let Ok(value) = u8::try_from(value) {
        return Ok(value);
    }
    i8::try_from(value)
        .map(|value| value as u8)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ uint8 field range"))
}

fn read_string_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<String> {
    result
        .try_read::<String>(column)
        .with_context(|| format!("missing or non-string {field} SQL column {column}"))
}

fn read_binary_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<Vec<u8>> {
    result
        .try_read::<Vec<u8>>(column)
        .with_context(|| format!("missing or non-binary {field} SQL column {column}"))
}

fn hotfix_blob_values_like_cpp(
    values: (i128, i128, String, Vec<u8>),
) -> Result<HotfixBlobPersistenceRowLikeCpp> {
    Ok(HotfixBlobPersistenceRowLikeCpp {
        table_hash: u32_field_like_cpp(values.0, "HotfixBlob.TableHash")?,
        record_id: i32_field_like_cpp(values.1, "HotfixBlob.RecordId")?,
        locale: values.2,
        blob: values.3,
    })
}

fn hotfix_data_values_like_cpp(
    values: (i128, i128, i128, i128, i128),
) -> Result<HotfixDataPersistenceRowLikeCpp> {
    Ok(HotfixDataPersistenceRowLikeCpp {
        push_id: i32_field_like_cpp(values.0, "HotfixData.Id")?,
        unique_id: u32_field_like_cpp(values.1, "HotfixData.UniqueId")?,
        table_hash: u32_field_like_cpp(values.2, "HotfixData.TableHash")?,
        record_id: i32_field_like_cpp(values.3, "HotfixData.RecordId")?,
        status: u8_field_like_cpp(values.4, "HotfixData.Status")?,
    })
}

fn hotfix_optional_data_values_like_cpp(
    values: (i128, i128, String, i128, Vec<u8>),
) -> Result<HotfixOptionalDataPersistenceRowLikeCpp> {
    Ok(HotfixOptionalDataPersistenceRowLikeCpp {
        table_hash: u32_field_like_cpp(values.0, "HotfixOptionalData.TableHash")?,
        record_id: i32_field_like_cpp(values.1, "HotfixOptionalData.RecordId")?,
        locale: values.2,
        key: u32_field_like_cpp(values.3, "HotfixOptionalData.Key")?,
        data: values.4,
    })
}

fn hotfix_blob_row_like_cpp(result: &SqlResult) -> Result<HotfixBlobPersistenceRowLikeCpp> {
    hotfix_blob_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "HotfixBlob.TableHash")?,
        read_integer_checked_like_cpp(result, 1, "HotfixBlob.RecordId")?,
        read_string_checked_like_cpp(result, 2, "HotfixBlob.Locale")?,
        read_binary_checked_like_cpp(result, 3, "HotfixBlob.Blob")?,
    ))
}

fn hotfix_data_row_like_cpp(result: &SqlResult) -> Result<HotfixDataPersistenceRowLikeCpp> {
    hotfix_data_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "HotfixData.Id")?,
        read_integer_checked_like_cpp(result, 1, "HotfixData.UniqueId")?,
        read_integer_checked_like_cpp(result, 2, "HotfixData.TableHash")?,
        read_integer_checked_like_cpp(result, 3, "HotfixData.RecordId")?,
        read_integer_checked_like_cpp(result, 4, "HotfixData.Status")?,
    ))
}

fn hotfix_optional_data_row_like_cpp(
    result: &SqlResult,
) -> Result<HotfixOptionalDataPersistenceRowLikeCpp> {
    hotfix_optional_data_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "HotfixOptionalData.TableHash")?,
        read_integer_checked_like_cpp(result, 1, "HotfixOptionalData.RecordId")?,
        read_string_checked_like_cpp(result, 2, "HotfixOptionalData.Locale")?,
        read_integer_checked_like_cpp(result, 3, "HotfixOptionalData.Key")?,
        read_binary_checked_like_cpp(result, 4, "HotfixOptionalData.Data")?,
    ))
}

async fn query_rows_like_cpp<T>(
    db: &HotfixDatabase,
    statement: HotfixStatements,
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

fn classify_rows_like_cpp<T>(
    result: Result<Vec<T>>,
) -> HotfixDeliveryMetadataLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => HotfixDeliveryMetadataLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl HotfixDeliveryMetadataPersistencePortLikeCpp
    for MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp
{
    fn load_hotfix_blob_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixBlobPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.hotfix_db,
                    STARTUP_STATEMENTS_LIKE_CPP[0],
                    hotfix_blob_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_hotfix_data_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixDataPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.hotfix_db,
                    STARTUP_STATEMENTS_LIKE_CPP[1],
                    hotfix_data_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_hotfix_optional_data_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixOptionalDataPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.hotfix_db,
                    STARTUP_STATEMENTS_LIKE_CPP[2],
                    hotfix_optional_data_row_like_cpp,
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
    fn hotfix_delivery_statements_keep_cpp_startup_order_and_exact_sql() {
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP,
            [
                HotfixStatements::SEL_HOTFIX_BLOB,
                HotfixStatements::SEL_HOTFIX_DATA,
                HotfixStatements::SEL_HOTFIX_OPTIONAL_DATA,
            ]
        );
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP[0].sql(),
            "SELECT TableHash, RecordId, locale, `Blob` FROM hotfix_blob ORDER BY TableHash"
        );
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP[1].sql(),
            "SELECT Id, UniqueId, TableHash, RecordId, Status FROM hotfix_data ORDER BY Id"
        );
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP[2].sql(),
            "SELECT TableHash, RecordId, locale, `Key`, `Data` FROM hotfix_optional_data ORDER BY TableHash"
        );
    }

    #[test]
    fn typed_rows_preserve_fields_binary_data_and_cpp_integer_widths() {
        assert_eq!(
            hotfix_blob_values_like_cpp((1, -2, "esES".into(), vec![3, 4])).unwrap(),
            HotfixBlobPersistenceRowLikeCpp {
                table_hash: 1,
                record_id: -2,
                locale: "esES".into(),
                blob: vec![3, 4],
            }
        );
        assert_eq!(
            hotfix_data_values_like_cpp((-1, 2, 3, -4, 255)).unwrap(),
            HotfixDataPersistenceRowLikeCpp {
                push_id: -1,
                unique_id: 2,
                table_hash: 3,
                record_id: -4,
                status: 255,
            }
        );
        assert_eq!(
            hotfix_optional_data_values_like_cpp((5, 6, "enUS".into(), 7, vec![8])).unwrap(),
            HotfixOptionalDataPersistenceRowLikeCpp {
                table_hash: 5,
                record_id: 6,
                locale: "enUS".into(),
                key: 7,
                data: vec![8],
            }
        );
        assert_eq!(u32_field_like_cpp(-1, "field").unwrap(), u32::MAX);
        assert_eq!(i32_field_like_cpp(u32::MAX.into(), "field").unwrap(), -1);
        assert!(u8_field_like_cpp(256, "field").is_err());
    }
}
