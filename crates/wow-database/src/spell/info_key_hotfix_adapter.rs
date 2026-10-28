//! MariaDB adapter for the exact regular `SpellInfo` key Hotfix overlays.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    PersistenceFutureLikeCpp, SpellInfoKeyContributorHotfixBatchLikeCpp,
    SpellInfoKeyContributorHotfixRowLikeCpp, SpellInfoKeyContributorLikeCpp,
    SpellInfoKeyHotfixLoadOutcomeLikeCpp, SpellInfoKeyHotfixPersistencePortLikeCpp,
    SpellInfoKeyHotfixRowsLikeCpp, SpellInfoPowerDifficultyHotfixRowLikeCpp,
};

use crate::field_like_cpp::{FieldNumberLikeCpp, FieldRowLikeCpp, NativeFieldLikeCpp};
use crate::{HotfixDatabase, HotfixStatements};

const SPELL_EFFECT_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_effect WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_AURA_OPTIONS_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_aura_options WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_AURA_RESTRICTIONS_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_aura_restrictions WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_CASTING_REQUIREMENTS_SQL: &str =
    "SELECT ID, SpellID FROM spell_casting_requirements WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_CATEGORIES_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_categories WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_CLASS_OPTIONS_SQL: &str =
    "SELECT ID, SpellID FROM spell_class_options WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_COOLDOWNS_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_cooldowns WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_EQUIPPED_ITEMS_SQL: &str =
    "SELECT ID, SpellID FROM spell_equipped_items WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_INTERRUPTS_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_interrupts WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_LABEL_SQL: &str = "SELECT ID, SpellID FROM spell_label WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_LEVELS_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_levels WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_MISC_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_misc WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_POWER_SQL: &str = "SELECT ID, SpellID FROM spell_power WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_REAGENTS_SQL: &str =
    "SELECT ID, SpellID FROM spell_reagents WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_REAGENTS_CURRENCY_SQL: &str =
    "SELECT ID, SpellID FROM spell_reagents_currency WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_SCALING_SQL: &str =
    "SELECT ID, SpellID FROM spell_scaling WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_SHAPESHIFT_SQL: &str =
    "SELECT ID, SpellID FROM spell_shapeshift WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_TARGET_RESTRICTIONS_SQL: &str = "SELECT ID, SpellID, DifficultyID FROM spell_target_restrictions WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_TOTEMS_SQL: &str =
    "SELECT ID, SpellID FROM spell_totems WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_X_SPELL_VISUAL_SQL: &str =
    "SELECT ID, SpellID, DifficultyID FROM spell_x_spell_visual WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_POWER_DIFFICULTY_SQL: &str =
    "SELECT ID, DifficultyID FROM spell_power_difficulty WHERE (`VerifiedBuild` > 0) = ?";
const OFFICIAL_THEN_CUSTOM: [bool; 2] = [true, false];

const CONTRIBUTOR_QUERIES_LIKE_CPP: [(SpellInfoKeyContributorLikeCpp, &str, &str); 20] = [
    (
        SpellInfoKeyContributorLikeCpp::SpellEffect,
        SPELL_EFFECT_SQL,
        "spell_effect",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellAuraOptions,
        SPELL_AURA_OPTIONS_SQL,
        "spell_aura_options",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellAuraRestrictions,
        SPELL_AURA_RESTRICTIONS_SQL,
        "spell_aura_restrictions",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellCastingRequirements,
        SPELL_CASTING_REQUIREMENTS_SQL,
        "spell_casting_requirements",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellCategories,
        SPELL_CATEGORIES_SQL,
        "spell_categories",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellClassOptions,
        SPELL_CLASS_OPTIONS_SQL,
        "spell_class_options",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellCooldowns,
        SPELL_COOLDOWNS_SQL,
        "spell_cooldowns",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellEquippedItems,
        SPELL_EQUIPPED_ITEMS_SQL,
        "spell_equipped_items",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellInterrupts,
        SPELL_INTERRUPTS_SQL,
        "spell_interrupts",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellLabel,
        SPELL_LABEL_SQL,
        "spell_label",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellLevels,
        SPELL_LEVELS_SQL,
        "spell_levels",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellMisc,
        SPELL_MISC_SQL,
        "spell_misc",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellPower,
        SPELL_POWER_SQL,
        "spell_power",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellReagents,
        SPELL_REAGENTS_SQL,
        "spell_reagents",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellReagentsCurrency,
        SPELL_REAGENTS_CURRENCY_SQL,
        "spell_reagents_currency",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellScaling,
        SPELL_SCALING_SQL,
        "spell_scaling",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellShapeshift,
        SPELL_SHAPESHIFT_SQL,
        "spell_shapeshift",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellTargetRestrictions,
        SPELL_TARGET_RESTRICTIONS_SQL,
        "spell_target_restrictions",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellTotems,
        SPELL_TOTEMS_SQL,
        "spell_totems",
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellXSpellVisual,
        SPELL_X_SPELL_VISUAL_SQL,
        "spell_x_spell_visual",
    ),
];

