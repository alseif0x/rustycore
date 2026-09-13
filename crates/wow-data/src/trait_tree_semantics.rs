//! Immutable TraitMgr semantic validation and login fallback inputs.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    TraitCondEntry, TraitCostEntry, TraitCurrencyEntry, TraitCurrencySourceStore,
    TraitCurrencyStore, TraitTreeSkillLineIndexLikeCpp, TraitTreeXTraitCurrencyStore,
};

/// Packet-independent input used by the semantic TraitMgr validator. Keeping
/// this shape in `wow-data` prevents the catalog owner from depending on the
/// wire packet crate while still preserving C++'s signed fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraitConfigEntryLikeCpp {
    pub trait_node_id: i32,
    pub trait_node_entry_id: i32,
    pub rank: i32,
    pub granted_ranks: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitConfigValidationResultLikeCpp {
    Ok,
    Unknown,
    NotEnoughTalentsInPrimaryTree,
}

/// Canonical Player facts consulted by `TraitMgr::MeetsTraitCondition` and
/// `FillOwnedCurrenciesMap`. The references are snapshots made while the
/// represented Player is exclusively borrowed; no second mutable authority is
/// introduced by the validation operation.
#[derive(Clone, Copy)]
pub struct TraitPlayerFactsLikeCpp<'a> {
    pub level: i32,
    pub primary_specialization_id: u32,
    pub money: u64,
    pub currency_quantities: &'a BTreeMap<u32, i32>,
    pub rewarded_quest_ids: &'a BTreeSet<u32>,
    pub achievement_ids: &'a BTreeSet<u32>,
}

impl TraitTreeSkillLineIndexLikeCpp {
    /// Add the immutable currency/source side of the C++ `TraitMgr::Load`
    /// projection. Currency sources are retained by value so login validation
    /// never borrows a transient DB2 loader or creates a second mutable owner.
    pub fn with_trait_currency_data_like_cpp(
        mut self,
        currencies: &TraitCurrencyStore,
        currency_sources: &TraitCurrencySourceStore,
        tree_currencies: &TraitTreeXTraitCurrencyStore,
        spec_set_members: &crate::SpecSetMemberStore,
    ) -> Self {
        let mut currency_by_id = BTreeMap::<u32, TraitCurrencyEntry>::new();
        for currency in currencies.iter() {
            currency_by_id.insert(currency.id, currency.clone());
        }
        for row in tree_currencies.iter() {
            let Some(currency_id) = u32::try_from(row.trait_currency_id).ok() else {
                continue;
            };
            let Some(currency) = currency_by_id.get(&currency_id) else {
                continue;
            };
            self.currencies_by_tree
                .entry(row.trait_tree_id)
                .or_default()
                .push(currency.clone());
        }
        for rows in self.currencies_by_tree.values_mut() {
            rows.sort_unstable_by_key(|row| row.id);
            rows.dedup_by_key(|row| row.id);
        }
        for source in currency_sources.iter() {
            self.currency_sources_by_currency
                .entry(source.trait_currency_id)
                .or_default()
                .push(source.clone());
        }
        for rows in self.currency_sources_by_currency.values_mut() {
            rows.sort_unstable_by_key(|row| (row.order_index, row.id));
        }
        self.spec_set_members = spec_set_members
            .entries()
            .map(|row| (row.spec_set_id as i32, row.chr_specialization_id))
            .collect();
        self
    }

    /// C++ `TraitMgr::IsValidEntry`: reject malformed persisted rows before
    /// they are attached to a config. Tree membership is validated separately
    /// by `ValidateConfig`, because the native helper itself only resolves the
    /// node and its entry/rank tuple.
    pub fn is_valid_entry_like_cpp(&self, entry: &TraitConfigEntryLikeCpp) -> bool {
        let (Some(node_id), Some(entry_id)) = (
            u32::try_from(entry.trait_node_id).ok(),
            u32::try_from(entry.trait_node_entry_id).ok(),
        ) else {
            return false;
        };
        let Some(_node) = self.nodes.get(&node_id) else {
            return false;
        };
        if !self.entries_for_node_like_cpp(node_id).contains(&entry_id) {
            return false;
        }
        let Some(node_entry) = self.node_entries.get(&entry_id) else {
            return false;
        };
        node_entry.max_ranks >= entry.rank.saturating_add(entry.granted_ranks)
    }

