//! MariaDB adapter for the DB2 overlays that hydrate core `SpellInfo`.

mod decode;

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, SpellAuraOptionsHotfixRow, SpellAuraRestrictionsHotfixRowLikeCpp,
    SpellCastTimesHotfixRowLikeCpp, SpellCastingRequirementsHotfixRowLikeCpp,
    SpellCategoriesHotfixRowLikeCpp, SpellCategoryHotfixRowLikeCpp, SpellCooldownsHotfixRowLikeCpp,
    SpellClassOptionsHotfixRow, SpellCoreDb2HotfixLoadOutcomeLikeCpp,
    SpellCoreDb2HotfixPersistencePortLikeCpp, SpellDurationHotfixRowLikeCpp,
    SpellEffectHotfixRowLikeCpp, SpellEquippedItemsHotfixRowLikeCpp,
    SpellInterruptsHotfixRowLikeCpp, SpellMiscHotfixRowLikeCpp, SpellNameHotfixRowLikeCpp,
    SpellLabelHotfixRow, SpellLevelsHotfixRow, SpellPowerDifficultyHotfixRowLikeCpp,
    SpellPowerHotfixRowLikeCpp, SpellProcsPerMinuteHotfixRow, SpellRadiusHotfixRowLikeCpp,
    SpellRangeHotfixRowLikeCpp,
    SpellShapeshiftHotfixRowLikeCpp,
    SpellTargetRestrictionsHotfixRowLikeCpp, SpellXSpellVisualHotfixRowLikeCpp,
};

use anyhow::{Context, Result};

use crate::field_like_cpp::NativeFieldLikeCpp;
use crate::{HotfixDatabase, HotfixStatements};

