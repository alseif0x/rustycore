//! MariaDB adapter for C++ Difficulty Hotfix overlays.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    DifficultyHotfixLoadOutcomeLikeCpp, DifficultyHotfixPersistencePortLikeCpp,
    DifficultyHotfixRowLikeCpp, DifficultyHotfixRowsLikeCpp, PersistenceFutureLikeCpp,
};

use crate::field_like_cpp::{FieldRowLikeCpp, NativeFieldLikeCpp};
use crate::{HotfixDatabase, HotfixStatements};

const OFFICIAL_THEN_CUSTOM_LIKE_CPP: [bool; 2] = [true, false];

fn difficulty_row(
    native: &[NativeFieldLikeCpp],
    row_index: usize,
) -> Result<DifficultyHotfixRowLikeCpp> {
    let context = format!("difficulty row {row_index}");
    Ok(DifficultyHotfixRowLikeCpp {
        id: native
            .get_like_cpp::<u32>(0, "Difficulty.ID")
            .with_context(|| context.clone())?,
        instance_type: native
            .get_like_cpp::<u8>(2, "Difficulty.InstanceType")
            .with_context(|| context.clone())?,
        fallback_difficulty_id: native
            .get_like_cpp::<u8>(5, "Difficulty.FallbackDifficultyID")
            .with_context(|| context.clone())?,
        flags: native
            .get_like_cpp::<u8>(8, "Difficulty.Flags")
            .with_context(|| context.clone())?,
        toggle_difficulty_id: native
            .get_like_cpp::<u8>(10, "Difficulty.ToggleDifficultyID")
            .with_context(|| context.clone())?,
    })
}

fn classify_difficulty_rows(
    result: Result<DifficultyHotfixRowsLikeCpp>,
) -> DifficultyHotfixLoadOutcomeLikeCpp {
    match result {
        Ok(rows) => DifficultyHotfixLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => DifficultyHotfixLoadOutcomeLikeCpp::Failed {
            reason: format!("{error:#}"),
        },
    }
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
                    let mut rows = self
                        .hotfix_db
                        .query(&statement)
                        .await
                        .with_context(|| format!("query difficulty official={official}"))?;
                    if rows.is_empty() {
                        continue;
                    }
                    let mut row_index = 0;
                    loop {
                        let native = rows
                            .native_row_like_cpp()
                            .with_context(|| format!("difficulty row {row_index} native fields"))?;
                        batches[batch_index].push(
                            difficulty_row(&native, row_index)
                                .with_context(|| format!("difficulty official={official}"))?,
                        );
                        row_index += 1;
                        if !rows.next_row() {
                            break;
                        }
                    }
                }
                let [official, custom] = batches;
                Ok::<_, anyhow::Error>(DifficultyHotfixRowsLikeCpp { official, custom })
            }
            .await;

            classify_difficulty_rows(result)
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
    fn native_getters_preserve_source_widths_and_cpp_signed_byte_conversion() {
        assert_eq!(
            difficulty_row(
                &[
                    NativeFieldLikeCpp::UInt32(u32::MAX),
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::UInt8(2),
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Int8(-1),
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::UInt8(4),
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::UInt8(5),
                ],
                0,
            )
            .unwrap(),
            DifficultyHotfixRowLikeCpp {
                id: u32::MAX,
                instance_type: 2,
                fallback_difficulty_id: u8::MAX,
                flags: 4,
                toggle_difficulty_id: 5,
            }
        );
        assert_eq!(
            difficulty_row(
                &[
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                ],
                1,
            )
            .unwrap(),
            DifficultyHotfixRowLikeCpp {
                id: 0,
                instance_type: 0,
                fallback_difficulty_id: 0,
                flags: 0,
                toggle_difficulty_id: 0,
            }
        );
        let mut wide_negative = vec![NativeFieldLikeCpp::UInt32(1); 11];
        wide_negative[5] = NativeFieldLikeCpp::Int64(-1);
        let error = difficulty_row(&wide_negative, 8).unwrap_err();
        let error = format!("{error:#}");
        assert!(error.contains("difficulty row 8"), "{error}");
        assert!(error.contains("FallbackDifficultyID"), "{error}");
        assert!(error.contains("GetUInt8"), "{error}");

        let mut byte_overflow = vec![NativeFieldLikeCpp::UInt32(1); 11];
        byte_overflow[8] = NativeFieldLikeCpp::UInt16(256);
        assert!(difficulty_row(&byte_overflow, 0).is_err());
        assert!(difficulty_row(&[NativeFieldLikeCpp::UInt32(1)], 4).is_err());
    }

    #[test]
    fn malformed_row_classifies_the_whole_batch_as_failed_with_context() {
        let mut malformed = vec![NativeFieldLikeCpp::UInt32(1); 11];
        malformed[2] = NativeFieldLikeCpp::Text("bad".into());
        let error = difficulty_row(&malformed, 6).unwrap_err();
        let outcome = classify_difficulty_rows(Err(error));
        let reason = match outcome {
            DifficultyHotfixLoadOutcomeLikeCpp::Failed { reason } => reason,
            DifficultyHotfixLoadOutcomeLikeCpp::Loaded(_) => {
                panic!("one malformed row must fail the complete batch")
            }
        };
        assert!(reason.contains("difficulty row 6"), "{reason}");
        assert!(reason.contains("Difficulty.InstanceType"), "{reason}");
        assert!(reason.contains("GetUInt8"), "{reason}");
    }
}