    fn spent_currencies_like_cpp(&self, entries: &[TraitConfigEntryLikeCpp]) -> BTreeMap<u32, i32> {
        let mut spent = BTreeMap::new();
        for entry in entries {
            let Some(node_id) = u32::try_from(entry.trait_node_id).ok() else {
                continue;
            };
            let Some(node) = self.nodes.get(&node_id) else {
                continue;
            };
            let rank = entry.rank.max(0);
            for group_id in self.groups_for_node_like_cpp(node_id) {
                for cost_id in self.cost_ids_for_group_like_cpp(*group_id) {
                    if let Some(cost) = self.costs.get(cost_id) {
                        add_trait_cost(&mut spent, cost, rank);
                    }
                }
            }
            if let Ok(entry_id) = u32::try_from(entry.trait_node_entry_id) {
                if self.entries_for_node_like_cpp(node_id).contains(&entry_id) {
                    for cost_id in self.cost_ids_for_entry_like_cpp(entry_id) {
                        if let Some(cost) = self.costs.get(cost_id) {
                            add_trait_cost(&mut spent, cost, rank);
                        }
                    }
                }
            }
            for cost_id in self.cost_ids_for_node_like_cpp(node_id) {
                if let Some(cost) = self.costs.get(cost_id) {
                    add_trait_cost(&mut spent, cost, rank);
                }
            }
            for cost_id in self.cost_ids_for_tree_like_cpp(node.trait_tree_id) {
                if let Some(cost) = self.costs.get(cost_id) {
                    add_trait_cost(&mut spent, cost, rank);
                }
            }
        }
        spent
    }

    fn condition_meets_like_cpp(
        &self,
        config_type: i32,
        config_specialization_id: i32,
        condition: &TraitCondEntry,
        facts: &TraitPlayerFactsLikeCpp<'_>,
        spent: &mut Option<BTreeMap<u32, i32>>,
        config_entries: &[TraitConfigEntryLikeCpp],
    ) -> bool {
        if condition.quest_id > 0
            && !facts
                .rewarded_quest_ids
                .contains(&(condition.quest_id as u32))
        {
            return false;
        }
        if condition.achievement_id > 0
            && !facts
                .achievement_ids
                .contains(&(condition.achievement_id as u32))
        {
            return false;
        }
        if condition.spec_set_id > 0 {
            let specialization = if config_type == 1 {
                config_specialization_id
            } else {
                facts.primary_specialization_id as i32
            };
            if !self
                .spec_set_members
                .contains(&(condition.spec_set_id, specialization))
            {
                return false;
            }
        }
        if condition.trait_currency_id > 0 && condition.spent_amount_required > 0 {
            let spent_map =
                spent.get_or_insert_with(|| self.spent_currencies_like_cpp(config_entries));
            let amount = spent_map
                .get(&(condition.trait_currency_id as u32))
                .copied()
                .unwrap_or_default();
            if (condition.trait_node_group_id > 0 || condition.trait_node_id > 0)
                && amount < condition.spent_amount_required
            {
                return false;
            }
        }
        condition.required_level <= 0 || facts.level >= condition.required_level
    }

