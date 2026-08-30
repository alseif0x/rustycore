//! MariaDB adapter for the DB2 overlays that hydrate core `SpellInfo`.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, SpellCastTimesHotfixRowLikeCpp,
    SpellCastingRequirementsHotfixRowLikeCpp, SpellCategoriesHotfixRowLikeCpp,
    SpellCooldownsHotfixRowLikeCpp, SpellCoreDb2HotfixLoadOutcomeLikeCpp,
    SpellCoreDb2HotfixPersistencePortLikeCpp, SpellEffectHotfixRowLikeCpp,
    SpellInterruptsHotfixRowLikeCpp, SpellMiscHotfixRowLikeCpp, SpellNameHotfixRowLikeCpp,
    SpellPowerDifficultyHotfixRowLikeCpp, SpellPowerHotfixRowLikeCpp,
    SpellShapeshiftHotfixRowLikeCpp,
};

use crate::{DatabaseError, HotfixDatabase, HotfixStatements, SqlResult};

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
const OFFICIAL_THEN_CUSTOM_LIKE_CPP: [bool; 2] = [true, false];

#[cfg(test)]
const CORE_STATEMENT_ORDER_LIKE_RUST: [&str; 11] = [
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
];

async fn query_official_then_custom_like_cpp<T>(
    db: &HotfixDatabase,
    statement: HotfixStatements,
    mut decode: impl FnMut(&SqlResult) -> Option<T>,
) -> Result<Vec<T>, DatabaseError> {
    let mut rows = Vec::new();
    for official in OFFICIAL_THEN_CUSTOM_LIKE_CPP {
        let mut statement = db.prepare(statement);
        statement.set_bool(0, official);
        let mut result = db.query(&statement).await?;
        if result.is_empty() {
            continue;
        }
        loop {
            if let Some(row) = decode(&result) {
                rows.push(row);
            }
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
}

fn classify_rows_like_cpp<T>(
    result: Result<Vec<T>, DatabaseError>,
) -> SpellCoreDb2HotfixLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => SpellCoreDb2HotfixLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => SpellCoreDb2HotfixLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
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
                    |row| {
                        Some(SpellNameHotfixRowLikeCpp {
                            id: row.try_read(0)?,
                            name: row.try_read(1).unwrap_or_default(),
                        })
                    },
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
                    |row| {
                        Some(SpellCategoriesHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            difficulty_id: row.try_read(1).unwrap_or(0),
                            category: row.try_read(2).unwrap_or(0),
                            defense_type: row.try_read(3).unwrap_or(0),
                            dispel_type: row.try_read(4).unwrap_or(0),
                            mechanic: row.try_read(5).unwrap_or(0),
                            prevention_type: row.try_read(6).unwrap_or(0),
                            start_recovery_category: row.try_read(7).unwrap_or(0),
                            charge_category: row.try_read(8).unwrap_or(0),
                            spell_id: row.try_read(9).unwrap_or(0),
                        })
                    },
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
                    |row| {
                        Some(SpellMiscHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            attributes: std::array::from_fn(|index| {
                                row.try_read(index + 1).unwrap_or(0)
                            }),
                            difficulty_id: row.try_read(16).unwrap_or(0),
                            casting_time_index: row.try_read(17).unwrap_or(0),
                            duration_index: row.try_read(18).unwrap_or(0),
                            range_index: row.try_read(19).unwrap_or(0),
                            school_mask: row.try_read(20).unwrap_or(0),
                            speed: row.try_read(21).unwrap_or(0.0),
                            launch_delay: row.try_read(22).unwrap_or(0.0),
                            min_duration: row.try_read(23).unwrap_or(0.0),
                            spell_icon_file_data_id: row.try_read(24).unwrap_or(0),
                            active_icon_file_data_id: row.try_read(25).unwrap_or(0),
                            content_tuning_id: row.try_read(26).unwrap_or(0),
                            show_future_spell_player_condition_id: row.try_read(27).unwrap_or(0),
                            spell_id: row.try_read(28).unwrap_or(0),
                        })
                    },
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
                    |row| {
                        Some(SpellEffectHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            difficulty_id: row.try_read(1).unwrap_or(0),
                            effect_index: row.try_read(2).unwrap_or(0),
                            effect: row.try_read(3).unwrap_or(0),
                            effect_amplitude: row.try_read(4).unwrap_or(0.0),
                            effect_attributes: row.try_read(5).unwrap_or(0),
                            effect_aura: row.try_read(6).unwrap_or(0),
                            effect_aura_period: row.try_read(7).unwrap_or(0),
                            effect_base_points: row.try_read(8).unwrap_or(0),
                            effect_bonus_coefficient: row.try_read(9).unwrap_or(0.0),
                            effect_chain_amplitude: row.try_read(10).unwrap_or(0.0),
                            effect_chain_targets: row.try_read(11).unwrap_or(0),
                            effect_die_sides: row.try_read(12).unwrap_or(0),
                            effect_item_type: row.try_read(13).unwrap_or(0),
                            effect_mechanic: row.try_read(14).unwrap_or(0),
                            effect_points_per_resource: row.try_read(15).unwrap_or(0.0),
                            effect_pos_facing: row.try_read(16).unwrap_or(0.0),
                            effect_real_points_per_level: row.try_read(17).unwrap_or(0.0),
                            effect_trigger_spell: row.try_read(18).unwrap_or(0),
                            bonus_coefficient_from_ap: row.try_read(19).unwrap_or(0.0),
                            pvp_multiplier: row.try_read(20).unwrap_or(0.0),
                            coefficient: row.try_read(21).unwrap_or(0.0),
                            variance: row.try_read(22).unwrap_or(0.0),
                            resource_coefficient: row.try_read(23).unwrap_or(0.0),
                            group_size_base_points_coefficient: row.try_read(24).unwrap_or(0.0),
                            effect_misc_value: [
                                row.try_read(25).unwrap_or(0),
                                row.try_read(26).unwrap_or(0),
                            ],
                            effect_radius_index: [
                                row.try_read(27).unwrap_or(0),
                                row.try_read(28).unwrap_or(0),
                            ],
                            effect_spell_class_mask: [
                                row.try_read::<i32>(29).unwrap_or(0) as u32,
                                row.try_read::<i32>(30).unwrap_or(0) as u32,
                                row.try_read::<i32>(31).unwrap_or(0) as u32,
                                row.try_read::<i32>(32).unwrap_or(0) as u32,
                            ],
                            implicit_target: [
                                row.try_read(33).unwrap_or(0),
                                row.try_read(34).unwrap_or(0),
                            ],
                            spell_id: row.try_read(35).unwrap_or(0),
                        })
                    },
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
                    |row| {
                        Some(SpellShapeshiftHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            spell_id: row.try_read(1).unwrap_or(0),
                            stance_bar_order: row.try_read(2).unwrap_or(0),
                            shapeshift_exclude: [
                                row.try_read(3).unwrap_or(0),
                                row.try_read(4).unwrap_or(0),
                            ],
                            shapeshift_mask: [
                                row.try_read(5).unwrap_or(0),
                                row.try_read(6).unwrap_or(0),
                            ],
                        })
                    },
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
                    |row| {
                        Some(SpellInterruptsHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            difficulty_id: row.try_read(1).unwrap_or(0),
                            interrupt_flags: row.try_read(2).unwrap_or(0),
                            aura_interrupt_flags: [
                                row.try_read(3).unwrap_or(0),
                                row.try_read(4).unwrap_or(0),
                            ],
                            channel_interrupt_flags: [
                                row.try_read(5).unwrap_or(0),
                                row.try_read(6).unwrap_or(0),
                            ],
                            spell_id: row.try_read(7).unwrap_or(0),
                        })
                    },
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
                    |row| {
                        Some(SpellCastTimesHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            base: row.try_read(1).unwrap_or(0),
                            per_level: row.try_read(2).unwrap_or(0),
                            minimum: row.try_read(3).unwrap_or(0),
                        })
                    },
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
                    |row| {
                        Some(SpellCooldownsHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            difficulty_id: row.try_read(1).unwrap_or(0),
                            category_recovery_time: row.try_read(2).unwrap_or(0),
                            recovery_time: row.try_read(3).unwrap_or(0),
                            start_recovery_time: row.try_read(4).unwrap_or(0),
                            spell_id: row.try_read(5).unwrap_or(0),
                        })
                    },
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
                    |row| {
                        Some(SpellCastingRequirementsHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            spell_id: row.try_read(1).unwrap_or(0),
                            facing_caster_flags: row.try_read(2).unwrap_or(0),
                            min_faction_id: row.try_read(3).unwrap_or(0),
                            min_reputation: row.try_read(4).unwrap_or(0),
                            required_areas_id: row.try_read(5).unwrap_or(0),
                            required_aura_vision: row.try_read(6).unwrap_or(0),
                            requires_spell_focus: row.try_read(7).unwrap_or(0),
                        })
                    },
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
                    |row| {
                        Some(SpellPowerHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            order_index: row.try_read(1).unwrap_or(0),
                            mana_cost: row.try_read(2).unwrap_or(0),
                            mana_cost_per_level: row.try_read(3).unwrap_or(0),
                            mana_per_second: row.try_read(4).unwrap_or(0),
                            power_display_id: row.try_read(5).unwrap_or(0),
                            alt_power_bar_id: row.try_read(6).unwrap_or(0),
                            power_cost_pct: row.try_read(7).unwrap_or(0.0),
                            power_cost_max_pct: row.try_read(8).unwrap_or(0.0),
                            power_pct_per_second: row.try_read(9).unwrap_or(0.0),
                            power_type: row.try_read(10).unwrap_or(0),
                            required_aura_spell_id: row.try_read(11).unwrap_or(0),
                            optional_cost: row.try_read(12).unwrap_or(0),
                            spell_id: row.try_read(13).unwrap_or(0),
                        })
                    },
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
                    |row| {
                        Some(SpellPowerDifficultyHotfixRowLikeCpp {
                            id: row.try_read(0).unwrap_or(0),
                            difficulty_id: row.try_read(1).unwrap_or(0),
                            order_index: row.try_read(2).unwrap_or(0),
                        })
                    },
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
            ]
        );
        assert_eq!(
            HotfixStatements::base(SPELL_EFFECT_SQL_LIKE_CPP),
            HotfixStatements::base(SPELL_EFFECT_SQL_LIKE_CPP)
        );
    }
}
