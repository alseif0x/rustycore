// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell acquisition sources and skill-line abilities.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSkillNodeLikeCpp {
    pub skill: u16,
    pub step: u16,
    pub value: u16,
    pub maxvalue: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSkillEffectLikeCpp {
    pub effect: u32,
    pub misc_value: i32,
    /// Deterministic C++ `SpellEffectInfo::CalcValue()` result for
    /// `SPELL_EFFECT_SKILL`. Ranged results are retained separately as a
    /// typed indeterminate source and never enter this compatibility shape.
    pub calc_value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSkillSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty_none: bool,
    pub effects: Vec<SpellLearnSkillEffectLikeCpp>,
}

/// Why the deterministic Rust acquisition authority cannot publish C++'s
/// compatibility `SpellLearnSkillNode`.
///
/// C++ samples `SpellEffectInfo::CalcValue()` once during startup.  A ranged
/// result can therefore select a different persisted skill tier after a
/// restart.  The official 3.4.3 `SKILL` data is entirely singleton-valued;
/// Rust keeps custom/future ranged metadata explicit so the pure acquisition
/// planner can fail closed instead of confusing it with a covered spell that
/// has no learn-skill effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellLearnSkillIndeterminateReasonLikeCpp {
    MissingEffectiveCoverage {
        difficulty_id: u32,
    },
    EffectiveMetadata(Vec<SpellAcquisitionIndeterminateReasonLikeCpp>),
    InvalidEffectiveValue {
        record_id: u32,
        field: &'static str,
        raw: i64,
    },
    RngDependentCalcValue {
        record_id: u32,
        domain: AcquisitionValueDomainLikeCpp,
    },
    SkillOutOfRange {
        value: i32,
    },
    StepOutOfRange {
        value: i32,
    },
    DuplicateSourceSpell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellLearnSkillLookupLikeCpp<'a> {
    Present(&'a SpellLearnSkillNodeLikeCpp),
    CoveredWithoutNode,
    Indeterminate(&'a SpellLearnSkillIndeterminateReasonLikeCpp),
    MissingCoverage,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellLearnSkillStoreLikeCpp {
    pub skill_by_spell_id: BTreeMap<u32, SpellLearnSkillNodeLikeCpp>,
    pub covered_spell_ids: BTreeSet<u32>,
    pub indeterminate_by_spell_id: BTreeMap<u32, SpellLearnSkillIndeterminateReasonLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLearnSkillLoadErrorKindLikeCpp {
    SkillOutOfRange { value: i32 },
    StepOutOfRange { value: i32 },
    DuplicateSourceSpell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSkillLoadErrorLikeCpp {
    pub spell_id: u32,
    pub kind: SpellLearnSkillLoadErrorKindLikeCpp,
}

impl SpellLearnSkillStoreLikeCpp {
    /// Build C++'s first matching `SKILL` / `DUAL_WIELD` node.
    ///
    /// The selected effect order remains faithful, while Rust deliberately
    /// rejects values that C++ would narrow into `uint16`: the complete
    /// acquisition catalog retains the source value so authorization can
    /// classify the spell as indeterminate instead of accepting a wrapped ID.
    pub fn from_spell_infos_like_cpp<I>(source_spells: I) -> SpellLearnSkillLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellLearnSkillSourceSpellInfoLikeCpp>,
    {
        let mut store = Self::default();
        let mut dbc_loaded_row_count = 0;
        let mut errors = Vec::new();

        for source_spell in source_spells {
            if !source_spell.difficulty_none {
                continue;
            }

            if !store.covered_spell_ids.insert(source_spell.spell_id) {
                if store
                    .skill_by_spell_id
                    .remove(&source_spell.spell_id)
                    .is_some()
                {
                    dbc_loaded_row_count -= 1;
                }
                store.indeterminate_by_spell_id.insert(
                    source_spell.spell_id,
                    SpellLearnSkillIndeterminateReasonLikeCpp::DuplicateSourceSpell,
                );
                errors.push(SpellLearnSkillLoadErrorLikeCpp {
                    spell_id: source_spell.spell_id,
                    kind: SpellLearnSkillLoadErrorKindLikeCpp::DuplicateSourceSpell,
                });
                continue;
            }
            for effect in source_spell.effects {
                let node = match effect.effect {
                    spell_effect_types::SPELL_EFFECT_SKILL => {
                        let Ok(skill) = u16::try_from(effect.misc_value) else {
                            store.indeterminate_by_spell_id.insert(
                                source_spell.spell_id,
                                SpellLearnSkillIndeterminateReasonLikeCpp::SkillOutOfRange {
                                    value: effect.misc_value,
                                },
                            );
                            errors.push(SpellLearnSkillLoadErrorLikeCpp {
                                spell_id: source_spell.spell_id,
                                kind: SpellLearnSkillLoadErrorKindLikeCpp::SkillOutOfRange {
                                    value: effect.misc_value,
                                },
                            });
                            break;
                        };
                        let Ok(step) = u16::try_from(effect.calc_value) else {
                            store.indeterminate_by_spell_id.insert(
                                source_spell.spell_id,
                                SpellLearnSkillIndeterminateReasonLikeCpp::StepOutOfRange {
                                    value: effect.calc_value,
                                },
                            );
                            errors.push(SpellLearnSkillLoadErrorLikeCpp {
                                spell_id: source_spell.spell_id,
                                kind: SpellLearnSkillLoadErrorKindLikeCpp::StepOutOfRange {
                                    value: effect.calc_value,
                                },
                            });
                            break;
                        };
                        SpellLearnSkillNodeLikeCpp {
                            skill,
                            step,
                            value: 0,
                            maxvalue: 0,
                        }
                    }
                    spell_effect_types::SPELL_EFFECT_DUAL_WIELD => SpellLearnSkillNodeLikeCpp {
                        skill: SKILL_DUAL_WIELD_LIKE_CPP,
                        step: 1,
                        value: 1,
                        maxvalue: 1,
                    },
                    _ => continue,
                };

                store
                    .indeterminate_by_spell_id
                    .remove(&source_spell.spell_id);
                store.skill_by_spell_id.insert(source_spell.spell_id, node);
                dbc_loaded_row_count += 1;
                break;
            }
        }

        SpellLearnSkillLoadOutcomeLikeCpp {
            store,
            dbc_loaded_row_count,
            errors,
        }
    }

    pub fn get_spell_learn_skill_like_cpp(
        &self,
        spell_id: u32,
    ) -> Option<&SpellLearnSkillNodeLikeCpp> {
        self.skill_by_spell_id.get(&spell_id)
    }

    pub fn mark_spell_learn_skill_indeterminate_like_cpp(
        &mut self,
        spell_id: u32,
        reason: SpellLearnSkillIndeterminateReasonLikeCpp,
    ) {
        self.skill_by_spell_id.remove(&spell_id);
        self.indeterminate_by_spell_id.insert(spell_id, reason);
    }

    pub fn spell_learn_skill_lookup_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellLearnSkillLookupLikeCpp<'_> {
        if let Some(reason) = self.indeterminate_by_spell_id.get(&spell_id) {
            return SpellLearnSkillLookupLikeCpp::Indeterminate(reason);
        }
        if let Some(node) = self.skill_by_spell_id.get(&spell_id) {
            return SpellLearnSkillLookupLikeCpp::Present(node);
        }
        if self.covered_spell_ids.contains(&spell_id) {
            SpellLearnSkillLookupLikeCpp::CoveredWithoutNode
        } else {
            SpellLearnSkillLookupLikeCpp::MissingCoverage
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSkillLoadOutcomeLikeCpp {
    pub store: SpellLearnSkillStoreLikeCpp,
    pub dbc_loaded_row_count: usize,
    pub errors: Vec<SpellLearnSkillLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellSqlRowLikeCpp {
    pub entry: u32,
    pub spell_id: u32,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellNodeLikeCpp {
    pub spell: u32,
    pub overrides_spell: u32,
    pub active: bool,
    pub auto_learned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellEffectLikeCpp {
    pub trigger_spell: u32,
    pub target_unit_pet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty_none: bool,
    pub is_talent: bool,
    pub is_passive: bool,
    pub has_skill_step_effect: bool,
    pub learn_spell_effects: Vec<SpellLearnSpellEffectLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLearnSpellLoadErrorKindLikeCpp {
    SqlSourceSpellMissing,
    SqlLearnedSpellMissing,
    SqlSourceIsTalent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellLoadErrorLikeCpp {
    pub row: SpellLearnSpellSqlRowLikeCpp,
    pub kind: SpellLearnSpellLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLearnSpellLoadWarningKindLikeCpp {
    RedundantSqlRowForSpellEffect {
        source_spell: u32,
        learned_spell: u32,
    },
    RedundantSqlRowForDb2 {
        source_spell: u32,
        learned_spell: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellLoadWarningLikeCpp {
    pub kind: SpellLearnSpellLoadWarningKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellLearnSpellStoreLikeCpp {
    pub learned_by_spell_id: BTreeMap<u32, Vec<SpellLearnSpellNodeLikeCpp>>,
}

impl SpellLearnSpellStoreLikeCpp {
    pub async fn load_like_cpp<SourceSpells, Db2Rows, SpellLookup, SpellExists>(
        db: &WorldDatabase,
        source_spells: SourceSpells,
        db2_rows: Db2Rows,
        mut spell_lookup: SpellLookup,
        spell_exists: SpellExists,
    ) -> Result<SpellLearnSpellLoadOutcomeLikeCpp>
    where
        SourceSpells: IntoIterator<Item = SpellLearnSourceSpellInfoLikeCpp>,
        Db2Rows: IntoIterator<Item = crate::spell_db2::SpellLearnSpellEntry>,
        SpellLookup: FnMut(u32) -> Option<SpellLearnSourceSpellInfoLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut result = db
            .direct_query(WorldStatements::SEL_SPELL_LEARN_SPELL.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellLearnSpellSqlRowLikeCpp {
                    entry: result.try_read::<u32>(0).unwrap_or(0),
                    spell_id: result.try_read::<u32>(1).unwrap_or(0),
                    active: result.try_read::<u8>(2).unwrap_or(0) != 0,
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_sources_like_cpp(
            rows,
            source_spells,
            db2_rows,
            &mut spell_lookup,
            spell_exists,
        ))
    }

    /// Compose the represented C++ learning graph.
    ///
    /// This intentionally repairs the legacy `SpellMgr::LoadSpellLearnSpells`
    /// empty-query early return: an empty world table contributes zero custom
    /// rows but does not suppress canonical `SpellEffect` or
    /// `SpellLearnSpell.db2` edges.
    pub fn from_sources_like_cpp<SqlRows, SourceSpells, Db2Rows, SpellLookup, SpellExists>(
        sql_rows: SqlRows,
        source_spells: SourceSpells,
        db2_rows: Db2Rows,
        mut spell_lookup: SpellLookup,
        mut spell_exists: SpellExists,
    ) -> SpellLearnSpellLoadOutcomeLikeCpp
    where
        SqlRows: IntoIterator<Item = SpellLearnSpellSqlRowLikeCpp>,
        SourceSpells: IntoIterator<Item = SpellLearnSourceSpellInfoLikeCpp>,
        Db2Rows: IntoIterator<Item = crate::spell_db2::SpellLearnSpellEntry>,
        SpellLookup: FnMut(u32) -> Option<SpellLearnSourceSpellInfoLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut sql_loaded_row_count = 0;
        let mut dbc_loaded_row_count = 0;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let sql_rows = sql_rows.into_iter().collect::<Vec<_>>();
        let sql_result_empty = sql_rows.is_empty();

        for row in sql_rows {
            let Some(source_spell) = spell_lookup(row.entry) else {
                errors.push(SpellLearnSpellLoadErrorLikeCpp {
                    row,
                    kind: SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceSpellMissing,
                });
                continue;
            };

            if !spell_exists(row.spell_id) {
                errors.push(SpellLearnSpellLoadErrorLikeCpp {
                    row,
                    kind: SpellLearnSpellLoadErrorKindLikeCpp::SqlLearnedSpellMissing,
                });
                continue;
            }

            if source_spell.is_talent {
                errors.push(SpellLearnSpellLoadErrorLikeCpp {
                    row,
                    kind: SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceIsTalent,
                });
                continue;
            }

            store
                .learned_by_spell_id
                .entry(row.entry)
                .or_default()
                .push(SpellLearnSpellNodeLikeCpp {
                    spell: row.spell_id,
                    overrides_spell: 0,
                    active: row.active,
                    auto_learned: false,
                });
            sql_loaded_row_count += 1;
        }

        let db_spell_learn_spells = store.learned_by_spell_id.clone();

        for source_spell in source_spells {
            if !source_spell.difficulty_none {
                continue;
            }

            for effect in source_spell.learn_spell_effects {
                let dbc_node = SpellLearnSpellNodeLikeCpp {
                    spell: effect.trigger_spell,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: effect.target_unit_pet
                        || source_spell.is_talent
                        || source_spell.is_passive
                        || source_spell.has_skill_step_effect,
                };

                if !spell_exists(dbc_node.spell) {
                    continue;
                }

                if Self::contains_learn_pair_in_map(
                    &db_spell_learn_spells,
                    source_spell.spell_id,
                    dbc_node.spell,
                ) {
                    warnings.push(SpellLearnSpellLoadWarningLikeCpp {
                        kind:
                            SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForSpellEffect {
                                source_spell: source_spell.spell_id,
                                learned_spell: dbc_node.spell,
                            },
                    });
                    continue;
                }

                store
                    .learned_by_spell_id
                    .entry(source_spell.spell_id)
                    .or_default()
                    .push(dbc_node);
                dbc_loaded_row_count += 1;
            }
        }

        for db2_row in db2_rows {
            let source_spell = db2_row.spell_id as u32;
            let learned_spell = db2_row.learn_spell_id as u32;

            if !spell_exists(source_spell) || !spell_exists(learned_spell) {
                continue;
            }

            if db_spell_learn_spells
                .get(&source_spell)
                .is_some_and(|nodes| {
                    nodes
                        .iter()
                        .any(|node| node.spell as i32 == db2_row.learn_spell_id)
                })
            {
                warnings.push(SpellLearnSpellLoadWarningLikeCpp {
                    kind: SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForDb2 {
                        source_spell,
                        learned_spell: db2_row.learn_spell_id,
                    },
                });
                continue;
            }

            if Self::contains_learn_pair_in_map(
                &store.learned_by_spell_id,
                source_spell,
                learned_spell,
            ) {
                continue;
            }

            store
                .learned_by_spell_id
                .entry(source_spell)
                .or_default()
                .push(SpellLearnSpellNodeLikeCpp {
                    spell: learned_spell,
                    overrides_spell: db2_row.overrides_spell_id as u32,
                    active: true,
                    auto_learned: false,
                });
            dbc_loaded_row_count += 1;
        }

        SpellLearnSpellLoadOutcomeLikeCpp {
            store,
            sql_loaded_row_count,
            dbc_loaded_row_count,
            sql_result_empty,
            errors,
            warnings,
        }
    }

    fn contains_learn_pair_in_map(
        map: &BTreeMap<u32, Vec<SpellLearnSpellNodeLikeCpp>>,
        source_spell: u32,
        learned_spell: u32,
    ) -> bool {
        map.get(&source_spell)
            .is_some_and(|nodes| nodes.iter().any(|node| node.spell == learned_spell))
    }

    pub fn get_spell_learn_spell_map_bounds_like_cpp(
        &self,
        spell_id: u32,
    ) -> &[SpellLearnSpellNodeLikeCpp] {
        self.learned_by_spell_id
            .get(&spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn is_spell_learn_spell_like_cpp(&self, spell_id: u32) -> bool {
        self.learned_by_spell_id.contains_key(&spell_id)
    }

    pub fn is_spell_learn_to_spell_like_cpp(&self, spell_id1: u32, spell_id2: u32) -> bool {
        self.get_spell_learn_spell_map_bounds_like_cpp(spell_id1)
            .iter()
            .any(|node| node.spell == spell_id2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSpellLoadOutcomeLikeCpp {
    pub store: SpellLearnSpellStoreLikeCpp,
    pub sql_loaded_row_count: usize,
    pub dbc_loaded_row_count: usize,
    pub sql_result_empty: bool,
    pub errors: Vec<SpellLearnSpellLoadErrorLikeCpp>,
    pub warnings: Vec<SpellLearnSpellLoadWarningLikeCpp>,
}
