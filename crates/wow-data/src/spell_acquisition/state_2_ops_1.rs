//! Spell acquisition model state definitions, part 2 of 3. operations, part 1 of 2.
//!
//! The inherent impl is divided by responsibility under #646; every
//! method keeps its original body.

use super::*;

impl SpellAcquisitionCatalogLikeCpp {
    /// Whether the canonical spell-info key exists, independently of whether
    /// every acquisition table for that key is hydrated.  This is the narrow
    /// equivalent needed for C++ `GetSpellInfo` short-circuits before later
    /// gates decide whether full metadata is required.
    pub fn contains_spell_difficulty_key_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> bool {
        self.coverage_by_key
            .contains_key(&(spell_id, difficulty_id))
    }
    /// Build every derived index from already-final effective rows.
    ///
    /// Rows are sorted by record id before projection. For duplicate
    /// `(spell, difficulty, effect index)` slots this reproduces the C++ DB2
    /// iteration result: the higher final record id wins. Every SUMMON row is
    /// retained separately because C++ builds its battle-pet map while
    /// iterating the store, before assigning the effect slot.
    pub fn from_effective_rows_like_cpp(
        coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
        rows: EffectiveSpellAcquisitionRowsLikeCpp,
        table_hashes: SpellAcquisitionTableHashesLikeCpp,
        diagnostics: Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) -> Self {
        Self::from_effective_rows_and_removed_like_cpp(
            coverage,
            rows,
            Vec::new(),
            table_hashes,
            diagnostics,
        )
    }
    pub fn from_effective_rows_and_removed_like_cpp(
        coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
        mut rows: EffectiveSpellAcquisitionRowsLikeCpp,
        mut removed_rows: Vec<SpellAcquisitionRemovedRowLikeCpp>,
        table_hashes: SpellAcquisitionTableHashesLikeCpp,
        mut diagnostics: Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) -> Self {
        removed_rows.sort_by_key(|row| (row.table_like_cpp(), row.record_id_like_cpp()));
        let mut catalog = Self {
            table_hashes,
            coverage_by_key: BTreeMap::new(),
            effects_by_key: BTreeMap::new(),
            acquisition_effects_by_key: BTreeMap::new(),
            summon_effects_by_spell: BTreeMap::new(),
            dependencies_by_spell: BTreeMap::new(),
            dependency_rows: Vec::new(),
            misc_by_key: BTreeMap::new(),
            levels_by_key: BTreeMap::new(),
            talent_spell_ids: BTreeSet::new(),
            summon_properties_by_id: BTreeMap::new(),
            species_by_creature: BTreeMap::new(),
            removed_rows,
            removed_summon_properties_ids: BTreeSet::new(),
            removed_species_by_creature: BTreeMap::new(),
            global_indeterminate_by_table: BTreeMap::new(),
            diagnostics: Vec::new(),
        };

        for removed in &catalog.removed_rows {
            match removed {
                SpellAcquisitionRemovedRowLikeCpp::SummonProperties(row) => {
                    catalog.removed_summon_properties_ids.insert(row.record_id);
                }
                SpellAcquisitionRemovedRowLikeCpp::BattlePetSpecies(row) => {
                    if let Ok(creature_id) =
                        source_i32(row.creature_id_raw, "BattlePetSpecies.CreatureID")
                        && creature_id > 0
                    {
                        catalog
                            .removed_species_by_creature
                            .entry(creature_id as u32)
                            .or_default()
                            .insert(row.species_id);
                    }
                }
                SpellAcquisitionRemovedRowLikeCpp::Unknown {
                    table: SpellAcquisitionTableLikeCpp::SummonProperties,
                    record_id,
                } if *record_id > 0 => {
                    catalog
                        .removed_summon_properties_ids
                        .insert(*record_id as u32);
                }
                _ => {}
            }
        }

        for diagnostic in &diagnostics {
            if diagnostic.severity == SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate
                && diagnostic.record_id.is_none()
                && matches!(
                    &diagnostic.kind,
                    SpellAcquisitionDiagnosticKindLikeCpp::UnreadableSqlField { .. }
                )
            {
                catalog.push_global_reason_like_cpp(
                    diagnostic.table,
                    SpellAcquisitionIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table: diagnostic.table,
                    },
                );
            }
        }

