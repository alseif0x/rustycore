// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell DB2 stores and their loaders.

use super::catalog::{SpellHitEffectMechanicRowLikeCpp, SpellInterruptRowLikeCpp};
// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{HotfixDatabase, WorldDatabase, WorldStatements};

use super::*;

#[derive(Debug, Clone, Default)]
pub struct SpellTargetPositionStoreLikeCpp {
    positions: HashMap<(u32, u32), SpellTargetPositionLikeCpp>,
    load_report: SpellTargetPositionLoadReportLikeCpp,
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

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
    ) -> Result<SpellPetAuraLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_PET_AURAS);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellPetAuraRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    effect_index: result.try_read::<u8>(1).unwrap_or(0),
                    pet_entry: result.try_read::<u32>(2).unwrap_or(0),
                    aura_id: result.try_read::<u32>(3).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::load_spell_pet_auras_like_cpp(
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
        ))
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
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
    ) -> Result<SpellThreatLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_THREATS);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellThreatRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    flat_mod: result.try_read::<i32>(1).unwrap_or(0),
                    pct_mod: result.try_read::<f32>(2).unwrap_or(0.0),
                    ap_pct_mod: result.try_read::<f32>(3).unwrap_or(0.0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows, |spell_id| {
            spells.get(spell_id as i32).is_some()
        }))
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
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
    ) -> Result<SpellLinkedLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_LINKED);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellLinkedRowLikeCpp {
                    spell_trigger: result.try_read::<i32>(0).unwrap_or(0),
                    spell_effect: result.try_read::<i32>(1).unwrap_or(0),
                    link_type: result.try_read::<u8>(2).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows, |spell_id| {
            spells
                .get(spell_id as i32)
                .map(SpellLinkedSpellInfoLikeCpp::from_represented_spell_info_base_points)
        }))
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
    pub async fn load_like_cpp<SpellExists, RaceExists, DisplayExists>(
        db: &WorldDatabase,
        spell_exists: SpellExists,
        race_exists: RaceExists,
        display_exists: DisplayExists,
    ) -> Result<SpellTotemModelLoadOutcomeLikeCpp>
    where
        SpellExists: FnMut(u32) -> bool,
        RaceExists: FnMut(u8) -> bool,
        DisplayExists: FnMut(u32) -> bool,
    {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_TOTEM_MODEL);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellTotemModelRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    race_id: result.try_read::<u8>(1).unwrap_or(0),
                    display_id: result.try_read::<u32>(2).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            spell_exists,
            race_exists,
            display_exists,
        ))
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
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> Result<SpellRequiredLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_REQUIRED);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellRequiredRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    req_spell: result.try_read::<u32>(1).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            |spell_id| spells.get(spell_id as i32).is_some(),
            |spell_id, req_spell| spell_chains.is_rank_of_like_cpp(spell_id, req_spell),
        ))
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

