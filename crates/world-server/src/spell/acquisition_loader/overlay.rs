use super::*;

pub(super) struct SpellAcquisitionSqlOverlaySourceBridgeLikeCpp<'a> {
    pub(super) persistence: &'a dyn SpellAcquisitionStartupPersistencePortLikeCpp,
}

fn persistence_table_like_cpp(
    table: SpellAcquisitionTableLikeCpp,
) -> SpellAcquisitionHotfixTablePersistenceLikeCpp {
    match table {
        SpellAcquisitionTableLikeCpp::SpellEffect => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellEffect
        }
        SpellAcquisitionTableLikeCpp::SpellLearnSpell => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLearnSpell
        }
        SpellAcquisitionTableLikeCpp::SpellMisc => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellMisc
        }
        SpellAcquisitionTableLikeCpp::SpellLevels => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLevels
        }
        SpellAcquisitionTableLikeCpp::Talent => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::Talent
        }
        SpellAcquisitionTableLikeCpp::SummonProperties => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SummonProperties
        }
        SpellAcquisitionTableLikeCpp::BattlePetSpecies => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::BattlePetSpecies
        }
    }
}

pub(super) fn overlay_row_like_cpp(
    table: SpellAcquisitionTableLikeCpp,
    row: SpellAcquisitionHotfixPersistenceRowLikeCpp,
) -> Result<SpellAcquisitionSqlOverlayRowLikeCpp> {
    let (integer_columns, float_columns_bits) = match (table, row) {
        (
            SpellAcquisitionTableLikeCpp::SpellEffect,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellEffect(row),
        ) => {
            let integers = vec![
                row.record_id,
                row.difficulty_id,
                row.effect_index,
                row.effect,
                row.effect_base_points,
                row.effect_die_sides,
                row.effect_trigger_spell,
                row.effect_misc_value[0],
                row.effect_misc_value[1],
                row.implicit_target[0],
                row.implicit_target[1],
                None,
                None,
                row.spell_id,
                row.effect_chain_targets,
                None,
                None,
                row.effect_item_type,
                row.effect_aura,
                row.effect_mechanic,
                row.effect_attributes,
            ];
            let mut floats = vec![None; integers.len()];
            floats[11] = row.coefficient_bits;
            floats[12] = row.variance_bits;
            floats[15] = row.effect_points_per_resource_bits;
            floats[16] = row.effect_real_points_per_level_bits;
            (integers, floats)
        }
        (
            SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellLearnSpell(row),
        ) => (
            vec![
                row.record_id,
                row.spell_id,
                row.learn_spell_id,
                row.overrides_spell_id,
            ],
            vec![None; 4],
        ),
        (
            SpellAcquisitionTableLikeCpp::SpellMisc,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellMisc(row),
        ) => (
            vec![
                row.record_id,
                row.attributes[0],
                row.attributes[1],
                row.difficulty_id,
                row.show_future_spell_player_condition_id,
                row.spell_id,
            ],
            vec![None; 6],
        ),
        (
            SpellAcquisitionTableLikeCpp::SpellLevels,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellLevels(row),
        ) => (
            vec![
                row.record_id,
                row.difficulty_id,
                row.base_level,
                row.spell_level,
                row.spell_id,
            ],
            vec![None; 5],
        ),
        (
            SpellAcquisitionTableLikeCpp::Talent,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::Talent(row),
        ) => {
            let mut integers = Vec::with_capacity(10);
            integers.push(row.record_id);
            integers.extend(row.spell_rank);
            (integers, vec![None; 10])
        }
        (
            SpellAcquisitionTableLikeCpp::SummonProperties,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SummonProperties(row),
        ) => (vec![row.record_id, row.slot, row.flags_1], vec![None; 3]),
        (
            SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::BattlePetSpecies(row),
        ) => (vec![row.record_id, row.creature_id], vec![None; 2]),
        _ => {
            return Err(anyhow::anyhow!(
                "spell-acquisition overlay returned the wrong typed row family"
            ));
        }
    };
    Ok(SpellAcquisitionSqlOverlayRowLikeCpp {
        integer_columns,
        float_columns_bits,
    })
}

impl SpellAcquisitionSqlOverlaySourceLikeCpp for SpellAcquisitionSqlOverlaySourceBridgeLikeCpp<'_> {
    fn load_overlay_like_cpp(
        &self,
        table: SpellAcquisitionTableLikeCpp,
        official: bool,
    ) -> SpellAcquisitionSqlOverlayFutureLikeCpp<'_> {
        Box::pin(async move {
            match self
                .persistence
                .load_hotfix_overlay_like_cpp(persistence_table_like_cpp(table), official)
                .await
            {
                SpellAcquisitionStartupLoadOutcomeLikeCpp::Loaded(rows) => rows
                    .into_iter()
                    .map(|row| overlay_row_like_cpp(table, row))
                    .collect(),
                SpellAcquisitionStartupLoadOutcomeLikeCpp::Failed { reason } => {
                    Err(anyhow::anyhow!(reason))
                }
            }
        })
    }
}