        for seed in coverage {
            let entry = catalog
                .coverage_by_key
                .entry((seed.spell_id, seed.difficulty_id))
                .or_default();
            if let SpellAcquisitionSourceCoverageLikeCpp::Indeterminate(reason) = seed.source {
                entry.add_source_reason_like_cpp(reason);
            }
        }

        rows.spell_effects.sort_by_key(|row| row.record_id);
        let mut effect_slots =
            BTreeMap::<(u32, u32), BTreeMap<u8, SpellAcquisitionEffectLikeCpp>>::new();
        for row in rows.spell_effects {
            let spell_id = match row.spell_id_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let difficulty_id = match row.difficulty_id_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_invalid_spell_like_cpp(
                        spell_id,
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let key = (spell_id, difficulty_id);
            let effect_type = match row.effect_type_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_invalid_key_like_cpp(
                        key,
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            if effect_type == SPELL_EFFECT_SUMMON_LIKE_CPP {
                catalog
                    .summon_effects_by_spell
                    .entry(spell_id)
                    .or_default()
                    .push(row.clone());
            }
            let effect_index = match row.effect_index_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_invalid_key_like_cpp(
                        key,
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };

            // C++ ASSERTs these structural fields while iterating every
            // effective SpellEffect row, before slot replacement. Preserve
            // that fail-closed boundary even when a later RecordID shadows
            // this slot. Non-structural payload is validated only after the
            // final slot winner is known.
            for (field, raw) in [
                ("SpellEffect.ImplicitTarget1", row.implicit_target_raw[0]),
                ("SpellEffect.ImplicitTarget2", row.implicit_target_raw[1]),
            ] {
                if i16::try_from(raw).is_err() || !(0..TOTAL_SPELL_TARGETS_LIKE_CPP).contains(&raw)
                {
                    catalog.mark_invalid_key_like_cpp(
                        key,
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        invalid(field, raw, "0..TOTAL_SPELL_TARGETS"),
                        &mut diagnostics,
                    );
                }
            }

            let slots = effect_slots.entry(key).or_default();
            if let Some(replaced) = slots.insert(effect_index, row.clone()) {
                diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
                    severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Warning,
                    table: SpellAcquisitionTableLikeCpp::SpellEffect,
                    record_id: Some(row.record_id),
                    kind: SpellAcquisitionDiagnosticKindLikeCpp::EffectSlotCollisionResolved {
                        spell_id,
                        difficulty_id,
                        effect_index,
                        replaced_record_id: replaced.record_id,
                        winning_record_id: row.record_id,
                    },
                });
            }
        }
        catalog.effects_by_key = effect_slots
            .into_iter()
            .map(|(key, slots)| (key, slots.into_values().collect()))
            .collect();
        let final_effect_rows = catalog
            .effects_by_key
            .iter()
            .flat_map(|(key, effects)| {
                effects
                    .iter()
                    .cloned()
                    .map(|effect| (*key, effect))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for (_key, row) in final_effect_rows {
            for error in acquisition_effect_payload_errors_like_cpp(&row) {
                diagnostics.push(diagnostic_from_invalid(
                    SpellAcquisitionTableLikeCpp::SpellEffect,
                    row.record_id,
                    error,
                ));
            }
        }
        catalog.acquisition_effects_by_key = catalog
            .effects_by_key
            .iter()
            .filter_map(|(key, effects)| {
                let acquisition_effects = effects
                    .iter()
                    .filter(|effect| {
                        effect
                            .effect_type_checked()
                            .is_ok_and(is_acquisition_effect_like_cpp)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                (!acquisition_effects.is_empty()).then_some((*key, acquisition_effects))
            })
            .collect();
        for effects in catalog.summon_effects_by_spell.values_mut() {
            effects.sort_by_key(|effect| {
                (
                    effect.difficulty_id_raw,
                    effect.effect_index_raw,
                    effect.record_id,
                )
            });
        }

        rows.spell_learn_spells.sort_by_key(|row| row.record_id);
        for row in rows.spell_learn_spells {
            let source_spell = match row.spell_id_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    catalog.dependency_rows.push(row);
                    continue;
                }
            };
            let key = (source_spell, DIFFICULTY_NONE_LIKE_CPP);
            if let Err(error) = row.learned_spell_id_checked() {
                catalog.mark_invalid_key_like_cpp(
                    key,
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    row.record_id,
                    error,
                    &mut diagnostics,
                );
            }
            if let Err(error) = row.overrides_spell_id_checked() {
                catalog.mark_invalid_key_like_cpp(
                    key,
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    row.record_id,
                    error,
                    &mut diagnostics,
                );
            }
            catalog
                .dependencies_by_spell
                .entry(source_spell)
                .or_default()
                .push(row.clone());
            catalog.dependency_rows.push(row);
        }

        rows.spell_misc.sort_by_key(|row| row.record_id);
        for row in rows.spell_misc {
            let spell_id = match positive_u32(row.spell_id_raw, "SpellMisc.SpellID") {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let difficulty_id = match checked_u8(row.difficulty_id_raw, "SpellMisc.DifficultyID") {
                Ok(value) => u32::from(value),
                Err(error) => {
                    catalog.mark_invalid_spell_like_cpp(
                        spell_id,
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let key = (spell_id, difficulty_id);
            if let Some(replaced) = catalog.misc_by_key.insert(key, row.clone()) {
                diagnostics.push(metadata_collision_diagnostic(
                    SpellAcquisitionTableLikeCpp::SpellMisc,
                    key,
                    replaced.record_id,
                    row.record_id,
                ));
            }
        }
        let final_misc_rows = catalog
            .misc_by_key
            .iter()
            .map(|(key, row)| (*key, row.clone()))
            .collect::<Vec<_>>();
        for (_key, row) in final_misc_rows {
            for result in [
                checked_u32_bits(row.attributes_raw[0], "SpellMisc.Attributes1").map(|_| ()),
                checked_u32_bits(row.attributes_raw[1], "SpellMisc.Attributes2").map(|_| ()),
                row.future_player_condition_id_checked().map(|_| ()),
            ] {
                if let Err(error) = result {
                    diagnostics.push(diagnostic_from_invalid(
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        row.record_id,
                        error,
                    ));
                }
            }
        }

        rows.spell_levels.sort_by_key(|row| row.record_id);
        for row in rows.spell_levels {
            let spell_id = match positive_u32(row.spell_id_raw, "SpellLevels.SpellID") {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::SpellLevels,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let difficulty_id = match checked_u8(row.difficulty_id_raw, "SpellLevels.DifficultyID")
            {
                Ok(value) => u32::from(value),
                Err(error) => {
                    catalog.mark_invalid_spell_like_cpp(
                        spell_id,
                        SpellAcquisitionTableLikeCpp::SpellLevels,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let key = (spell_id, difficulty_id);
            if let Some(replaced) = catalog.levels_by_key.insert(key, row.clone()) {
                diagnostics.push(metadata_collision_diagnostic(
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    key,
                    replaced.record_id,
                    row.record_id,
                ));
            }
        }
        let final_levels_rows = catalog
            .levels_by_key
            .iter()
            .map(|(key, row)| (*key, row.clone()))
            .collect::<Vec<_>>();
        for (_key, row) in final_levels_rows {
            for result in [
                row.base_level_checked().map(|_| ()),
                row.spell_level_checked().map(|_| ()),
            ] {
                if let Err(error) = result {
                    diagnostics.push(diagnostic_from_invalid(
                        SpellAcquisitionTableLikeCpp::SpellLevels,
                        row.record_id,
                        error,
                    ));
                }
            }
        }

        rows.talents.sort_by_key(|row| row.record_id);
        for row in rows.talents {
            for raw in row.spell_rank_raw {
                if raw == 0 {
                    continue;
                }
                match source_i32(raw, "Talent.SpellRank")
                    .and_then(|_| positive_u32(raw, "Talent.SpellRank"))
                {
                    Ok(spell_id) => {
                        catalog.talent_spell_ids.insert(spell_id);
                    }
                    Err(error) => catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::Talent,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    ),
                }
            }
        }

        rows.summon_properties.sort_by_key(|row| row.record_id);
        for row in rows.summon_properties {
            if let Err(error) = source_i32(row.slot_raw, "SummonProperties.Slot") {
                diagnostics.push(diagnostic_from_invalid(
                    SpellAcquisitionTableLikeCpp::SummonProperties,
                    row.record_id,
                    error,
                ));
            }
            if let Err(error) = checked_u32_bits(row.flags_1_raw, "SummonProperties.Flags1") {
                diagnostics.push(diagnostic_from_invalid(
                    SpellAcquisitionTableLikeCpp::SummonProperties,
                    row.record_id,
                    error,
                ));
            }
            catalog.summon_properties_by_id.insert(row.record_id, row);
        }

        rows.battle_pet_species.sort_by_key(|row| row.species_id);
        for row in rows.battle_pet_species {
            match source_i32(row.creature_id_raw, "BattlePetSpecies.CreatureID") {
                Ok(0) => {}
                Ok(value) if value > 0 => {
                    catalog
                        .species_by_creature
                        .entry(value as u32)
                        .or_default()
                        .insert(row.species_id);
                }
                Err(_) if row.creature_id_raw == UNREADABLE_SQL_RAW_LIKE_CPP => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                        row.species_id,
                        invalid(
                            "BattlePetSpecies.CreatureID",
                            row.creature_id_raw,
                            "readable zero or positive i32",
                        ),
                        &mut diagnostics,
                    );
                }
                Ok(_) | Err(_) => diagnostics.push(diagnostic_from_invalid(
                    SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                    row.species_id,
                    invalid(
                        "BattlePetSpecies.CreatureID",
                        row.creature_id_raw,
                        "zero or positive i32",
                    ),
                )),
            }
        }
        for (creature_id, species_ids) in &catalog.species_by_creature {
            if species_ids.len() > 1 {
                diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
                    severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
                    table: SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                    record_id: None,
                    kind: SpellAcquisitionDiagnosticKindLikeCpp::ConflictingSpeciesForCreature {
                        creature_id: *creature_id,
                        species_ids: species_ids.iter().copied().collect(),
                    },
                });
            }
        }

        let global_reasons_by_table = catalog.global_indeterminate_by_table.clone();
        for coverage in catalog.coverage_by_key.values_mut() {
            for (table, reasons) in &global_reasons_by_table {
                for reason in reasons {
                    coverage.add_table_reason_like_cpp(*table, reason.clone());
                }
            }
        }

        catalog.diagnostics = diagnostics;
        catalog
    }
    pub(super) fn mark_invalid_key_like_cpp(
        &mut self,
        key: (u32, u32),
        table: SpellAcquisitionTableLikeCpp,
        record_id: u32,
        error: InvalidAcquisitionValueLikeCpp,
        diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) {
        diagnostics.push(diagnostic_from_invalid(table, record_id, error));
        if let Some(coverage) = self.coverage_by_key.get_mut(&key) {
            let reason = SpellAcquisitionIndeterminateReasonLikeCpp::InvalidEffectiveRow {
                table,
                record_id,
                field: error.field,
                raw: error.raw,
            };
            coverage.add_table_reason_like_cpp(table, reason);
        }
    }
    pub(super) fn mark_invalid_spell_like_cpp(
        &mut self,
        spell_id: u32,
        table: SpellAcquisitionTableLikeCpp,
        record_id: u32,
        error: InvalidAcquisitionValueLikeCpp,
        diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) {
        diagnostics.push(diagnostic_from_invalid(table, record_id, error));
        let reason = SpellAcquisitionIndeterminateReasonLikeCpp::InvalidEffectiveRow {
            table,
            record_id,
            field: error.field,
            raw: error.raw,
        };
        for (_, coverage) in self
            .coverage_by_key
            .range_mut((spell_id, u32::MIN)..=(spell_id, u32::MAX))
        {
            coverage.add_table_reason_like_cpp(table, reason.clone());
        }
    }
    pub(super) fn mark_global_invalid_like_cpp(
        &mut self,
        table: SpellAcquisitionTableLikeCpp,
        record_id: u32,
        error: InvalidAcquisitionValueLikeCpp,
        diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) {
        diagnostics.push(diagnostic_from_invalid(table, record_id, error));
        self.push_global_reason_like_cpp(
            table,
            SpellAcquisitionIndeterminateReasonLikeCpp::InvalidEffectiveRow {
                table,
                record_id,
                field: error.field,
                raw: error.raw,
            },
        );
    }
    pub(super) fn push_global_reason_like_cpp(
        &mut self,
        table: SpellAcquisitionTableLikeCpp,
        reason: SpellAcquisitionIndeterminateReasonLikeCpp,
    ) {
        let reasons = self.global_indeterminate_by_table.entry(table).or_default();
        if !reasons.contains(&reason) {
            reasons.push(reason);
        }
    }
    pub const fn table_hashes_like_cpp(&self) -> SpellAcquisitionTableHashesLikeCpp {
        self.table_hashes
    }
    pub fn diagnostics_like_cpp(&self) -> &[SpellAcquisitionDiagnosticLikeCpp] {
        &self.diagnostics
    }
    pub fn removed_rows_like_cpp(&self) -> &[SpellAcquisitionRemovedRowLikeCpp] {
        &self.removed_rows
    }
    pub fn effects_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> SpellAcquisitionEffectsLookupLikeCpp<'_> {
        self.effects_lookup_from_map_like_cpp((spell_id, difficulty_id), &self.effects_by_key)
    }
    pub(super) fn effects_lookup_from_map_like_cpp<'a>(
        &'a self,
        key: (u32, u32),
        effects_by_key: &'a BTreeMap<(u32, u32), Vec<SpellAcquisitionEffectLikeCpp>>,
    ) -> SpellAcquisitionEffectsLookupLikeCpp<'a> {
        let Some(coverage) = self.coverage_by_key.get(&key) else {
            return SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage;
        };
        let reasons =
            coverage.reasons_for_table_like_cpp(SpellAcquisitionTableLikeCpp::SpellEffect);
        if !reasons.is_empty() {
            return SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(reasons);
        }
        SpellAcquisitionEffectsLookupLikeCpp::Covered(
            effects_by_key.get(&key).map(Vec::as_slice).unwrap_or(&[]),
        )
    }
    /// Ordered final acquisition effects for `DIFFICULTY_NONE`.
    pub fn acquisition_effects_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionEffectsLookupLikeCpp<'_> {
        self.effects_lookup_from_map_like_cpp(
            (spell_id, DIFFICULTY_NONE_LIKE_CPP),
            &self.acquisition_effects_by_key,
        )
    }
    /// Every final `DIFFICULTY_NONE` effect slot, including effects that the
    /// acquisition planner must explicitly classify as unsupported or
    /// runtime-dependent.
    pub fn difficulty_none_effects_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionEffectsLookupLikeCpp<'_> {
        self.effects_for_spell_difficulty_like_cpp(spell_id, DIFFICULTY_NONE_LIKE_CPP)
    }
    /// Reproduce C++ `SpellInfoLoadHelper` fallback filling without requiring
    /// this specialized catalog to own the general Difficulty graph.
    ///
    /// The caller supplies `[requested, fallback, fallback-of-fallback, ...]`.
    /// Earlier difficulties win per effect slot; later rows fill only blanks.
    pub fn resolved_effects_for_difficulty_chain_like_cpp(
        &self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
    ) -> SpellAcquisitionResolvedEffectsLookupLikeCpp<'_> {
        let mut slots = BTreeMap::<u8, &SpellAcquisitionEffectLikeCpp>::new();
        for (chain_index, difficulty_id) in difficulty_chain.into_iter().enumerate() {
            let key = (spell_id, difficulty_id);
            let Some(coverage) = self.coverage_by_key.get(&key) else {
                if chain_index == 0 {
                    return SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage {
                        difficulty_id,
                    };
                }
                // C++ `SpellInfoLoadHelper` skips absent fallback rows and
                // continues walking the remainder of the Difficulty chain.
                continue;
            };
            let reasons =
                coverage.reasons_for_table_like_cpp(SpellAcquisitionTableLikeCpp::SpellEffect);
            if !reasons.is_empty() {
                return SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(
                    reasons.to_vec(),
                );
            }
            for effect in self.effects_by_key.get(&key).into_iter().flatten() {
                // Every retained winner already passed the structural index
                // check during construction.
                if let Ok(effect_index) = effect.effect_index_checked() {
                    slots.entry(effect_index).or_insert(effect);
                }
            }
        }
        SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(slots.into_values().collect())
    }
    /// Every final SUMMON row at every difficulty, before effect-slot
    /// collision reduction, in deterministic order.
    pub fn summon_effects_all_difficulties_like_cpp(
        &self,
        spell_id: u32,
    ) -> impl Iterator<Item = &SpellAcquisitionEffectLikeCpp> {
        self.summon_effects_by_spell
            .get(&spell_id)
            .into_iter()
            .flatten()
    }
    pub fn dependency_rows_from_spell_like_cpp(
        &self,
        spell_id: u32,
    ) -> &[SpellAcquisitionDependencyLikeCpp] {
        self.dependencies_by_spell
            .get(&spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub fn dependency_rows_lookup_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionDependenciesLookupLikeCpp<'_> {
        let key = (spell_id, DIFFICULTY_NONE_LIKE_CPP);
        let Some(coverage) = self.coverage_by_key.get(&key) else {
            return SpellAcquisitionDependenciesLookupLikeCpp::MissingCoverage;
        };
        let reasons =
            coverage.reasons_for_table_like_cpp(SpellAcquisitionTableLikeCpp::SpellLearnSpell);
        if !reasons.is_empty() {
            return SpellAcquisitionDependenciesLookupLikeCpp::Indeterminate(reasons);
        }
        SpellAcquisitionDependenciesLookupLikeCpp::Covered(
            self.dependency_rows_from_spell_like_cpp(spell_id),
        )
    }
    pub fn effective_dependency_rows_like_cpp(
        &self,
    ) -> impl Iterator<Item = &SpellAcquisitionDependencyLikeCpp> {
        self.dependency_rows.iter()
    }
    pub fn misc_for_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> SpellAcquisitionMetadataLookupLikeCpp<'_, SpellAcquisitionMiscLikeCpp> {
        self.metadata_lookup_like_cpp(
            (spell_id, difficulty_id),
            SpellAcquisitionTableLikeCpp::SpellMisc,
            &self.misc_by_key,
        )
    }
    pub fn levels_for_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> SpellAcquisitionMetadataLookupLikeCpp<'_, SpellAcquisitionLevelsLikeCpp> {
        self.metadata_lookup_like_cpp(
            (spell_id, difficulty_id),
            SpellAcquisitionTableLikeCpp::SpellLevels,
            &self.levels_by_key,
        )
    }
    pub fn resolved_misc_for_difficulty_chain_like_cpp(
        &self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
    ) -> SpellAcquisitionResolvedMetadataLookupLikeCpp<'_, SpellAcquisitionMiscLikeCpp> {
        self.resolved_metadata_for_difficulty_chain_like_cpp(
            spell_id,
            difficulty_chain,
            SpellAcquisitionTableLikeCpp::SpellMisc,
            &self.misc_by_key,
        )
    }
    pub fn resolved_levels_for_difficulty_chain_like_cpp(
        &self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
    ) -> SpellAcquisitionResolvedMetadataLookupLikeCpp<'_, SpellAcquisitionLevelsLikeCpp> {
        self.resolved_metadata_for_difficulty_chain_like_cpp(
            spell_id,
            difficulty_chain,
            SpellAcquisitionTableLikeCpp::SpellLevels,
            &self.levels_by_key,
        )
    }
    pub(super) fn resolved_metadata_for_difficulty_chain_like_cpp<'a, T>(
        &'a self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
        table: SpellAcquisitionTableLikeCpp,
        rows: &'a BTreeMap<(u32, u32), T>,
    ) -> SpellAcquisitionResolvedMetadataLookupLikeCpp<'a, T> {
        for (chain_index, difficulty_id) in difficulty_chain.into_iter().enumerate() {
            let key = (spell_id, difficulty_id);
            let Some(coverage) = self.coverage_by_key.get(&key) else {
                if chain_index == 0 {
                    return SpellAcquisitionResolvedMetadataLookupLikeCpp::MissingCoverage {
                        difficulty_id,
                    };
                }
                // Missing fallback data is not fatal in C++; continue to the
                // next fallback difficulty.
                continue;
            };
            let reasons = coverage.reasons_for_table_like_cpp(table);
            if !reasons.is_empty() {
                return SpellAcquisitionResolvedMetadataLookupLikeCpp::Indeterminate(
                    reasons.to_vec(),
                );
            }
            if let Some(row) = rows.get(&key) {
                return SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(row);
            }
        }
        SpellAcquisitionResolvedMetadataLookupLikeCpp::CoveredWithoutRow
    }
}
