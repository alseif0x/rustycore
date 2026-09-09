//! Spell acquisition model state definitions, part 3 of 3.
//!
//! Separated from the spell_acquisition.rs root under #646. Behaviour is preserved.

use super::*;

pub(super) fn is_acquisition_effect_like_cpp(effect_type: u32) -> bool {
    matches!(
        effect_type,
        SPELL_EFFECT_SUMMON_LIKE_CPP
            | SPELL_EFFECT_LEARN_SPELL_LIKE_CPP
            | SPELL_EFFECT_DUAL_WIELD_LIKE_CPP
            | SPELL_EFFECT_SKILL_STEP_LIKE_CPP
            | SPELL_EFFECT_SKILL_LIKE_CPP
    )
}

pub(super) fn acquisition_effect_payload_errors_like_cpp(
    row: &SpellAcquisitionEffectLikeCpp,
) -> Vec<InvalidAcquisitionValueLikeCpp> {
    let mut errors = Vec::new();
    let mut push = |error| {
        if !errors.contains(&error) {
            errors.push(error);
        }
    };
    let Ok(effect_type) = row.effect_type_checked() else {
        return errors;
    };
    match effect_type {
        SPELL_EFFECT_LEARN_SPELL_LIKE_CPP => {
            if let Err(error) = row.trigger_spell_id_checked() {
                push(error);
            }
        }
        SPELL_EFFECT_SKILL_LIKE_CPP | SPELL_EFFECT_SKILL_STEP_LIKE_CPP => {
            if let Err(error) = row.misc_value_id_checked(0) {
                push(error);
            }
            if let Err(error) = row.base_points_die_sides_domain_checked() {
                push(error);
            }
        }
        _ => {}
    }
    errors
}

pub(super) fn metadata_collision_diagnostic(
    table: SpellAcquisitionTableLikeCpp,
    (spell_id, difficulty_id): (u32, u32),
    replaced_record_id: u32,
    winning_record_id: u32,
) -> SpellAcquisitionDiagnosticLikeCpp {
    SpellAcquisitionDiagnosticLikeCpp {
        severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Warning,
        table,
        record_id: Some(winning_record_id),
        kind: SpellAcquisitionDiagnosticKindLikeCpp::MetadataCollisionResolved {
            spell_id,
            difficulty_id,
            replaced_record_id,
            winning_record_id,
        },
    }
}

pub(super) fn diagnostic_from_invalid(
    table: SpellAcquisitionTableLikeCpp,
    record_id: u32,
    error: InvalidAcquisitionValueLikeCpp,
) -> SpellAcquisitionDiagnosticLikeCpp {
    SpellAcquisitionDiagnosticLikeCpp {
        severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
        table,
        record_id: Some(record_id),
        kind: SpellAcquisitionDiagnosticKindLikeCpp::InvalidField {
            field: error.field,
            raw: error.raw,
            expected: error.expected,
        },
    }
}

