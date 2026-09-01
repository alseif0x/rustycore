//! MariaDB adapter for the complete spell-acquisition startup source family.

use std::sync::Arc;

use anyhow::Result;
use wow_persistence::{
    BattlePetSpeciesHotfixPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
    ServersideSpellEffectPersistenceRowLikeCpp, ServersideSpellPersistenceRowLikeCpp,
    SpellAcquisitionHotfixPersistenceRowLikeCpp, SpellAcquisitionHotfixTablePersistenceLikeCpp,
    SpellAcquisitionStartupLoadOutcomeLikeCpp, SpellAcquisitionStartupPersistencePortLikeCpp,
    SpellCustomAttributePersistenceRowLikeCpp, SpellEffectHotfixPersistenceRowLikeCpp,
    SpellLearnSpellHotfixPersistenceRowLikeCpp, SpellLearnSpellWorldPersistenceRowLikeCpp,
    SpellLevelsHotfixPersistenceRowLikeCpp, SpellMiscHotfixPersistenceRowLikeCpp,
    SpellReagentsPersistenceRowLikeCpp, SummonPropertiesHotfixPersistenceRowLikeCpp,
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

fn decode_serverside_effect_like_cpp(
    result: &SqlResult,
) -> ServersideSpellEffectPersistenceRowLikeCpp {
    ServersideSpellEffectPersistenceRowLikeCpp {
        spell_id: result.try_read(0).unwrap_or(0),
        effect_index: result.try_read(1).unwrap_or(0),
        difficulty_id: result.try_read(2).unwrap_or(0),
        effect: result.try_read(3).unwrap_or(0),
        effect_aura: result.try_read(4).unwrap_or(0),
        effect_amplitude: result.try_read(5).unwrap_or(0.0),
        effect_attributes: result.try_read(6).unwrap_or(0),
        effect_aura_period: result.try_read(7).unwrap_or(0),
        effect_bonus_coefficient: result.try_read(8).unwrap_or(0.0),
        effect_chain_amplitude: result.try_read(9).unwrap_or(0.0),
        effect_chain_targets: result.try_read(10).unwrap_or(0),
        effect_item_type: result.try_read(11).unwrap_or(0),
        effect_mechanic: result.try_read(12).unwrap_or(0),
        effect_points_per_resource: result.try_read(13).unwrap_or(0.0),
        effect_pos_facing: result.try_read(14).unwrap_or(0.0),
        effect_real_points_per_level: result.try_read(15).unwrap_or(0.0),
        effect_trigger_spell: result.try_read(16).unwrap_or(0),
        bonus_coefficient_from_ap: result.try_read(17).unwrap_or(0.0),
        pvp_multiplier: result.try_read(18).unwrap_or(0.0),
        coefficient: result.try_read(19).unwrap_or(0.0),
        variance: result.try_read(20).unwrap_or(0.0),
        resource_coefficient: result.try_read(21).unwrap_or(0.0),
        group_size_base_points_coefficient: result.try_read(22).unwrap_or(0.0),
        effect_base_points: result.try_read(23).unwrap_or(0.0),
        effect_misc_value: [
            result.try_read(24).unwrap_or(0),
            result.try_read(25).unwrap_or(0),
        ],
        effect_radius_index: [
            result.try_read(26).unwrap_or(0),
            result.try_read(27).unwrap_or(0),
        ],
        effect_spell_class_mask: std::array::from_fn(|index| {
            result.try_read(28 + index).unwrap_or(0)
        }),
        implicit_target: [
            result.try_read(32).unwrap_or(0),
            result.try_read(33).unwrap_or(0),
        ],
    }
}

fn decode_serverside_spell_like_cpp(result: &SqlResult) -> ServersideSpellPersistenceRowLikeCpp {
    ServersideSpellPersistenceRowLikeCpp {
        spell_id: result.try_read(0).unwrap_or(0),
        difficulty_id: result.try_read(1).unwrap_or(0),
        category_id: result.try_read(2).unwrap_or(0),
        dispel: result.try_read(3).unwrap_or(0),
        mechanic: result.try_read(4).unwrap_or(0),
        attributes: result.try_read(5).unwrap_or(0),
        attributes_ex: std::array::from_fn(|i| result.try_read(6 + i).unwrap_or(0)),
        stances: result.try_read(20).unwrap_or(0),
        stances_not: result.try_read(21).unwrap_or(0),
        targets: result.try_read(22).unwrap_or(0),
        target_creature_type: result.try_read(23).unwrap_or(0),
        requires_spell_focus: result.try_read(24).unwrap_or(0),
        facing_caster_flags: result.try_read(25).unwrap_or(0),
        caster_aura_state: result.try_read(26).unwrap_or(0),
        target_aura_state: result.try_read(27).unwrap_or(0),
        exclude_caster_aura_state: result.try_read(28).unwrap_or(0),
        exclude_target_aura_state: result.try_read(29).unwrap_or(0),
        caster_aura_spell: result.try_read(30).unwrap_or(0),
        target_aura_spell: result.try_read(31).unwrap_or(0),
        exclude_caster_aura_spell: result.try_read(32).unwrap_or(0),
        exclude_target_aura_spell: result.try_read(33).unwrap_or(0),
        caster_aura_type: result.try_read(34).unwrap_or(0),
        target_aura_type: result.try_read(35).unwrap_or(0),
        exclude_caster_aura_type: result.try_read(36).unwrap_or(0),
        exclude_target_aura_type: result.try_read(37).unwrap_or(0),
        casting_time_index: result.try_read(38).unwrap_or(0),
        recovery_time: result.try_read(39).unwrap_or(0),
        category_recovery_time: result.try_read(40).unwrap_or(0),
        start_recovery_category: result.try_read(41).unwrap_or(0),
        start_recovery_time: result.try_read(42).unwrap_or(0),
        interrupt_flags: result.try_read(43).unwrap_or(0),
        aura_interrupt_flags: [
            result.try_read(44).unwrap_or(0),
            result.try_read(45).unwrap_or(0),
        ],
        channel_interrupt_flags: [
            result.try_read(46).unwrap_or(0),
            result.try_read(47).unwrap_or(0),
        ],
        proc_flags: [
            result.try_read(48).unwrap_or(0),
            result.try_read(49).unwrap_or(0),
        ],
        proc_chance: result.try_read(50).unwrap_or(0),
        proc_charges: result.try_read(51).unwrap_or(0),
        proc_cooldown: result.try_read(52).unwrap_or(0),
        proc_base_ppm: result.try_read(53).unwrap_or(0.0),
        max_level: result.try_read(54).unwrap_or(0),
        base_level: result.try_read(55).unwrap_or(0),
        spell_level: result.try_read(56).unwrap_or(0),
        duration_index: result.try_read(57).unwrap_or(0),
        range_index: result.try_read(58).unwrap_or(0),
        speed: result.try_read(59).unwrap_or(0.0),
        launch_delay: result.try_read(60).unwrap_or(0.0),
        stack_amount: result.try_read(61).unwrap_or(0),
        equipped_item_class: result.try_read(62).unwrap_or(0),
        equipped_item_sub_class_mask: result.try_read(63).unwrap_or(0),
        equipped_item_inventory_type_mask: result.try_read(64).unwrap_or(0),
        content_tuning_id: result.try_read(65).unwrap_or(0),
        spell_name: result.try_read(66).unwrap_or_default(),
        cone_angle: result.try_read(67).unwrap_or(0.0),
        cone_width: result.try_read(68).unwrap_or(0.0),
        max_target_level: result.try_read(69).unwrap_or(0),
        max_affected_targets: result.try_read(70).unwrap_or(0),
        spell_family_name: result.try_read(71).unwrap_or(0),
        spell_family_flags: std::array::from_fn(|i| result.try_read(72 + i).unwrap_or(0)),
        dmg_class: result.try_read(76).unwrap_or(0),
        prevention_type: result.try_read(77).unwrap_or(0),
        area_group_id: result.try_read(78).unwrap_or(0),
        school_mask: result.try_read(79).unwrap_or(0),
        charge_category_id: result.try_read(80).unwrap_or(0),
    }
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
            id: result.try_read(0).unwrap_or(0),
            spell_id: result.try_read(1).unwrap_or(0),
            reagent: std::array::from_fn(|i| result.try_read(2 + i).unwrap_or(0)),
            reagent_count: std::array::from_fn(|i| result.try_read(10 + i).unwrap_or(0)),
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
            classify_like_cpp(
                query_world_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_SERVERSIDE_SPELL_EFFECT,
                    decode_serverside_effect_like_cpp,
                )
                .await,
            )
        })
    }
    fn load_serverside_spells_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellAcquisitionStartupLoadOutcomeLikeCpp<Vec<ServersideSpellPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                query_world_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_SERVERSIDE_SPELL,
                    decode_serverside_spell_like_cpp,
                )
                .await,
            )
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