fn spell_id_is_signed(contributor: SpellInfoKeyContributorLikeCpp) -> bool {
    matches!(
        contributor,
        SpellInfoKeyContributorLikeCpp::SpellCastingRequirements
            | SpellInfoKeyContributorLikeCpp::SpellClassOptions
            | SpellInfoKeyContributorLikeCpp::SpellEquippedItems
            | SpellInfoKeyContributorLikeCpp::SpellReagents
            | SpellInfoKeyContributorLikeCpp::SpellScaling
            | SpellInfoKeyContributorLikeCpp::SpellShapeshift
            | SpellInfoKeyContributorLikeCpp::SpellTotems
    )
}

fn contributor_has_difficulty(contributor: SpellInfoKeyContributorLikeCpp) -> bool {
    !matches!(
        contributor,
        SpellInfoKeyContributorLikeCpp::SpellCastingRequirements
            | SpellInfoKeyContributorLikeCpp::SpellClassOptions
            | SpellInfoKeyContributorLikeCpp::SpellEquippedItems
            | SpellInfoKeyContributorLikeCpp::SpellLabel
            | SpellInfoKeyContributorLikeCpp::SpellPower
            | SpellInfoKeyContributorLikeCpp::SpellReagents
            | SpellInfoKeyContributorLikeCpp::SpellReagentsCurrency
            | SpellInfoKeyContributorLikeCpp::SpellScaling
            | SpellInfoKeyContributorLikeCpp::SpellShapeshift
            | SpellInfoKeyContributorLikeCpp::SpellTotems
    )
}

fn read_native<T: FieldNumberLikeCpp>(
    native: &[NativeFieldLikeCpp],
    column: usize,
    field: &str,
) -> Result<T> {
    native
        .get_like_cpp(column, field)
        .with_context(|| format!("{} on {field}", T::GETTER))
}

fn decode_contributor_row(
    contributor: SpellInfoKeyContributorLikeCpp,
    native: &[NativeFieldLikeCpp],
    table: &str,
    row_index: usize,
) -> Result<SpellInfoKeyContributorHotfixRowLikeCpp> {
    let record_id = read_native::<u32>(native, 0, &format!("{table}.ID"))
        .with_context(|| format!("{table} row {row_index}"))?;
    let spell_id = if spell_id_is_signed(contributor) {
        read_native::<i32>(native, 1, &format!("{table}.SpellID"))
            .with_context(|| format!("{table} row {row_index}"))? as u32
    } else {
        read_native::<u32>(native, 1, &format!("{table}.SpellID"))
            .with_context(|| format!("{table} row {row_index}"))?
    };
    let difficulty_id = if contributor_has_difficulty(contributor) {
        if contributor == SpellInfoKeyContributorLikeCpp::SpellEffect {
            read_native::<i32>(native, 2, &format!("{table}.DifficultyID"))
                .with_context(|| format!("{table} row {row_index}"))? as u8
        } else {
            read_native::<u8>(native, 2, &format!("{table}.DifficultyID"))
                .with_context(|| format!("{table} row {row_index}"))?
        }
    } else {
        0
    };
    Ok(SpellInfoKeyContributorHotfixRowLikeCpp {
        record_id,
        spell_id,
        difficulty_id,
    })
}

fn decode_power_difficulty_row(
    native: &[NativeFieldLikeCpp],
    row_index: usize,
) -> Result<SpellInfoPowerDifficultyHotfixRowLikeCpp> {
    Ok(SpellInfoPowerDifficultyHotfixRowLikeCpp {
        power_record_id: read_native::<u32>(native, 0, "spell_power_difficulty.ID")
            .with_context(|| format!("spell_power_difficulty row {row_index}"))?,
        difficulty_id: read_native::<u8>(native, 1, "spell_power_difficulty.DifficultyID")
            .with_context(|| format!("spell_power_difficulty row {row_index}"))?,
    })
}