pub(super) fn spell_acquisition_effect_from_wdc_like_cpp(
    record_id: u32,
    index: usize,
    reader: &Wdc4Reader,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: i64::from(reader.get_relationship_id(index).unwrap_or(0)),
        difficulty_id_raw: i64::from(reader.get_field_i32(index, 0)),
        effect_index_raw: i64::from(reader.get_field_i32(index, 1)),
        effect_type_raw: i64::from(reader.get_field_u32(index, 2)),
        effect_aura_raw: i64::from(reader.get_field_i16(index, 5)),
        effect_mechanic_raw: i64::from(reader.get_field_i32(index, 13)),
        effect_attributes_raw: i64::from(reader.get_field_i32(index, 4)),
        effect_base_points_raw: i64::from(reader.get_field_i32(index, 7)),
        effect_die_sides_raw: i64::from(reader.get_field_i32(index, 11)),
        effect_chain_targets_raw: i64::from(
            reader.get_field_i32(index, SPELL_EFFECT_WDC_CHAIN_TARGETS_FIELD),
        ),
        effect_points_per_resource_bits: reader
            .get_field_f32(index, SPELL_EFFECT_WDC_POINTS_PER_RESOURCE_FIELD)
            .to_bits(),
        effect_real_points_per_level_bits: reader
            .get_field_f32(index, SPELL_EFFECT_WDC_REAL_POINTS_PER_LEVEL_FIELD)
            .to_bits(),
        effect_coefficient_bits: reader.get_field_f32(index, 20).to_bits(),
        effect_variance_bits: reader.get_field_f32(index, 21).to_bits(),
        effect_trigger_spell_raw: i64::from(reader.get_field_i32(index, 17)),
        effect_item_type_raw: i64::from(reader.get_field_i32(index, 12)),
        effect_misc_value_raw: std::array::from_fn(|array_index| {
            i64::from(reader.get_array_element(index, 24, array_index, 32) as i32)
        }),
        implicit_target_raw: std::array::from_fn(|array_index| {
            i64::from(reader.get_array_element(index, 27, array_index, 16) as i16)
        }),
    }
}

pub(super) trait SpellEffectSqlFieldSourceLikeCpp {
    fn raw(&mut self, column: usize, field: &'static str) -> i64;
    fn f32_bits(&mut self, column: usize, field: &'static str) -> u32;
}

pub(super) struct SpellEffectOverlayFieldSourceLikeCpp<'a> {
    pub(super) result: &'a SpellAcquisitionSqlOverlayRowLikeCpp,
    pub(super) diagnostics: &'a mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    pub(super) record_id: u32,
}

impl SpellEffectSqlFieldSourceLikeCpp for SpellEffectOverlayFieldSourceLikeCpp<'_> {
    fn raw(&mut self, column: usize, field: &'static str) -> i64 {
        sql_raw_or_invalid(
            self.result,
            column,
            field,
            SpellAcquisitionTableLikeCpp::SpellEffect,
            self.record_id,
            self.diagnostics,
        )
    }

    fn f32_bits(&mut self, column: usize, field: &'static str) -> u32 {
        sql_f32_bits_or_invalid(
            self.result,
            column,
            field,
            SpellAcquisitionTableLikeCpp::SpellEffect,
            self.record_id,
            self.diagnostics,
        )
    }
}

pub(super) fn spell_acquisition_effect_from_sql_source_like_cpp(
    record_id: u32,
    source: &mut impl SpellEffectSqlFieldSourceLikeCpp,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        difficulty_id_raw: source.raw(1, "SpellEffect.DifficultyID"),
        effect_index_raw: source.raw(2, "SpellEffect.EffectIndex"),
        effect_type_raw: source.raw(3, "SpellEffect.Effect"),
        effect_aura_raw: source.raw(SPELL_EFFECT_SQL_AURA_COLUMN, "SpellEffect.EffectAura"),
        effect_mechanic_raw: source.raw(
            SPELL_EFFECT_SQL_MECHANIC_COLUMN,
            "SpellEffect.EffectMechanic",
        ),
        effect_attributes_raw: source.raw(
            SPELL_EFFECT_SQL_ATTRIBUTES_COLUMN,
            "SpellEffect.EffectAttributes",
        ),
        effect_base_points_raw: source.raw(4, "SpellEffect.EffectBasePoints"),
        effect_die_sides_raw: source.raw(5, "SpellEffect.EffectDieSides"),
        effect_chain_targets_raw: source.raw(
            SPELL_EFFECT_SQL_CHAIN_TARGETS_COLUMN,
            "SpellEffect.EffectChainTargets",
        ),
        effect_points_per_resource_bits: source.f32_bits(
            SPELL_EFFECT_SQL_POINTS_PER_RESOURCE_COLUMN,
            "SpellEffect.EffectPointsPerResource",
        ),
        effect_real_points_per_level_bits: source.f32_bits(
            SPELL_EFFECT_SQL_REAL_POINTS_PER_LEVEL_COLUMN,
            "SpellEffect.EffectRealPointsPerLevel",
        ),
        effect_coefficient_bits: source.f32_bits(11, "SpellEffect.Coefficient"),
        effect_variance_bits: source.f32_bits(12, "SpellEffect.Variance"),
        effect_trigger_spell_raw: source.raw(6, "SpellEffect.EffectTriggerSpell"),
        effect_item_type_raw: source.raw(
            SPELL_EFFECT_SQL_ITEM_TYPE_COLUMN,
            "SpellEffect.EffectItemType",
        ),
        effect_misc_value_raw: [
            source.raw(7, "SpellEffect.EffectMiscValue1"),
            source.raw(8, "SpellEffect.EffectMiscValue2"),
        ],
        implicit_target_raw: [
            source.raw(9, "SpellEffect.ImplicitTarget1"),
            source.raw(10, "SpellEffect.ImplicitTarget2"),
        ],
        spell_id_raw: source.raw(13, "SpellEffect.SpellID"),
    }
}

