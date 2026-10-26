//! MariaDB adapter for the complete spell-acquisition startup source family.

mod checked_rows;

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use wow_persistence::{
    BattlePetSpeciesHotfixPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
    ServersideSpellEffectPersistenceRowLikeCpp, ServersideSpellPersistenceRowLikeCpp,
    SpellAcquisitionHotfixPersistenceRowLikeCpp, SpellAcquisitionHotfixTablePersistenceLikeCpp,
    SpellAcquisitionStartupLoadOutcomeLikeCpp, SpellAcquisitionStartupPersistencePortLikeCpp,
    SpellCustomAttributePersistenceRowLikeCpp, SpellEffectHotfixPersistenceRowLikeCpp,
    SpellLearnSpellHotfixPersistenceRowLikeCpp, SpellLearnSpellWorldPersistenceRowLikeCpp,
    SpellLevelsHotfixPersistenceRowLikeCpp, SpellMiscHotfixPersistenceRowLikeCpp,
    SpellReagentsCurrencyPersistenceRowLikeCpp, SpellReagentsPersistenceRowLikeCpp,
    SummonPropertiesHotfixPersistenceRowLikeCpp,
    TalentHotfixPersistenceRowLikeCpp, TrainerSpellAuditPersistenceCatalogLikeCpp,
};

use crate::{HotfixDatabase, HotfixStatements, SqlResult, WorldDatabase, WorldStatements};

const SPELL_EFFECT_SQL: &str = concat!(
    "SELECT ID, DifficultyID, EffectIndex, Effect, EffectBasePoints, EffectDieSides, ",
    "EffectTriggerSpell, EffectMiscValue1, EffectMiscValue2, ImplicitTarget1, ",
    "ImplicitTarget2, Coefficient, Variance, SpellID, EffectChainTargets, ",
    "EffectPointsPerResource, EffectRealPointsPerLevel, EffectItemType, EffectAura, EffectMechanic, EffectAttributes FROM spell_effect ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_LEARN_SPELL_SQL: &str = "SELECT ID, SpellID, LearnSpellID, OverridesSpellID FROM spell_learn_spell WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_MISC_SQL: &str = "SELECT ID, Attributes1, Attributes2, DifficultyID, ShowFutureSpellPlayerConditionID, SpellID FROM spell_misc WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_LEVELS_SQL: &str = "SELECT ID, DifficultyID, BaseLevel, SpellLevel, SpellID FROM spell_levels WHERE (`VerifiedBuild` > 0) = ?";
const TALENT_SQL: &str = concat!(
    "SELECT ID, SpellRank1, SpellRank2, SpellRank3, SpellRank4, SpellRank5, ",
    "SpellRank6, SpellRank7, SpellRank8, SpellRank9 FROM talent WHERE (`VerifiedBuild` > 0) = ?"
);
const SUMMON_PROPERTIES_SQL: &str =
    "SELECT ID, Slot, Flags1 FROM summon_properties WHERE (`VerifiedBuild` > 0) = ?";
const BATTLE_PET_SPECIES_SQL: &str =
    "SELECT ID, CreatureID FROM battle_pet_species WHERE (`VerifiedBuild` > 0) = ?";
const SPELL_REAGENTS_SQL: &str = concat!(
    "SELECT ID, SpellID, Reagent1, Reagent2, Reagent3, Reagent4, ",
    "Reagent5, Reagent6, Reagent7, Reagent8, ReagentCount1, ReagentCount2, ",
    "ReagentCount3, ReagentCount4, ReagentCount5, ReagentCount6, ",
    "ReagentCount7, ReagentCount8 FROM spell_reagents WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_REAGENTS_CURRENCY_SQL: &str = "SELECT ID, SpellID, CurrencyTypesID, CurrencyCount FROM spell_reagents_currency WHERE (`VerifiedBuild` > 0) = ?";

fn raw_i64_like_cpp(result: &SqlResult, column: usize) -> Option<i64> {
    result
        .try_read::<i64>(column)
        .or_else(|| result.try_read::<i32>(column).map(i64::from))
        .or_else(|| result.try_read::<i16>(column).map(i64::from))
        .or_else(|| result.try_read::<i8>(column).map(i64::from))
        .or_else(|| {
            result
                .try_read::<u64>(column)
                .and_then(|value| i64::try_from(value).ok())
        })
        .or_else(|| result.try_read::<u32>(column).map(i64::from))
        .or_else(|| result.try_read::<u16>(column).map(i64::from))
        .or_else(|| result.try_read::<u8>(column).map(i64::from))
}

fn required_i32_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<i32> {
    let raw = raw_i64_like_cpp(result, column)
        .ok_or_else(|| anyhow!("missing or unsupported numeric value for {field}"))?;
    i32::try_from(raw).with_context(|| format!("{field} is outside the signed 32-bit range"))
}

fn required_i16_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<i16> {
    let raw = raw_i64_like_cpp(result, column)
        .ok_or_else(|| anyhow!("missing or unsupported numeric value for {field}"))?;
    i16::try_from(raw).with_context(|| format!("{field} is outside the signed 16-bit range"))
}

fn required_u32_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<u32> {
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(value);
    }
    let raw = raw_i64_like_cpp(result, column)
        .ok_or_else(|| anyhow!("missing or unsupported numeric value for {field}"))?;
    u32::try_from(raw).with_context(|| format!("{field} is outside the unsigned 32-bit range"))
}

fn required_u16_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<u16> {
    let raw = raw_i64_like_cpp(result, column)
        .ok_or_else(|| anyhow!("missing or unsupported numeric value for {field}"))?;
    u16::try_from(raw).with_context(|| format!("{field} is outside the unsigned 16-bit range"))
}

fn required_spell_id_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<i32> {
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(value as i32);
    }
    required_i32_like_cpp(result, column, field)
}

