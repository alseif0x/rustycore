//! MariaDB adapter for C++ ChrSpecialization Hotfix overlays.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    ChrSpecializationHotfixLoadOutcomeLikeCpp, ChrSpecializationHotfixPersistencePortLikeCpp,
    ChrSpecializationHotfixRowLikeCpp, ChrSpecializationHotfixRowsLikeCpp,
    PersistenceFutureLikeCpp,
};

use crate::{HotfixDatabase, HotfixStatements, SqlResult};

const OFFICIAL_THEN_CUSTOM_LIKE_CPP: [bool; 2] = [true, false];

// C++ `DB2Store.cpp::DB2StorageBase::LoadFromDB` calls `Load(false)` before
// `Load(true)`. `DB2DatabaseLoader` binds `!custom`, so these are respectively
// official (`VerifiedBuild > 0`) and custom rows. Field indexes follow
// `DB2LoadInfo.h::ChrSpecializationLoadInfo`.

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

fn chr_specialization_values_like_cpp(
    values: (i128, i128, i128, i128),
) -> Result<ChrSpecializationHotfixRowLikeCpp> {
    Ok(ChrSpecializationHotfixRowLikeCpp {
        id: u32::try_from(values.0)
            .with_context(|| format!("ChrSpecialization.ID SQL value {} is not u32", values.0))?,
        class_id: u8::try_from(values.1).with_context(|| {
            format!("ChrSpecialization.ClassID SQL value {} is not u8", values.1)
        })?,
        order_index: i8::try_from(values.2).with_context(|| {
            format!(
                "ChrSpecialization.OrderIndex SQL value {} is not i8",
                values.2
            )
        })?,
        role: i8::try_from(values.3)
            .with_context(|| format!("ChrSpecialization.Role SQL value {} is not i8", values.3))?,
    })
}

fn chr_specialization_row_like_cpp(
    result: &SqlResult,
) -> Result<ChrSpecializationHotfixRowLikeCpp> {
    chr_specialization_values_like_cpp((
        read_integer_checked_like_cpp(result, 3, "ChrSpecialization.ID")?,
        read_integer_checked_like_cpp(result, 4, "ChrSpecialization.ClassID")?,
        read_integer_checked_like_cpp(result, 5, "ChrSpecialization.OrderIndex")?,
        read_integer_checked_like_cpp(result, 7, "ChrSpecialization.Role")?,
    ))
}

pub struct MariaDbChrSpecializationHotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbChrSpecializationHotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl ChrSpecializationHotfixPersistencePortLikeCpp
    for MariaDbChrSpecializationHotfixPersistenceAdapterLikeCpp
{
    fn load_chr_specialization_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ChrSpecializationHotfixLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let mut batches = [Vec::new(), Vec::new()];
                for (batch_index, official) in OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate()
                {
                    let mut stmt = self
                        .hotfix_db
                        .prepare(HotfixStatements::SEL_CHR_SPECIALIZATION);
                    stmt.set_bool(0, official);
                    let mut rows = self.hotfix_db.query(&stmt).await?;
                    if rows.is_empty() {
                        continue;
                    }
                    loop {
                        batches[batch_index].push(chr_specialization_row_like_cpp(&rows)?);
                        if !rows.next_row() {
                            break;
                        }
                    }
                }
                let [official, custom] = batches;
                Ok::<_, anyhow::Error>(ChrSpecializationHotfixRowsLikeCpp { official, custom })
            }
            .await;

            match result {
                Ok(rows) => ChrSpecializationHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => ChrSpecializationHotfixLoadOutcomeLikeCpp::Failed {
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
    fn chr_specialization_statement_and_bind_order_match_cpp() {
        assert_eq!(OFFICIAL_THEN_CUSTOM_LIKE_CPP, [true, false]);
        assert_eq!(
            HotfixStatements::SEL_CHR_SPECIALIZATION.sql(),
            concat!(
                "SELECT Name, FemaleName, Description, ID, ClassID, OrderIndex, PetTalentType, Role, Flags, ",
                "SpellIconFileID, PrimaryStatPriority, AnimReplacements, MasterySpellID1, MasterySpellID2 ",
                "FROM chr_specialization WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
    }

    #[test]
    fn checked_boundary_mapping_preserves_signed_fields_and_rejects_narrowing() {
        assert_eq!(
            chr_specialization_values_like_cpp((1, 2, -3, 4)).unwrap(),
            ChrSpecializationHotfixRowLikeCpp {
                id: 1,
                class_id: 2,
                order_index: -3,
                role: 4,
            }
        );
        assert_eq!(
            chr_specialization_values_like_cpp((-1, 2, -3, 4))
                .unwrap_err()
                .to_string(),
            "ChrSpecialization.ID SQL value -1 is not u32"
        );
        assert_eq!(
            chr_specialization_values_like_cpp((1, 256, -3, 4))
                .unwrap_err()
                .to_string(),
            "ChrSpecialization.ClassID SQL value 256 is not u8"
        );
        assert_eq!(
            chr_specialization_values_like_cpp((1, 2, 128, 4))
                .unwrap_err()
                .to_string(),
            "ChrSpecialization.OrderIndex SQL value 128 is not i8"
        );
    }
}
