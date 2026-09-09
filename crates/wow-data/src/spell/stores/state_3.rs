//! C++-shaped spell stores state definitions, part 3 of 4.
//!
//! Separated from the stores.rs root under #646. Behaviour is preserved.

use super::*;

impl SpellGroupStoreLikeCpp {
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

    pub(super) fn collect_spells_in_group_like_cpp(
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
    pub fn from_rows_and_stores_like_cpp<I>(
        rows: I,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
        spell_aura_options: &crate::spell_db2::SpellAuraOptionsStore,
        spell_misc: &crate::spell_db2::SpellMiscStore,
        spell_class_options: &crate::spell_db2::SpellClassOptionsStore,
        spell_procs_per_minute: &crate::spell_db2::SpellProcsPerMinuteStore,
    ) -> SpellProcLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellProcRowLikeCpp>,
    {
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

        Self::from_rows_and_spell_infos_like_cpp(
            rows,
            |spell_id| spell_infos_by_id.get(&spell_id).cloned(),
            spell_infos,
        )
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
    pub(super) spell_effects_by_difficulty: HashMap<(i32, u8), Vec<SpellEffectInfo>>,
    pub(super) spell_misc_attributes: HashMap<i32, [u32; 15]>,
    pub(super) spell_misc_attributes_by_difficulty: HashMap<(i32, u8), [u32; 15]>,
    pub(super) spell_interrupt_flags: HashMap<(i32, u8), ([u32; 2], [u32; 2])>,
    pub(super) spell_interrupt_rows_by_id: BTreeMap<u32, SpellInterruptRowLikeCpp>,
    pub(super) spell_hit_categories_by_difficulty: HashMap<(i32, u8), SpellHitCategoriesRowLikeCpp>,
    pub(super) spell_hit_misc_by_difficulty: HashMap<(i32, u8), SpellHitMiscRowLikeCpp>,
    pub(super) spell_hit_effect_mechanics_by_difficulty:
        HashMap<(i32, u8), BTreeMap<u32, SpellHitEffectMechanicRowLikeCpp>>,
    pub(super) spell_shapeshift_masks: HashMap<i32, (u64, u64)>,
    pub(super) implicit_target_conditions: HashMap<(i32, u32), ConditionsReference>,
}

/// Effective DB2 authorities consumed together by C++
/// `SpellMgr::LoadSpellInfoStore`.
///
/// The stores own DB2 parsing, row replacement and final tombstones. The
/// composition root supplies already-decoded rows without exposing a database
/// or persistence contract to `wow-data`.
pub struct EffectiveCoreSpellDb2StoresLikeCpp {
    pub(super) spell_categories: crate::spell_db2::SpellCategoriesStore,
    pub(super) spell_misc: crate::spell_db2::SpellMiscStore,
    pub(super) spell_effect: crate::spell_db2::SpellEffectDb2Store,
    pub(super) spell_shapeshift: crate::spell_db2::SpellShapeshiftStore,
    pub(super) spell_interrupts: crate::spell_db2::SpellInterruptsStore,
    pub(super) spell_cast_times: crate::spell_db2::SpellCastTimesStore,
    pub(super) spell_cooldowns: crate::spell_db2::SpellCooldownsStore,
    pub(super) spell_casting_requirements: crate::spell_db2::SpellCastingRequirementsStore,
    pub(super) spell_power: crate::spell_db2::SpellPowerStore,
    pub(super) spell_power_difficulty: crate::spell_db2::SpellPowerDifficultyStore,
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