fn required_array_like_cpp<T, const N: usize>(
    mut read: impl FnMut(usize, &str) -> Result<T>,
    first_column: usize,
    field: &str,
) -> Result<[T; N]>
where
    T: Copy + Default,
{
    let mut values = [T::default(); N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read(first_column + index, &format!("{field}{}", index + 1))?;
    }
    Ok(values)
}

fn f32_bits_like_cpp(result: &SqlResult, column: usize) -> Option<u32> {
    result
        .try_read::<f32>(column)
        .or_else(|| result.try_read::<f64>(column).map(|value| value as f32))
        .map(f32::to_bits)
}

fn hotfix_sql_like_cpp(table: SpellAcquisitionHotfixTablePersistenceLikeCpp) -> &'static str {
    match table {
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellEffect => SPELL_EFFECT_SQL,
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLearnSpell => SPELL_LEARN_SPELL_SQL,
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellMisc => SPELL_MISC_SQL,
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLevels => SPELL_LEVELS_SQL,
        SpellAcquisitionHotfixTablePersistenceLikeCpp::Talent => TALENT_SQL,
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SummonProperties => SUMMON_PROPERTIES_SQL,
        SpellAcquisitionHotfixTablePersistenceLikeCpp::BattlePetSpecies => BATTLE_PET_SPECIES_SQL,
    }
}