const SPELL_CATEGORIES_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, DifficultyID, Category, DefenseType, DispelType, Mechanic, ",
    "PreventionType, StartRecoveryCategory, ChargeCategory, SpellID ",
    "FROM spell_categories WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_MISC_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, Attributes1, Attributes2, Attributes3, Attributes4, ",
    "Attributes5, Attributes6, Attributes7, Attributes8, Attributes9, ",
    "Attributes10, Attributes11, Attributes12, Attributes13, Attributes14, ",
    "Attributes15, DifficultyID, CastingTimeIndex, DurationIndex, RangeIndex, ",
    "SchoolMask, Speed, LaunchDelay, MinDuration, SpellIconFileDataID, ",
    "ActiveIconFileDataID, ContentTuningID, ShowFutureSpellPlayerConditionID, ",
    "SpellID FROM spell_misc WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_EFFECT_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, DifficultyID, EffectIndex, Effect, EffectAmplitude, ",
    "EffectAttributes, EffectAura, EffectAuraPeriod, EffectBasePoints, ",
    "EffectBonusCoefficient, EffectChainAmplitude, EffectChainTargets, ",
    "EffectDieSides, EffectItemType, EffectMechanic, EffectPointsPerResource, ",
    "EffectPosFacing, EffectRealPointsPerLevel, EffectTriggerSpell, ",
    "BonusCoefficientFromAP, PvpMultiplier, Coefficient, Variance, ",
    "ResourceCoefficient, GroupSizeBasePointsCoefficient, EffectMiscValue1, ",
    "EffectMiscValue2, EffectRadiusIndex1, EffectRadiusIndex2, ",
    "EffectSpellClassMask1, EffectSpellClassMask2, EffectSpellClassMask3, ",
    "EffectSpellClassMask4, ImplicitTarget1, ImplicitTarget2, SpellID ",
    "FROM spell_effect WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_SHAPESHIFT_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, SpellID, StanceBarOrder, ShapeshiftExclude1, ShapeshiftExclude2, ",
    "ShapeshiftMask1, ShapeshiftMask2 FROM spell_shapeshift ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_CAST_TIMES_SQL_LIKE_CPP: &str =
    "SELECT ID, Base, PerLevel, Minimum FROM spell_cast_times WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_COOLDOWNS_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, DifficultyID, CategoryRecoveryTime, RecoveryTime, ",
    "StartRecoveryTime, SpellID FROM spell_cooldowns ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_CASTING_REQUIREMENTS_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, SpellID, FacingCasterFlags, MinFactionID, MinReputation, ",
    "RequiredAreasID, RequiredAuraVision, RequiresSpellFocus ",
    "FROM spell_casting_requirements WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_POWER_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, OrderIndex, ManaCost, ManaCostPerLevel, ManaPerSecond, ",
    "PowerDisplayID, AltPowerBarID, PowerCostPct, PowerCostMaxPct, ",
    "PowerPctPerSecond, PowerType, RequiredAuraSpellID, OptionalCost, SpellID ",
    "FROM spell_power WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_POWER_DIFFICULTY_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, DifficultyID, OrderIndex FROM spell_power_difficulty ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_AURA_RESTRICTIONS_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, DifficultyID, CasterAuraState, TargetAuraState, ",
    "ExcludeCasterAuraState, ExcludeTargetAuraState, CasterAuraSpell, ",
    "TargetAuraSpell, ExcludeCasterAuraSpell, ExcludeTargetAuraSpell, SpellID ",
    "FROM spell_aura_restrictions WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_CATEGORY_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, Name, Flags, UsesPerWeek, MaxCharges, ChargeRecoveryTime, ",
    "TypeMask FROM spell_category WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_DURATION_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, Duration, DurationPerLevel, MaxDuration FROM spell_duration ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_RADIUS_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, Radius, RadiusPerLevel, RadiusMin, RadiusMax FROM spell_radius ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_RANGE_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, DisplayName, DisplayNameShort, Flags, ",
    "RangeMin1, RangeMin2, RangeMax1, RangeMax2 ",
    "FROM spell_range WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_EQUIPPED_ITEMS_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, SpellID, EquippedItemClass, EquippedItemInvTypes, EquippedItemSubclass ",
    "FROM spell_equipped_items WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_TARGET_RESTRICTIONS_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, DifficultyID, ConeDegrees, MaxTargets, MaxTargetLevel, ",
    "TargetCreatureType, Targets, Width, SpellID FROM spell_target_restrictions ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_X_SPELL_VISUAL_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, DifficultyID, SpellVisualID, Probability, Flags, Priority, ",
    "SpellIconFileID, ActiveIconFileID, ViewerUnitConditionID, ",
    "ViewerPlayerConditionID, CasterUnitConditionID, CasterPlayerConditionID, SpellID ",
    "FROM spell_x_spell_visual WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_AURA_OPTIONS_SQL: &str = concat!(
    "SELECT ID, DifficultyID, CumulativeAura, ProcCategoryRecovery, ProcChance, ProcCharges, ",
    "SpellProcsPerMinuteID, ProcTypeMask1, ProcTypeMask2, SpellID ",
    "FROM spell_aura_options WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_CLASS_OPTIONS_SQL: &str = concat!(
    "SELECT ID, SpellID, ModalNextSpell, SpellClassSet, SpellClassMask1, SpellClassMask2, ",
    "SpellClassMask3, SpellClassMask4 FROM spell_class_options ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_LABEL_SQL: &str =
    "SELECT ID, LabelID, SpellID FROM spell_label WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_LEVELS_SQL: &str = concat!(
    "SELECT ID, DifficultyID, BaseLevel, MaxLevel, SpellLevel, MaxPassiveAuraLevel, SpellID ",
    "FROM spell_levels WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_PROCS_PER_MINUTE_SQL: &str =
    "SELECT ID, BaseProcRate, Flags FROM spell_procs_per_minute WHERE (`VerifiedBuild` > 0) = ?";
const OFFICIAL_THEN_CUSTOM_LIKE_CPP: [bool; 2] = [true, false];