pub(super) fn spell_acquisition_effect_from_sql_like_cpp(
    record_id: u32,
    result: &SpellAcquisitionSqlOverlayRowLikeCpp,
    diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
) -> SpellAcquisitionEffectLikeCpp {
    let mut source = SpellEffectOverlayFieldSourceLikeCpp {
        result,
        diagnostics,
        record_id,
    };
    spell_acquisition_effect_from_sql_source_like_cpp(record_id, &mut source)
}

impl SpellAcquisitionCatalogLikeCpp {
    /// Load and compose the seven acquisition source families.
    ///
    /// `coverage` must contain the exact regular `(SpellID, DifficultyID)`
    /// keys and any explicitly represented server-side keys. This lets a
    /// caller distinguish an existing spell with zero acquisition effects
    /// from a key for which no authoritative payload exists.
    pub async fn load_effective_like_cpp(
        data_dir: &str,
        locale: &str,
        overlay_source: &dyn SpellAcquisitionSqlOverlaySourceLikeCpp,
        removed_records: &Db2HotfixRemovalStoreLikeCpp,
        coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
    ) -> Result<Self> {
        let coverage = coverage.into_iter().collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        let mut effective = EffectiveSpellAcquisitionRowsLikeCpp::default();
        let mut removed_rows = Vec::new();

        let (spell_effect_hash, spell_effect_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SpellEffect,
            spell_acquisition_effect_from_wdc_like_cpp,
        )?;
        let [spell_effect_official, spell_effect_custom] = load_sql_overlays_like_cpp(
            overlay_source,
            SpellAcquisitionTableLikeCpp::SpellEffect,
            &mut diagnostics,
            spell_acquisition_effect_from_sql_like_cpp,
        )
        .await?;
        let composed_spell_effects = compose_effective_table_with_removed_like_cpp(
            spell_effect_base,
            spell_effect_official,
            spell_effect_custom,
            spell_effect_hash,
            removed_records,
        );
        effective.spell_effects = composed_spell_effects
            .effective_rows
            .into_values()
            .collect();
        removed_rows.extend(
            composed_spell_effects
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SpellEffect),
        );