fn decode_hotfix_row_like_cpp(
    table: SpellAcquisitionHotfixTablePersistenceLikeCpp,
    result: &SqlResult,
) -> SpellAcquisitionHotfixPersistenceRowLikeCpp {
    match table {
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellEffect => {
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellEffect(
                SpellEffectHotfixPersistenceRowLikeCpp {
                    record_id: raw_i64_like_cpp(result, 0),
                    difficulty_id: raw_i64_like_cpp(result, 1),
                    effect_index: raw_i64_like_cpp(result, 2),
                    effect: raw_i64_like_cpp(result, 3),
                    effect_base_points: raw_i64_like_cpp(result, 4),
                    effect_die_sides: raw_i64_like_cpp(result, 5),
                    effect_trigger_spell: raw_i64_like_cpp(result, 6),
                    effect_misc_value: [raw_i64_like_cpp(result, 7), raw_i64_like_cpp(result, 8)],
                    implicit_target: [raw_i64_like_cpp(result, 9), raw_i64_like_cpp(result, 10)],
                    coefficient_bits: f32_bits_like_cpp(result, 11),
                    variance_bits: f32_bits_like_cpp(result, 12),
                    spell_id: raw_i64_like_cpp(result, 13),
                    effect_chain_targets: raw_i64_like_cpp(result, 14),
                    effect_points_per_resource_bits: f32_bits_like_cpp(result, 15),
                    effect_real_points_per_level_bits: f32_bits_like_cpp(result, 16),
                    effect_item_type: raw_i64_like_cpp(result, 17),
                    effect_aura: raw_i64_like_cpp(result, 18),
                    effect_mechanic: raw_i64_like_cpp(result, 19),
                    effect_attributes: raw_i64_like_cpp(result, 20),
                },
            )
        }
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLearnSpell => {
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellLearnSpell(
                SpellLearnSpellHotfixPersistenceRowLikeCpp {
                    record_id: raw_i64_like_cpp(result, 0),
                    spell_id: raw_i64_like_cpp(result, 1),
                    learn_spell_id: raw_i64_like_cpp(result, 2),
                    overrides_spell_id: raw_i64_like_cpp(result, 3),
                },
            )
        }
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellMisc => {
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellMisc(
                SpellMiscHotfixPersistenceRowLikeCpp {
                    record_id: raw_i64_like_cpp(result, 0),
                    attributes: [raw_i64_like_cpp(result, 1), raw_i64_like_cpp(result, 2)],
                    difficulty_id: raw_i64_like_cpp(result, 3),
                    show_future_spell_player_condition_id: raw_i64_like_cpp(result, 4),
                    spell_id: raw_i64_like_cpp(result, 5),
                },
            )
        }
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLevels => {
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellLevels(
                SpellLevelsHotfixPersistenceRowLikeCpp {
                    record_id: raw_i64_like_cpp(result, 0),
                    difficulty_id: raw_i64_like_cpp(result, 1),
                    base_level: raw_i64_like_cpp(result, 2),
                    spell_level: raw_i64_like_cpp(result, 3),
                    spell_id: raw_i64_like_cpp(result, 4),
                },
            )
        }
        SpellAcquisitionHotfixTablePersistenceLikeCpp::Talent => {
            SpellAcquisitionHotfixPersistenceRowLikeCpp::Talent(TalentHotfixPersistenceRowLikeCpp {
                record_id: raw_i64_like_cpp(result, 0),
                spell_rank: std::array::from_fn(|index| raw_i64_like_cpp(result, 1 + index)),
            })
        }
        SpellAcquisitionHotfixTablePersistenceLikeCpp::SummonProperties => {
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SummonProperties(
                SummonPropertiesHotfixPersistenceRowLikeCpp {
                    record_id: raw_i64_like_cpp(result, 0),
                    slot: raw_i64_like_cpp(result, 1),
                    flags_1: raw_i64_like_cpp(result, 2),
                },
            )
        }
        SpellAcquisitionHotfixTablePersistenceLikeCpp::BattlePetSpecies => {
            SpellAcquisitionHotfixPersistenceRowLikeCpp::BattlePetSpecies(
                BattlePetSpeciesHotfixPersistenceRowLikeCpp {
                    record_id: raw_i64_like_cpp(result, 0),
                    creature_id: raw_i64_like_cpp(result, 1),
                },
            )
        }
    }
}

async fn load_hotfix_overlay_like_cpp(
    db: &HotfixDatabase,
    table: SpellAcquisitionHotfixTablePersistenceLikeCpp,
    official: bool,
) -> Result<Vec<SpellAcquisitionHotfixPersistenceRowLikeCpp>> {
    let mut statement = db.prepare(HotfixStatements::base(hotfix_sql_like_cpp(table)));
    statement.set_bool(0, official);
    let mut result = db.query(&statement).await?;
    let mut rows = Vec::with_capacity(result.count());
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode_hotfix_row_like_cpp(table, &result));
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

async fn query_world_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> T,
) -> Result<Vec<T>> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::with_capacity(result.count());
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode(&result));
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