    fn owned_currencies_like_cpp(
        &self,
        tree_ids: &[u32],
        config_entries: &[TraitConfigEntryLikeCpp],
        facts: &TraitPlayerFactsLikeCpp<'_>,
    ) -> BTreeMap<u32, i32> {
        let mut owned = BTreeMap::new();
        let active_entries = |entry_id: i32| {
            config_entries.iter().any(|entry| {
                entry.trait_node_entry_id == entry_id && (entry.rank > 0 || entry.granted_ranks > 0)
            })
        };
        for tree_id in tree_ids {
            for currency in self.currencies_by_tree.get(tree_id).into_iter().flatten() {
                match currency.currency_type {
                    0 => {
                        let amount = facts.money.min(i32::MAX as u64) as i32;
                        let slot: &mut i32 = owned.entry(currency.id).or_default();
                        *slot = slot.saturating_add(amount);
                    }
                    1 => {
                        let amount = facts
                            .currency_quantities
                            .get(&(currency.currency_types_id as u32))
                            .copied()
                            .unwrap_or_default();
                        *owned.entry(currency.id).or_default() = owned
                            .get(&currency.id)
                            .copied()
                            .unwrap_or_default()
                            .saturating_add(amount);
                    }
                    2 => {
                        if let Some(sources) = self.currency_sources_by_currency.get(&currency.id) {
                            for source in sources {
                                if source.quest_id > 0
                                    && !facts.rewarded_quest_ids.contains(&(source.quest_id as u32))
                                {
                                    continue;
                                }
                                if source.achievement_id > 0
                                    && !facts
                                        .achievement_ids
                                        .contains(&(source.achievement_id as u32))
                                {
                                    continue;
                                }
                                if source.player_level > 0 && facts.level < source.player_level {
                                    continue;
                                }
                                if source.trait_node_entry_id > 0
                                    && !active_entries(source.trait_node_entry_id)
                                {
                                    continue;
                                }
                                *owned.entry(source.trait_currency_id).or_default() = owned
                                    .get(&source.trait_currency_id)
                                    .copied()
                                    .unwrap_or_default()
                                    .saturating_add(source.amount);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        owned
    }

    /// C++ `TraitMgr::ValidateConfig`, restricted to the immutable catalog and
    /// a snapshot of canonical Player facts. The caller supplies the trees
    /// resolved for the config's explicit type.
    pub fn validate_config_like_cpp(
        &self,
        tree_ids: &[u32],
        config_type: i32,
        config_specialization_id: i32,
        entries: &[TraitConfigEntryLikeCpp],
        facts: &TraitPlayerFactsLikeCpp<'_>,
    ) -> TraitConfigValidationResultLikeCpp {
        if tree_ids.is_empty() || !self.graph_loaded {
            return TraitConfigValidationResultLikeCpp::Unknown;
        }
        let tree_ids = tree_ids.iter().copied().collect::<BTreeSet<_>>();
        let mut spent = Some(self.spent_currencies_like_cpp(entries));
        let meets_conditions = |condition_ids: &[u32], spent: &mut Option<BTreeMap<u32, i32>>| {
            let mut has_conditions = false;
            for condition_id in condition_ids {
                let Some(condition) = self.conditions.get(condition_id) else {
                    return false;
                };
                if condition.cond_type == 0 || condition.cond_type == 1 {
                    if self.condition_meets_like_cpp(
                        config_type,
                        config_specialization_id,
                        condition,
                        facts,
                        spent,
                        entries,
                    ) {
                        return true;
                    }
                    has_conditions = true;
                }
            }
            !has_conditions
        };
        let node_fully_filled = |node_id: u32| {
            let Some(node) = self.nodes.get(&node_id) else {
                return false;
            };
            let totals = |entry_id: u32| {
                entries
                    .iter()
                    .find(|entry| entry.trait_node_entry_id == entry_id as i32)
                    .map(|entry| entry.rank.saturating_add(entry.granted_ranks))
            };
            if node.node_type == 2 {
                self.entries_for_node_like_cpp(node_id)
                    .iter()
                    .any(|entry_id| {
                        self.node_entries
                            .get(entry_id)
                            .is_some_and(|entry| totals(*entry_id) == Some(entry.max_ranks))
                    })
            } else {
                self.entries_for_node_like_cpp(node_id)
                    .iter()
                    .all(|entry_id| {
                        self.node_entries
                            .get(entry_id)
                            .is_some_and(|entry| totals(*entry_id) == Some(entry.max_ranks))
                    })
            }
        };

        for entry in entries {
            let (Some(node_id), Some(entry_id)) = (
                u32::try_from(entry.trait_node_id).ok(),
                u32::try_from(entry.trait_node_entry_id).ok(),
            ) else {
                return TraitConfigValidationResultLikeCpp::Unknown;
            };
            let Some(node) = self.nodes.get(&node_id) else {
                return TraitConfigValidationResultLikeCpp::Unknown;
            };
            if !tree_ids.contains(&node.trait_tree_id)
                || !self.entries_for_node_like_cpp(node_id).contains(&entry_id)
            {
                return TraitConfigValidationResultLikeCpp::Unknown;
            }
            let Some(node_entry) = self.node_entries.get(&entry_id) else {
                return TraitConfigValidationResultLikeCpp::Unknown;
            };
            if entry.rank < 0
                || entry.granted_ranks < 0
                || entry.rank.saturating_add(entry.granted_ranks) > node_entry.max_ranks
            {
                return TraitConfigValidationResultLikeCpp::Unknown;
            }
            if node.node_type == 2
                && entries
                    .iter()
                    .filter(|candidate| candidate.trait_node_id == entry.trait_node_id)
                    .count()
                    != 1
            {
                return TraitConfigValidationResultLikeCpp::Unknown;
            }
            if !self
                .entries_for_node_like_cpp(node_id)
                .iter()
                .all(|candidate| {
                    meets_conditions(
                        self.condition_ids_for_entry_like_cpp(*candidate),
                        &mut spent,
                    )
                })
                || !meets_conditions(self.condition_ids_for_node_like_cpp(node_id), &mut spent)
            {
                return TraitConfigValidationResultLikeCpp::Unknown;
            }
            if self
                .groups_for_node_like_cpp(node_id)
                .iter()
                .any(|group_id| {
                    !meets_conditions(self.condition_ids_for_group_like_cpp(*group_id), &mut spent)
                })
            {
                return TraitConfigValidationResultLikeCpp::Unknown;
            }
            let parents = self.parent_nodes_for_node_like_cpp(node_id);
            if !parents.is_empty() {
                let mut has_any_parent = false;
                for (parent_id, edge_type) in parents {
                    if !node_fully_filled(*parent_id) {
                        if *edge_type == 2 {
                            return TraitConfigValidationResultLikeCpp::NotEnoughTalentsInPrimaryTree;
                        }
                        continue;
                    }
                    has_any_parent = true;
                }
                if !has_any_parent {
                    return TraitConfigValidationResultLikeCpp::NotEnoughTalentsInPrimaryTree;
                }
            }
        }
        let owned = self.owned_currencies_like_cpp(
            &tree_ids.iter().copied().collect::<Vec<_>>(),
            entries,
            facts,
        );
        for (currency_id, spent_amount) in spent.as_ref().into_iter().flatten() {
            if *spent_amount <= 0 {
                continue;
            }
            if self
                .currencies_by_tree
                .values()
                .flatten()
                .find(|currency| currency.id == *currency_id)
                .is_some_and(|currency| currency.currency_type == 2)
                && owned.get(currency_id).copied().unwrap_or_default() < *spent_amount
            {
                return TraitConfigValidationResultLikeCpp::NotEnoughTalentsInPrimaryTree;
            }
        }
        TraitConfigValidationResultLikeCpp::Ok
    }

    /// C++ `TraitMgr::GetGrantedTraitEntriesForConfig` for the resolved tree
    /// set. Entries are returned in deterministic tree/node/store order.
    pub fn granted_entries_for_config_like_cpp(
        &self,
        tree_ids: &[u32],
        config_type: i32,
        config_specialization_id: i32,
        entries: &[TraitConfigEntryLikeCpp],
        facts: &TraitPlayerFactsLikeCpp<'_>,
    ) -> Vec<TraitConfigEntryLikeCpp> {
        let mut granted = Vec::new();
        let mut spent = None;
        let mut add = |node_id: u32, entry_id: u32, ranks: i32| {
            if ranks <= 0 {
                return;
            }
            if let Some(existing) =
                granted
                    .iter_mut()
                    .find(|entry: &&mut TraitConfigEntryLikeCpp| {
                        entry.trait_node_id == node_id as i32
                            && entry.trait_node_entry_id == entry_id as i32
                    })
            {
                existing.granted_ranks = existing.granted_ranks.saturating_add(ranks);
            } else {
                granted.push(TraitConfigEntryLikeCpp {
                    trait_node_id: node_id as i32,
                    trait_node_entry_id: entry_id as i32,
                    rank: 0,
                    granted_ranks: ranks,
                });
            }
        };
        for tree_id in tree_ids {
            for node_id in self.nodes_for_tree_like_cpp(*tree_id) {
                for entry_id in self.entries_for_node_like_cpp(*node_id) {
                    for condition_id in self.condition_ids_for_entry_like_cpp(*entry_id) {
                        if let Some(condition) = self.conditions.get(condition_id)
                            && condition.cond_type == 2
                            && self.condition_meets_like_cpp(
                                config_type,
                                config_specialization_id,
                                condition,
                                facts,
                                &mut spent,
                                entries,
                            )
                        {
                            add(*node_id, *entry_id, condition.granted_ranks);
                        }
                    }
                }
                for condition_id in self.condition_ids_for_node_like_cpp(*node_id) {
                    if let Some(condition) = self.conditions.get(condition_id)
                        && condition.cond_type == 2
                        && self.condition_meets_like_cpp(
                            config_type,
                            config_specialization_id,
                            condition,
                            facts,
                            &mut spent,
                            entries,
                        )
                    {
                        for entry_id in self.entries_for_node_like_cpp(*node_id) {
                            add(*node_id, *entry_id, condition.granted_ranks);
                        }
                    }
                }
                for group_id in self.groups_for_node_like_cpp(*node_id) {
                    for condition_id in self.condition_ids_for_group_like_cpp(*group_id) {
                        if let Some(condition) = self.conditions.get(condition_id)
                            && condition.cond_type == 2
                            && self.condition_meets_like_cpp(
                                config_type,
                                config_specialization_id,
                                condition,
                                facts,
                                &mut spent,
                                entries,
                            )
                        {
                            for entry_id in self.entries_for_node_like_cpp(*node_id) {
                                add(*node_id, *entry_id, condition.granted_ranks);
                            }
                        }
                    }
                }
            }
        }
        granted
    }
}

fn add_trait_cost(target: &mut BTreeMap<u32, i32>, cost: &TraitCostEntry, rank: i32) {
    let Some(currency_id) = u32::try_from(cost.trait_currency_id).ok() else {
        return;
    };
    let amount = cost.amount.saturating_mul(rank);
    *target.entry(currency_id).or_default() = target
        .get(&currency_id)
        .copied()
        .unwrap_or_default()
        .saturating_add(amount);
}
