//! C++-shaped spell stores state definitions, part 1 of 4.
//!
//! Separated from the stores.rs root under #646. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone, Default)]
pub struct SpellTargetPositionStoreLikeCpp {
    pub(super) positions: HashMap<(u32, u32), SpellTargetPositionLikeCpp>,
    pub(super) load_report: SpellTargetPositionLoadReportLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpellPetAuraStoreLikeCpp {
    pub auras_by_spell_effect_key: BTreeMap<u32, PetAuraLikeCpp>,
}

impl SpellPetAuraStoreLikeCpp {
    pub const fn key_like_cpp(spell_id: u32, effect_index: u8) -> u32 {
        (spell_id << 8) + effect_index as u32
    }

    pub fn get_pet_aura_like_cpp(
        &self,
        spell_id: u32,
        effect_index: u8,
    ) -> Option<&PetAuraLikeCpp> {
        self.auras_by_spell_effect_key
            .get(&Self::key_like_cpp(spell_id, effect_index))
    }

    pub fn from_rows_and_spell_store_like_cpp<I>(
        rows: I,
        spells: &SpellStore,
    ) -> SpellPetAuraLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellPetAuraRowLikeCpp>,
    {
        Self::load_spell_pet_auras_like_cpp(
            rows,
            |spell_id, effect_index| {
                let Some(spell) = spells.get(spell_id as i32) else {
                    return SpellPetAuraSourceLookupLikeCpp::SpellMissing;
                };
                let Some(effect) = spell
                    .effects()
                    .iter()
                    .find(|effect| effect.effect_index == u32::from(effect_index))
                else {
                    return SpellPetAuraSourceLookupLikeCpp::EffectIndexMissing;
                };
                SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                    effect: effect.effect,
                    apply_aura_name: effect.effect_aura,
                    target_a: effect.implicit_target_1,
                    calc_value: effect.calc_value_no_caster_like_cpp(),
                })
            },
            |aura_id| spells.get(aura_id as i32).is_some(),
        )
    }

    pub fn load_spell_pet_auras_like_cpp<I, SourceEffect, AuraExists>(
        rows: I,
        mut source_effect_lookup: SourceEffect,
        mut aura_spell_exists: AuraExists,
    ) -> SpellPetAuraLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellPetAuraRowLikeCpp>,
        SourceEffect: FnMut(u32, u8) -> SpellPetAuraSourceLookupLikeCpp,
        AuraExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            let key = Self::key_like_cpp(row.spell_id, row.effect_index);
            if let Some(pet_aura) = store.auras_by_spell_effect_key.get_mut(&key) {
                pet_aura.add_aura_like_cpp(row.pet_entry, row.aura_id);
                loaded_row_count += 1;
                continue;
            }

            let source_effect = match source_effect_lookup(row.spell_id, row.effect_index) {
                SpellPetAuraSourceLookupLikeCpp::SpellMissing => {
                    errors.push(SpellPetAuraLoadErrorLikeCpp {
                        row,
                        kind: SpellPetAuraLoadErrorKindLikeCpp::SpellMissing,
                    });
                    continue;
                }
                SpellPetAuraSourceLookupLikeCpp::EffectIndexMissing => {
                    errors.push(SpellPetAuraLoadErrorLikeCpp {
                        row,
                        kind: SpellPetAuraLoadErrorKindLikeCpp::EffectIndexMissing,
                    });
                    continue;
                }
                SpellPetAuraSourceLookupLikeCpp::Found(effect) => effect,
            };

            if !source_effect.is_valid_pet_aura_source_like_cpp() {
                errors.push(SpellPetAuraLoadErrorLikeCpp {
                    row,
                    kind: SpellPetAuraLoadErrorKindLikeCpp::SourceEffectNotDummy,
                });
                continue;
            }

            if !aura_spell_exists(row.aura_id) {
                errors.push(SpellPetAuraLoadErrorLikeCpp {
                    row,
                    kind: SpellPetAuraLoadErrorKindLikeCpp::AuraSpellMissing,
                });
                continue;
            }

            let pet_aura = PetAuraLikeCpp::new(
                row.pet_entry,
                row.aura_id,
                source_effect.target_a == TARGET_UNIT_PET_LIKE_CPP,
                source_effect.calc_value,
            );
            store.auras_by_spell_effect_key.insert(key, pet_aura);
            loaded_row_count += 1;
        }

        SpellPetAuraLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellThreatStoreLikeCpp {
    pub entries_by_spell_id: HashMap<u32, SpellThreatEntryLikeCpp>,
}

impl SpellThreatStoreLikeCpp {
    pub fn from_rows_and_spell_store_like_cpp<I>(
        rows: I,
        spells: &SpellStore,
    ) -> SpellThreatLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellThreatRowLikeCpp>,
    {
        Self::from_rows_like_cpp(rows, |spell_id| spells.get(spell_id as i32).is_some())
    }