async fn load_spell_reagents_like_cpp(
    db: &HotfixDatabase,
    official: bool,
) -> Result<Vec<SpellReagentsPersistenceRowLikeCpp>> {
    let mut statement = db.prepare(HotfixStatements::base(SPELL_REAGENTS_SQL));
    statement.set_bool(0, official);
    let mut result = db.query(&statement).await?;
    let mut rows = Vec::with_capacity(result.count());
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(SpellReagentsPersistenceRowLikeCpp {
            id: required_u32_like_cpp(&result, 0, "spell_reagents.ID")?,
            spell_id: required_spell_id_like_cpp(&result, 1, "spell_reagents.SpellID")?,
            reagent: required_array_like_cpp(
                |column, field| required_i32_like_cpp(&result, column, field),
                2,
                "spell_reagents.Reagent",
            )?,
            reagent_count: required_array_like_cpp(
                |column, field| required_i16_like_cpp(&result, column, field),
                10,
                "spell_reagents.ReagentCount",
            )?,
        });
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

async fn load_spell_reagents_currency_like_cpp(
    db: &HotfixDatabase,
    official: bool,
) -> Result<Vec<SpellReagentsCurrencyPersistenceRowLikeCpp>> {
    let mut statement = db.prepare(HotfixStatements::base(SPELL_REAGENTS_CURRENCY_SQL));
    statement.set_bool(0, official);
    let mut result = db.query(&statement).await?;
    let mut rows = Vec::with_capacity(result.count());
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(SpellReagentsCurrencyPersistenceRowLikeCpp {
            id: required_u32_like_cpp(&result, 0, "spell_reagents_currency.ID")?,
            spell_id: required_u32_like_cpp(&result, 1, "spell_reagents_currency.SpellID")?,
            currency_types_id: required_u16_like_cpp(
                &result,
                2,
                "spell_reagents_currency.CurrencyTypesID",
            )?,
            currency_count: required_u16_like_cpp(
                &result,
                3,
                "spell_reagents_currency.CurrencyCount",
            )?,
        });
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn classify_like_cpp<T>(result: Result<T>) -> SpellAcquisitionStartupLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => SpellAcquisitionStartupLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => SpellAcquisitionStartupLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

fn classify_checked<T>(result: Result<T>) -> SpellAcquisitionStartupLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => SpellAcquisitionStartupLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => SpellAcquisitionStartupLoadOutcomeLikeCpp::Failed {
            reason: format!("{error:#}"),
        },
    }
}

pub struct MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
    world_db: Arc<WorldDatabase>,
}

impl MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>, world_db: Arc<WorldDatabase>) -> Self {
        Self {
            hotfix_db,
            world_db,
        }
    }
}