impl SpellChainStoreLikeCpp {
    pub fn from_skill_line_ability_supercedes_like_cpp<I, SpellExists>(
        rows: I,
        spell_exists: SpellExists,
    ) -> Self
    where
        I: IntoIterator<Item = SpellRankEdgeLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        Self::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(rows, spell_exists).store
    }

    /// Build ranks from the final rank-specific raw authority. Valid
    /// endpoints remain usable even when unrelated `SkillLineAbility` fields
    /// failed hydration; invalid endpoints become explicit component/global
    /// indeterminacy.
    pub fn from_skill_line_ability_rank_rows_with_diagnostics_like_cpp<I, SpellExists>(
        rows: I,
        mut spell_exists: SpellExists,
    ) -> SpellChainLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SkillLineAbilityRankRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        struct PendingIndeterminateRankRowLikeCpp {
            source_order: usize,
            record_id: u32,
            spell_raw: i128,
            supercedes_spell_raw: i128,
            affected_spell_ids: Vec<u32>,
        }

        enum EffectiveRankCandidateLikeCpp {
            Edge(SpellRankEdgeLikeCpp),
            Indeterminate(PendingIndeterminateRankRowLikeCpp),
        }

        let mut existence_by_spell_id = BTreeMap::new();
        let mut candidate_by_predecessor = BTreeMap::new();
        let mut unkeyed_indeterminate_rows = Vec::new();
        let mut malformed_source_diagnostics = Vec::new();

        for (source_order, row) in rows.into_iter().enumerate() {
            match row {
                SkillLineAbilityRankRowLikeCpp::Edge {
                    spell_id,
                    supercedes_spell_id,
                    ..
                } => {
                    if supercedes_spell_id == 0 {
                        continue;
                    }
                    let has_spell = *existence_by_spell_id
                        .entry(spell_id)
                        .or_insert_with(|| spell_exists(spell_id));
                    let has_supercedes = *existence_by_spell_id
                        .entry(supercedes_spell_id)
                        .or_insert_with(|| spell_exists(supercedes_spell_id));
                    if has_spell && has_supercedes {
                        candidate_by_predecessor.insert(
                            supercedes_spell_id,
                            EffectiveRankCandidateLikeCpp::Edge(SpellRankEdgeLikeCpp {
                                spell_id,
                                supercedes_spell_id,
                            }),
                        );
                    }
                }
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id,
                    spell_raw,
                    supercedes_spell_raw,
                } => {
                    let spell_id = spell_rank_endpoint_id_from_raw_like_cpp(spell_raw);
                    let supercedes_spell_id =
                        spell_rank_endpoint_id_from_raw_like_cpp(supercedes_spell_raw);
                    if supercedes_spell_id == Some(0) {
                        continue;
                    }

                    let mut affected_spell_ids = [spell_id, supercedes_spell_id]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    affected_spell_ids.sort_unstable();
                    affected_spell_ids.dedup();

                    // C++ skips a row unless both endpoint lookups succeed. If
                    // any representable endpoint is proven absent, an
                    // unrepresentable endpoint cannot make the row relevant.
                    let mut every_representable_endpoint_exists = true;
                    for affected_spell_id in &affected_spell_ids {
                        let exists = *existence_by_spell_id
                            .entry(*affected_spell_id)
                            .or_insert_with(|| spell_exists(*affected_spell_id));
                        every_representable_endpoint_exists &= exists;
                    }
                    if !affected_spell_ids.is_empty() && !every_representable_endpoint_exists {
                        continue;
                    }

                    // Normalize a manually constructed but fully
                    // representable variant instead of letting it bypass the
                    // same predecessor authority as `Edge`.
                    if let (Some(spell_id), Some(supercedes_spell_id)) =
                        (spell_id, supercedes_spell_id)
                    {
                        candidate_by_predecessor.insert(
                            supercedes_spell_id,
                            EffectiveRankCandidateLikeCpp::Edge(SpellRankEdgeLikeCpp {
                                spell_id,
                                supercedes_spell_id,
                            }),
                        );
                        continue;
                    }

                    let diagnostic =
                        SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                            record_id,
                            spell_raw,
                            supercedes_spell_raw,
                            affected_spell_ids: affected_spell_ids.clone(),
                        };
                    if !malformed_source_diagnostics.contains(&diagnostic) {
                        malformed_source_diagnostics.push(diagnostic);
                    }
                    let pending = PendingIndeterminateRankRowLikeCpp {
                        source_order,
                        record_id,
                        spell_raw,
                        supercedes_spell_raw,
                        affected_spell_ids,
                    };

                    if let Some(supercedes_spell_id) = supercedes_spell_id {
                        // This candidate participates in the exact same
                        // last-wins predecessor authority as a valid edge. A
                        // later valid row can repair it; a later ambiguous row
                        // can eclipse an earlier valid edge.
                        candidate_by_predecessor.insert(
                            supercedes_spell_id,
                            EffectiveRankCandidateLikeCpp::Indeterminate(pending),
                        );
                    } else {
                        unkeyed_indeterminate_rows.push(pending);
                    }
                }
            }
        }

        let mut filtered_edges = Vec::new();
        let mut indeterminate_rows = unkeyed_indeterminate_rows;
        for candidate in candidate_by_predecessor.into_values() {
            match candidate {
                EffectiveRankCandidateLikeCpp::Edge(edge) => filtered_edges.push(edge),
                EffectiveRankCandidateLikeCpp::Indeterminate(row) => {
                    indeterminate_rows.push(row);
                }
            }
        }
        indeterminate_rows.sort_by_key(|row| row.source_order);

        let mut outcome = Self::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            filtered_edges,
            |_| true,
        );
        let graph_diagnostics = std::mem::take(&mut outcome.diagnostics_in_order_like_cpp);
        outcome.diagnostics_in_order_like_cpp = malformed_source_diagnostics;
        for diagnostic in graph_diagnostics {
            if !outcome.diagnostics_in_order_like_cpp.contains(&diagnostic) {
                outcome.diagnostics_in_order_like_cpp.push(diagnostic);
            }
        }

        for row in indeterminate_rows {
            outcome.mark_invalid_skill_line_ability_rank_row_like_cpp(
                row.record_id,
                row.spell_raw,
                row.supercedes_spell_raw,
                &row.affected_spell_ids,
            );
        }
        outcome
    }

    /// Builds the effective `SpellMgr::LoadSpellRanks` projection and retains
    /// malformed custom/hotfix graph evidence instead of inheriting C++'s
    /// startup hang or silently treating an ambiguous rank as unranked.
    ///
    /// Input order is significant for the C++ `std::map::operator[]`
    /// last-wins rule when multiple records name the same predecessor. The
    /// production caller supplies final `SkillLineAbility` rows in ascending
    /// RecordID order.
    pub fn from_skill_line_ability_supercedes_with_diagnostics_like_cpp<I, SpellExists>(
        rows: I,
        mut spell_exists: SpellExists,
    ) -> SpellChainLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellRankEdgeLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut chain_next_by_spell_id = BTreeMap::new();

        for row in rows {
            if row.supercedes_spell_id == 0 {
                continue;
            }

            if !spell_exists(row.supercedes_spell_id) || !spell_exists(row.spell_id) {
                continue;
            }

            chain_next_by_spell_id.insert(row.supercedes_spell_id, row.spell_id);
        }

        let mut store = Self::default();
        let mut diagnostics_in_order_like_cpp = Vec::new();
        let mut parents_by_spell_id = BTreeMap::<u32, BTreeSet<u32>>::new();
        let mut adjacent_by_spell_id = BTreeMap::<u32, BTreeSet<u32>>::new();
        for (&spell_id, &next_spell_id) in &chain_next_by_spell_id {
            parents_by_spell_id
                .entry(next_spell_id)
                .or_default()
                .insert(spell_id);
            adjacent_by_spell_id
                .entry(spell_id)
                .or_default()
                .insert(next_spell_id);
            adjacent_by_spell_id
                .entry(next_spell_id)
                .or_default()
                .insert(spell_id);
        }

        let mut unvisited = adjacent_by_spell_id
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        while let Some(component_start) = unvisited.first().copied() {
            let mut pending = vec![component_start];
            let mut component = BTreeSet::new();
            while let Some(spell_id) = pending.pop() {
                if !component.insert(spell_id) {
                    continue;
                }
                unvisited.remove(&spell_id);
                if let Some(adjacent) = adjacent_by_spell_id.get(&spell_id) {
                    pending.extend(
                        adjacent
                            .iter()
                            .rev()
                            .filter(|adjacent_spell_id| !component.contains(adjacent_spell_id))
                            .copied(),
                    );
                }
            }

            let component_spell_ids = component.iter().copied().collect::<Vec<_>>();
            let mut component_diagnostics = Vec::new();
            for &spell_id in &component_spell_ids {
                if chain_next_by_spell_id.get(&spell_id) == Some(&spell_id) {
                    component_diagnostics
                        .push(SpellChainLoadDiagnosticLikeCpp::SelfLoop { spell_id });
                }

                if let Some(predecessors) = parents_by_spell_id.get(&spell_id)
                    && predecessors.len() > 1
                {
                    component_diagnostics.push(
                        SpellChainLoadDiagnosticLikeCpp::MultiplePredecessors {
                            spell_id,
                            predecessor_spell_ids: predecessors.iter().copied().collect(),
                        },
                    );
                }
            }

            for spell_ids in
                spell_chain_cycles_like_cpp(&component_spell_ids, &chain_next_by_spell_id)
            {
                if spell_ids.len() > 1 {
                    component_diagnostics
                        .push(SpellChainLoadDiagnosticLikeCpp::Cycle { spell_ids });
                }
            }

            let roots = component_spell_ids
                .iter()
                .copied()
                .filter(|spell_id| !parents_by_spell_id.contains_key(spell_id))
                .collect::<Vec<_>>();
            let mut ordered_chain = Vec::new();
            if component_diagnostics.is_empty() && roots.len() == 1 {
                let mut current_spell_id = Some(roots[0]);
                let mut seen = BTreeSet::new();
                while let Some(spell_id) = current_spell_id {
                    if !seen.insert(spell_id) {
                        break;
                    }
                    ordered_chain.push(spell_id);
                    current_spell_id = chain_next_by_spell_id.get(&spell_id).copied();
                }

                if let Some(&spell_id) = ordered_chain.get(usize::from(u8::MAX)) {
                    component_diagnostics.push(SpellChainLoadDiagnosticLikeCpp::RankOutOfRange {
                        first_spell_id: roots[0],
                        spell_id,
                        rank: usize::from(u8::MAX) + 1,
                    });
                }
            }

            if !component_diagnostics.is_empty() {
                let shared_diagnostics: std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]> =
                    component_diagnostics.clone().into();
                for spell_id in component_spell_ids {
                    store
                        .indeterminate_by_spell_id_like_cpp
                        .insert(spell_id, shared_diagnostics.clone());
                }
                diagnostics_in_order_like_cpp.extend(component_diagnostics);
                continue;
            }

            // A weakly connected functional component with no cycle and no
            // merge has exactly one root and one path covering every node.
            // Keep a defensive fail-closed guard in case that invariant is
            // changed by a future graph representation.
            if roots.len() != 1 || ordered_chain.len() != component_spell_ids.len() {
                let diagnostic = SpellChainLoadDiagnosticLikeCpp::Cycle {
                    spell_ids: component_spell_ids.clone(),
                };
                let shared_diagnostics: std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]> =
                    vec![diagnostic.clone()].into();
                for spell_id in component_spell_ids {
                    store
                        .indeterminate_by_spell_id_like_cpp
                        .insert(spell_id, shared_diagnostics.clone());
                }
                diagnostics_in_order_like_cpp.push(diagnostic);
                continue;
            }

            let first_spell_id = ordered_chain[0];
            let last_spell_id = *ordered_chain.last().expect("non-empty rank chain");
            for (index, &spell_id) in ordered_chain.iter().enumerate() {
                let rank = u8::try_from(index + 1).expect("rank overflow diagnosed above");
                store.chains_by_spell_id.insert(
                    spell_id,
                    SpellChainNodeLikeCpp {
                        prev_spell_id: index.checked_sub(1).map(|previous| ordered_chain[previous]),
                        next_spell_id: ordered_chain.get(index + 1).copied(),
                        first_spell_id,
                        last_spell_id,
                        rank,
                    },
                );
            }
        }

        SpellChainLoadOutcomeLikeCpp {
            store,
            diagnostics_in_order_like_cpp,
        }
    }

    pub fn spell_chain_lookup_like_cpp(&self, spell_id: u32) -> SpellChainLookupLikeCpp<'_> {
        if let Some(diagnostics) = &self.global_indeterminate_like_cpp {
            return SpellChainLookupLikeCpp::Indeterminate(diagnostics);
        }
        if let Some(diagnostics) = self.indeterminate_by_spell_id_like_cpp.get(&spell_id) {
            return SpellChainLookupLikeCpp::Indeterminate(diagnostics);
        }

        self.chains_by_spell_id
            .get(&spell_id)
            .map(SpellChainLookupLikeCpp::Node)
            .unwrap_or(SpellChainLookupLikeCpp::Unranked)
    }

    pub fn indeterminate_diagnostics_for_spell_like_cpp(
        &self,
        spell_id: u32,
    ) -> Option<&[SpellChainLoadDiagnosticLikeCpp]> {
        if let Some(diagnostics) = &self.global_indeterminate_like_cpp {
            return Some(diagnostics);
        }
        self.indeterminate_by_spell_id_like_cpp
            .get(&spell_id)
            .map(AsRef::as_ref)
    }

    pub fn spell_chain_node_like_cpp(&self, spell_id: u32) -> Option<&SpellChainNodeLikeCpp> {
        self.chains_by_spell_id.get(&spell_id)
    }

    pub fn first_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_chain_node_like_cpp(spell_id)
            .map(|node| node.first_spell_id)
            .unwrap_or(spell_id)
    }

    pub fn is_rank_of_like_cpp(&self, spell_id: u32, other_spell_id: u32) -> bool {
        self.first_spell_in_chain_like_cpp(spell_id)
            == self.first_spell_in_chain_like_cpp(other_spell_id)
    }

    pub fn last_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_chain_node_like_cpp(spell_id)
            .map(|node| node.last_spell_id)
            .unwrap_or(spell_id)
    }

    pub fn next_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_chain_node_like_cpp(spell_id)
            .and_then(|node| node.next_spell_id)
            .unwrap_or(0)
    }

    pub fn prev_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_chain_node_like_cpp(spell_id)
            .and_then(|node| node.prev_spell_id)
            .unwrap_or(0)
    }

    pub fn spell_rank_like_cpp(&self, spell_id: u32) -> u8 {
        self.spell_chain_node_like_cpp(spell_id)
            .map(|node| node.rank)
            .unwrap_or(0)
    }

    pub fn spell_with_rank_like_cpp(&self, spell_id: u32, rank: u32, strict: bool) -> u32 {
        let mut current_spell_id = spell_id;
        let mut seen = BTreeSet::new();

        loop {
            let Some(node) = self.spell_chain_node_like_cpp(current_spell_id) else {
                return if strict && rank > 1 {
                    0
                } else {
                    current_spell_id
                };
            };

            if u32::from(node.rank) == rank {
                return current_spell_id;
            }

            let next = if u32::from(node.rank) < rank {
                node.next_spell_id
            } else {
                node.prev_spell_id
            };

            let Some(next_spell_id) = next else {
                return if strict { 0 } else { current_spell_id };
            };

            if !seen.insert(current_spell_id) {
                return if strict { 0 } else { current_spell_id };
            }

            current_spell_id = next_spell_id;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellAreaStoreLikeCpp {
    areas: Vec<SpellAreaLikeCpp>,
    area_indices_by_spell_id: BTreeMap<u32, Vec<usize>>,
    area_indices_by_quest_start_or_end: BTreeMap<u32, Vec<usize>>,
    area_indices_by_quest_end: BTreeMap<u32, Vec<usize>>,
    area_indices_by_aura_spell: BTreeMap<u32, Vec<usize>>,
    area_indices_by_area_id: BTreeMap<u32, Vec<usize>>,
}

impl SpellAreaStoreLikeCpp {
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spell_exists: impl FnMut(u32) -> bool,
        area_exists: impl FnMut(u32) -> bool,
        quest_exists: impl FnMut(u32) -> bool,
    ) -> Result<SpellAreaLoadOutcomeLikeCpp> {
        let mut result = db
            .direct_query(WorldStatements::SEL_SPELL_AREA.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellAreaRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    area_id: result.try_read::<u32>(1).unwrap_or(0),
                    quest_start: result.try_read::<u32>(2).unwrap_or(0),
                    quest_start_status: result.try_read::<u32>(3).unwrap_or(0),
                    quest_end_status: result.try_read::<u32>(4).unwrap_or(0),
                    quest_end: result.try_read::<u32>(5).unwrap_or(0),
                    aura_spell: result.try_read::<i32>(6).unwrap_or(0),
                    race_mask: result.try_read::<u64>(7).unwrap_or(0),
                    gender: result.try_read::<u8>(8).unwrap_or(GENDER_NONE_LIKE_CPP),
                    flags: result.try_read::<u8>(9).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            spell_exists,
            area_exists,
            quest_exists,
        ))
    }

    pub fn from_rows_like_cpp<I, SpellExists, AreaExists, QuestExists>(
        rows: I,
        mut spell_exists: SpellExists,
        mut area_exists: AreaExists,
        mut quest_exists: QuestExists,
    ) -> SpellAreaLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellAreaRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        AreaExists: FnMut(u32) -> bool,
        QuestExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut errors = Vec::new();

        for row in rows {
            let spell_area = SpellAreaLikeCpp::from(row);

            if !spell_exists(spell_area.spell_id) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            if store.has_similar_requirements_like_cpp(&spell_area) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::DuplicateSimilarRequirements,
                });
                continue;
            }

            if spell_area.area_id != 0 && !area_exists(spell_area.area_id) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::AreaMissing,
                });
                continue;
            }

            if spell_area.quest_start != 0 && !quest_exists(spell_area.quest_start) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::QuestStartMissing,
                });
                continue;
            }

            if spell_area.quest_end != 0 && !quest_exists(spell_area.quest_end) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::QuestEndMissing,
                });
                continue;
            }

            if spell_area.aura_spell != 0 {
                let aura_spell_id = spell_area.aura_spell.unsigned_abs();
                if !spell_exists(aura_spell_id) {
                    errors.push(SpellAreaLoadErrorLikeCpp {
                        row,
                        kind: SpellAreaLoadErrorKindLikeCpp::AuraSpellMissing,
                    });
                    continue;
                }

                if aura_spell_id == spell_area.spell_id {
                    errors.push(SpellAreaLoadErrorLikeCpp {
                        row,
                        kind: SpellAreaLoadErrorKindLikeCpp::AuraSpellSelfRequirement,
                    });
                    continue;
                }

                if spell_area.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP != 0
                    && spell_area.aura_spell > 0
                    && store.has_autocast_aura_chain_like_cpp(&spell_area)
                {
                    errors.push(SpellAreaLoadErrorLikeCpp {
                        row,
                        kind: SpellAreaLoadErrorKindLikeCpp::AuraAutocastChain,
                    });
                    continue;
                }
            }

            if spell_area.race_mask != 0
                && (spell_area.race_mask & RACEMASK_ALL_PLAYABLE_LIKE_CPP) == 0
            {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::InvalidRaceMask,
                });
                continue;
            }

            if !matches!(
                spell_area.gender,
                GENDER_NONE_LIKE_CPP | GENDER_FEMALE_LIKE_CPP | GENDER_MALE_LIKE_CPP
            ) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::InvalidGender,
                });
                continue;
            }

            store.insert_like_cpp(spell_area);
        }

        SpellAreaLoadOutcomeLikeCpp {
            loaded_row_count: store.areas.len(),
            store,
            errors,
        }
    }

    pub fn spell_area_map_bounds_like_cpp(&self, spell_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_spell_id, spell_id)
    }

    pub fn spell_area_for_quest_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_quest_start_or_end, quest_id)
    }

    pub fn spell_area_for_quest_end_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_quest_end, quest_id)
    }

    pub fn spell_area_for_aura_map_bounds_like_cpp(&self, spell_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_aura_spell, spell_id)
    }

    pub fn spell_area_for_area_map_bounds_like_cpp(&self, area_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_area_id, area_id)
    }

    pub fn areas_like_cpp(&self) -> &[SpellAreaLikeCpp] {
        &self.areas
    }

    fn lookup_indices_like_cpp(
        &self,
        index: &BTreeMap<u32, Vec<usize>>,
        key: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        index
            .get(&key)
            .into_iter()
            .flat_map(|indices| indices.iter())
            .filter_map(|idx| self.areas.get(*idx))
            .collect()
    }

    fn has_similar_requirements_like_cpp(&self, spell_area: &SpellAreaLikeCpp) -> bool {
        self.spell_area_map_bounds_like_cpp(spell_area.spell_id)
            .into_iter()
            .any(|existing| {
                spell_area.spell_id == existing.spell_id
                    && spell_area.area_id == existing.area_id
                    && spell_area.quest_start == existing.quest_start
                    && spell_area.aura_spell == existing.aura_spell
                    && (spell_area.race_mask & existing.race_mask) != 0
                    && spell_area.gender == existing.gender
            })
    }

    fn has_autocast_aura_chain_like_cpp(&self, spell_area: &SpellAreaLikeCpp) -> bool {
        self.spell_area_for_aura_map_bounds_like_cpp(spell_area.spell_id)
            .into_iter()
            .any(|existing| {
                existing.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP != 0 && existing.aura_spell > 0
            })
            || self
                .spell_area_map_bounds_like_cpp(spell_area.aura_spell as u32)
                .into_iter()
                .any(|existing| {
                    existing.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP != 0
                        && existing.aura_spell > 0
                })
    }

    fn insert_like_cpp(&mut self, spell_area: SpellAreaLikeCpp) {
        let idx = self.areas.len();
        self.areas.push(spell_area);
        self.area_indices_by_spell_id
            .entry(spell_area.spell_id)
            .or_default()
            .push(idx);

        if spell_area.area_id != 0 {
            self.area_indices_by_area_id
                .entry(spell_area.area_id)
                .or_default()
                .push(idx);
        }

        if spell_area.quest_start != 0 || spell_area.quest_end != 0 {
            if spell_area.quest_start == spell_area.quest_end {
                self.area_indices_by_quest_start_or_end
                    .entry(spell_area.quest_start)
                    .or_default()
                    .push(idx);
            } else {
                if spell_area.quest_start != 0 {
                    self.area_indices_by_quest_start_or_end
                        .entry(spell_area.quest_start)
                        .or_default()
                        .push(idx);
                }
                if spell_area.quest_end != 0 {
                    self.area_indices_by_quest_start_or_end
                        .entry(spell_area.quest_end)
                        .or_default()
                        .push(idx);
                }
            }
        }

        if spell_area.quest_end != 0 {
            self.area_indices_by_quest_end
                .entry(spell_area.quest_end)
                .or_default()
                .push(idx);
        }

        if spell_area.aura_spell != 0 {
            self.area_indices_by_aura_spell
                .entry(spell_area.aura_spell.unsigned_abs())
                .or_default()
                .push(idx);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellGroupStoreLikeCpp {
    pub spell_entries_by_group_id: BTreeMap<u32, Vec<i32>>,
    pub group_ids_by_spell_id: BTreeMap<u32, Vec<u32>>,
}

impl SpellGroupStoreLikeCpp {
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> Result<SpellGroupLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_GROUP);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellGroupRowLikeCpp {
                    group_id: result.try_read::<u32>(0).unwrap_or(0),
                    spell_id: result.try_read::<i32>(1).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            |spell_id| spells.get(spell_id as i32).is_some(),
            |spell_id| u32::from(spell_chains.spell_rank_like_cpp(spell_id)),
        ))
    }

    pub fn from_rows_like_cpp<I, SpellExists, SpellRank>(
        rows: I,
        mut spell_exists: SpellExists,
        mut spell_rank: SpellRank,
    ) -> SpellGroupLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellGroupRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        SpellRank: FnMut(u32) -> u32,
    {
        let mut store = Self::default();
        let mut group_ids = BTreeSet::new();
        let mut errors = Vec::new();

        for row in rows {
            if row.group_id <= SPELL_GROUP_DB_RANGE_MIN_LIKE_CPP
                && row.group_id >= SPELL_GROUP_CORE_RANGE_MAX_LIKE_CPP
            {
                errors.push(SpellGroupLoadErrorLikeCpp {
                    row,
                    kind: SpellGroupLoadErrorKindLikeCpp::CoreRangeGroupMissing,
                });
                continue;
            }

            group_ids.insert(row.group_id);
            store
                .spell_entries_by_group_id
                .entry(row.group_id)
                .or_default()
                .push(row.spell_id);
        }

        for (group_id, entries) in store.spell_entries_by_group_id.clone() {
            let mut retained_entries = Vec::new();

            for spell_id in entries {
                let row = SpellGroupRowLikeCpp { group_id, spell_id };
                if spell_id < 0 {
                    if !group_ids.contains(&spell_id.unsigned_abs()) {
                        errors.push(SpellGroupLoadErrorLikeCpp {
                            row,
                            kind: SpellGroupLoadErrorKindLikeCpp::ReferencedGroupMissing,
                        });
                        continue;
                    }
                } else {
                    let spell_id_u32 = spell_id as u32;
                    if !spell_exists(spell_id_u32) {
                        errors.push(SpellGroupLoadErrorLikeCpp {
                            row,
                            kind: SpellGroupLoadErrorKindLikeCpp::SpellMissing,
                        });
                        continue;
                    }

                    if spell_rank(spell_id_u32) > 1 {
                        errors.push(SpellGroupLoadErrorLikeCpp {
                            row,
                            kind: SpellGroupLoadErrorKindLikeCpp::SpellNotFirstRank,
                        });
                        continue;
                    }
                }

                retained_entries.push(spell_id);
            }

            if retained_entries.is_empty() {
                store.spell_entries_by_group_id.remove(&group_id);
            } else {
                store
                    .spell_entries_by_group_id
                    .insert(group_id, retained_entries);
            }
        }

        let mut loaded_row_count = 0;
        for group_id in group_ids {
            let spells = store.set_of_spells_in_spell_group_like_cpp(group_id);
            for spell_id in spells {
                store
                    .group_ids_by_spell_id
                    .entry(spell_id)
                    .or_default()
                    .push(group_id);
                loaded_row_count += 1;
            }
        }

        SpellGroupLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn spell_group_spell_map_bounds_like_cpp(&self, group_id: u32) -> &[i32] {
        self.spell_entries_by_group_id
            .get(&group_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn spell_spell_group_map_bounds_like_cpp<FirstSpellInChain>(
        &self,
        spell_id: u32,
        mut first_spell_in_chain: FirstSpellInChain,
    ) -> &[u32]
    where
        FirstSpellInChain: FnMut(u32) -> u32,
    {
        let first_spell_id = first_spell_in_chain(spell_id);
        self.group_ids_by_spell_id
            .get(&first_spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn is_spell_member_of_spell_group_like_cpp<FirstSpellInChain>(
        &self,
        spell_id: u32,
        group_id: u32,
        first_spell_in_chain: FirstSpellInChain,
    ) -> bool
    where
        FirstSpellInChain: FnMut(u32) -> u32,
    {
        self.spell_spell_group_map_bounds_like_cpp(spell_id, first_spell_in_chain)
            .contains(&group_id)
    }

    pub fn set_of_spells_in_spell_group_like_cpp(&self, group_id: u32) -> BTreeSet<u32> {
        let mut found_spells = BTreeSet::new();
        let mut used_groups = BTreeSet::new();
        self.collect_spells_in_group_like_cpp(group_id, &mut found_spells, &mut used_groups);
        found_spells
    }

    fn collect_spells_in_group_like_cpp(
        &self,
        group_id: u32,
        found_spells: &mut BTreeSet<u32>,
        used_groups: &mut BTreeSet<u32>,
    ) {
        if !used_groups.insert(group_id) {
            return;
        }

        for spell_id in self.spell_group_spell_map_bounds_like_cpp(group_id) {
            if *spell_id < 0 {
                self.collect_spells_in_group_like_cpp(
                    spell_id.unsigned_abs(),
                    found_spells,
                    used_groups,
                );
            } else {
                found_spells.insert(*spell_id as u32);
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellGroupStackRuleStoreLikeCpp {
    pub stack_rule_by_group_id: BTreeMap<u32, SpellGroupStackRuleLikeCpp>,
    pub same_effect_stack_by_group_id: BTreeMap<u32, BTreeSet<i32>>,
}

impl SpellGroupStackRuleStoreLikeCpp {
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spell_groups: &SpellGroupStoreLikeCpp,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> Result<SpellGroupStackRuleLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_GROUP_STACK_RULES);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellGroupStackRuleRowLikeCpp {
                    group_id: result.try_read::<u32>(0).unwrap_or(0),
                    stack_rule: result.try_read::<u8>(1).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            spell_groups,
            |spell_id| spells.get(spell_id as i32).cloned(),
            |spell_id| {
                let next_spell_id = spell_chains.next_spell_in_chain_like_cpp(spell_id);
                (next_spell_id != 0).then_some(next_spell_id)
            },
        ))
    }

    pub fn from_rows_like_cpp<I, SpellInfoById, NextRankSpell>(
        rows: I,
        spell_groups: &SpellGroupStoreLikeCpp,
        mut spell_info_by_id: SpellInfoById,
        mut next_rank_spell: NextRankSpell,
    ) -> SpellGroupStackRuleLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellGroupStackRuleRowLikeCpp>,
        SpellInfoById: FnMut(u32) -> Option<SpellInfo>,
        NextRankSpell: FnMut(u32) -> Option<u32>,
    {
        let mut store = Self::default();
        let mut same_effect_groups = Vec::new();
        let mut errors = Vec::new();
        let mut loaded_row_count = 0;

        for row in rows {
            let Some(stack_rule) = SpellGroupStackRuleLikeCpp::from_u8_like_cpp(row.stack_rule)
            else {
                errors.push(SpellGroupStackRuleLoadErrorLikeCpp {
                    row,
                    spell_id: None,
                    kind: SpellGroupStackRuleLoadErrorKindLikeCpp::StackRuleMissing,
                });
                continue;
            };

            if spell_groups
                .spell_group_spell_map_bounds_like_cpp(row.group_id)
                .is_empty()
            {
                errors.push(SpellGroupStackRuleLoadErrorLikeCpp {
                    row,
                    spell_id: None,
                    kind: SpellGroupStackRuleLoadErrorKindLikeCpp::GroupMissing,
                });
                continue;
            }

            store
                .stack_rule_by_group_id
                .entry(row.group_id)
                .or_insert(stack_rule);

            if stack_rule == SpellGroupStackRuleLikeCpp::ExclusiveSameEffect {
                same_effect_groups.push(row.group_id);
            }

            loaded_row_count += 1;
        }

        let mut same_effect_parsed_count = 0;
        for group_id in same_effect_groups {
            let spell_ids = spell_groups.set_of_spells_in_spell_group_like_cpp(group_id);
            let aura_types =
                infer_same_effect_stack_aura_types_like_cpp(&spell_ids, &mut spell_info_by_id);

            for spell_id in spell_ids {
                if !spell_rank_chain_has_any_aura_like_cpp(
                    spell_id,
                    &aura_types,
                    &mut spell_info_by_id,
                    &mut next_rank_spell,
                ) {
                    let kind = if spell_info_by_id(spell_id).is_some() {
                        SpellGroupStackRuleLoadErrorKindLikeCpp::SameEffectSpellAuraMissing
                    } else {
                        SpellGroupStackRuleLoadErrorKindLikeCpp::SameEffectSpellMissing
                    };
                    errors.push(SpellGroupStackRuleLoadErrorLikeCpp {
                        row: SpellGroupStackRuleRowLikeCpp {
                            group_id,
                            stack_rule: SpellGroupStackRuleLikeCpp::ExclusiveSameEffect as u8,
                        },
                        spell_id: Some(spell_id),
                        kind,
                    });
                }
            }

            store
                .same_effect_stack_by_group_id
                .insert(group_id, aura_types);
            same_effect_parsed_count += 1;
        }

        SpellGroupStackRuleLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            same_effect_parsed_count,
            errors,
        }
    }

    pub fn spell_group_stack_rule_like_cpp(&self, group_id: u32) -> SpellGroupStackRuleLikeCpp {
        self.stack_rule_by_group_id
            .get(&group_id)
            .copied()
            .unwrap_or(SpellGroupStackRuleLikeCpp::Default)
    }

    pub fn same_effect_stack_rule_aura_types_like_cpp(
        &self,
        group_id: u32,
    ) -> Option<&BTreeSet<i32>> {
        self.same_effect_stack_by_group_id.get(&group_id)
    }

    pub fn check_spell_group_stack_rules_like_cpp(
        &self,
        spell_groups: &SpellGroupStoreLikeCpp,
        first_rank_spell_id_1: u32,
        first_rank_spell_id_2: u32,
    ) -> SpellGroupStackRuleLikeCpp {
        let mut common_groups = BTreeSet::new();

        for group_id in spell_groups
            .spell_spell_group_map_bounds_like_cpp(first_rank_spell_id_1, |spell_id| spell_id)
        {
            if spell_groups.is_spell_member_of_spell_group_like_cpp(
                first_rank_spell_id_2,
                *group_id,
                |spell_id| spell_id,
            ) {
                let mut add = true;
                for entry in spell_groups.spell_group_spell_map_bounds_like_cpp(*group_id) {
                    if *entry < 0 {
                        let nested_group_id = entry.unsigned_abs();
                        if spell_groups.is_spell_member_of_spell_group_like_cpp(
                            first_rank_spell_id_1,
                            nested_group_id,
                            |spell_id| spell_id,
                        ) && spell_groups.is_spell_member_of_spell_group_like_cpp(
                            first_rank_spell_id_2,
                            nested_group_id,
                            |spell_id| spell_id,
                        ) {
                            add = false;
                            break;
                        }
                    }
                }

                if add {
                    common_groups.insert(*group_id);
                }
            }
        }

        let mut rule = SpellGroupStackRuleLikeCpp::Default;
        for group_id in common_groups {
            rule = self.spell_group_stack_rule_like_cpp(group_id);
            if rule != SpellGroupStackRuleLikeCpp::Default {
                break;
            }
        }
        rule
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellProcStoreLikeCpp {
    pub proc_entries_by_spell_and_difficulty: BTreeMap<SpellProcKeyLikeCpp, SpellProcEntryLikeCpp>,
}

impl SpellProcStoreLikeCpp {
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
        spell_aura_options: &crate::spell_db2::SpellAuraOptionsStore,
        spell_misc: &crate::spell_db2::SpellMiscStore,
        spell_class_options: &crate::spell_db2::SpellClassOptionsStore,
        spell_procs_per_minute: &crate::spell_db2::SpellProcsPerMinuteStore,
    ) -> Result<SpellProcLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_PROC);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellProcRowLikeCpp {
                    spell_id: result.try_read::<i32>(0).unwrap_or(0),
                    school_mask: result.try_read::<u8>(1).unwrap_or(0),
                    spell_family_name: result.try_read::<u16>(2).unwrap_or(0),
                    spell_family_mask: [
                        result.try_read::<u32>(3).unwrap_or(0),
                        result.try_read::<u32>(4).unwrap_or(0),
                        result.try_read::<u32>(5).unwrap_or(0),
                        result.try_read::<u32>(6).unwrap_or(0),
                    ],
                    proc_flags: [
                        result.try_read::<u32>(7).unwrap_or(0),
                        result.try_read::<u32>(8).unwrap_or(0),
                    ],
                    spell_type_mask: result.try_read::<u32>(9).unwrap_or(0),
                    spell_phase_mask: result.try_read::<u32>(10).unwrap_or(0),
                    hit_mask: result.try_read::<u32>(11).unwrap_or(0),
                    attributes_mask: result.try_read::<u32>(12).unwrap_or(0),
                    disable_effects_mask: result.try_read::<u32>(13).unwrap_or(0),
                    procs_per_minute: result.try_read::<f32>(14).unwrap_or(0.0),
                    chance: result.try_read::<f32>(15).unwrap_or(0.0),
                    cooldown_ms: result.try_read::<u32>(16).unwrap_or(0),
                    charges: result.try_read::<u8>(17).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        let spell_infos = spells
            .iter()
            .filter_map(|spell| {
                let spell_id = u32::try_from(spell.spell_id).ok()?;
                SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
                    spell_id,
                    0,
                    spells,
                    spell_chains,
                    spell_aura_options,
                    spell_misc,
                    spell_class_options,
                    spell_procs_per_minute,
                )
            })
            .collect::<Vec<_>>();

        let spell_infos_by_id = spell_infos
            .iter()
            .cloned()
            .map(|spell_info| (spell_info.spell_id, spell_info))
            .collect::<BTreeMap<_, _>>();

        Ok(Self::from_rows_and_spell_infos_like_cpp(
            rows,
            |spell_id| spell_infos_by_id.get(&spell_id).cloned(),
            spell_infos,
        ))
    }

    pub fn from_rows_like_cpp<I, SpellInfoById>(
        rows: I,
        mut spell_info_by_id: SpellInfoById,
    ) -> SpellProcLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellProcRowLikeCpp>,
        SpellInfoById: FnMut(u32) -> Option<SpellProcSourceSpellInfoLikeCpp>,
    {
        let mut store = Self::default();
        let mut errors = Vec::new();
        let mut loaded_row_count = 0;

        for row in rows {
            let all_ranks = row.spell_id < 0;
            let spell_id = row.spell_id.unsigned_abs();
            let Some(mut spell_info) = spell_info_by_id(spell_id) else {
                errors.push(SpellProcLoadErrorLikeCpp {
                    spell_id,
                    difficulty: None,
                    effect_index: None,
                    kind: SpellProcLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            };

            if all_ranks {
                if !spell_info.is_ranked_like_cpp() {
                    errors.push(SpellProcLoadErrorLikeCpp {
                        spell_id,
                        difficulty: Some(spell_info.difficulty),
                        effect_index: None,
                        kind: SpellProcLoadErrorKindLikeCpp::AllRanksSpellNotRanked,
                    });
                }

                if spell_info.first_rank_spell_id != spell_id {
                    errors.push(SpellProcLoadErrorLikeCpp {
                        spell_id,
                        difficulty: Some(spell_info.difficulty),
                        effect_index: None,
                        kind: SpellProcLoadErrorKindLikeCpp::AllRanksSpellNotFirstRank,
                    });
                    continue;
                }
            }

            loop {
                let key = SpellProcKeyLikeCpp {
                    spell_id: spell_info.spell_id,
                    difficulty: spell_info.difficulty,
                };

                if store
                    .proc_entries_by_spell_and_difficulty
                    .contains_key(&key)
                {
                    errors.push(SpellProcLoadErrorLikeCpp {
                        spell_id: spell_info.spell_id,
                        difficulty: Some(spell_info.difficulty),
                        effect_index: None,
                        kind: SpellProcLoadErrorKindLikeCpp::DuplicateSpell,
                    });
                    break;
                }

                let mut entry = SpellProcEntryLikeCpp::from_row_like_cpp(&row);
                apply_spell_proc_defaults_like_cpp(&mut entry, &spell_info);
                validate_spell_proc_entry_like_cpp(&mut entry, &spell_info, &mut errors);
                store
                    .proc_entries_by_spell_and_difficulty
                    .insert(key, entry);

                if !all_ranks {
                    break;
                }

                let Some(next_rank_spell_id) = spell_info.next_rank_spell_id else {
                    break;
                };
                let Some(next_spell_info) = spell_info_by_id(next_rank_spell_id) else {
                    break;
                };
                spell_info = next_spell_info;
            }

            loaded_row_count += 1;
        }

        SpellProcLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            generated_entry_count: 0,
            errors,
        }
    }

    pub fn from_rows_and_implicit_sources_like_cpp<I, SpellInfoById, ImplicitSources>(
        rows: I,
        spell_info_by_id: SpellInfoById,
        implicit_sources: ImplicitSources,
    ) -> SpellProcLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellProcRowLikeCpp>,
        SpellInfoById: FnMut(u32) -> Option<SpellProcSourceSpellInfoLikeCpp>,
        ImplicitSources: IntoIterator<Item = ImplicitSpellProcSourceLikeCpp>,
    {
        let mut outcome = Self::from_rows_like_cpp(rows, spell_info_by_id);

        for source in implicit_sources {
            let key = SpellProcKeyLikeCpp {
                spell_id: source.spell_id,
                difficulty: source.difficulty,
            };

            if outcome
                .store
                .proc_entries_by_spell_and_difficulty
                .contains_key(&key)
            {
                continue;
            }

            let Some(entry) = implicit_spell_proc_entry_like_cpp(&source) else {
                continue;
            };

            outcome
                .store
                .proc_entries_by_spell_and_difficulty
                .insert(key, entry);
            outcome.generated_entry_count += 1;
        }

        outcome
    }

    pub fn from_rows_and_spell_infos_like_cpp<I, SpellInfoById, SpellInfos>(
        rows: I,
        spell_info_by_id: SpellInfoById,
        spell_infos: SpellInfos,
    ) -> SpellProcLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellProcRowLikeCpp>,
        SpellInfoById: FnMut(u32) -> Option<SpellProcSourceSpellInfoLikeCpp>,
        SpellInfos: IntoIterator<Item = SpellProcSourceSpellInfoLikeCpp>,
    {
        Self::from_rows_and_implicit_sources_like_cpp(
            rows,
            spell_info_by_id,
            spell_infos
                .into_iter()
                .map(|spell_info| spell_info.implicit_proc_source_like_cpp()),
        )
    }

    pub fn spell_proc_entry_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&SpellProcEntryLikeCpp> {
        self.proc_entries_by_spell_and_difficulty
            .get(&SpellProcKeyLikeCpp {
                spell_id,
                difficulty,
            })
    }

    pub fn spell_proc_entry_with_fallback_like_cpp<FallbackDifficulty>(
        &self,
        spell_id: u32,
        difficulty: u32,
        mut fallback_difficulty: FallbackDifficulty,
    ) -> Option<&SpellProcEntryLikeCpp>
    where
        FallbackDifficulty: FnMut(u32) -> Option<u32>,
    {
        if let Some(entry) = self.spell_proc_entry_like_cpp(spell_id, difficulty) {
            return Some(entry);
        }

        let mut current_difficulty = difficulty;
        while let Some(next_difficulty) = fallback_difficulty(current_difficulty) {
            if let Some(entry) = self.spell_proc_entry_like_cpp(spell_id, next_difficulty) {
                return Some(entry);
            }
            current_difficulty = next_difficulty;
        }

        None
    }
}

impl SpellTargetPositionStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = SpellTargetPositionRowLikeCpp>,
        spells: &SpellStore,
        mut map_exists: impl FnMut(u16) -> bool,
    ) -> Self {
        let mut store = Self::default();

        for row in rows {
            if !map_exists(row.target_map_id) {
                store.load_report.skipped_missing_map += 1;
                continue;
            }

            if row.x == 0.0 && row.y == 0.0 && row.z == 0.0 {
                store.load_report.skipped_zero_position += 1;
                continue;
            }

            let Some(spell) = spells.get(row.spell_id as i32) else {
                store.load_report.skipped_missing_spell += 1;
                continue;
            };
            let Some(effect) = spell
                .effects()
                .iter()
                .find(|effect| effect.effect_index == row.effect_index)
            else {
                store.load_report.skipped_missing_effect += 1;
                continue;
            };

            if !effect.has_spell_target_position_target_like_cpp() {
                store.load_report.skipped_unsupported_target += 1;
                continue;
            }

            let orientation = row.orientation.unwrap_or_else(|| {
                if effect.position_facing > TAU {
                    effect.position_facing * std::f32::consts::PI / 180.0
                } else {
                    effect.position_facing
                }
            });

            store.positions.insert(
                (row.spell_id, row.effect_index),
                SpellTargetPositionLikeCpp {
                    target_map_id: row.target_map_id,
                    position: wow_core::Position::new(row.x, row.y, row.z, orientation),
                },
            );
            store.load_report.loaded += 1;
        }

        store
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
        map_exists: impl FnMut(u16) -> bool,
    ) -> Result<Self> {
        let mut result = db
            .direct_query(wow_database::WorldStatements::SEL_SPELL_TARGET_POSITION.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellTargetPositionRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    effect_index: result.try_read::<u8>(1).unwrap_or(0) as u32,
                    target_map_id: result.try_read::<u16>(2).unwrap_or(0),
                    x: result.try_read::<f32>(3).unwrap_or(0.0),
                    y: result.try_read::<f32>(4).unwrap_or(0.0),
                    z: result.try_read::<f32>(5).unwrap_or(0.0),
                    orientation: result.try_read::<Option<f32>>(6).unwrap_or(None),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows, spells, map_exists))
    }

    pub fn get(&self, spell_id: u32, effect_index: u32) -> Option<&SpellTargetPositionLikeCpp> {
        self.positions.get(&(spell_id, effect_index))
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn load_report_like_cpp(&self) -> &SpellTargetPositionLoadReportLikeCpp {
        &self.load_report
    }
}

#[derive(Default)]
pub struct SpellStore {
    pub(crate) spells: HashMap<i32, SpellInfo>,
    pub(crate) spell_info_keys_like_cpp: crate::spell_info_keys::SpellInfoKeyStoreLikeCpp,
    spell_effects_by_difficulty: HashMap<(i32, u8), Vec<SpellEffectInfo>>,
    spell_misc_attributes: HashMap<i32, [u32; 15]>,
    spell_misc_attributes_by_difficulty: HashMap<(i32, u8), [u32; 15]>,
    spell_interrupt_flags: HashMap<(i32, u8), ([u32; 2], [u32; 2])>,
    spell_interrupt_rows_by_id: BTreeMap<u32, SpellInterruptRowLikeCpp>,
    spell_hit_categories_by_difficulty: HashMap<(i32, u8), SpellHitCategoriesRowLikeCpp>,
    spell_hit_misc_by_difficulty: HashMap<(i32, u8), SpellHitMiscRowLikeCpp>,
    spell_hit_effect_mechanics_by_difficulty:
        HashMap<(i32, u8), BTreeMap<u32, SpellHitEffectMechanicRowLikeCpp>>,
    spell_shapeshift_masks: HashMap<i32, (u64, u64)>,
    implicit_target_conditions: HashMap<(i32, u32), ConditionsReference>,
}

/// Effective DB2 authorities consumed together by C++
/// `SpellMgr::LoadSpellInfoStore`.
///
/// The stores own DB2 parsing, row replacement and final tombstones. The
/// composition root supplies already-decoded rows without exposing a database
/// or persistence contract to `wow-data`.
pub struct EffectiveCoreSpellDb2StoresLikeCpp {
    spell_categories: crate::spell_db2::SpellCategoriesStore,
    spell_misc: crate::spell_db2::SpellMiscStore,
    spell_effect: crate::spell_db2::SpellEffectDb2Store,
    spell_shapeshift: crate::spell_db2::SpellShapeshiftStore,
    spell_interrupts: crate::spell_db2::SpellInterruptsStore,
    spell_cast_times: crate::spell_db2::SpellCastTimesStore,
    spell_cooldowns: crate::spell_db2::SpellCooldownsStore,
    spell_casting_requirements: crate::spell_db2::SpellCastingRequirementsStore,
    spell_power: crate::spell_db2::SpellPowerStore,
    spell_power_difficulty: crate::spell_db2::SpellPowerDifficultyStore,
}

impl EffectiveCoreSpellDb2StoresLikeCpp {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spell_categories: crate::spell_db2::SpellCategoriesStore,
        spell_misc: crate::spell_db2::SpellMiscStore,
        spell_effect: crate::spell_db2::SpellEffectDb2Store,
        spell_shapeshift: crate::spell_db2::SpellShapeshiftStore,
        spell_interrupts: crate::spell_db2::SpellInterruptsStore,
        spell_cast_times: crate::spell_db2::SpellCastTimesStore,
        spell_cooldowns: crate::spell_db2::SpellCooldownsStore,
        spell_casting_requirements: crate::spell_db2::SpellCastingRequirementsStore,
        spell_power: crate::spell_db2::SpellPowerStore,
        spell_power_difficulty: crate::spell_db2::SpellPowerDifficultyStore,
    ) -> Self {
        Self {
            spell_categories,
            spell_misc,
            spell_effect,
            spell_shapeshift,
            spell_interrupts,
            spell_cast_times,
            spell_cooldowns,
            spell_casting_requirements,
            spell_power,
            spell_power_difficulty,
        }
    }
}

impl SpellStore {
    /// Create a new empty spell store.
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
            spell_info_keys_like_cpp: crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::default(),
            spell_effects_by_difficulty: HashMap::new(),
            spell_misc_attributes: HashMap::new(),
            spell_misc_attributes_by_difficulty: HashMap::new(),
            spell_interrupt_flags: HashMap::new(),
            spell_interrupt_rows_by_id: BTreeMap::new(),
            spell_hit_categories_by_difficulty: HashMap::new(),
            spell_hit_misc_by_difficulty: HashMap::new(),
            spell_hit_effect_mechanics_by_difficulty: HashMap::new(),
            spell_shapeshift_masks: HashMap::new(),
            implicit_target_conditions: HashMap::new(),
        }
    }

    fn make_pair64_like_cpp(low: i32, high: i32) -> u64 {
        u64::from(low as u32) | (u64::from(high as u32) << 32)
    }

    pub fn effects_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<&[SpellEffectInfo]> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = HashSet::new();
        loop {
            if let Some(effects) = self
                .spell_effects_by_difficulty
                .get(&(spell_id, difficulty_id))
            {
                return Some(effects);
            }
            if difficulty_id == 0 || !visited.insert(difficulty_id) {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
        self.spells.get(&spell_id).map(|spell| spell.effects())
    }

    /// Resolve the fields consumed by C++ spell-hit logic through the
    /// requested difficulty and its `FallbackDifficultyID` chain.
    ///
    /// `SpellCategories`, `SpellMisc`, and every `SpellEffect` slot fall back
    /// independently. This matters when a difficulty overrides only one of
    /// those contributors.
    pub fn hit_metadata_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<SpellHitMetadataLikeCpp> {
        let mut metadata = SpellHitMetadataLikeCpp::default();
        let mut has_metadata = false;
        let mut categories_resolved = false;
        let mut misc_resolved = false;
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = [false; 256];

        loop {
            let visited_slot = &mut visited[usize::from(difficulty_id)];
            if *visited_slot {
                break;
            }
            *visited_slot = true;

            if !categories_resolved
                && let Some(categories) = self
                    .spell_hit_categories_by_difficulty
                    .get(&(spell_id, difficulty_id))
            {
                metadata.category_id = categories.category_id;
                metadata.charge_category_id = categories.charge_category_id;
                metadata.defense_type = categories.defense_type;
                metadata.spell_mechanic = categories.spell_mechanic;
                categories_resolved = true;
                has_metadata = true;
            }
            if !misc_resolved
                && let Some(misc) = self
                    .spell_hit_misc_by_difficulty
                    .get(&(spell_id, difficulty_id))
            {
                metadata.school_mask = misc.school_mask;
                misc_resolved = true;
                has_metadata = true;
            }
            if let Some(effect_mechanics) = self
                .spell_hit_effect_mechanics_by_difficulty
                .get(&(spell_id, difficulty_id))
            {
                has_metadata = true;
                for (&effect_index, effect) in effect_mechanics {
                    metadata
                        .effect_mechanics
                        .entry(effect_index)
                        .or_insert(effect.mechanic);
                }
            }

            if difficulty_id == 0 {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }

        has_metadata.then_some(metadata)
    }

    pub(super) fn empty_spell_info_like_cpp(spell_id: i32) -> SpellInfo {
        SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        }
    }

    fn spell_effect_from_db2_like_cpp(
        effect: &crate::spell_db2::SpellEffectDb2Entry,
    ) -> SpellEffectInfo {
        SpellEffectInfo {
            effect_index: u32::try_from(effect.effect_index).unwrap_or(0),
            effect: effect.effect,
            effect_aura: i32::from(effect.effect_aura),
            effect_base_points: effect.effect_base_points,
            effect_die_sides: effect.effect_die_sides,
            effect_spell_class_mask: effect.effect_spell_class_mask,
            effect_misc_value_1: effect.effect_misc_value[0],
            effect_misc_value_2: effect.effect_misc_value[1],
            effect_trigger_spell: effect.effect_trigger_spell,
            effect_radius_index_1: effect.effect_radius_index[0],
            position_facing: effect.effect_pos_facing,
            chain_targets: effect.effect_chain_targets,
            implicit_target_1: u32::try_from(effect.implicit_target[0]).unwrap_or(0),
            implicit_target_2: u32::try_from(effect.implicit_target[1]).unwrap_or(0),
        }
    }

    fn hydrate_primary_effect_like_cpp(info: &mut SpellInfo) {
        info.effects.sort_by_key(|effect| effect.effect_index);
        if let Some(primary) = info.effects.iter().find(|effect| effect.effect != 0) {
            info.effect_type = primary.effect;
            info.effect_base_points = primary.effect_base_points;
            info.effect_bonus_coefficient = 0.0;
            info.aura_type = Some(primary.effect_aura);
        }
    }

    /// Load the exact regular `SpellInfo` key authority that still has
    /// non-core Hotfix DB2 contributors outside #509.
    pub async fn load_spell_info_key_seed_like_cpp(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
        spell_name_store: &crate::spell_db2::SpellNameStore,
        hotfix_removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let spell_info_keys_like_cpp =
            crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::load_like_cpp(
                data_dir,
                locale,
                hotfix_db,
                spell_name_store,
                hotfix_removals,
            )
            .await?;
        let mut store = Self::new();
        store.spell_info_keys_like_cpp = spell_info_keys_like_cpp;
        Ok(store)
    }

    /// Hydrate the represented `SpellInfo` payload from already-effective DB2
    /// authorities while preserving C++ `SpellMgr::LoadSpellInfoStore` order.
    pub fn hydrate_effective_core_db2_like_cpp(
        self,
        stores: EffectiveCoreSpellDb2StoresLikeCpp,
    ) -> Self {
        let mut store = Self::from_spell_db2_stores_like_cpp(
            &stores.spell_categories,
            &stores.spell_misc,
            &stores.spell_effect,
            &stores.spell_shapeshift,
        );
        store.spell_info_keys_like_cpp = self.spell_info_keys_like_cpp;
        store.apply_db2_interrupts_like_cpp(&stores.spell_interrupts);
        store.apply_db2_cast_times_like_cpp(&stores.spell_misc, &stores.spell_cast_times);
        store.apply_db2_cooldowns_like_cpp(&stores.spell_cooldowns);
        store.apply_db2_casting_requirements_like_cpp(&stores.spell_casting_requirements);
        store.apply_db2_power_costs_like_cpp(&stores.spell_power, &stores.spell_power_difficulty);
        store.apply_interrupt_flag_corrections_like_cpp();

        info!(
            "Loaded {} spells from SpellMisc/SpellEffect DB2 with hotfix overlay",
            store.spells.len()
        );
        store
    }

    /// Whether C++ `SpellMgr::GetSpellInfo` has an exact regular-spell key.
    ///
    /// This is deliberately separate from [`Self::get`]. `get` exposes the
    /// subset of `SpellInfo` payload fields Rust currently hydrates, whereas
    /// C++ creates existence keys from twenty DB2 contributors.
    pub fn contains_spell_info_exact_like_cpp(&self, spell_id: u32, difficulty_id: u8) -> bool {
        self.spell_info_keys_like_cpp
            .contains_exact_like_cpp(spell_id, difficulty_id)
    }

    /// Exact regular `SpellInfo` keys in deterministic `(SpellID, Difficulty)` order.
    pub fn spell_info_keys_in_order_like_cpp(&self) -> Vec<(u32, u8)> {
        self.spell_info_keys_like_cpp.exact_keys_in_order_like_cpp()
    }

    /// Whether C++ `GetSpellInfo(id, DIFFICULTY_NONE)` would find a regular
    /// or server-side spell.
    ///
    /// The shipped DB2 has no difficulty-zero row, but C++ permits SQL
    /// overlays to add one and then follows its `FallbackDifficultyID` chain.
    /// Keep that behavior for loader foreign-key checks while stopping an
    /// invalid custom cycle instead of hanging startup forever.
    pub fn contains_spell_info_difficulty_none_like_cpp(
        &self,
        serverside_spells: &ServersideSpellStoreLikeCpp,
        difficulty_store: &crate::difficulty::DifficultyStore,
        spell_id: u32,
    ) -> bool {
        let mut difficulty_id = 0u8;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(difficulty_id) {
                return false;
            }
            if self.contains_spell_info_exact_like_cpp(spell_id, difficulty_id)
                || serverside_spells
                    .get_serverside_spell_like_cpp(spell_id, u32::from(difficulty_id))
                    .is_some()
            {
                return true;
            }
            let Some(difficulty) = difficulty_store.get(u32::from(difficulty_id)) else {
                return false;
            };
            difficulty_id = difficulty.fallback_difficulty_id;
        }
    }

    /// Whether C++ `_GetSpellInfo(id)` would find any regular difficulty.
    pub fn contains_spell_info_any_difficulty_like_cpp(&self, spell_id: u32) -> bool {
        self.spell_info_keys_like_cpp
            .contains_any_difficulty_like_cpp(spell_id)
    }

    pub fn spell_info_key_count_like_cpp(&self) -> usize {
        self.spell_info_keys_like_cpp.len()
    }

    fn apply_db2_hit_metadata_like_cpp(
        &mut self,
        spell_categories_store: &crate::spell_db2::SpellCategoriesStore,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_effect_store: &crate::spell_db2::SpellEffectDb2Store,
    ) {
        for categories in spell_categories_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(categories.spell_id) else {
                continue;
            };
            let row = SpellHitCategoriesRowLikeCpp {
                record_id: categories.id,
                // C++ assigns the signed DB2 fields directly into the
                // corresponding uint32 SpellInfo members.
                category_id: categories.category as u32,
                charge_category_id: categories.charge_category as u32,
                defense_type: categories.defense_type,
                spell_mechanic: categories.mechanic,
            };
            self.spell_hit_categories_by_difficulty
                .entry((spell_id, categories.difficulty_id))
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }

        for misc in spell_misc_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let row = SpellHitMiscRowLikeCpp {
                record_id: misc.id,
                school_mask: misc.school_mask,
            };
            self.spell_hit_misc_by_difficulty
                .entry((spell_id, misc.difficulty_id))
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }

        for effect in spell_effect_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(effect.spell_id) else {
                continue;
            };
            let Ok(difficulty_id) = u8::try_from(effect.difficulty_id) else {
                continue;
            };
            let Ok(effect_index) = u32::try_from(effect.effect_index) else {
                continue;
            };
            if effect_index >= MAX_SPELL_EFFECTS_LIKE_CPP as u32 {
                continue;
            }
            let row = SpellHitEffectMechanicRowLikeCpp {
                record_id: effect.id,
                mechanic: effect.effect_mechanic,
            };
            self.spell_hit_effect_mechanics_by_difficulty
                .entry((spell_id, difficulty_id))
                .or_default()
                .entry(effect_index)
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }
    }

    pub(super) fn from_spell_db2_stores_like_cpp(
        spell_categories_store: &crate::spell_db2::SpellCategoriesStore,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_effect_store: &crate::spell_db2::SpellEffectDb2Store,
        spell_shapeshift_store: &crate::spell_db2::SpellShapeshiftStore,
    ) -> Self {
        let mut store = Self::new();

        store.apply_db2_hit_metadata_like_cpp(
            spell_categories_store,
            spell_misc_store,
            spell_effect_store,
        );

        for misc in spell_misc_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let difficulty_id = misc.difficulty_id;
            let attributes = misc.attributes.map(|attribute| attribute as u32);
            store
                .spell_misc_attributes_by_difficulty
                .insert((spell_id, difficulty_id), attributes);
            if difficulty_id != 0 {
                continue;
            }
            store
                .spells
                .entry(spell_id)
                .or_insert_with(|| Self::empty_spell_info_like_cpp(spell_id));
            store.spell_misc_attributes.insert(spell_id, attributes);
        }

        for effect in spell_effect_store.entries_like_cpp() {
            if effect.effect == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(effect.spell_id) else {
                continue;
            };
            let Ok(difficulty_id) = u8::try_from(effect.difficulty_id) else {
                continue;
            };
            let converted = Self::spell_effect_from_db2_like_cpp(effect);
            store
                .spell_effects_by_difficulty
                .entry((spell_id, difficulty_id))
                .or_default()
                .push(converted.clone());
            if difficulty_id != 0 {
                continue;
            }
            let spell = store
                .spells
                .entry(spell_id)
                .or_insert_with(|| Self::empty_spell_info_like_cpp(spell_id));
            spell.effects.push(converted);
        }

        for shapeshift in spell_shapeshift_store.entries_like_cpp() {
            if shapeshift.spell_id <= 0 {
                continue;
            }
            store.spell_shapeshift_masks.insert(
                shapeshift.spell_id,
                (
                    Self::make_pair64_like_cpp(
                        shapeshift.shapeshift_mask[0],
                        shapeshift.shapeshift_mask[1],
                    ),
                    Self::make_pair64_like_cpp(
                        shapeshift.shapeshift_exclude[0],
                        shapeshift.shapeshift_exclude[1],
                    ),
                ),
            );
        }

        for spell in store.spells.values_mut() {
            Self::hydrate_primary_effect_like_cpp(spell);
        }

        store
    }

    /// C++ `SpellMgr::LoadSpellInfoStore` copies the difficulty-specific
    /// `SpellInterrupts` row into `SpellInfo`. The current Rust `SpellInfo`
    /// keeps related DB2 joins in `SpellStore`, so retain both interrupt masks
    /// here without widening every dynamically constructed test SpellInfo.
    pub(super) fn apply_db2_interrupts_like_cpp(
        &mut self,
        spell_interrupts_store: &crate::spell_db2::SpellInterruptsStore,
    ) {
        for interrupts in spell_interrupts_store.entries_like_cpp() {
            self.store_signed_interrupt_row_by_id_like_cpp(
                interrupts.id,
                interrupts.spell_id,
                interrupts.difficulty_id,
                interrupts.aura_interrupt_flags,
                interrupts.channel_interrupt_flags,
            );
        }
        self.rebuild_interrupt_flags_from_rows_like_cpp();
    }

    /// Apply one file/hotfix `SpellInterrupts` row. DB2 stores the bit fields
    /// as signed integers, while C++ preserves their complete `uint32` bit
    /// pattern in `SpellInfo`.
    pub(super) fn store_signed_interrupt_row_by_id_like_cpp(
        &mut self,
        row_id: u32,
        spell_id: u32,
        difficulty_id: u8,
        aura_interrupt_flags: [i32; 2],
        channel_interrupt_flags: [i32; 2],
    ) -> bool {
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        self.spell_interrupt_rows_by_id.insert(
            row_id,
            SpellInterruptRowLikeCpp {
                key: (spell_id, difficulty_id),
                flags: (
                    aura_interrupt_flags.map(|flag| flag as u32),
                    channel_interrupt_flags.map(|flag| flag as u32),
                ),
            },
        );
        true
    }

    /// Rebuild the relational lookup once per load phase. C++ DB2 storage is
    /// indexed and iterated by ascending record ID, so later IDs win if two
    /// records resolve to the same spell/difficulty key.
    pub(super) fn rebuild_interrupt_flags_from_rows_like_cpp(&mut self) {
        self.spell_interrupt_flags.clear();
        for row in self.spell_interrupt_rows_by_id.values() {
            self.spell_interrupt_flags.insert(row.key, row.flags);
        }
    }

    /// Import world-DB `serverside_spell` interrupt masks into the same
    /// effective lookup used by live aura/channel decisions. C++ inserts these
    /// SpellInfo rows before applying corrections; effective file plus SQL
    /// `SpellName` IDs were already rejected while the server-side store was
    /// built.
    pub fn apply_serverside_spell_interrupts_like_cpp(
        &mut self,
        serverside_spells: &ServersideSpellStoreLikeCpp,
    ) {
        for info in serverside_spells
            .spell_infos_by_spell_and_difficulty
            .values()
        {
            let Ok(spell_id) = i32::try_from(info.row.spell_id) else {
                continue;
            };
            self.insert_spell_interrupt_flags_for_difficulty_like_cpp(
                spell_id,
                info.row.difficulty_id as u8,
                info.row.aura_interrupt_flags,
                info.row.channel_interrupt_flags,
            );
        }
        self.apply_interrupt_flag_corrections_like_cpp();
    }

    /// Interrupt-mask subset of C++ `SpellMgr::LoadSpellInfoCorrections`.
    /// `ApplySpellFix` mutates every difficulty variant, so update every stored
    /// key for each affected spell after DB2/hotfix/server-side composition.
    pub(super) fn apply_interrupt_flag_corrections_like_cpp(&mut self) {
        const HOSTILE_ACTION_RECEIVED: u32 = 0x0000_0001;
        const DAMAGE: u32 = 0x0000_0002;
        const ACTION: u32 = 0x0000_0004;
        const MOVING: u32 = 0x0000_0008;
        const ANIM: u32 = 0x0000_0020;
        const LEAVE_WORLD: u32 = 0x0008_0000;

        for spell_id in [61_719, 29_726, 63_414, 24_314, 99_252] {
            if self.spells.contains_key(&spell_id)
                && !self
                    .spell_interrupt_flags
                    .keys()
                    .any(|(known_spell_id, _)| *known_spell_id == spell_id)
            {
                self.spell_interrupt_flags
                    .insert((spell_id, 0), ([0; 2], [0; 2]));
            }
        }

        for ((spell_id, _), (aura, channel)) in &mut self.spell_interrupt_flags {
            match *spell_id {
                // Easter Lay Noblegarden Egg Aura.
                61_719 => aura[0] = HOSTILE_ACTION_RECEIVED | DAMAGE,
                // Test Ribbon Pole Channel.
                29_726 => channel[0] &= !ACTION,
                // Spinning Up (Mimiron).
                63_414 => *channel = [0; 2],
                // Threatening Gaze.
                24_314 => aura[0] |= ACTION | MOVING | ANIM,
                // Blaze of Glory.
                99_252 => aura[0] |= LEAVE_WORLD,
                _ => {}
            }
        }
    }

    /// [M0.1/#14] Apply DB2 cast times onto already-built SpellInfo rows.
    ///
    /// Mirrors the C++ SpellInfo ctor `CastTimeEntry =
    /// sSpellCastTimesStore.LookupEntry(_misc->CastingTimeIndex)` (SpellInfo.cpp:1185)
    /// + `CalcCastTime`: cast time = `max(Base, Minimum)`, clamped to ≥ 0
    /// (SpellInfo.cpp:3922). Must run AFTER the hotfix merge, which overwrites
    /// `cast_time_ms` (and would clobber this back to 0).
    pub(super) fn apply_db2_cast_times_like_cpp(
        &mut self,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_cast_times_store: &crate::spell_db2::SpellCastTimesStore,
    ) {
        for misc in spell_misc_store.entries_like_cpp() {
            if misc.difficulty_id != 0 || misc.casting_time_index == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let Some(entry) = spell_cast_times_store.get(u32::from(misc.casting_time_index)) else {
                continue;
            };
            if let Some(spell) = self.spells.get_mut(&spell_id) {
                spell.cast_time_ms = entry.base.max(entry.minimum).max(0) as u32;
            }
        }
    }

    /// [M0.1/#14] Apply DB2 per-spell cooldowns onto already-built SpellInfo rows.
    ///
    /// Mirrors C++ SpellInfo `RecoveryTime/CategoryRecoveryTime` from
    /// `sSpellCooldownsStore` (SpellInfo.cpp:1263) and `GetRecoveryTime() =
    /// max(RecoveryTime, CategoryRecoveryTime)` (SpellInfo.cpp:3981) — the per-spell
    /// cooldown the cast gate checks (`recovery_time_ms`). `StartRecoveryTime` (the
    /// GCD) is a separate mechanic and is intentionally left to the GCD path.
    /// Must run AFTER the hotfix merge (which overwrites `recovery_time_ms`).
    pub(super) fn apply_db2_cooldowns_like_cpp(
        &mut self,
        spell_cooldowns_store: &crate::spell_db2::SpellCooldownsStore,
    ) {
        for entry in spell_cooldowns_store.entries_like_cpp() {
            if entry.difficulty_id != 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(entry.spell_id) else {
                continue;
            };
            if let Some(spell) = self.spells.get_mut(&spell_id) {
                spell.recovery_time_ms =
                    entry.recovery_time.max(entry.category_recovery_time).max(0) as u32;
            }
        }
    }

    /// [M0.1/#72] Apply DB2 power costs onto already-built SpellInfo rows.
    ///
    /// Mirrors C++ `SpellMgr::LoadSpellInfoStore`, which stores
    /// `SpellPowerEntry` rows in `SpellInfo::PowerCosts` keyed by
    /// `SpellID`/difficulty/order (`SpellMgr.cpp:2550`, `DB2Stores.cpp:301`).
    /// C++ `SpellMgr::LoadSpellInfoStore` copies
    /// `SpellCastingRequirements::RequiresSpellFocus` into an already
    /// constructed `SpellInfo`. It never creates a spell from this table, so a
    /// requirements row for an unknown spell stays inert.
    pub(super) fn apply_db2_casting_requirements_like_cpp(
        &mut self,
        spell_casting_requirements_store: &crate::spell_db2::SpellCastingRequirementsStore,
    ) {
        // C++'s DB2 iteration assigns this DIFFICULTY_NONE slot in record-ID
        // order, so a duplicated SpellID resolves to the highest record ID.
        let mut effective_by_spell: HashMap<i32, (u32, u16)> = HashMap::new();
        for entry in spell_casting_requirements_store.entries_like_cpp() {
            effective_by_spell
                .entry(entry.spell_id)
                .and_modify(|current| {
                    if entry.id > current.0 {
                        *current = (entry.id, entry.requires_spell_focus);
                    }
                })
                .or_insert((entry.id, entry.requires_spell_focus));
        }

        for spell in self.spells.values_mut() {
            spell.requires_spell_focus = effective_by_spell
                .get(&spell.spell_id)
                .map_or(0, |(_, requires_spell_focus)| {
                    u32::from(*requires_spell_focus)
                });
        }
    }

    pub(super) fn apply_db2_power_costs_like_cpp(
        &mut self,
        spell_power_store: &crate::spell_db2::SpellPowerStore,
        spell_power_difficulty_store: &crate::spell_db2::SpellPowerDifficultyStore,
    ) {
        for spell in self.spells.values_mut() {
            spell.power_costs.clear();
        }

        // C++ walks `sSpellPowerStore` through its record-ID ordered index, so
        // two rows that collide on the same spell and order index must resolve
        // to the highest record ID rather than to a `HashMap` iteration order.
        for power in spell_power_store.entries_by_record_id_like_cpp() {
            if power.spell_id == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(power.spell_id) else {
                continue;
            };

            let (difficulty_id, order_index) = spell_power_difficulty_store
                .get(power.id)
                .map(|difficulty| (difficulty.difficulty_id, difficulty.order_index))
                .unwrap_or((0, power.order_index));
            if difficulty_id != 0 {
                continue;
            }

            let Some(spell) = self.spells.get_mut(&spell_id) else {
                continue;
            };
            let power_cost = SpellPowerCostInfoLikeCpp {
                order_index,
                power_type: power.power_type,
                mana_cost: power.mana_cost,
                mana_cost_per_level: power.mana_cost_per_level,
                mana_per_second: power.mana_per_second,
                power_cost_pct: power.power_cost_pct,
                power_cost_max_pct: power.power_cost_max_pct,
                power_pct_per_second: power.power_pct_per_second,
                required_aura_spell_id: power.required_aura_spell_id,
                optional_cost: power.optional_cost,
            };

            if let Some(existing) = spell
                .power_costs
                .iter_mut()
                .find(|existing| existing.order_index == order_index)
            {
                *existing = power_cost;
            } else {
                spell.power_costs.push(power_cost);
            }
            spell.power_costs.sort_by_key(|entry| entry.order_index);
        }
    }

    /// Look up a spell by ID.
    pub fn get(&self, spell_id: i32) -> Option<&SpellInfo> {
        self.spells.get(&spell_id)
    }

    /// Resolve the `SpellMisc` attributes owned by the same difficulty-specific
    /// C++ `SpellInfo` selected by `SpellMgr::GetSpellInfo`.
    pub fn misc_attributes_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 15]> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = HashSet::new();
        loop {
            if let Some(attributes) = self
                .spell_misc_attributes_by_difficulty
                .get(&(spell_id, difficulty_id))
                .copied()
            {
                return Some(attributes);
            }
            if difficulty_id == 0 || !visited.insert(difficulty_id) {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
        self.spell_misc_attributes.get(&spell_id).copied()
    }

    pub fn has_attribute_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
        attribute_word: usize,
        attribute: u32,
    ) -> bool {
        self.misc_attributes_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .and_then(|attributes| attributes.get(attribute_word).copied())
        .is_some_and(|attributes| attributes & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute0_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[0] & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute(SpellAttr1)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute1_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[1] & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute(SpellAttr2)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute2_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[2] & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute(SpellAttr4)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute4_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[4] & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute(SpellAttr8)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute8_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[8] & attribute != 0)
    }

    /// C++ `SpellInfo::Stances` / `StancesNot` for login passive-cast gates.
    pub fn shapeshift_masks_like_cpp(&self, spell_id: i32) -> (u64, u64) {
        self.spell_shapeshift_masks
            .get(&spell_id)
            .copied()
            .unwrap_or((0, 0))
    }

    /// C++ `SpellInfo::IsPassive`, for the represented paths that currently
    /// only need the `SPELL_ATTR0_PASSIVE` gate.
    pub fn is_passive_like_cpp(&self, spell_id: i32) -> bool {
        self.has_attribute0_like_cpp(spell_id, attributes::SPELL_ATTR0_PASSIVE)
    }

    /// C++ `SpellInfo::IsChanneled`.
    pub fn is_channeled_like_cpp(&self, spell_id: i32) -> bool {
        self.has_attribute1_like_cpp(
            spell_id,
            attributes::SPELL_ATTR1_IS_CHANNELLED | attributes::SPELL_ATTR1_IS_SELF_CHANNELLED,
        )
    }

    /// Resolve the C++ `SpellInterrupts` row for one spell/difficulty.
    ///
    /// `SpellMgr::GetSpellInfo` tries the exact map difficulty before walking
    /// `DifficultyEntry::FallbackDifficultyID`. Keep both aura and channel
    /// words coupled to the same selected row rather than merging metadata
    /// across difficulties.
    pub fn interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<([u32; 2], [u32; 2])> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = [false; 256];
        loop {
            if let Some(flags) = self
                .spell_interrupt_flags
                .get(&(spell_id, difficulty_id))
                .copied()
            {
                return Some(flags);
            }

            let visited_entry = &mut visited[usize::from(difficulty_id)];
            if *visited_entry {
                return None;
            }
            *visited_entry = true;

            difficulty_id = difficulty_store?.fallback_difficulty_id_like_cpp(difficulty_id)?;
        }
    }

    /// C++ `SpellInfo::HasAuraInterruptFlag` for the two
    /// `SpellAuraInterruptFlags` words loaded from difficulty zero.
    ///
    /// Transitional callers without map context retain the original base-row
    /// behavior; live paths should call the difficulty-aware variant.
    pub fn aura_interrupt_flags_like_cpp(&self, spell_id: i32) -> Option<[u32; 2]> {
        self.aura_interrupt_flags_for_difficulty_like_cpp(spell_id, 0, None)
    }

    pub fn aura_interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 2]> {
        self.interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .map(|(aura, _)| aura)
    }

    pub fn has_aura_interrupt_flag_like_cpp(&self, spell_id: i32, flags: u32, flags2: u32) -> bool {
        self.aura_interrupt_flags_like_cpp(spell_id)
            .is_some_and(|known| {
                (flags != 0 && known[0] & flags != 0) || (flags2 != 0 && known[1] & flags2 != 0)
            })
    }

    /// C++ `SpellInfo::HasChannelInterruptFlag` for the two
    /// `SpellAuraInterruptFlags` words loaded from difficulty zero.
    pub fn channel_interrupt_flags_like_cpp(&self, spell_id: i32) -> Option<[u32; 2]> {
        self.channel_interrupt_flags_for_difficulty_like_cpp(spell_id, 0, None)
    }

    pub fn channel_interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 2]> {
        self.interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .map(|(_, channel)| channel)
    }

    pub fn has_channel_interrupt_flag_like_cpp(
        &self,
        spell_id: i32,
        flags: u32,
        flags2: u32,
    ) -> bool {
        self.channel_interrupt_flags_like_cpp(spell_id)
            .is_some_and(|known| {
                (flags != 0 && known[0] & flags != 0) || (flags2 != 0 && known[1] & flags2 != 0)
            })
    }

    /// Port of C++ `SpellInfo::CheckShapeshift` for regular `SpellInfo`
    /// entries composed by `SpellMgr::LoadSpellInfoStore`.
    pub fn check_shapeshift_like_cpp<'a, F>(
        &self,
        spell_id: i32,
        form: u32,
        mut lookup_form: F,
    ) -> Option<SpellCastResult>
    where
        F: FnMut(u32) -> Option<&'a crate::spell_db2::SpellShapeshiftFormEntry>,
    {
        self.spells.get(&spell_id)?;

        let (stances, stances_not) = self
            .spell_shapeshift_masks
            .get(&spell_id)
            .copied()
            .unwrap_or((0, 0));
        let attributes = self
            .spell_misc_attributes
            .get(&spell_id)
            .copied()
            .unwrap_or([0; 15]);
        let stance_mask = form
            .checked_sub(1)
            .and_then(|shift| 1u64.checked_shl(shift))
            .unwrap_or(0);

        if stance_mask & stances_not != 0 {
            return Some(SpellCastResult::NotShapeshift);
        }

        if stance_mask & stances != 0 {
            return Some(SpellCastResult::Success);
        }

        let mut act_as_shifted = false;
        let mut form_flags = 0;
        if form > 0 {
            let Some(shape_info) = lookup_form(form) else {
                return Some(SpellCastResult::Success);
            };
            form_flags = shape_info.flags;
            act_as_shifted = form_flags & shapeshift_form_flags::STANCE == 0;
        }

        if act_as_shifted {
            if attributes[0] & attributes::SPELL_ATTR0_NOT_SHAPESHIFTED != 0
                || form_flags & shapeshift_form_flags::CAN_ONLY_CAST_SHAPESHIFT_SPELLS != 0
            {
                return Some(SpellCastResult::NotShapeshift);
            }

            if stances != 0 {
                return Some(SpellCastResult::OnlyShapeshift);
            }
        } else if attributes[2] & attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM
            == 0
            && stances != 0
        {
            return Some(SpellCastResult::OnlyShapeshift);
        }

        Some(SpellCastResult::Success)
    }

    pub fn iter(&self) -> impl Iterator<Item = &SpellInfo> {
        self.spells.values()
    }

    pub fn implicit_target_conditions_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
    ) -> Option<&ConditionsReference> {
        self.implicit_target_conditions
            .get(&(spell_id, effect_index))
    }

    pub fn attach_spell_implicit_target_conditions_like_cpp(
        &mut self,
        conditions: &ConditionEntriesByTypeStore,
    ) -> usize {
        let mut attached = 0;
        let Some(entries) = conditions.entries_for_source_type_like_cpp(
            wow_constants::ConditionSourceType::SpellImplicitTarget,
        ) else {
            return attached;
        };

        self.implicit_target_conditions.clear();
        for (id, bucket) in entries {
            let Some(spell) = self.spells.get(&id.source_entry) else {
                continue;
            };

            for effect in &spell.effects {
                let bit = 1_u32.checked_shl(effect.effect_index).unwrap_or(0);
                if bit == 0 || (id.source_group & bit) == 0 {
                    continue;
                }

                self.implicit_target_conditions.insert(
                    (id.source_entry, effect.effect_index),
                    ConditionsReference::new(bucket),
                );
                attached += bucket.len();
            }
        }

        attached
    }

    /// Insert a spell into the store (for testing or dynamic registration).
    #[allow(dead_code)]
    pub fn insert(&mut self, spell_id: i32, info: SpellInfo) {
        self.spells.insert(spell_id, info);
    }

    #[allow(dead_code)]
    pub fn insert_spell_misc_attributes_like_cpp(&mut self, spell_id: i32, attributes: [u32; 15]) {
        self.spell_misc_attributes.insert(spell_id, attributes);
        self.spell_misc_attributes_by_difficulty
            .insert((spell_id, 0), attributes);
    }

    #[allow(dead_code)]
    pub fn insert_spell_misc_attributes_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        attributes: [u32; 15],
    ) {
        self.spell_misc_attributes_by_difficulty
            .insert((spell_id, difficulty_id), attributes);
        if difficulty_id == 0 {
            self.spell_misc_attributes.insert(spell_id, attributes);
        }
    }

    /// Insert one synthetic hit-metadata projection for focused tests or
    /// dynamic registration without widening `SpellInfo`/`SpellEffectInfo`.
    #[allow(dead_code)]
    pub fn insert_spell_hit_metadata_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        metadata: SpellHitMetadataLikeCpp,
    ) {
        let SpellHitMetadataLikeCpp {
            category_id,
            charge_category_id,
            defense_type,
            spell_mechanic,
            school_mask,
            effect_mechanics,
        } = metadata;
        self.spell_hit_categories_by_difficulty.insert(
            (spell_id, difficulty_id),
            SpellHitCategoriesRowLikeCpp {
                record_id: u32::MAX,
                category_id,
                charge_category_id,
                defense_type,
                spell_mechanic,
            },
        );
        self.spell_hit_misc_by_difficulty.insert(
            (spell_id, difficulty_id),
            SpellHitMiscRowLikeCpp {
                record_id: u32::MAX,
                school_mask,
            },
        );
        self.spell_hit_effect_mechanics_by_difficulty.insert(
            (spell_id, difficulty_id),
            effect_mechanics
                .into_iter()
                .filter(|(effect_index, _)| *effect_index < MAX_SPELL_EFFECTS_LIKE_CPP as u32)
                .map(|(effect_index, mechanic)| {
                    (
                        effect_index,
                        SpellHitEffectMechanicRowLikeCpp {
                            record_id: u32::MAX,
                            mechanic,
                        },
                    )
                })
                .collect(),
        );
    }

    #[allow(dead_code)]
    pub fn insert_spell_interrupt_flags_like_cpp(
        &mut self,
        spell_id: i32,
        aura_interrupt_flags: [u32; 2],
        channel_interrupt_flags: [u32; 2],
    ) {
        self.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            0,
            aura_interrupt_flags,
            channel_interrupt_flags,
        );
    }

    #[allow(dead_code)]
    pub fn insert_spell_interrupt_flags_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        aura_interrupt_flags: [u32; 2],
        channel_interrupt_flags: [u32; 2],
    ) {
        self.spell_interrupt_flags.insert(
            (spell_id, difficulty_id),
            (aura_interrupt_flags, channel_interrupt_flags),
        );
    }

    #[allow(dead_code)]
    pub fn insert_spell_shapeshift_masks_like_cpp(
        &mut self,
        spell_id: i32,
        stances: u64,
        stances_not: u64,
    ) {
        self.spell_shapeshift_masks
            .insert(spell_id, (stances, stances_not));
    }

    /// Get the total number of loaded spells.
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }
}