        let (spell_learn_spell_hash, spell_learn_spell_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            |record_id, index, reader| SpellAcquisitionDependencyLikeCpp {
                record_id,
                spell_id_raw: i64::from(reader.get_field_i32(index, 0)),
                learn_spell_id_raw: i64::from(reader.get_field_i32(index, 1)),
                overrides_spell_id_raw: i64::from(reader.get_field_i32(index, 2)),
            },
        )?;
        let [spell_learn_spell_official, spell_learn_spell_custom] = load_sql_overlays_like_cpp(
            overlay_source,
            SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionDependencyLikeCpp {
                record_id,
                spell_id_raw: sql_raw_or_invalid(
                    result,
                    1,
                    "SpellLearnSpell.SpellID",
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    record_id,
                    diagnostics,
                ),
                learn_spell_id_raw: sql_raw_or_invalid(
                    result,
                    2,
                    "SpellLearnSpell.LearnSpellID",
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    record_id,
                    diagnostics,
                ),
                overrides_spell_id_raw: sql_raw_or_invalid(
                    result,
                    3,
                    "SpellLearnSpell.OverridesSpellID",
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_spell_learn_spells = compose_effective_table_with_removed_like_cpp(
            spell_learn_spell_base,
            spell_learn_spell_official,
            spell_learn_spell_custom,
            spell_learn_spell_hash,
            removed_records,
        );
        effective.spell_learn_spells = composed_spell_learn_spells
            .effective_rows
            .into_values()
            .collect();
        removed_rows.extend(
            composed_spell_learn_spells
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SpellLearnSpell),
        );

        let (spell_misc_hash, spell_misc_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SpellMisc,
            |record_id, index, reader| SpellAcquisitionMiscLikeCpp {
                record_id,
                attributes_raw: std::array::from_fn(|array_index| {
                    i64::from(reader.get_array_element(index, 0, array_index, 32))
                }),
                difficulty_id_raw: i64::from(reader.get_field_u8(index, 1)),
                show_future_spell_player_condition_id_raw: i64::from(
                    reader.get_field_i32(index, 12),
                ),
                spell_id_raw: i64::from(reader.get_relationship_id(index).unwrap_or(0)),
            },
        )?;
        let [spell_misc_official, spell_misc_custom] = load_sql_overlays_like_cpp(
            overlay_source,
            SpellAcquisitionTableLikeCpp::SpellMisc,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionMiscLikeCpp {
                record_id,
                attributes_raw: [
                    sql_raw_or_invalid(
                        result,
                        1,
                        "SpellMisc.Attributes1",
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        record_id,
                        diagnostics,
                    ),
                    sql_raw_or_invalid(
                        result,
                        2,
                        "SpellMisc.Attributes2",
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        record_id,
                        diagnostics,
                    ),
                ],
                difficulty_id_raw: sql_raw_or_invalid(
                    result,
                    3,
                    "SpellMisc.DifficultyID",
                    SpellAcquisitionTableLikeCpp::SpellMisc,
                    record_id,
                    diagnostics,
                ),
                show_future_spell_player_condition_id_raw: sql_raw_or_invalid(
                    result,
                    4,
                    "SpellMisc.ShowFutureSpellPlayerConditionID",
                    SpellAcquisitionTableLikeCpp::SpellMisc,
                    record_id,
                    diagnostics,
                ),
                spell_id_raw: sql_raw_or_invalid(
                    result,
                    5,
                    "SpellMisc.SpellID",
                    SpellAcquisitionTableLikeCpp::SpellMisc,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_spell_misc = compose_effective_table_with_removed_like_cpp(
            spell_misc_base,
            spell_misc_official,
            spell_misc_custom,
            spell_misc_hash,
            removed_records,
        );
        effective.spell_misc = composed_spell_misc.effective_rows.into_values().collect();
        removed_rows.extend(
            composed_spell_misc
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SpellMisc),
        );

        let (spell_levels_hash, spell_levels_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SpellLevels,
            |record_id, index, reader| SpellAcquisitionLevelsLikeCpp {
                record_id,
                difficulty_id_raw: i64::from(reader.get_field_u8(index, 0)),
                base_level_raw: i64::from(reader.get_field_i16(index, 1)),
                spell_level_raw: i64::from(reader.get_field_i16(index, 3)),
                spell_id_raw: i64::from(reader.get_relationship_id(index).unwrap_or(0)),
            },
        )?;
        let [spell_levels_official, spell_levels_custom] = load_sql_overlays_like_cpp(
            overlay_source,
            SpellAcquisitionTableLikeCpp::SpellLevels,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionLevelsLikeCpp {
                record_id,
                difficulty_id_raw: sql_raw_or_invalid(
                    result,
                    1,
                    "SpellLevels.DifficultyID",
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    record_id,
                    diagnostics,
                ),
                base_level_raw: sql_raw_or_invalid(
                    result,
                    2,
                    "SpellLevels.BaseLevel",
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    record_id,
                    diagnostics,
                ),
                spell_level_raw: sql_raw_or_invalid(
                    result,
                    3,
                    "SpellLevels.SpellLevel",
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    record_id,
                    diagnostics,
                ),
                spell_id_raw: sql_raw_or_invalid(
                    result,
                    4,
                    "SpellLevels.SpellID",
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_spell_levels = compose_effective_table_with_removed_like_cpp(
            spell_levels_base,
            spell_levels_official,
            spell_levels_custom,
            spell_levels_hash,
            removed_records,
        );
        effective.spell_levels = composed_spell_levels.effective_rows.into_values().collect();
        removed_rows.extend(
            composed_spell_levels
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SpellLevels),
        );

        let (talent_hash, talent_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::Talent,
            |record_id, index, reader| SpellAcquisitionTalentLikeCpp {
                record_id,
                spell_rank_raw: std::array::from_fn(|array_index| {
                    i64::from(reader.get_array_element(index, 11, array_index, 32) as i32)
                }),
            },
        )?;
        let [talent_official, talent_custom] = load_sql_overlays_like_cpp(
            overlay_source,
            SpellAcquisitionTableLikeCpp::Talent,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionTalentLikeCpp {
                record_id,
                spell_rank_raw: std::array::from_fn(|array_index| {
                    sql_raw_or_invalid(
                        result,
                        1 + array_index,
                        "Talent.SpellRank",
                        SpellAcquisitionTableLikeCpp::Talent,
                        record_id,
                        diagnostics,
                    )
                }),
            },
        )
        .await?;
        let composed_talents = compose_effective_table_with_removed_like_cpp(
            talent_base,
            talent_official,
            talent_custom,
            talent_hash,
            removed_records,
        );
        effective.talents = composed_talents.effective_rows.into_values().collect();
        removed_rows.extend(
            composed_talents
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::Talent),
        );

        let (summon_properties_hash, summon_properties_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SummonProperties,
            |record_id, index, reader| SpellAcquisitionSummonPropertiesLikeCpp {
                record_id,
                slot_raw: i64::from(reader.get_field_i32(index, 3)),
                flags_1_raw: i64::from(reader.get_array_element(index, 4, 0, 32) as i32),
            },
        )?;
        let [summon_properties_official, summon_properties_custom] = load_sql_overlays_like_cpp(
            overlay_source,
            SpellAcquisitionTableLikeCpp::SummonProperties,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionSummonPropertiesLikeCpp {
                record_id,
                slot_raw: sql_raw_or_invalid(
                    result,
                    1,
                    "SummonProperties.Slot",
                    SpellAcquisitionTableLikeCpp::SummonProperties,
                    record_id,
                    diagnostics,
                ),
                flags_1_raw: sql_raw_or_invalid(
                    result,
                    2,
                    "SummonProperties.Flags1",
                    SpellAcquisitionTableLikeCpp::SummonProperties,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_summon_properties = compose_effective_table_with_removed_like_cpp(
            summon_properties_base,
            summon_properties_official,
            summon_properties_custom,
            summon_properties_hash,
            removed_records,
        );
        effective.summon_properties = composed_summon_properties
            .effective_rows
            .into_values()
            .collect();
        removed_rows.extend(
            composed_summon_properties
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SummonProperties),
        );

        let (battle_pet_species_hash, battle_pet_species_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            |record_id, index, reader| SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: record_id,
                creature_id_raw: i64::from(reader.get_field_i32(index, 3)),
            },
        )?;
        let [battle_pet_species_official, battle_pet_species_custom] = load_sql_overlays_like_cpp(
            overlay_source,
            SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: record_id,
                creature_id_raw: sql_raw_or_invalid(
                    result,
                    1,
                    "BattlePetSpecies.CreatureID",
                    SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_battle_pet_species = compose_effective_table_with_removed_like_cpp(
            battle_pet_species_base,
            battle_pet_species_official,
            battle_pet_species_custom,
            battle_pet_species_hash,
            removed_records,
        );
        effective.battle_pet_species = composed_battle_pet_species
            .effective_rows
            .into_values()
            .collect();
        removed_rows.extend(
            composed_battle_pet_species
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::BattlePetSpecies),
        );

        let table_hashes = SpellAcquisitionTableHashesLikeCpp {
            spell_effect: spell_effect_hash,
            spell_learn_spell: spell_learn_spell_hash,
            spell_misc: spell_misc_hash,
            spell_levels: spell_levels_hash,
            talent: talent_hash,
            summon_properties: summon_properties_hash,
            battle_pet_species: battle_pet_species_hash,
        };
        let table_by_hash = [
            (
                table_hashes.spell_effect,
                SpellAcquisitionTableLikeCpp::SpellEffect,
            ),
            (
                table_hashes.spell_learn_spell,
                SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            ),
            (
                table_hashes.spell_misc,
                SpellAcquisitionTableLikeCpp::SpellMisc,
            ),
            (
                table_hashes.spell_levels,
                SpellAcquisitionTableLikeCpp::SpellLevels,
            ),
            (table_hashes.talent, SpellAcquisitionTableLikeCpp::Talent),
            (
                table_hashes.summon_properties,
                SpellAcquisitionTableLikeCpp::SummonProperties,
            ),
            (
                table_hashes.battle_pet_species,
                SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            ),
        ];
        for (table_hash, record_id) in removed_records.removed_records_in_order_like_cpp() {
            for (_, table) in table_by_hash
                .iter()
                .filter(|(candidate_hash, _)| *candidate_hash == table_hash)
            {
                if !removed_rows.iter().any(|row| {
                    row.table_like_cpp() == *table && row.hotfix_record_id_like_cpp() == record_id
                }) {
                    removed_rows.push(SpellAcquisitionRemovedRowLikeCpp::Unknown {
                        table: *table,
                        record_id,
                    });
                }
            }
        }

        Ok(Self::from_effective_rows_and_removed_like_cpp(
            coverage,
            effective,
            removed_rows,
            table_hashes,
            diagnostics,
        ))
    }
}

pub(super) fn load_wdc_rows_like_cpp<T>(
    data_dir: &str,
    locale: &str,
    table: SpellAcquisitionTableLikeCpp,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<(u32, Vec<(u32, T)>)> {
    let path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join(table.file_name());
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
    let table_hash = reader.table_hash();
    let rows = reader
        .iter_records()
        .map(|(record_id, index)| (record_id, read(record_id, index, &reader)))
        .collect();
    Ok((table_hash, rows))
}

pub(super) async fn load_sql_overlays_like_cpp<T>(
    overlay_source: &dyn SpellAcquisitionSqlOverlaySourceLikeCpp,
    table: SpellAcquisitionTableLikeCpp,
    diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    mut read: impl FnMut(
        u32,
        &SpellAcquisitionSqlOverlayRowLikeCpp,
        &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) -> T,
) -> Result<[Vec<(u32, T)>; 2]> {
    let mut batches = [Vec::new(), Vec::new()];
    for (batch_index, official) in [true, false].into_iter().enumerate() {
        let rows = overlay_source
            .load_overlay_like_cpp(table, official)
            .await
            .with_context(|| format!("failed to load {} SQL overlay", table.file_name()))?;
        for result in rows {
            match sql_raw_i64(&result, 0).and_then(|raw| u32::try_from(raw).ok()) {
                Some(record_id) => {
                    let row = read(record_id, &result, diagnostics);
                    batches[batch_index].push((record_id, row));
                }
                None => diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
                    severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
                    table,
                    record_id: None,
                    kind: SpellAcquisitionDiagnosticKindLikeCpp::UnreadableSqlField { field: "ID" },
                }),
            }
        }
    }
    Ok(batches)
}

pub(super) const UNREADABLE_SQL_RAW_LIKE_CPP: i64 = i64::MIN;

pub(super) fn sql_raw_or_invalid(
    result: &SpellAcquisitionSqlOverlayRowLikeCpp,
    column: usize,
    field: &'static str,
    table: SpellAcquisitionTableLikeCpp,
    record_id: u32,
    diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
) -> i64 {
    sql_raw_i64(result, column).unwrap_or_else(|| {
        diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
            severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
            table,
            record_id: Some(record_id),
            kind: SpellAcquisitionDiagnosticKindLikeCpp::UnreadableSqlField { field },
        });
        UNREADABLE_SQL_RAW_LIKE_CPP
    })
}

pub(super) fn sql_f32_bits_or_invalid(
    result: &SpellAcquisitionSqlOverlayRowLikeCpp,
    column: usize,
    field: &'static str,
    table: SpellAcquisitionTableLikeCpp,
    record_id: u32,
    diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
) -> u32 {
    result
        .float_columns_bits
        .get(column)
        .copied()
        .flatten()
        .unwrap_or_else(|| {
            diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
                severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
                table,
                record_id: Some(record_id),
                kind: SpellAcquisitionDiagnosticKindLikeCpp::UnreadableSqlField { field },
            });
            f32::NAN.to_bits()
        })
}

pub(super) fn sql_raw_i64(
    result: &SpellAcquisitionSqlOverlayRowLikeCpp,
    column: usize,
) -> Option<i64> {
    result.integer_columns.get(column).copied().flatten()
}

pub(super) const fn invalid(
    field: &'static str,
    raw: i64,
    expected: &'static str,
) -> InvalidAcquisitionValueLikeCpp {
    InvalidAcquisitionValueLikeCpp {
        field,
        raw,
        expected,
    }
}

pub(super) fn source_i32(
    raw: i64,
    field: &'static str,
) -> Result<i32, InvalidAcquisitionValueLikeCpp> {
    i32::try_from(raw).map_err(|_| invalid(field, raw, "i32"))
}

pub(super) fn positive_u32(
    raw: i64,
    field: &'static str,
) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
    u32::try_from(raw)
        .ok()
        .filter(|value| *value != 0)
        .ok_or_else(|| invalid(field, raw, "positive u32"))
}