impl SpellAcquisitionStartupPersistencePortLikeCpp
    for MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp
{
    fn load_hotfix_overlay_like_cpp(
        &self,
        table: SpellAcquisitionHotfixTablePersistenceLikeCpp,
        official: bool,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellAcquisitionHotfixPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(load_hotfix_overlay_like_cpp(&self.hotfix_db, table, official).await)
        })
    }
    fn load_serverside_spell_effects_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<ServersideSpellEffectPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_checked(checked_rows::serverside_effects(&self.world_db).await)
        })
    }
    fn load_serverside_spells_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<ServersideSpellPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_checked(checked_rows::serverside_spells(&self.world_db).await)
        })
    }
    fn load_spell_custom_attributes_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellCustomAttributePersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                query_world_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_SPELL_CUSTOM_ATTR,
                    |r| SpellCustomAttributePersistenceRowLikeCpp {
                        spell_id: r.try_read(0).unwrap_or(0),
                        attributes: r.try_read(1).unwrap_or(0),
                    },
                )
                .await,
            )
        })
    }
    fn load_spell_learn_spells_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellLearnSpellWorldPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                query_world_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_SPELL_LEARN_SPELL,
                    |r| SpellLearnSpellWorldPersistenceRowLikeCpp {
                        entry: r.try_read(0).unwrap_or(0),
                        spell_id: r.try_read(1).unwrap_or(0),
                        active: r.try_read::<u8>(2).unwrap_or(0) != 0,
                    },
                )
                .await,
            )
        })
    }
    fn load_trainer_spell_audit_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<TrainerSpellAuditPersistenceCatalogLikeCpp>,
    > {
        Box::pin(async move {
            let result = async {
                let script_binding_ids = query_world_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_TRAINER_CAST_SCRIPT_BINDING_IDS,
                    |r| r.try_read::<i32>(0),
                )
                .await?
                .into_iter()
                .flatten()
                .collect();
                let legacy_script_ids = query_world_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_TRAINER_CAST_LEGACY_SCRIPT_IDS,
                    |r| r.try_read::<u32>(0),
                )
                .await?
                .into_iter()
                .flatten()
                .collect();
                let condition_spell_ids = query_world_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_TRAINER_CAST_CONDITION_IDS,
                    |r| r.try_read::<i32>(0),
                )
                .await?
                .into_iter()
                .flatten()
                .collect();
                Ok(TrainerSpellAuditPersistenceCatalogLikeCpp {
                    script_binding_ids,
                    legacy_script_ids,
                    condition_spell_ids,
                })
            }
            .await;
            classify_like_cpp(result)
        })
    }
    fn load_spell_reagents_overlay_like_cpp(
        &self,
        official: bool,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellReagentsPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(load_spell_reagents_like_cpp(&self.hotfix_db, official).await)
        })
    }

    fn load_spell_reagents_currency_overlay_like_cpp(
        &self,
        official: bool,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<SpellReagentsCurrencyPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                load_spell_reagents_currency_like_cpp(&self.hotfix_db, official).await,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn acquisition_source_manifests_are_complete() {
        let tables = [
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellEffect,
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLearnSpell,
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellMisc,
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLevels,
            SpellAcquisitionHotfixTablePersistenceLikeCpp::Talent,
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SummonProperties,
            SpellAcquisitionHotfixTablePersistenceLikeCpp::BattlePetSpecies,
        ];
        assert!(
            tables
                .into_iter()
                .all(|table| hotfix_sql_like_cpp(table).contains("VerifiedBuild"))
        );
        let statements = [
            WorldStatements::SEL_SERVERSIDE_SPELL_EFFECT,
            WorldStatements::SEL_SERVERSIDE_SPELL,
            WorldStatements::SEL_SPELL_CUSTOM_ATTR,
            WorldStatements::SEL_SPELL_LEARN_SPELL,
            WorldStatements::SEL_TRAINER_CAST_SCRIPT_BINDING_IDS,
            WorldStatements::SEL_TRAINER_CAST_LEGACY_SCRIPT_IDS,
            WorldStatements::SEL_TRAINER_CAST_CONDITION_IDS,
        ];
        assert!(
            statements
                .into_iter()
                .all(|statement| !statement.sql().is_empty())
        );

        let projection = SPELL_EFFECT_SQL
            .strip_prefix("SELECT ")
            .and_then(|sql| sql.split_once(" FROM spell_effect "))
            .map(|(columns, _)| columns.split(", ").collect::<Vec<_>>())
            .expect("SpellEffect SQL projection");
        assert_eq!(projection.len(), 21);
        assert_eq!(
            &projection[14..=20],
            &[
                "EffectChainTargets",
                "EffectPointsPerResource",
                "EffectRealPointsPerLevel",
                "EffectItemType",
                "EffectAura",
                "EffectMechanic",
                "EffectAttributes",
            ]
        );
        assert_eq!(
            SPELL_REAGENTS_SQL,
            concat!(
                "SELECT ID, SpellID, Reagent1, Reagent2, Reagent3, Reagent4, ",
                "Reagent5, Reagent6, Reagent7, Reagent8, ReagentCount1, ReagentCount2, ",
                "ReagentCount3, ReagentCount4, ReagentCount5, ReagentCount6, ",
                "ReagentCount7, ReagentCount8 FROM spell_reagents ",
                "WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
    }
}