async fn query_contributor_like_cpp(
    db: &HotfixDatabase,
    contributor: SpellInfoKeyContributorLikeCpp,
    sql: &'static str,
    table: &'static str,
) -> Result<Vec<SpellInfoKeyContributorHotfixRowLikeCpp>> {
    let mut rows = Vec::new();
    for official in OFFICIAL_THEN_CUSTOM {
        let mut statement = db.prepare(HotfixStatements::base(sql));
        statement.set_bool(0, official);
        let mut result = db
            .query(&statement)
            .await
            .with_context(|| format!("query {table} official={official}"))?;
        if result.is_empty() {
            continue;
        }
        let mut row_index = 0;
        loop {
            let native = result
                .native_row_like_cpp()
                .with_context(|| format!("{table} row {row_index} native fields"))?;
            rows.push(decode_contributor_row(
                contributor,
                &native,
                table,
                row_index,
            )?);
            row_index += 1;
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
}

async fn query_power_difficulties_like_cpp(
    db: &HotfixDatabase,
) -> Result<Vec<SpellInfoPowerDifficultyHotfixRowLikeCpp>> {
    let mut rows = Vec::new();
    for official in OFFICIAL_THEN_CUSTOM {
        let mut statement = db.prepare(HotfixStatements::base(SPELL_POWER_DIFFICULTY_SQL));
        statement.set_bool(0, official);
        let mut result = db
            .query(&statement)
            .await
            .context("query spell_power_difficulty")?;
        if result.is_empty() {
            continue;
        }
        let mut row_index = 0;
        loop {
            let native = result
                .native_row_like_cpp()
                .with_context(|| format!("spell_power_difficulty row {row_index} native fields"))?;
            rows.push(decode_power_difficulty_row(&native, row_index)?);
            row_index += 1;
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
}

fn classify_key_rows(
    result: Result<SpellInfoKeyHotfixRowsLikeCpp>,
) -> SpellInfoKeyHotfixLoadOutcomeLikeCpp {
    match result {
        Ok(rows) => SpellInfoKeyHotfixLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => SpellInfoKeyHotfixLoadOutcomeLikeCpp::Failed {
            reason: format!("{error:#}"),
        },
    }
}

pub struct MariaDbSpellInfoKeyHotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbSpellInfoKeyHotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl SpellInfoKeyHotfixPersistencePortLikeCpp
    for MariaDbSpellInfoKeyHotfixPersistenceAdapterLikeCpp
{
    fn load_spell_info_key_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, SpellInfoKeyHotfixLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result: Result<SpellInfoKeyHotfixRowsLikeCpp> = async {
                let mut contributor_batches =
                    Vec::with_capacity(CONTRIBUTOR_QUERIES_LIKE_CPP.len());
                let mut power_difficulty_rows = None;
                for (contributor, sql, table) in CONTRIBUTOR_QUERIES_LIKE_CPP {
                    contributor_batches.push(SpellInfoKeyContributorHotfixBatchLikeCpp {
                        contributor,
                        rows: query_contributor_like_cpp(&self.hotfix_db, contributor, sql, table)
                            .await?,
                    });
                    if contributor == SpellInfoKeyContributorLikeCpp::SpellPower {
                        power_difficulty_rows =
                            Some(query_power_difficulties_like_cpp(&self.hotfix_db).await?);
                    }
                }
                Ok(SpellInfoKeyHotfixRowsLikeCpp {
                    contributor_batches,
                    power_difficulty_rows: power_difficulty_rows
                        .expect("the frozen contributor manifest contains SpellPower"),
                })
            }
            .await;

            classify_key_rows(result)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP;

    const SIGNED_SPELL_ID_CONTRIBUTORS: [SpellInfoKeyContributorLikeCpp; 7] = [
        SpellInfoKeyContributorLikeCpp::SpellCastingRequirements,
        SpellInfoKeyContributorLikeCpp::SpellClassOptions,
        SpellInfoKeyContributorLikeCpp::SpellEquippedItems,
        SpellInfoKeyContributorLikeCpp::SpellReagents,
        SpellInfoKeyContributorLikeCpp::SpellScaling,
        SpellInfoKeyContributorLikeCpp::SpellShapeshift,
        SpellInfoKeyContributorLikeCpp::SpellTotems,
    ];
    const BYTE_DIFFICULTY_CONTRIBUTORS: [SpellInfoKeyContributorLikeCpp; 9] = [
        SpellInfoKeyContributorLikeCpp::SpellAuraOptions,
        SpellInfoKeyContributorLikeCpp::SpellAuraRestrictions,
        SpellInfoKeyContributorLikeCpp::SpellCategories,
        SpellInfoKeyContributorLikeCpp::SpellCooldowns,
        SpellInfoKeyContributorLikeCpp::SpellInterrupts,
        SpellInfoKeyContributorLikeCpp::SpellLevels,
        SpellInfoKeyContributorLikeCpp::SpellMisc,
        SpellInfoKeyContributorLikeCpp::SpellTargetRestrictions,
        SpellInfoKeyContributorLikeCpp::SpellXSpellVisual,
    ];

    #[test]
    fn statement_manifest_preserves_cpp_contributor_order_and_verified_build_bind() {
        assert_eq!(OFFICIAL_THEN_CUSTOM, [true, false]);
        assert_eq!(
            CONTRIBUTOR_QUERIES_LIKE_CPP.map(|(contributor, _, _)| contributor),
            SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP
        );
        for (_, sql, _) in CONTRIBUTOR_QUERIES_LIKE_CPP {
            assert!(sql.ends_with("WHERE (`VerifiedBuild` > 0) = ?"));
        }
        assert!(SPELL_POWER_DIFFICULTY_SQL.ends_with("WHERE (`VerifiedBuild` > 0) = ?"));

        let mut statement_order = Vec::new();
        for (contributor, sql, _) in CONTRIBUTOR_QUERIES_LIKE_CPP {
            statement_order.push(sql);
            if contributor == wow_persistence::SpellInfoKeyContributorLikeCpp::SpellPower {
                statement_order.push(SPELL_POWER_DIFFICULTY_SQL);
            }
        }
        assert_eq!(statement_order.len(), 21);
        assert_eq!(statement_order[12], super::SPELL_POWER_SQL);
        assert_eq!(statement_order[13], SPELL_POWER_DIFFICULTY_SQL);
        assert_eq!(statement_order[14], super::SPELL_REAGENTS_SQL);
    }

    fn fields(values: Vec<NativeFieldLikeCpp>) -> Vec<NativeFieldLikeCpp> {
        values
    }

    #[test]
    fn contributor_getters_preserve_source_width_and_signed_spell_ids() {
        let contributors = SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP;
        assert_eq!(contributors.len(), 20);
        for contributor in contributors {
            let signed_spell_id = SIGNED_SPELL_ID_CONTRIBUTORS.contains(&contributor);
            let has_byte_difficulty = BYTE_DIFFICULTY_CONTRIBUTORS.contains(&contributor);
            let id = NativeFieldLikeCpp::UInt64(u64::from(u32::MAX));
            let spell_id = if signed_spell_id {
                NativeFieldLikeCpp::Int64(-1)
            } else {
                NativeFieldLikeCpp::UInt64(u64::from(u32::MAX))
            };
            let mut input = vec![id, spell_id];
            if contributor == SpellInfoKeyContributorLikeCpp::SpellEffect {
                input.push(NativeFieldLikeCpp::Int32(256));
            } else if has_byte_difficulty {
                input.push(NativeFieldLikeCpp::UInt8(231));
            }
            let decoded = decode_contributor_row(contributor, &input, "test_table", 0).unwrap();
            assert_eq!(decoded.record_id, u32::MAX, "{contributor:?}");
            assert_eq!(decoded.spell_id, u32::MAX, "{contributor:?}");
            assert_eq!(
                decoded.difficulty_id,
                if contributor == SpellInfoKeyContributorLikeCpp::SpellEffect {
                    0
                } else if has_byte_difficulty {
                    231
                } else {
                    0
                },
                "{contributor:?}"
            );

            let wrong_spell_width = if signed_spell_id {
                NativeFieldLikeCpp::UInt64(u64::from(u32::MAX))
            } else {
                NativeFieldLikeCpp::Int64(-1)
            };
            assert!(
                decode_contributor_row(
                    contributor,
                    &fields(vec![
                        NativeFieldLikeCpp::UInt32(1),
                        wrong_spell_width,
                        NativeFieldLikeCpp::UInt8(0),
                    ]),
                    "test_table",
                    0,
                )
                .is_err(),
                "{contributor:?} must enforce the source SpellID getter"
            );
        }
    }

    #[test]
    fn difficulty_and_power_difficulty_widths_are_checked() {
        let effect = SpellInfoKeyContributorLikeCpp::SpellEffect;
        for (source, expected) in [(-1, u8::MAX), (256, 0)] {
            let row = decode_contributor_row(
                effect,
                &fields(vec![
                    NativeFieldLikeCpp::UInt32(4),
                    NativeFieldLikeCpp::Int32(5),
                    NativeFieldLikeCpp::Int32(source),
                ]),
                "spell_effect",
                7,
            )
            .unwrap();
            assert_eq!(row.difficulty_id, expected);
        }

        for contributor in BYTE_DIFFICULTY_CONTRIBUTORS {
            let error = decode_contributor_row(
                contributor,
                &fields(vec![
                    NativeFieldLikeCpp::UInt32(1),
                    NativeFieldLikeCpp::UInt32(2),
                    NativeFieldLikeCpp::UInt16(256),
                ]),
                "checked_table",
                3,
            )
            .unwrap_err();
            let error = format!("{error:#}");
            assert!(error.contains("checked_table row 3"), "{error}");
            assert!(error.contains("DifficultyID"), "{error}");
            assert!(error.contains("GetUInt8"), "{error}");
        }

        let power = decode_power_difficulty_row(
            &fields(vec![
                NativeFieldLikeCpp::UInt64(u64::from(u32::MAX)),
                NativeFieldLikeCpp::UInt8(u8::MAX),
            ]),
            0,
        )
        .unwrap();
        assert_eq!(power.power_record_id, u32::MAX);
        assert_eq!(power.difficulty_id, u8::MAX);
        let error = decode_power_difficulty_row(
            &fields(vec![
                NativeFieldLikeCpp::UInt32(1),
                NativeFieldLikeCpp::UInt16(256),
            ]),
            9,
        )
        .unwrap_err();
        let error = format!("{error:#}");
        assert!(error.contains("spell_power_difficulty row 9"));
        assert!(error.contains("DifficultyID"));
        assert!(error.contains("GetUInt8"));
    }

    #[test]
    fn null_defaults_but_malformed_or_missing_columns_fail_with_row_context() {
        let contributor = SpellInfoKeyContributorLikeCpp::SpellEffect;
        assert_eq!(
            decode_contributor_row(
                contributor,
                &fields(vec![
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                    NativeFieldLikeCpp::Null,
                ]),
                "spell_effect",
                2,
            )
            .unwrap(),
            SpellInfoKeyContributorHotfixRowLikeCpp {
                record_id: 0,
                spell_id: 0,
                difficulty_id: 0,
            }
        );
        for bad in [
            NativeFieldLikeCpp::Text("3".into()),
            NativeFieldLikeCpp::Unsupported("DECIMAL".into()),
            NativeFieldLikeCpp::Int64(i64::MAX),
        ] {
            let error = decode_contributor_row(
                contributor,
                &fields(vec![NativeFieldLikeCpp::UInt32(1), bad]),
                "spell_effect",
                4,
            )
            .unwrap_err();
            let error = format!("{error:#}");
            assert!(error.contains("spell_effect row 4"), "{error}");
            assert!(error.contains("SpellID"), "{error}");
            assert!(error.contains("GetUInt32"), "{error}");
        }
        let error = decode_contributor_row(
            contributor,
            &[NativeFieldLikeCpp::UInt32(1)],
            "spell_effect",
            6,
        )
        .unwrap_err();
        let error = format!("{error:#}");
        assert!(error.contains("spell_effect row 6"), "{error}");
        assert!(error.contains("spell_effect.SpellID"), "{error}");
        assert!(error.contains("GetUInt32"), "{error}");
        assert!(error.contains("outside"), "{error}");
    }

    #[test]
    fn malformed_contributor_row_fails_the_whole_load_with_chained_context() {
        let error = decode_contributor_row(
            SpellInfoKeyContributorLikeCpp::SpellEffect,
            &fields(vec![
                NativeFieldLikeCpp::UInt32(1),
                NativeFieldLikeCpp::UInt32(2),
                NativeFieldLikeCpp::UInt64(u64::from(u32::MAX) + 1),
            ]),
            "spell_effect",
            5,
        )
        .unwrap_err();
        let result: Result<SpellInfoKeyHotfixRowsLikeCpp> = Err(error);
        let reason = match classify_key_rows(result) {
            SpellInfoKeyHotfixLoadOutcomeLikeCpp::Failed { reason } => reason,
            SpellInfoKeyHotfixLoadOutcomeLikeCpp::Loaded(_) => {
                panic!("one malformed contributor must fail the full load")
            }
        };
        assert!(reason.contains("spell_effect row 5"), "{reason}");
        assert!(reason.contains("spell_effect.DifficultyID"), "{reason}");
        assert!(reason.contains("GetInt32"), "{reason}");
    }
}