    pub fn from_rows_like_cpp<I, SpellExists>(
        rows: I,
        mut spell_exists: SpellExists,
    ) -> SpellThreatLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellThreatRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if !spell_exists(row.spell_id) {
                errors.push(SpellThreatLoadErrorLikeCpp { row });
                continue;
            }

            store.entries_by_spell_id.insert(
                row.spell_id,
                SpellThreatEntryLikeCpp {
                    flat_mod: row.flat_mod,
                    pct_mod: row.pct_mod,
                    ap_pct_mod: row.ap_pct_mod,
                },
            );
            loaded_row_count += 1;
        }

        SpellThreatLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn get_spell_threat_entry_like_cpp<FirstSpellInChain>(
        &self,
        spell_id: u32,
        mut first_spell_in_chain: FirstSpellInChain,
    ) -> Option<&SpellThreatEntryLikeCpp>
    where
        FirstSpellInChain: FnMut(u32) -> u32,
    {
        self.entries_by_spell_id.get(&spell_id).or_else(|| {
            self.entries_by_spell_id
                .get(&first_spell_in_chain(spell_id))
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellLinkedStoreLikeCpp {
    pub effects_by_type_and_trigger: BTreeMap<(SpellLinkedTypeLikeCpp, u32), Vec<i32>>,
}

impl SpellLinkedStoreLikeCpp {
    pub fn from_rows_and_spell_store_like_cpp<I>(
        rows: I,
        spells: &SpellStore,
    ) -> SpellLinkedLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellLinkedRowLikeCpp>,
    {
        Self::from_rows_like_cpp(rows, |spell_id| {
            spells
                .get(spell_id as i32)
                .map(SpellLinkedSpellInfoLikeCpp::from_represented_spell_info_base_points)
        })
    }

    pub fn from_rows_like_cpp<I, SpellLookup>(
        rows: I,
        mut spell_lookup: SpellLookup,
    ) -> SpellLinkedLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellLinkedRowLikeCpp>,
        SpellLookup: FnMut(u32) -> Option<SpellLinkedSpellInfoLikeCpp>,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for row in rows {
            let trigger_spell_id = row.spell_trigger.unsigned_abs();
            let effect_spell_id = row.spell_effect.unsigned_abs();
            let Some(trigger_spell) = spell_lookup(trigger_spell_id) else {
                errors.push(SpellLinkedLoadErrorLikeCpp {
                    row,
                    kind: SpellLinkedLoadErrorKindLikeCpp::TriggerSpellMissing,
                });
                continue;
            };

            if row.spell_effect >= 0 {
                for (effect_index, calc_value) in trigger_spell.effect_calc_values_by_index {
                    if calc_value == row.spell_effect.abs() {
                        warnings.push(SpellLinkedLoadWarningLikeCpp {
                            row: row.clone(),
                            kind: SpellLinkedLoadWarningKindLikeCpp::TriggerEffectSameBasePoint {
                                effect_index,
                            },
                        });
                    }
                }
            }

            if spell_lookup(effect_spell_id).is_none() {
                errors.push(SpellLinkedLoadErrorLikeCpp {
                    row,
                    kind: SpellLinkedLoadErrorKindLikeCpp::EffectSpellMissing,
                });
                continue;
            }

            let Some(mut link_type) = SpellLinkedTypeLikeCpp::from_u8_like_cpp(row.link_type)
            else {
                errors.push(SpellLinkedLoadErrorLikeCpp {
                    row,
                    kind: SpellLinkedLoadErrorKindLikeCpp::InvalidLinkType,
                });
                continue;
            };

            let trigger_key = if row.spell_trigger < 0 {
                if link_type != SpellLinkedTypeLikeCpp::Cast {
                    warnings.push(SpellLinkedLoadWarningLikeCpp {
                        row: row.clone(),
                        kind: SpellLinkedLoadWarningKindLikeCpp::NegativeTriggerLinkTypeCoercedToRemove,
                    });
                }
                link_type = SpellLinkedTypeLikeCpp::Remove;
                trigger_spell_id
            } else {
                row.spell_trigger as u32
            };

            if link_type != SpellLinkedTypeLikeCpp::Aura
                && trigger_key <= i32::MAX as u32
                && trigger_key as i32 == row.spell_effect
            {
                errors.push(SpellLinkedLoadErrorLikeCpp {
                    row,
                    kind: SpellLinkedLoadErrorKindLikeCpp::SelfTriggerLoop,
                });
                continue;
            }

            store
                .effects_by_type_and_trigger
                .entry((link_type, trigger_key))
                .or_default()
                .push(row.spell_effect);
            loaded_row_count += 1;
        }

        SpellLinkedLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
            warnings,
        }
    }

    pub fn get_spell_linked_like_cpp(
        &self,
        link_type: SpellLinkedTypeLikeCpp,
        spell_id: u32,
    ) -> Option<&[i32]> {
        self.effects_by_type_and_trigger
            .get(&(link_type, spell_id))
            .map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellTotemModelStoreLikeCpp {
    pub display_id_by_spell_and_race: BTreeMap<(u32, u8), u32>,
}

impl SpellTotemModelStoreLikeCpp {
    pub fn from_rows_and_stores_like_cpp<I, SpellExists, RaceExists, DisplayExists>(
        rows: I,
        spell_exists: SpellExists,
        race_exists: RaceExists,
        display_exists: DisplayExists,
    ) -> SpellTotemModelLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellTotemModelRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        RaceExists: FnMut(u8) -> bool,
        DisplayExists: FnMut(u32) -> bool,
    {
        Self::from_rows_like_cpp(rows, spell_exists, race_exists, display_exists)
    }

    pub fn from_rows_like_cpp<I, SpellExists, RaceExists, DisplayExists>(
        rows: I,
        mut spell_exists: SpellExists,
        mut race_exists: RaceExists,
        mut display_exists: DisplayExists,
    ) -> SpellTotemModelLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellTotemModelRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        RaceExists: FnMut(u8) -> bool,
        DisplayExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if !spell_exists(row.spell_id) {
                errors.push(SpellTotemModelLoadErrorLikeCpp {
                    row,
                    kind: SpellTotemModelLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            if !race_exists(row.race_id) {
                errors.push(SpellTotemModelLoadErrorLikeCpp {
                    row,
                    kind: SpellTotemModelLoadErrorKindLikeCpp::RaceMissing,
                });
                continue;
            }

            if !display_exists(row.display_id) {
                errors.push(SpellTotemModelLoadErrorLikeCpp {
                    row,
                    kind: SpellTotemModelLoadErrorKindLikeCpp::DisplayMissing,
                });
                continue;
            }

            store
                .display_id_by_spell_and_race
                .insert((row.spell_id, row.race_id), row.display_id);
            loaded_row_count += 1;
        }

        SpellTotemModelLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn get_model_for_totem_like_cpp(&self, spell_id: u32, race_id: u8) -> u32 {
        self.display_id_by_spell_and_race
            .get(&(spell_id, race_id))
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellRequiredStoreLikeCpp {
    pub required_by_spell_id: BTreeMap<u32, Vec<u32>>,
    pub requiring_by_required_spell_id: BTreeMap<u32, Vec<u32>>,
}

impl SpellRequiredStoreLikeCpp {
    pub fn from_rows_and_stores_like_cpp<I>(
        rows: I,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> SpellRequiredLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellRequiredRowLikeCpp>,
    {
        Self::from_rows_like_cpp(
            rows,
            |spell_id| spells.get(spell_id as i32).is_some(),
            |spell_id, req_spell| spell_chains.is_rank_of_like_cpp(spell_id, req_spell),
        )
    }

    pub fn from_rows_like_cpp<I, SpellExists, SameRankChain>(
        rows: I,
        mut spell_exists: SpellExists,
        mut same_rank_chain: SameRankChain,
    ) -> SpellRequiredLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellRequiredRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        SameRankChain: FnMut(u32, u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if !spell_exists(row.spell_id) {
                errors.push(SpellRequiredLoadErrorLikeCpp {
                    row,
                    kind: SpellRequiredLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            if !spell_exists(row.req_spell) {
                errors.push(SpellRequiredLoadErrorLikeCpp {
                    row,
                    kind: SpellRequiredLoadErrorKindLikeCpp::RequiredSpellMissing,
                });
                continue;
            }

            if same_rank_chain(row.spell_id, row.req_spell) {
                errors.push(SpellRequiredLoadErrorLikeCpp {
                    row,
                    kind: SpellRequiredLoadErrorKindLikeCpp::SameRankChain,
                });
                continue;
            }

            if store.is_spell_requiring_spell_like_cpp(row.spell_id, row.req_spell) {
                errors.push(SpellRequiredLoadErrorLikeCpp {
                    row,
                    kind: SpellRequiredLoadErrorKindLikeCpp::Duplicate,
                });
                continue;
            }

            store
                .required_by_spell_id
                .entry(row.spell_id)
                .or_default()
                .push(row.req_spell);
            store
                .requiring_by_required_spell_id
                .entry(row.req_spell)
                .or_default()
                .push(row.spell_id);
            loaded_row_count += 1;
        }

        SpellRequiredLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn spells_required_for_spell_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.required_by_spell_id
            .get(&spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn spells_requiring_spell_like_cpp(&self, req_spell: u32) -> &[u32] {
        self.requiring_by_required_spell_id
            .get(&req_spell)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn is_spell_requiring_spell_like_cpp(&self, spell_id: u32, req_spell: u32) -> bool {
        self.spells_requiring_spell_like_cpp(req_spell)
            .contains(&spell_id)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellChainStoreLikeCpp {
    pub chains_by_spell_id: BTreeMap<u32, SpellChainNodeLikeCpp>,
    pub(crate) indeterminate_by_spell_id_like_cpp:
        BTreeMap<u32, std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]>>,
    pub(crate) global_indeterminate_like_cpp:
        Option<std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]>>,
}