#[cfg(test)]
const CORE_STATEMENT_ORDER_LIKE_RUST: [&str; 19] = [
    "SEL_SPELL_NAME",
    "SPELL_CATEGORIES_SQL_LIKE_CPP",
    "SPELL_MISC_SQL_LIKE_CPP",
    "SPELL_EFFECT_SQL_LIKE_CPP",
    "SPELL_SHAPESHIFT_SQL_LIKE_CPP",
    "SEL_SPELL_INTERRUPTS",
    "SPELL_CAST_TIMES_SQL_LIKE_CPP",
    "SPELL_COOLDOWNS_SQL_LIKE_CPP",
    "SPELL_CASTING_REQUIREMENTS_SQL_LIKE_CPP",
    "SPELL_POWER_SQL_LIKE_CPP",
    "SPELL_POWER_DIFFICULTY_SQL_LIKE_CPP",
    "SPELL_AURA_RESTRICTIONS_SQL_LIKE_CPP",
    "SPELL_CATEGORY_SQL_LIKE_CPP",
    "SPELL_DURATION_SQL_LIKE_CPP",
    "SPELL_RADIUS_SQL_LIKE_CPP",
    "SPELL_RANGE_SQL_LIKE_CPP",
    "SPELL_EQUIPPED_ITEMS_SQL_LIKE_CPP",
    "SPELL_TARGET_RESTRICTIONS_SQL_LIKE_CPP",
    "SPELL_X_SPELL_VISUAL_SQL_LIKE_CPP",
];

async fn query_official_then_custom_like_cpp<T>(
    db: &HotfixDatabase,
    statement: HotfixStatements,
    table: &str,
    mut decode: impl FnMut(&[NativeFieldLikeCpp]) -> Result<T>,
) -> Result<Vec<T>> {
    let mut rows = Vec::new();
    let mut row_index = 0;
    for official in OFFICIAL_THEN_CUSTOM_LIKE_CPP {
        let mut statement = db.prepare(statement);
        statement.set_bool(0, official);
        let mut result = db
            .query(&statement)
            .await
            .with_context(|| format!("{table} query failed for official={official}"))?;
        if result.is_empty() {
            continue;
        }
        loop {
            let native_row = result
                .native_row_like_cpp()
                .with_context(|| format!("{table} row {row_index} native decode failed"))?;
            append_decoded_row(&mut rows, table, row_index, &native_row, &mut decode)?;
            row_index += 1;
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
}

fn classify_rows_like_cpp<T>(
    result: Result<Vec<T>>,
) -> SpellCoreDb2HotfixLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => SpellCoreDb2HotfixLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => SpellCoreDb2HotfixLoadOutcomeLikeCpp::Failed {
            reason: format!("{error:#}"),
        },
    }
}

fn append_decoded_row<T>(
    rows: &mut Vec<T>,
    table: &str,
    row_index: usize,
    native_row: &[NativeFieldLikeCpp],
    decode: &mut impl FnMut(&[NativeFieldLikeCpp]) -> Result<T>,
) -> Result<()> {
    let decoded = decode(native_row).with_context(|| format!("{table} row {row_index}"))?;
    rows.push(decoded);
    Ok(())
}

