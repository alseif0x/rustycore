//! Spell acquisition model state definitions, part 2 of 3. operations, part 2 of 2.
//!
//! The inherent impl is divided by responsibility under #646; every
//! method keeps its original body.

use super::*;

impl SpellAcquisitionCatalogLikeCpp {
    pub(super) fn metadata_lookup_like_cpp<'a, T>(
        &'a self,
        key: (u32, u32),
        table: SpellAcquisitionTableLikeCpp,
        rows: &'a BTreeMap<(u32, u32), T>,
    ) -> SpellAcquisitionMetadataLookupLikeCpp<'a, T> {
        let Some(coverage) = self.coverage_by_key.get(&key) else {
            return SpellAcquisitionMetadataLookupLikeCpp::MissingCoverage;
        };
        let reasons = coverage.reasons_for_table_like_cpp(table);
        if !reasons.is_empty() {
            return SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(reasons);
        }
        rows.get(&key).map_or(
            SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow,
            SpellAcquisitionMetadataLookupLikeCpp::Present,
        )
    }
    pub fn talent_membership_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionTalentLookupLikeCpp<'_> {
        // Membership is monotonic: one valid final rank proves talent status
        // even if another unrelated Talent row was unreadable.
        if self.talent_spell_ids.contains(&spell_id) {
            return SpellAcquisitionTalentLookupLikeCpp::Talent;
        }
        if let Some(reasons) = self
            .global_indeterminate_by_table
            .get(&SpellAcquisitionTableLikeCpp::Talent)
            && !reasons.is_empty()
        {
            return SpellAcquisitionTalentLookupLikeCpp::Indeterminate(reasons);
        }
        SpellAcquisitionTalentLookupLikeCpp::NotTalent
    }
    pub fn talent_spell_ids_like_cpp(&self) -> impl Iterator<Item = u32> + '_ {
        self.talent_spell_ids.iter().copied()
    }
    pub fn summon_properties_like_cpp(
        &self,
        properties_id: u32,
    ) -> Option<&SpellAcquisitionSummonPropertiesLikeCpp> {
        self.summon_properties_by_id.get(&properties_id)
    }
    pub fn battle_pet_classification_like_cpp(
        &self,
        spell_id: u32,
    ) -> BattlePetClassificationLikeCpp {
        let mut species_for_spell = BTreeSet::new();
        let mut reasons = Vec::new();

        let mut found_spell_coverage = false;
        for ((covered_spell_id, _difficulty_id), coverage) in self
            .coverage_by_key
            .range((spell_id, u32::MIN)..=(spell_id, u32::MAX))
        {
            debug_assert_eq!(*covered_spell_id, spell_id);
            found_spell_coverage = true;
            for reason in
                coverage.reasons_for_table_like_cpp(SpellAcquisitionTableLikeCpp::SpellEffect)
            {
                let mapped = BattlePetIndeterminateReasonLikeCpp::SpellCoverage {
                    spell_id,
                    reason: reason.clone(),
                };
                if !reasons.contains(&mapped) {
                    reasons.push(mapped);
                }
            }
        }
        if !found_spell_coverage {
            reasons.push(BattlePetIndeterminateReasonLikeCpp::MissingSpellCoverage { spell_id });
        }
        // An incomplete SpellEffect source can hide a SUMMON and therefore
        // prevents a negative classification. The referenced properties and
        // species tables matter only after an effective SUMMON reaches them.
        for table in [SpellAcquisitionTableLikeCpp::SpellEffect] {
            if let Some(table_reasons) = self.global_indeterminate_by_table.get(&table) {
                for reason in table_reasons {
                    let mapped = BattlePetIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table,
                        reason: reason.clone(),
                    };
                    if !reasons.contains(&mapped) {
                        reasons.push(mapped);
                    }
                }
            }
        }

        for effect in self.summon_effects_all_difficulties_like_cpp(spell_id) {
            let difficulty_id = match effect.difficulty_id_checked() {
                Ok(difficulty_id) => difficulty_id,
                Err(error) => {
                    reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    });
                    continue;
                }
            };
            if let Err(error) = effect.effect_index_checked() {
                reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                    record_id: effect.record_id,
                    field: error.field,
                    raw: error.raw,
                });
                continue;
            }
            if !self
                .coverage_by_key
                .contains_key(&(spell_id, difficulty_id))
            {
                reasons.push(
                    BattlePetIndeterminateReasonLikeCpp::MissingSpellDifficultyCoverage {
                        spell_id,
                        difficulty_id,
                        effect_record_id: effect.record_id,
                    },
                );
                continue;
            }

            let properties_id = match source_i32(
                effect.effect_misc_value_raw[1],
                "SpellEffect.EffectMiscValue",
            ) {
                Ok(0) => continue,
                Ok(value) if value > 0 => value as u32,
                Err(error) => {
                    reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    });
                    continue;
                }
                Ok(_) => {
                    reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                        record_id: effect.record_id,
                        field: "SpellEffect.EffectMiscValue",
                        raw: effect.effect_misc_value_raw[1],
                    });
                    continue;
                }
            };
            if let Some(table_reasons) = self
                .global_indeterminate_by_table
                .get(&SpellAcquisitionTableLikeCpp::SummonProperties)
            {
                for reason in table_reasons {
                    let mapped = BattlePetIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table: SpellAcquisitionTableLikeCpp::SummonProperties,
                        reason: reason.clone(),
                    };
                    if !reasons.contains(&mapped) {
                        reasons.push(mapped);
                    }
                }
            }
            let Some(properties) = self.summon_properties_by_id.get(&properties_id) else {
                reasons.push(
                    if self.removed_summon_properties_ids.contains(&properties_id) {
                        BattlePetIndeterminateReasonLikeCpp::RemovedSummonProperties {
                            effect_record_id: effect.record_id,
                            properties_id,
                        }
                    } else {
                        BattlePetIndeterminateReasonLikeCpp::MissingSummonProperties {
                            effect_record_id: effect.record_id,
                            properties_id,
                        }
                    },
                );
                continue;
            };
            let slot = match source_i32(properties.slot_raw, "SummonProperties.Slot") {
                Ok(value) => i64::from(value),
                Err(error) => {
                    reasons.push(
                        BattlePetIndeterminateReasonLikeCpp::InvalidSummonProperties {
                            effect_record_id: effect.record_id,
                            properties_id,
                            field: error.field,
                            raw: error.raw,
                        },
                    );
                    continue;
                }
            };
            let flags = match checked_u32_bits(properties.flags_1_raw, "SummonProperties.Flags1") {
                Ok(value) => value,
                Err(error) => {
                    reasons.push(
                        BattlePetIndeterminateReasonLikeCpp::InvalidSummonProperties {
                            effect_record_id: effect.record_id,
                            properties_id,
                            field: error.field,
                            raw: error.raw,
                        },
                    );
                    continue;
                }
            };
            if slot != SUMMON_SLOT_MINIPET_LIKE_CPP
                || flags & SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP == 0
            {
                continue;
            }

            if let Some(table_reasons) = self
                .global_indeterminate_by_table
                .get(&SpellAcquisitionTableLikeCpp::BattlePetSpecies)
            {
                for reason in table_reasons {
                    let mapped = BattlePetIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table: SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                        reason: reason.clone(),
                    };
                    if !reasons.contains(&mapped) {
                        reasons.push(mapped);
                    }
                }
            }
            let creature_id = match effect.misc_value_id_checked(0) {
                Ok(value) => value,
                Err(error) => {
                    reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    });
                    continue;
                }
            };
            let Some(species) = self.species_by_creature.get(&creature_id) else {
                reasons.push(
                    if let Some(removed_species) =
                        self.removed_species_by_creature.get(&creature_id)
                    {
                        BattlePetIndeterminateReasonLikeCpp::RemovedSpeciesForCreature {
                            effect_record_id: effect.record_id,
                            creature_id,
                            species_ids: removed_species.iter().copied().collect(),
                        }
                    } else {
                        BattlePetIndeterminateReasonLikeCpp::MissingSpeciesForCreature {
                            effect_record_id: effect.record_id,
                            creature_id,
                        }
                    },
                );
                continue;
            };
            if species.len() > 1 {
                reasons.push(
                    BattlePetIndeterminateReasonLikeCpp::ConflictingSpeciesForCreature {
                        effect_record_id: effect.record_id,
                        creature_id,
                        species_ids: species.iter().copied().collect(),
                    },
                );
                continue;
            }
            species_for_spell.extend(species);
        }

        if species_for_spell.len() > 1 {
            reasons.push(
                BattlePetIndeterminateReasonLikeCpp::ConflictingSpeciesForSpell {
                    spell_id,
                    species_ids: species_for_spell.iter().copied().collect(),
                },
            );
        }
        if !reasons.is_empty() {
            return BattlePetClassificationLikeCpp::Indeterminate(reasons);
        }
        match species_for_spell.iter().next().copied() {
            Some(species_id) => BattlePetClassificationLikeCpp::Species(species_id),
            None => BattlePetClassificationLikeCpp::NotBattlePet,
        }
    }
}