pub(super) fn optional_positive_u32(
    raw: i64,
    field: &'static str,
) -> Result<Option<u32>, InvalidAcquisitionValueLikeCpp> {
    if raw == 0 {
        return Ok(None);
    }
    positive_u32(raw, field).map(Some)
}

pub(super) fn nonnegative_u32(
    raw: i64,
    field: &'static str,
) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
    u32::try_from(raw).map_err(|_| invalid(field, raw, "u32"))
}

pub(super) fn checked_u8(
    raw: i64,
    field: &'static str,
) -> Result<u8, InvalidAcquisitionValueLikeCpp> {
    u8::try_from(raw).map_err(|_| invalid(field, raw, "u8"))
}

pub(super) fn checked_u32_bits(
    raw: i64,
    field: &'static str,
) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
    if let Ok(value) = u32::try_from(raw) {
        return Ok(value);
    }
    i32::try_from(raw)
        .map(|value| value as u32)
        .map_err(|_| invalid(field, raw, "u32/i32 bit field"))
}

pub(super) fn finite_f32_from_bits(
    bits: u32,
    field: &'static str,
) -> Result<f32, InvalidAcquisitionValueLikeCpp> {
    let value = f32::from_bits(bits);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(invalid(field, i64::from(bits), "finite f32"))
    }
}
