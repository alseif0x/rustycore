//! C++-shaped spell stores state definitions, part 2 of 4.
//!
//! Separated from the stores.rs root under #646. Behaviour is preserved.

use super::*;

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
    pub(super) areas: Vec<SpellAreaLikeCpp>,
    pub(super) area_indices_by_spell_id: BTreeMap<u32, Vec<usize>>,
    pub(super) area_indices_by_quest_start_or_end: BTreeMap<u32, Vec<usize>>,
    pub(super) area_indices_by_quest_end: BTreeMap<u32, Vec<usize>>,
    pub(super) area_indices_by_aura_spell: BTreeMap<u32, Vec<usize>>,
    pub(super) area_indices_by_area_id: BTreeMap<u32, Vec<usize>>,
}

impl SpellAreaStoreLikeCpp {
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

    pub(super) fn lookup_indices_like_cpp(
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

    pub(super) fn has_similar_requirements_like_cpp(&self, spell_area: &SpellAreaLikeCpp) -> bool {
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

    pub(super) fn has_autocast_aura_chain_like_cpp(&self, spell_area: &SpellAreaLikeCpp) -> bool {
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

    pub(super) fn insert_like_cpp(&mut self, spell_area: SpellAreaLikeCpp) {
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
