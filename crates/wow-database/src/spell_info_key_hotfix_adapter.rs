//! MariaDB adapter for the exact regular `SpellInfo` key Hotfix overlays.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, SpellInfoKeyContributorHotfixBatchLikeCpp,
    SpellInfoKeyContributorHotfixRowLikeCpp, SpellInfoKeyContributorLikeCpp,
    SpellInfoKeyHotfixLoadOutcomeLikeCpp, SpellInfoKeyHotfixPersistencePortLikeCpp,
    SpellInfoKeyHotfixRowsLikeCpp, SpellInfoPowerDifficultyHotfixRowLikeCpp,
};

use crate::{DatabaseError, HotfixDatabase, HotfixStatements, SqlResult};

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

const CONTRIBUTOR_QUERIES_LIKE_CPP: [(SpellInfoKeyContributorLikeCpp, &str, bool); 20] = [
    (
        SpellInfoKeyContributorLikeCpp::SpellEffect,
        SPELL_EFFECT_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellAuraOptions,
        SPELL_AURA_OPTIONS_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellAuraRestrictions,
        SPELL_AURA_RESTRICTIONS_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellCastingRequirements,
        SPELL_CASTING_REQUIREMENTS_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellCategories,
        SPELL_CATEGORIES_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellClassOptions,
        SPELL_CLASS_OPTIONS_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellCooldowns,
        SPELL_COOLDOWNS_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellEquippedItems,
        SPELL_EQUIPPED_ITEMS_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellInterrupts,
        SPELL_INTERRUPTS_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellLabel,
        SPELL_LABEL_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellLevels,
        SPELL_LEVELS_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellMisc,
        SPELL_MISC_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellPower,
        SPELL_POWER_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellReagents,
        SPELL_REAGENTS_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellReagentsCurrency,
        SPELL_REAGENTS_CURRENCY_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellScaling,
        SPELL_SCALING_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellShapeshift,
        SPELL_SHAPESHIFT_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellTargetRestrictions,
        SPELL_TARGET_RESTRICTIONS_SQL,
        true,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellTotems,
        SPELL_TOTEMS_SQL,
        false,
    ),
    (
        SpellInfoKeyContributorLikeCpp::SpellXSpellVisual,
        SPELL_X_SPELL_VISUAL_SQL,
        true,
    ),
];

fn read_u32_like_cpp(result: &SqlResult, column: usize) -> u32 {
    result
        .try_read::<u32>(column)
        .or_else(|| result.try_read::<i32>(column).map(|value| value as u32))
        .or_else(|| result.try_read::<u64>(column).map(|value| value as u32))
        .or_else(|| result.try_read::<i64>(column).map(|value| value as u32))
        .unwrap_or(0)
}

fn read_u8_like_cpp(result: &SqlResult, column: usize) -> u8 {
    result
        .try_read::<u8>(column)
        .or_else(|| result.try_read::<u16>(column).map(|value| value as u8))
        .or_else(|| result.try_read::<u32>(column).map(|value| value as u8))
        .or_else(|| result.try_read::<i32>(column).map(|value| value as u8))
        .unwrap_or(0)
}

async fn query_contributor_like_cpp(
    db: &HotfixDatabase,
    sql: &'static str,
    has_difficulty: bool,
) -> Result<Vec<SpellInfoKeyContributorHotfixRowLikeCpp>, DatabaseError> {
    let mut rows = Vec::new();
    for official in [true, false] {
        let mut statement = db.prepare(HotfixStatements::base(sql));
        statement.set_bool(0, official);
        let mut result = db.query(&statement).await?;
        if result.is_empty() {
            continue;
        }
        loop {
            rows.push(SpellInfoKeyContributorHotfixRowLikeCpp {
                record_id: read_u32_like_cpp(&result, 0),
                spell_id: read_u32_like_cpp(&result, 1),
                difficulty_id: has_difficulty
                    .then(|| read_u8_like_cpp(&result, 2))
                    .unwrap_or(0),
            });
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
}

async fn query_power_difficulties_like_cpp(
    db: &HotfixDatabase,
) -> Result<Vec<SpellInfoPowerDifficultyHotfixRowLikeCpp>, DatabaseError> {
    let mut rows = Vec::new();
    for official in [true, false] {
        let mut statement = db.prepare(HotfixStatements::base(SPELL_POWER_DIFFICULTY_SQL));
        statement.set_bool(0, official);
        let mut result = db.query(&statement).await?;
        if result.is_empty() {
            continue;
        }
        loop {
            rows.push(SpellInfoPowerDifficultyHotfixRowLikeCpp {
                power_record_id: read_u32_like_cpp(&result, 0),
                difficulty_id: read_u8_like_cpp(&result, 1),
            });
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
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
            let result: Result<SpellInfoKeyHotfixRowsLikeCpp, DatabaseError> = async {
                let mut contributor_batches =
                    Vec::with_capacity(CONTRIBUTOR_QUERIES_LIKE_CPP.len());
                let mut power_difficulty_rows = None;
                for (contributor, sql, has_difficulty) in CONTRIBUTOR_QUERIES_LIKE_CPP {
                    contributor_batches.push(SpellInfoKeyContributorHotfixBatchLikeCpp {
                        contributor,
                        rows: query_contributor_like_cpp(&self.hotfix_db, sql, has_difficulty)
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

            match result {
                Ok(rows) => SpellInfoKeyHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => SpellInfoKeyHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CONTRIBUTOR_QUERIES_LIKE_CPP, SPELL_POWER_DIFFICULTY_SQL};
    use wow_persistence::SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP;

    #[test]
    fn statement_manifest_preserves_cpp_contributor_order_and_verified_build_bind() {
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
}