pub struct MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl SpellCoreDb2HotfixPersistencePortLikeCpp
    for MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp
{
    fn load_spell_name_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellNameHotfixRowLikeCpp>>
    {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::SEL_SPELL_NAME,
                    "spell_name",
                    decode::spell_name,
                )
                .await,
            )
        })
    }

    fn load_spell_categories_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCategoriesHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_CATEGORIES_SQL_LIKE_CPP),
                    "spell_categories",
                    decode::spell_categories,
                )
                .await,
            )
        })
    }

    fn load_spell_misc_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellMiscHotfixRowLikeCpp>>
    {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_MISC_SQL_LIKE_CPP),
                    "spell_misc",
                    decode::spell_misc,
                )
                .await,
            )
        })
    }

    fn load_spell_effect_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellEffectHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_EFFECT_SQL_LIKE_CPP),
                    "spell_effect",
                    decode::spell_effect,
                )
                .await,
            )
        })
    }

    fn load_spell_shapeshift_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellShapeshiftHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_SHAPESHIFT_SQL_LIKE_CPP),
                    "spell_shapeshift",
                    decode::spell_shapeshift,
                )
                .await,
            )
        })
    }

    fn load_spell_interrupts_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellInterruptsHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::SEL_SPELL_INTERRUPTS,
                    "spell_interrupts",
                    decode::spell_interrupts,
                )
                .await,
            )
        })
    }

    fn load_spell_cast_times_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCastTimesHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_CAST_TIMES_SQL_LIKE_CPP),
                    "spell_cast_times",
                    decode::spell_cast_times,
                )
                .await,
            )
        })
    }

    fn load_spell_cooldowns_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCooldownsHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_COOLDOWNS_SQL_LIKE_CPP),
                    "spell_cooldowns",
                    decode::spell_cooldowns,
                )
                .await,
            )
        })
    }

    fn load_spell_casting_requirements_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCastingRequirementsHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_CASTING_REQUIREMENTS_SQL_LIKE_CPP),
                    "spell_casting_requirements",
                    decode::spell_casting_requirements,
                )
                .await,
            )
        })
    }

    fn load_spell_power_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellPowerHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_POWER_SQL_LIKE_CPP),
                    "spell_power",
                    decode::spell_power,
                )
                .await,
            )
        })
    }

    fn load_spell_power_difficulty_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellPowerDifficultyHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_POWER_DIFFICULTY_SQL_LIKE_CPP),
                    "spell_power_difficulty",
                    decode::spell_power_difficulty,
                )
                .await,
            )
        })
    }

    fn load_spell_aura_restrictions_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellAuraRestrictionsHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_AURA_RESTRICTIONS_SQL_LIKE_CPP),
                    "spell_aura_restrictions",
                    decode::spell_aura_restrictions,
                )
                .await,
            )
        })
    }

    fn load_spell_category_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellCategoryHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_CATEGORY_SQL_LIKE_CPP),
                    "spell_category",
                    decode::spell_category,
                )
                .await,
            )
        })
    }

    fn load_spell_duration_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellDurationHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_DURATION_SQL_LIKE_CPP),
                    "spell_duration",
                    decode::spell_duration,
                )
                .await,
            )
        })
    }

    fn load_spell_radius_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellRadiusHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_RADIUS_SQL_LIKE_CPP),
                    "spell_radius",
                    decode::spell_radius,
                )
                .await,
            )
        })
    }

    fn load_spell_range_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellRangeHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_RANGE_SQL_LIKE_CPP),
                    "spell_range",
                    decode::spell_range,
                )
                .await,
            )
        })
    }

    fn load_spell_equipped_items_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellEquippedItemsHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_EQUIPPED_ITEMS_SQL_LIKE_CPP),
                    "spell_equipped_items",
                    decode::spell_equipped_items,
                )
                .await,
            )
        })
    }

    fn load_spell_target_restrictions_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellTargetRestrictionsHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_TARGET_RESTRICTIONS_SQL_LIKE_CPP),
                    "spell_target_restrictions",
                    decode::spell_target_restrictions,
                )
                .await,
            )
        })
    }

    fn load_spell_x_spell_visual_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellXSpellVisualHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_X_SPELL_VISUAL_SQL_LIKE_CPP),
                    "spell_x_spell_visual",
                    decode::spell_x_spell_visual,
                )
                .await,
            )
        })
    }

    fn load_spell_aura_options_rows(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellAuraOptionsHotfixRow>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_AURA_OPTIONS_SQL),
                    "spell_aura_options",
                    decode::spell_aura_options,
                )
                .await,
            )
        })
    }

    fn load_spell_class_options_rows(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellClassOptionsHotfixRow>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_CLASS_OPTIONS_SQL),
                    "spell_class_options",
                    decode::spell_class_options,
                )
                .await,
            )
        })
    }

    fn load_spell_label_rows(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellLabelHotfixRow>> {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_LABEL_SQL),
                    "spell_label",
                    decode::spell_label,
                )
                .await,
            )
        })
    }

    fn load_spell_levels_rows(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellLevelsHotfixRow>> {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_LEVELS_SQL),
                    "spell_levels",
                    decode::spell_levels,
                )
                .await,
            )
        })
    }

    fn load_spell_procs_per_minute_rows(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellCoreDb2HotfixLoadOutcomeLikeCpp<SpellProcsPerMinuteHotfixRow>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_official_then_custom_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::base(SPELL_PROCS_PER_MINUTE_SQL),
                    "spell_procs_per_minute",
                    decode::spell_procs_per_minute,
                )
                .await,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_spellinfo_statement_order_keeps_bootstrap_hydration_order() {
        assert_eq!(OFFICIAL_THEN_CUSTOM_LIKE_CPP, [true, false]);
        assert_eq!(
            CORE_STATEMENT_ORDER_LIKE_RUST,
            [
                "SEL_SPELL_NAME",
                "SPELL_CATEGORIES_SQL_LIKE_CPP",
                "SPELL_MISC_SQL_LIKE_CPP",
                "SPELL_EFFECT_SQL_LIKE_CPP",
                "SPELL_SHAPESHIFT_SQL_LIKE_CPP",
                "SEL_SPELL_INTERRUPTS",
                "SPELL_CAST_TIMES_SQL_LIKE_CPP",
                "SPELL_COOLDOWNS_SQL_LIKE_CPP",
                "SPELL_CASTING_REQUIREMENTS_SQL_LIKE_CPP",
                "SPELL_POWER_SQL_LIKE_CPP",
                "SPELL_POWER_DIFFICULTY_SQL_LIKE_CPP",
                "SPELL_AURA_RESTRICTIONS_SQL_LIKE_CPP",
                "SPELL_CATEGORY_SQL_LIKE_CPP",
                "SPELL_DURATION_SQL_LIKE_CPP",
                "SPELL_RADIUS_SQL_LIKE_CPP",
                "SPELL_RANGE_SQL_LIKE_CPP",
                "SPELL_EQUIPPED_ITEMS_SQL_LIKE_CPP",
                "SPELL_TARGET_RESTRICTIONS_SQL_LIKE_CPP",
                "SPELL_X_SPELL_VISUAL_SQL_LIKE_CPP",
            ]
        );
        assert_eq!(
            HotfixStatements::base(SPELL_EFFECT_SQL_LIKE_CPP),
            HotfixStatements::base(SPELL_EFFECT_SQL_LIKE_CPP)
        );
        for sql in [
            SPELL_AURA_RESTRICTIONS_SQL_LIKE_CPP,
            SPELL_CATEGORY_SQL_LIKE_CPP,
            SPELL_DURATION_SQL_LIKE_CPP,
            SPELL_RADIUS_SQL_LIKE_CPP,
            SPELL_RANGE_SQL_LIKE_CPP,
            SPELL_EQUIPPED_ITEMS_SQL_LIKE_CPP,
            SPELL_TARGET_RESTRICTIONS_SQL_LIKE_CPP,
            SPELL_X_SPELL_VISUAL_SQL_LIKE_CPP,
        ] {
            assert!(sql.ends_with("WHERE (`VerifiedBuild` > 0) = ?"));
        }
    }

    #[test]
    fn selected_scalar_queries_keep_source_projection_and_bind_order() {
        assert_eq!(OFFICIAL_THEN_CUSTOM_LIKE_CPP, [true, false]);
        assert_eq!(
            SPELL_AURA_OPTIONS_SQL,
            concat!(
                "SELECT ID, DifficultyID, CumulativeAura, ProcCategoryRecovery, ProcChance, ",
                "ProcCharges, SpellProcsPerMinuteID, ProcTypeMask1, ProcTypeMask2, SpellID ",
                "FROM spell_aura_options WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
        assert_eq!(
            SPELL_CLASS_OPTIONS_SQL,
            concat!(
                "SELECT ID, SpellID, ModalNextSpell, SpellClassSet, SpellClassMask1, ",
                "SpellClassMask2, SpellClassMask3, SpellClassMask4 FROM spell_class_options ",
                "WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
        assert_eq!(
            SPELL_LABEL_SQL,
            "SELECT ID, LabelID, SpellID FROM spell_label WHERE (`VerifiedBuild` > 0) = ?"
        );
        assert_eq!(
            SPELL_LEVELS_SQL,
            concat!(
                "SELECT ID, DifficultyID, BaseLevel, MaxLevel, SpellLevel, ",
                "MaxPassiveAuraLevel, SpellID FROM spell_levels ",
                "WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
        assert_eq!(
            SPELL_PROCS_PER_MINUTE_SQL,
            "SELECT ID, BaseProcRate, Flags FROM spell_procs_per_minute WHERE (`VerifiedBuild` > 0) = ?"
        );
    }
}
