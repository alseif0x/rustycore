//! Trait tree DB2 readers.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::skill_talent::SkillLineXTraitTreeStore;
use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitCondEntry {
    pub id: u32,
    pub cond_type: i32,
    pub trait_tree_id: u32,
    pub granted_ranks: i32,
    pub quest_id: i32,
    pub achievement_id: i32,
    pub spec_set_id: i32,
    pub trait_node_group_id: i32,
    pub trait_node_id: i32,
    pub trait_currency_id: i32,
    pub spent_amount_required: i32,
    pub flags: i32,
    pub required_level: i32,
    pub free_shared_string_id: i32,
    pub spend_more_shared_string_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitCostEntry {
    pub id: u32,
    pub internal_name: String,
    pub amount: i32,
    pub trait_currency_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitCurrencyEntry {
    pub id: u32,
    pub currency_type: i32,
    pub currency_types_id: i32,
    pub flags: i32,
    pub icon: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitCurrencySourceEntry {
    pub id: u32,
    pub requirement: String,
    pub trait_currency_id: u32,
    pub amount: i32,
    pub quest_id: i32,
    pub achievement_id: i32,
    pub player_level: i32,
    pub trait_node_entry_id: i32,
    pub order_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDefinitionEntry {
    pub id: u32,
    pub override_name: String,
    pub override_subtext: String,
    pub override_description: String,
    pub spell_id: i32,
    pub override_icon: i32,
    pub overrides_spell_id: i32,
    pub visible_spell_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDefinitionEffectPointsEntry {
    pub id: u32,
    pub trait_definition_id: u32,
    pub effect_index: i32,
    pub operation_type: i32,
    pub curve_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitEdgeEntry {
    pub id: u32,
    pub visual_style: i32,
    pub left_trait_node_id: u32,
    pub right_trait_node_id: i32,
    pub edge_type: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeEntry {
    pub id: u32,
    pub trait_tree_id: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub node_type: u8,
    pub flags: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeEntryEntry {
    pub id: u32,
    pub trait_definition_id: i32,
    pub max_ranks: i32,
    pub node_entry_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeEntryXTraitCondEntry {
    pub id: u32,
    pub trait_cond_id: i32,
    pub trait_node_entry_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeEntryXTraitCostEntry {
    pub id: u32,
    pub trait_node_entry_id: u32,
    pub trait_cost_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeGroupEntry {
    pub id: u32,
    pub trait_tree_id: u32,
    pub flags: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeGroupXTraitCondEntry {
    pub id: u32,
    pub trait_cond_id: i32,
    pub trait_node_group_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeGroupXTraitCostEntry {
    pub id: u32,
    pub trait_node_group_id: u32,
    pub trait_cost_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeGroupXTraitNodeEntry {
    pub id: u32,
    pub trait_node_group_id: u32,
    pub trait_node_id: i32,
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeXTraitCondEntry {
    pub id: u32,
    pub trait_cond_id: i32,
    pub trait_node_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeXTraitCostEntry {
    pub id: u32,
    pub trait_node_id: u32,
    pub trait_cost_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitNodeXTraitNodeEntryEntry {
    pub id: u32,
    pub trait_node_id: u32,
    pub trait_node_entry_id: i32,
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitTreeEntry {
    pub id: u32,
    pub trait_system_id: u32,
    pub unused1000_1: i32,
    pub first_trait_node_id: i32,
    pub player_condition_id: i32,
    pub flags: i32,
    pub unused1000_2: f32,
    pub unused1000_3: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitTreeLoadoutEntry {
    pub id: u32,
    pub trait_tree_id: u32,
    pub chr_specialization_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitTreeLoadoutEntryEntry {
    pub id: u32,
    pub trait_tree_loadout_id: u32,
    pub selected_trait_node_id: i32,
    pub selected_trait_node_entry_id: i32,
    pub num_points: i32,
    pub order_index: i32,
}

/// Immutable selection from `TraitTreeLoadoutEntry.db2`, retained in the
/// process-owned TraitMgr projection.  The runtime does not expose DB2 rows
/// directly to Player state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraitTreeLoadoutSelectionLikeCpp {
    pub trait_tree_id: u32,
    pub selected_trait_node_id: i32,
    pub selected_trait_node_entry_id: i32,
    pub num_points: i32,
    pub order_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitTreeXTraitCostEntry {
    pub id: u32,
    pub trait_tree_id: u32,
    pub trait_cost_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitTreeXTraitCurrencyEntry {
    pub id: u32,
    pub index: i32,
    pub trait_tree_id: u32,
    pub trait_currency_id: i32,
}

macro_rules! db2_store {
    ($store:ident, $entry:ty) => {
        pub struct $store {
            entries: HashMap<u32, $entry>,
        }

        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                }
            }

            pub fn get(&self, id: u32) -> Option<&$entry> {
                self.entries.get(&id)
            }

            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }

            pub fn iter(&self) -> impl Iterator<Item = &$entry> {
                self.entries.values()
            }
        }
    };
}

db2_store!(TraitCondStore, TraitCondEntry);
db2_store!(TraitCostStore, TraitCostEntry);
db2_store!(TraitCurrencyStore, TraitCurrencyEntry);
db2_store!(TraitCurrencySourceStore, TraitCurrencySourceEntry);
db2_store!(TraitDefinitionStore, TraitDefinitionEntry);
db2_store!(
    TraitDefinitionEffectPointsStore,
    TraitDefinitionEffectPointsEntry
);
db2_store!(TraitEdgeStore, TraitEdgeEntry);
db2_store!(TraitNodeStore, TraitNodeEntry);
db2_store!(TraitNodeEntryStore, TraitNodeEntryEntry);
db2_store!(TraitNodeEntryXTraitCondStore, TraitNodeEntryXTraitCondEntry);
db2_store!(TraitNodeEntryXTraitCostStore, TraitNodeEntryXTraitCostEntry);
db2_store!(TraitNodeGroupStore, TraitNodeGroupEntry);
db2_store!(TraitNodeGroupXTraitCondStore, TraitNodeGroupXTraitCondEntry);
db2_store!(TraitNodeGroupXTraitCostStore, TraitNodeGroupXTraitCostEntry);
db2_store!(TraitNodeGroupXTraitNodeStore, TraitNodeGroupXTraitNodeEntry);
db2_store!(TraitNodeXTraitCondStore, TraitNodeXTraitCondEntry);
db2_store!(TraitNodeXTraitCostStore, TraitNodeXTraitCostEntry);
db2_store!(TraitNodeXTraitNodeEntryStore, TraitNodeXTraitNodeEntryEntry);
db2_store!(TraitTreeStore, TraitTreeEntry);
db2_store!(TraitTreeLoadoutStore, TraitTreeLoadoutEntry);
db2_store!(TraitTreeLoadoutEntryStore, TraitTreeLoadoutEntryEntry);
db2_store!(TraitTreeXTraitCostStore, TraitTreeXTraitCostEntry);
db2_store!(TraitTreeXTraitCurrencyStore, TraitTreeXTraitCurrencyEntry);

#[path = "trait_tree_semantics.rs"]
mod trait_tree_semantics;
pub use trait_tree_semantics::{
    TraitConfigEntryLikeCpp, TraitConfigValidationResultLikeCpp, TraitPlayerFactsLikeCpp,
};

/// The startup projection built by C++ `TraitMgr::Load` from
/// `SkillLineXTraitTree`. It deliberately stores only validated immutable IDs;
/// trait rules, costs and conditions remain owned by their respective stores.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraitTreeSkillLineIndexLikeCpp {
    trees_by_skill_line: BTreeMap<u32, Vec<u32>>,
    trees_by_trait_system: BTreeMap<u32, Vec<u32>>,
    skill_line_by_class: BTreeMap<u8, u32>,
    nodes_by_tree: BTreeMap<u32, Vec<u32>>,
    entries_by_node: BTreeMap<u32, Vec<u32>>,
    groups_by_node: BTreeMap<u32, Vec<u32>>,
    parents_by_node: BTreeMap<u32, Vec<(u32, i32)>>,
    tree_costs: BTreeMap<u32, Vec<u32>>,
    node_costs: BTreeMap<u32, Vec<u32>>,
    group_costs: BTreeMap<u32, Vec<u32>>,
    entry_costs: BTreeMap<u32, Vec<u32>>,
    node_conditions: BTreeMap<u32, Vec<u32>>,
    group_conditions: BTreeMap<u32, Vec<u32>>,
    entry_conditions: BTreeMap<u32, Vec<u32>>,
    loadouts_by_specialization: BTreeMap<i32, Vec<TraitTreeLoadoutSelectionLikeCpp>>,
    trees: BTreeMap<u32, TraitTreeEntry>,
    nodes: BTreeMap<u32, TraitNodeEntry>,
    node_entries: BTreeMap<u32, TraitNodeEntryEntry>,
    groups: BTreeMap<u32, TraitNodeGroupEntry>,
    costs: BTreeMap<u32, TraitCostEntry>,
    conditions: BTreeMap<u32, TraitCondEntry>,
    currencies_by_tree: BTreeMap<u32, Vec<TraitCurrencyEntry>>,
    currency_sources_by_currency: BTreeMap<u32, Vec<TraitCurrencySourceEntry>>,
    spec_set_members: BTreeSet<(i32, i32)>,
    graph_loaded: bool,
}

impl TraitTreeSkillLineIndexLikeCpp {
    /// Build the profession portion of `TraitMgr`'s cross-store index.
    /// Missing SkillLine or TraitTree references are skipped exactly as the
    /// C++ loader does, and links are ordered by their DB2 order field.
    pub fn from_effective_stores_like_cpp(
        links: &SkillLineXTraitTreeStore,
        trees: &TraitTreeStore,
        skill_line_exists: impl Fn(u32) -> bool,
        class_ids_for_skill_line: impl Fn(u32) -> Vec<u8>,
    ) -> Self {
        let mut by_skill_line = BTreeMap::<u32, Vec<(i32, u32)>>::new();
        let mut skill_line_by_class = BTreeMap::<u8, u32>::new();
        let mut valid_links = links
            .iter()
            .filter(|link| {
                let Some(trait_tree_id) = u32::try_from(link.trait_tree_id).ok() else {
                    return false;
                };
                link.skill_line_id != 0
                    && skill_line_exists(link.skill_line_id)
                    && trees.get(trait_tree_id).is_some()
            })
            .collect::<Vec<_>>();
        // C++ DB2Storage iterates its dense ID table in record-ID order. The
        // Rust store is HashMap-backed, so sort before reproducing the
        // `_skillLinesByClass[class] = skillLine` overwrite semantics.
        valid_links.sort_unstable_by_key(|link| link.id);
        for link in valid_links {
            let Some(trait_tree_id) = u32::try_from(link.trait_tree_id).ok() else {
                continue;
            };
            by_skill_line
                .entry(link.skill_line_id)
                .or_default()
                .push((link.order_index, trait_tree_id));
            for class_id in class_ids_for_skill_line(link.skill_line_id) {
                skill_line_by_class.insert(class_id, link.skill_line_id);
            }
        }

        let mut trees_by_trait_system = trees.iter().filter(|tree| tree.trait_system_id != 0).fold(
            BTreeMap::<u32, Vec<u32>>::new(),
            |mut index, tree| {
                index.entry(tree.trait_system_id).or_default().push(tree.id);
                index
            },
        );
        // DB2 records are keyed by ID in the Rust store; keep the projected
        // vectors deterministic and stable across HashMap iteration order.
        for tree_ids in trees_by_trait_system.values_mut() {
            tree_ids.sort_unstable();
        }

        Self {
            trees_by_skill_line: by_skill_line
                .into_iter()
                .map(|(skill_line_id, mut rows)| {
                    rows.sort_unstable();
                    (
                        skill_line_id,
                        rows.into_iter().map(|(_, tree_id)| tree_id).collect(),
                    )
                })
                .collect(),
            trees_by_trait_system,
            skill_line_by_class,
            nodes_by_tree: BTreeMap::new(),
            entries_by_node: BTreeMap::new(),
            groups_by_node: BTreeMap::new(),
            parents_by_node: BTreeMap::new(),
            tree_costs: BTreeMap::new(),
            node_costs: BTreeMap::new(),
            group_costs: BTreeMap::new(),
            entry_costs: BTreeMap::new(),
            node_conditions: BTreeMap::new(),
            group_conditions: BTreeMap::new(),
            entry_conditions: BTreeMap::new(),
            loadouts_by_specialization: BTreeMap::new(),
            trees: BTreeMap::new(),
            nodes: BTreeMap::new(),
            node_entries: BTreeMap::new(),
            groups: BTreeMap::new(),
            costs: BTreeMap::new(),
            conditions: BTreeMap::new(),
            currencies_by_tree: BTreeMap::new(),
            currency_sources_by_currency: BTreeMap::new(),
            spec_set_members: std::collections::BTreeSet::new(),
            graph_loaded: false,
        }
    }

    /// Extend the skill-line projection with the immutable graph assembled by
    /// C++ `TraitMgr::Load`.  Relation rows are accepted only when both sides
    /// resolve in the effective stores; this preserves C++'s fail-closed
    /// lookup behaviour while giving production consumers one canonical index.
    #[allow(clippy::too_many_arguments)]
    pub fn with_trait_graph_like_cpp(
        mut self,
        trees: &TraitTreeStore,
        nodes: &TraitNodeStore,
        node_entries: &TraitNodeEntryStore,
        node_entry_conditions: &TraitNodeEntryXTraitCondStore,
        node_entry_costs: &TraitNodeEntryXTraitCostStore,
        groups: &TraitNodeGroupStore,
        group_conditions: &TraitNodeGroupXTraitCondStore,
        group_costs: &TraitNodeGroupXTraitCostStore,
        group_nodes: &TraitNodeGroupXTraitNodeStore,
        node_conditions: &TraitNodeXTraitCondStore,
        node_costs: &TraitNodeXTraitCostStore,
        node_entries_by_node: &TraitNodeXTraitNodeEntryStore,
        edges: &TraitEdgeStore,
        costs: &TraitCostStore,
        conditions: &TraitCondStore,
        loadouts: &TraitTreeLoadoutStore,
        loadout_entries: &TraitTreeLoadoutEntryStore,
        tree_costs: &TraitTreeXTraitCostStore,
    ) -> Self {
        self.graph_loaded = true;

        self.trees = trees.iter().map(|row| (row.id, row.clone())).collect();
        self.nodes = nodes.iter().map(|row| (row.id, row.clone())).collect();
        self.node_entries = node_entries
            .iter()
            .map(|row| (row.id, row.clone()))
            .collect();
        self.groups = groups.iter().map(|row| (row.id, row.clone())).collect();
        self.costs = costs.iter().map(|row| (row.id, row.clone())).collect();
        self.conditions = conditions.iter().map(|row| (row.id, row.clone())).collect();

        for node in nodes
            .iter()
            .filter(|node| node.trait_tree_id != 0 && trees.get(node.trait_tree_id).is_some())
        {
            self.nodes_by_tree
                .entry(node.trait_tree_id)
                .or_default()
                .push(node.id);
        }
        for ids in self.nodes_by_tree.values_mut() {
            ids.sort_unstable();
            ids.dedup();
        }

        let mut node_entry_links = node_entries_by_node.iter().collect::<Vec<_>>();
        node_entry_links.sort_unstable_by_key(|row| row.id);
        for link in node_entry_links {
            let Some(entry_id) = u32::try_from(link.trait_node_entry_id).ok() else {
                continue;
            };
            if nodes.get(link.trait_node_id).is_some() && node_entries.get(entry_id).is_some() {
                self.entries_by_node
                    .entry(link.trait_node_id)
                    .or_default()
                    .push(entry_id);
            }
        }
        for ids in self.entries_by_node.values_mut() {
            ids.sort_unstable();
            ids.dedup();
        }

        let mut group_node_links = group_nodes.iter().collect::<Vec<_>>();
        group_node_links.sort_unstable_by_key(|row| row.id);
        for link in group_node_links {
            let Some(node_id) = u32::try_from(link.trait_node_id).ok() else {
                continue;
            };
            let Some(group) = groups.get(link.trait_node_group_id) else {
                continue;
            };
            if nodes.get(node_id).is_none() {
                continue;
            }
            self.groups_by_node
                .entry(node_id)
                .or_default()
                .push(group.id);
        }
        for ids in self.groups_by_node.values_mut() {
            ids.sort_unstable();
            ids.dedup();
        }

        let mut edge_rows = edges.iter().collect::<Vec<_>>();
        edge_rows.sort_unstable_by_key(|row| row.id);
        for edge in edge_rows {
            let Some(right) = u32::try_from(edge.right_trait_node_id).ok() else {
                continue;
            };
            if nodes.get(edge.left_trait_node_id).is_some() && nodes.get(right).is_some() {
                self.parents_by_node
                    .entry(right)
                    .or_default()
                    .push((edge.left_trait_node_id, edge.edge_type));
            }
        }

        let add_cost = |target: &mut BTreeMap<u32, Vec<u32>>, owner: u32, cost_id: i32| {
            let Some(cost_id) = u32::try_from(cost_id).ok() else {
                return;
            };
            if costs.get(cost_id).is_some() {
                target.entry(owner).or_default().push(cost_id);
            }
        };
        let mut tree_cost_rows = tree_costs.iter().collect::<Vec<_>>();
        tree_cost_rows.sort_unstable_by_key(|row| row.id);
        for row in tree_cost_rows {
            if trees.get(row.trait_tree_id).is_some() {
                add_cost(&mut self.tree_costs, row.trait_tree_id, row.trait_cost_id);
            }
        }
        let mut node_cost_rows = node_costs.iter().collect::<Vec<_>>();
        node_cost_rows.sort_unstable_by_key(|row| row.id);
        for row in node_cost_rows {
            if nodes.get(row.trait_node_id).is_some() {
                add_cost(&mut self.node_costs, row.trait_node_id, row.trait_cost_id);
            }
        }
        let mut group_cost_rows = group_costs.iter().collect::<Vec<_>>();
        group_cost_rows.sort_unstable_by_key(|row| row.id);
        for row in group_cost_rows {
            if groups.get(row.trait_node_group_id).is_some() {
                add_cost(
                    &mut self.group_costs,
                    row.trait_node_group_id,
                    row.trait_cost_id,
                );
            }
        }
        let mut entry_cost_rows = node_entry_costs.iter().collect::<Vec<_>>();
        entry_cost_rows.sort_unstable_by_key(|row| row.id);
        for row in entry_cost_rows {
            if node_entries.get(row.trait_node_entry_id).is_some() {
                add_cost(
                    &mut self.entry_costs,
                    row.trait_node_entry_id,
                    row.trait_cost_id,
                );
            }
        }
        for costs in [
            &mut self.tree_costs,
            &mut self.node_costs,
            &mut self.group_costs,
            &mut self.entry_costs,
        ] {
            for ids in costs.values_mut() {
                ids.sort_unstable();
                ids.dedup();
            }
        }

        let add_condition =
            |target: &mut BTreeMap<u32, Vec<u32>>, owner: u32, condition_id: i32| {
                let Some(condition_id) = u32::try_from(condition_id).ok() else {
                    return;
                };
                if conditions.get(condition_id).is_some() {
                    target.entry(owner).or_default().push(condition_id);
                }
            };
        let mut entry_condition_rows = node_entry_conditions.iter().collect::<Vec<_>>();
        entry_condition_rows.sort_unstable_by_key(|row| row.id);
        for row in entry_condition_rows {
            if node_entries.get(row.trait_node_entry_id).is_some() {
                add_condition(
                    &mut self.entry_conditions,
                    row.trait_node_entry_id,
                    row.trait_cond_id,
                );
            }
        }
        let mut node_condition_rows = node_conditions.iter().collect::<Vec<_>>();
        node_condition_rows.sort_unstable_by_key(|row| row.id);
        for row in node_condition_rows {
            if nodes.get(row.trait_node_id).is_some() {
                add_condition(
                    &mut self.node_conditions,
                    row.trait_node_id,
                    row.trait_cond_id,
                );
            }
        }
        let mut group_condition_rows = group_conditions.iter().collect::<Vec<_>>();
        group_condition_rows.sort_unstable_by_key(|row| row.id);
        for row in group_condition_rows {
            if groups.get(row.trait_node_group_id).is_some() {
                add_condition(
                    &mut self.group_conditions,
                    row.trait_node_group_id,
                    row.trait_cond_id,
                );
            }
        }
        for conditions in [
            &mut self.entry_conditions,
            &mut self.node_conditions,
            &mut self.group_conditions,
        ] {
            for ids in conditions.values_mut() {
                ids.sort_unstable();
                ids.dedup();
            }
        }

        let mut loadout_rows = loadouts.iter().collect::<Vec<_>>();
        loadout_rows.sort_unstable_by_key(|row| row.id);
        for loadout in loadout_rows {
            let mut entries = loadout_entries
                .iter()
                .filter(|entry| entry.trait_tree_loadout_id == loadout.id)
                .map(|entry| {
                    (
                        entry.order_index,
                        entry.id,
                        TraitTreeLoadoutSelectionLikeCpp {
                            trait_tree_id: loadout.trait_tree_id,
                            selected_trait_node_id: entry.selected_trait_node_id,
                            selected_trait_node_entry_id: entry.selected_trait_node_entry_id,
                            num_points: entry.num_points,
                            order_index: entry.order_index,
                        },
                    )
                })
                .collect::<Vec<_>>();
            entries.sort_unstable_by_key(|(order, id, _)| (*order, *id));
            if !entries.is_empty() {
                self.loadouts_by_specialization.insert(
                    loadout.chr_specialization_id,
                    entries.into_iter().map(|(_, _, entry)| entry).collect(),
                );
            }
        }

        self
    }

    pub fn graph_loaded_like_cpp(&self) -> bool {
        self.graph_loaded
    }

    pub fn nodes_for_tree_like_cpp(&self, tree_id: u32) -> &[u32] {
        self.nodes_by_tree
            .get(&tree_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn entries_for_node_like_cpp(&self, node_id: u32) -> &[u32] {
        self.entries_by_node
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn groups_for_node_like_cpp(&self, node_id: u32) -> &[u32] {
        self.groups_by_node
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn parent_nodes_for_node_like_cpp(&self, node_id: u32) -> &[(u32, i32)] {
        self.parents_by_node
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn cost_ids_for_tree_like_cpp(&self, tree_id: u32) -> &[u32] {
        self.tree_costs
            .get(&tree_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn cost_ids_for_node_like_cpp(&self, node_id: u32) -> &[u32] {
        self.node_costs
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn cost_ids_for_group_like_cpp(&self, group_id: u32) -> &[u32] {
        self.group_costs
            .get(&group_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn cost_ids_for_entry_like_cpp(&self, entry_id: u32) -> &[u32] {
        self.entry_costs
            .get(&entry_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn condition_ids_for_node_like_cpp(&self, node_id: u32) -> &[u32] {
        self.node_conditions
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn condition_ids_for_group_like_cpp(&self, group_id: u32) -> &[u32] {
        self.group_conditions
            .get(&group_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn condition_ids_for_entry_like_cpp(&self, entry_id: u32) -> &[u32] {
        self.entry_conditions
            .get(&entry_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn loadout_for_specialization_like_cpp(
        &self,
        specialization_id: i32,
    ) -> &[TraitTreeLoadoutSelectionLikeCpp] {
        self.loadouts_by_specialization
            .get(&specialization_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Validate a starter-build selection before it is copied into a Player
    /// config.  C++ retains the raw loadout rows at startup and resolves the
    /// node/entry later; Rust keeps the same raw projection but exposes the
    /// fail-closed check at the publication boundary.
    pub fn loadout_selection_is_valid_like_cpp(
        &self,
        selection: &TraitTreeLoadoutSelectionLikeCpp,
    ) -> bool {
        let Some(node_id) = u32::try_from(selection.selected_trait_node_id).ok() else {
            return false;
        };
        if !self
            .nodes_by_tree
            .get(&selection.trait_tree_id)
            .is_some_and(|nodes| nodes.contains(&node_id))
        {
            return false;
        }
        if selection.selected_trait_node_entry_id == 0 {
            return !self.entries_for_node_like_cpp(node_id).is_empty();
        }
        let Some(entry_id) = u32::try_from(selection.selected_trait_node_entry_id).ok() else {
            return false;
        };
        self.entries_for_node_like_cpp(node_id).contains(&entry_id)
    }

    /// Validate the two IDs persisted by C++ `TraitEntry` against the selected
    /// tree set.  This is the production fail-closed boundary; cost and
    /// condition rows remain immutable inputs for later spending checks.
    pub fn entry_belongs_to_tree_set_like_cpp(
        &self,
        tree_ids: &[u32],
        node_id: u32,
        entry_id: u32,
    ) -> bool {
        tree_ids.iter().any(|tree_id| {
            self.nodes_for_tree_like_cpp(*tree_id).contains(&node_id)
                && self.entries_for_node_like_cpp(node_id).contains(&entry_id)
        })
    }

    /// C++ `TraitMgr` profession lookup used by Player trait-config loading.
    pub fn trees_for_skill_line_like_cpp(&self, skill_line_id: u32) -> &[u32] {
        self.trees_by_skill_line
            .get(&skill_line_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn has_skill_line_like_cpp(&self, skill_line_id: u32) -> bool {
        !self.trees_for_skill_line_like_cpp(skill_line_id).is_empty()
    }

    /// C++ `TraitMgr::GetTreesForConfig` generic branch, indexed by
    /// `TraitTreeEntry::TraitSystemID`.
    pub fn trees_for_trait_system_like_cpp(&self, trait_system_id: u32) -> &[u32] {
        self.trees_by_trait_system
            .get(&trait_system_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn has_trait_system_like_cpp(&self, trait_system_id: u32) -> bool {
        !self
            .trees_for_trait_system_like_cpp(trait_system_id)
            .is_empty()
    }

    /// C++ `TraitMgr::GetTreesForConfig` combat branch after resolving the
    /// specialization's class through `_skillLinesByClass`.
    pub fn trees_for_class_like_cpp(&self, class_id: u8) -> &[u32] {
        self.skill_line_by_class
            .get(&class_id)
            .and_then(|skill_line_id| self.trees_by_skill_line.get(skill_line_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn has_class_like_cpp(&self, class_id: u8) -> bool {
        !self.trees_for_class_like_cpp(class_id).is_empty()
    }

    pub fn len(&self) -> usize {
        self.trees_by_skill_line.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.trees_by_skill_line.is_empty()
    }
}

impl TraitCondStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitCond.db2", |id, idx, r| {
            TraitCondEntry {
                id,
                cond_type: r.get_field_i32(idx, 1),
                trait_tree_id: r.get_field_u32(idx, 2),
                granted_ranks: r.get_field_i32(idx, 3),
                quest_id: r.get_field_i32(idx, 4),
                achievement_id: r.get_field_i32(idx, 5),
                spec_set_id: r.get_field_i32(idx, 6),
                trait_node_group_id: r.get_field_i32(idx, 7),
                trait_node_id: r.get_field_i32(idx, 8),
                trait_currency_id: r.get_field_i32(idx, 9),
                spent_amount_required: r.get_field_i32(idx, 10),
                flags: r.get_field_i32(idx, 11),
                required_level: r.get_field_i32(idx, 12),
                free_shared_string_id: r.get_field_i32(idx, 13),
                spend_more_shared_string_id: r.get_field_i32(idx, 14),
            }
        })
    }
}

impl TraitCostStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitCost.db2", |id, idx, r| {
            TraitCostEntry {
                id,
                internal_name: r.get_field_string(idx, 0),
                amount: r.get_field_i32(idx, 2),
                trait_currency_id: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl TraitCurrencyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitCurrency.db2", |id, idx, r| {
            TraitCurrencyEntry {
                id,
                currency_type: r.get_field_i32(idx, 1),
                currency_types_id: r.get_field_i32(idx, 2),
                flags: r.get_field_i32(idx, 3),
                icon: r.get_field_i32(idx, 4),
            }
        })
    }
}

impl TraitCurrencySourceStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitCurrencySource.db2", |id, idx, r| {
            TraitCurrencySourceEntry {
                id,
                requirement: r.get_field_string(idx, 0),
                trait_currency_id: r.get_field_u32(idx, 2),
                amount: r.get_field_i32(idx, 3),
                quest_id: r.get_field_i32(idx, 4),
                achievement_id: r.get_field_i32(idx, 5),
                player_level: r.get_field_i32(idx, 6),
                trait_node_entry_id: r.get_field_i32(idx, 7),
                order_index: r.get_field_i32(idx, 8),
            }
        })
    }
}

impl TraitDefinitionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitDefinition.db2", |id, idx, r| {
            TraitDefinitionEntry {
                id,
                override_name: r.get_field_string(idx, 0),
                override_subtext: r.get_field_string(idx, 1),
                override_description: r.get_field_string(idx, 2),
                spell_id: r.get_field_i32(idx, 4),
                override_icon: r.get_field_i32(idx, 5),
                overrides_spell_id: r.get_field_i32(idx, 6),
                visible_spell_id: r.get_field_i32(idx, 7),
            }
        })
    }
}

impl TraitDefinitionEffectPointsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitDefinitionEffectPoints.db2",
            |id, idx, r| TraitDefinitionEffectPointsEntry {
                id,
                trait_definition_id: r.get_relationship_id(idx).unwrap_or(0),
                effect_index: r.get_field_i32(idx, 2),
                operation_type: r.get_field_i32(idx, 3),
                curve_id: r.get_field_i32(idx, 4),
            },
        )
    }
}

impl TraitEdgeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitEdge.db2", |id, idx, r| {
            TraitEdgeEntry {
                id,
                visual_style: r.get_field_i32(idx, 1),
                left_trait_node_id: r.get_relationship_id(idx).unwrap_or(0),
                right_trait_node_id: r.get_field_i32(idx, 3),
                edge_type: r.get_field_i32(idx, 4),
            }
        })
    }
}

impl TraitNodeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitNode.db2", |id, idx, r| {
            TraitNodeEntry {
                id,
                trait_tree_id: r.get_relationship_id(idx).unwrap_or(0),
                pos_x: r.get_field_i32(idx, 2),
                pos_y: r.get_field_i32(idx, 3),
                node_type: r.get_field_u8(idx, 4),
                flags: r.get_field_i32(idx, 5),
            }
        })
    }
}

impl TraitNodeEntryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitNodeEntry.db2", |id, idx, r| {
            TraitNodeEntryEntry {
                id,
                trait_definition_id: r.get_field_i32(idx, 1),
                max_ranks: r.get_field_i32(idx, 2),
                node_entry_type: r.get_field_u8(idx, 3),
            }
        })
    }
}

impl TraitNodeEntryXTraitCondStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitNodeEntryXTraitCond.db2",
            |id, idx, r| TraitNodeEntryXTraitCondEntry {
                id,
                trait_cond_id: r.get_field_i32(idx, 1),
                trait_node_entry_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl TraitNodeEntryXTraitCostStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitNodeEntryXTraitCost.db2",
            |id, idx, r| TraitNodeEntryXTraitCostEntry {
                id,
                trait_node_entry_id: r.get_relationship_id(idx).unwrap_or(0),
                trait_cost_id: r.get_field_i32(idx, 2),
            },
        )
    }
}

impl TraitNodeGroupStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitNodeGroup.db2", |id, idx, r| {
            TraitNodeGroupEntry {
                id,
                trait_tree_id: r.get_relationship_id(idx).unwrap_or(0),
                flags: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl TraitNodeGroupXTraitCondStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitNodeGroupXTraitCond.db2",
            |id, idx, r| TraitNodeGroupXTraitCondEntry {
                id,
                trait_cond_id: r.get_field_i32(idx, 1),
                trait_node_group_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl TraitNodeGroupXTraitCostStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitNodeGroupXTraitCost.db2",
            |id, idx, r| TraitNodeGroupXTraitCostEntry {
                id,
                trait_node_group_id: r.get_relationship_id(idx).unwrap_or(0),
                trait_cost_id: r.get_field_i32(idx, 2),
            },
        )
    }
}

impl TraitNodeGroupXTraitNodeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitNodeGroupXTraitNode.db2",
            |id, idx, r| TraitNodeGroupXTraitNodeEntry {
                id,
                trait_node_group_id: r.get_relationship_id(idx).unwrap_or(0),
                trait_node_id: r.get_field_i32(idx, 2),
                index: r.get_field_i32(idx, 3),
            },
        )
    }
}

impl TraitNodeXTraitCondStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitNodeXTraitCond.db2", |id, idx, r| {
            TraitNodeXTraitCondEntry {
                id,
                trait_cond_id: r.get_field_i32(idx, 1),
                trait_node_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl TraitNodeXTraitCostStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitNodeXTraitCost.db2", |id, idx, r| {
            TraitNodeXTraitCostEntry {
                id,
                trait_node_id: r.get_relationship_id(idx).unwrap_or(0),
                trait_cost_id: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl TraitNodeXTraitNodeEntryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitNodeXTraitNodeEntry.db2",
            |id, idx, r| TraitNodeXTraitNodeEntryEntry {
                id,
                trait_node_id: r.get_relationship_id(idx).unwrap_or(0),
                trait_node_entry_id: r.get_field_i32(idx, 2),
                index: r.get_field_i32(idx, 3),
            },
        )
    }
}

impl TraitTreeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitTree.db2", |id, idx, r| {
            TraitTreeEntry {
                id,
                trait_system_id: r.get_field_u32(idx, 1),
                unused1000_1: r.get_field_i32(idx, 2),
                first_trait_node_id: r.get_field_i32(idx, 3),
                player_condition_id: r.get_field_i32(idx, 4),
                flags: r.get_field_i32(idx, 5),
                unused1000_2: f32_field(r, idx, 6),
                unused1000_3: f32_field(r, idx, 7),
            }
        })
    }
}

impl TraitTreeLoadoutStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitTreeLoadout.db2", |id, idx, r| {
            TraitTreeLoadoutEntry {
                id,
                trait_tree_id: r.get_relationship_id(idx).unwrap_or(0),
                chr_specialization_id: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl TraitTreeLoadoutEntryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitTreeLoadoutEntry.db2",
            |id, idx, r| TraitTreeLoadoutEntryEntry {
                id,
                trait_tree_loadout_id: r.get_relationship_id(idx).unwrap_or(0),
                selected_trait_node_id: r.get_field_i32(idx, 2),
                selected_trait_node_entry_id: r.get_field_i32(idx, 3),
                num_points: r.get_field_i32(idx, 4),
                order_index: r.get_field_i32(idx, 5),
            },
        )
    }
}

impl TraitTreeXTraitCostStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TraitTreeXTraitCost.db2", |id, idx, r| {
            TraitTreeXTraitCostEntry {
                id,
                trait_tree_id: r.get_relationship_id(idx).unwrap_or(0),
                trait_cost_id: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl TraitTreeXTraitCurrencyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "TraitTreeXTraitCurrency.db2",
            |id, idx, r| TraitTreeXTraitCurrencyEntry {
                id,
                index: r.get_field_i32(idx, 1),
                trait_tree_id: r.get_relationship_id(idx).unwrap_or(0),
                trait_currency_id: r.get_field_i32(idx, 3),
            },
        )
    }
}

fn load_store<T, S>(
    data_dir: &str,
    locale: &str,
    file_name: &str,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<S>
where
    S: FromEntries<T>,
{
    let path = Path::new(data_dir).join("dbc").join(locale).join(file_name);
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;

    let mut entries = Vec::with_capacity(reader.total_count());
    for (id, idx) in reader.iter_records() {
        entries.push(read(id, idx, &reader));
    }

    let store = S::from_entries(entries);
    info!("Loaded {} rows from {}", store.len(), path.display());
    Ok(store)
}

fn f32_field(reader: &Wdc4Reader, record_idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(record_idx, field))
}

trait FromEntries<T> {
    fn from_entries(entries: impl IntoIterator<Item = T>) -> Self;
    fn len(&self) -> usize;
}

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }

            fn len(&self) -> usize {
                self.len()
            }
        }
    };
}

impl_from_entries!(TraitCondStore, TraitCondEntry);
impl_from_entries!(TraitCostStore, TraitCostEntry);
impl_from_entries!(TraitCurrencyStore, TraitCurrencyEntry);
impl_from_entries!(TraitCurrencySourceStore, TraitCurrencySourceEntry);
impl_from_entries!(TraitDefinitionStore, TraitDefinitionEntry);
impl_from_entries!(
    TraitDefinitionEffectPointsStore,
    TraitDefinitionEffectPointsEntry
);
impl_from_entries!(TraitEdgeStore, TraitEdgeEntry);
impl_from_entries!(TraitNodeStore, TraitNodeEntry);
impl_from_entries!(TraitNodeEntryStore, TraitNodeEntryEntry);
impl_from_entries!(TraitNodeEntryXTraitCondStore, TraitNodeEntryXTraitCondEntry);
impl_from_entries!(TraitNodeEntryXTraitCostStore, TraitNodeEntryXTraitCostEntry);
impl_from_entries!(TraitNodeGroupStore, TraitNodeGroupEntry);
impl_from_entries!(TraitNodeGroupXTraitCondStore, TraitNodeGroupXTraitCondEntry);
impl_from_entries!(TraitNodeGroupXTraitCostStore, TraitNodeGroupXTraitCostEntry);
impl_from_entries!(TraitNodeGroupXTraitNodeStore, TraitNodeGroupXTraitNodeEntry);
impl_from_entries!(TraitNodeXTraitCondStore, TraitNodeXTraitCondEntry);
impl_from_entries!(TraitNodeXTraitCostStore, TraitNodeXTraitCostEntry);
impl_from_entries!(TraitNodeXTraitNodeEntryStore, TraitNodeXTraitNodeEntryEntry);
impl_from_entries!(TraitTreeStore, TraitTreeEntry);
impl_from_entries!(TraitTreeLoadoutStore, TraitTreeLoadoutEntry);
impl_from_entries!(TraitTreeLoadoutEntryStore, TraitTreeLoadoutEntryEntry);
impl_from_entries!(TraitTreeXTraitCostStore, TraitTreeXTraitCostEntry);
impl_from_entries!(TraitTreeXTraitCurrencyStore, TraitTreeXTraitCurrencyEntry);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_talent::SkillLineXTraitTreeEntry;

    #[test]
    fn trait_node_store_uses_cpp_tree_parent_relationship() {
        let store = TraitNodeStore::from_entries([TraitNodeEntry {
            id: 10,
            trait_tree_id: 20,
            pos_x: 1,
            pos_y: 2,
            node_type: 3,
            flags: 4,
        }]);

        assert_eq!(store.get(10).unwrap().trait_tree_id, 20);
    }

    #[test]
    fn trait_tree_skill_line_index_matches_valid_links_and_order() {
        let links = SkillLineXTraitTreeStore::from_entries([
            SkillLineXTraitTreeEntry {
                id: 1,
                skill_line_id: 164,
                trait_tree_id: 20,
                order_index: 2,
            },
            SkillLineXTraitTreeEntry {
                id: 2,
                skill_line_id: 164,
                trait_tree_id: 10,
                order_index: 1,
            },
            SkillLineXTraitTreeEntry {
                id: 3,
                skill_line_id: 999,
                trait_tree_id: 20,
                order_index: 0,
            },
            SkillLineXTraitTreeEntry {
                id: 4,
                skill_line_id: 164,
                trait_tree_id: -1,
                order_index: 0,
            },
        ]);
        let trees = TraitTreeStore::from_entries([
            TraitTreeEntry {
                id: 10,
                trait_system_id: 0,
                unused1000_1: 0,
                first_trait_node_id: 0,
                player_condition_id: 0,
                flags: 0,
                unused1000_2: 0.0,
                unused1000_3: 0.0,
            },
            TraitTreeEntry {
                id: 20,
                trait_system_id: 0,
                unused1000_1: 0,
                first_trait_node_id: 0,
                player_condition_id: 0,
                flags: 0,
                unused1000_2: 0.0,
                unused1000_3: 0.0,
            },
        ]);

        let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &links,
            &trees,
            |skill_line_id| skill_line_id == 164,
            |_| Vec::new(),
        );
        assert_eq!(index.trees_for_skill_line_like_cpp(164), &[10, 20]);
        assert!(index.has_skill_line_like_cpp(164));
        assert!(!index.has_skill_line_like_cpp(999));
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn trait_tree_index_exposes_generic_trait_systems_like_cpp() {
        let links = SkillLineXTraitTreeStore::from_entries([]);
        let trees = TraitTreeStore::from_entries([
            TraitTreeEntry {
                id: 10,
                trait_system_id: 7,
                unused1000_1: 0,
                first_trait_node_id: 0,
                player_condition_id: 0,
                flags: 0,
                unused1000_2: 0.0,
                unused1000_3: 0.0,
            },
            TraitTreeEntry {
                id: 20,
                trait_system_id: 7,
                unused1000_1: 0,
                first_trait_node_id: 0,
                player_condition_id: 0,
                flags: 0,
                unused1000_2: 0.0,
                unused1000_3: 0.0,
            },
            TraitTreeEntry {
                id: 30,
                trait_system_id: 0,
                unused1000_1: 0,
                first_trait_node_id: 0,
                player_condition_id: 0,
                flags: 0,
                unused1000_2: 0.0,
                unused1000_3: 0.0,
            },
        ]);

        let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &links,
            &trees,
            |_| false,
            |_| Vec::new(),
        );
        assert_eq!(index.trees_for_trait_system_like_cpp(7), &[10, 20]);
        assert!(index.has_trait_system_like_cpp(7));
        assert!(!index.has_trait_system_like_cpp(30));
    }

    #[test]
    fn trait_mgr_graph_indexes_nodes_relations_costs_conditions_edges_and_loadouts() {
        let links = SkillLineXTraitTreeStore::from_entries([]);
        let trees = TraitTreeStore::from_entries([TraitTreeEntry {
            id: 10,
            trait_system_id: 7,
            unused1000_1: 0,
            first_trait_node_id: 100,
            player_condition_id: 0,
            flags: 0,
            unused1000_2: 0.0,
            unused1000_3: 0.0,
        }]);
        let nodes = TraitNodeStore::from_entries([
            TraitNodeEntry {
                id: 100,
                trait_tree_id: 10,
                pos_x: 0,
                pos_y: 0,
                node_type: 0,
                flags: 0,
            },
            TraitNodeEntry {
                id: 101,
                trait_tree_id: 10,
                pos_x: 1,
                pos_y: 0,
                node_type: 0,
                flags: 0,
            },
        ]);
        let node_entries = TraitNodeEntryStore::from_entries([TraitNodeEntryEntry {
            id: 1000,
            trait_definition_id: 5,
            max_ranks: 2,
            node_entry_type: 0,
        }]);
        let groups = TraitNodeGroupStore::from_entries([TraitNodeGroupEntry {
            id: 200,
            trait_tree_id: 10,
            flags: 0,
        }]);
        let costs = TraitCostStore::from_entries([TraitCostEntry {
            id: 300,
            internal_name: "cost".into(),
            amount: 1,
            trait_currency_id: 4,
        }]);
        let conditions = TraitCondStore::from_entries([TraitCondEntry {
            id: 400,
            cond_type: 1,
            trait_tree_id: 10,
            granted_ranks: 1,
            quest_id: 0,
            achievement_id: 0,
            spec_set_id: 0,
            trait_node_group_id: 0,
            trait_node_id: 0,
            trait_currency_id: 0,
            spent_amount_required: 0,
            flags: 0,
            required_level: 0,
            free_shared_string_id: 0,
            spend_more_shared_string_id: 0,
        }]);
        let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &links,
            &trees,
            |_| false,
            |_| Vec::new(),
        )
        .with_trait_graph_like_cpp(
            &trees,
            &nodes,
            &node_entries,
            &TraitNodeEntryXTraitCondStore::from_entries([TraitNodeEntryXTraitCondEntry {
                id: 500,
                trait_cond_id: 400,
                trait_node_entry_id: 1000,
            }]),
            &TraitNodeEntryXTraitCostStore::from_entries([TraitNodeEntryXTraitCostEntry {
                id: 501,
                trait_node_entry_id: 1000,
                trait_cost_id: 300,
            }]),
            &groups,
            &TraitNodeGroupXTraitCondStore::from_entries([]),
            &TraitNodeGroupXTraitCostStore::from_entries([]),
            &TraitNodeGroupXTraitNodeStore::from_entries([TraitNodeGroupXTraitNodeEntry {
                id: 600,
                trait_node_group_id: 200,
                trait_node_id: 100,
                index: 0,
            }]),
            &TraitNodeXTraitCondStore::from_entries([]),
            &TraitNodeXTraitCostStore::from_entries([]),
            &TraitNodeXTraitNodeEntryStore::from_entries([TraitNodeXTraitNodeEntryEntry {
                id: 601,
                trait_node_id: 100,
                trait_node_entry_id: 1000,
                index: 0,
            }]),
            &TraitEdgeStore::from_entries([TraitEdgeEntry {
                id: 700,
                visual_style: 0,
                left_trait_node_id: 100,
                right_trait_node_id: 101,
                edge_type: 2,
            }]),
            &costs,
            &conditions,
            &TraitTreeLoadoutStore::from_entries([TraitTreeLoadoutEntry {
                id: 800,
                trait_tree_id: 10,
                chr_specialization_id: 71,
            }]),
            &TraitTreeLoadoutEntryStore::from_entries([TraitTreeLoadoutEntryEntry {
                id: 801,
                trait_tree_loadout_id: 800,
                selected_trait_node_id: 100,
                selected_trait_node_entry_id: 1000,
                num_points: 1,
                order_index: 0,
            }]),
            &TraitTreeXTraitCostStore::from_entries([TraitTreeXTraitCostEntry {
                id: 900,
                trait_tree_id: 10,
                trait_cost_id: 300,
            }]),
        );

        assert!(index.graph_loaded_like_cpp());
        assert_eq!(index.nodes_for_tree_like_cpp(10), &[100, 101]);
        assert_eq!(index.entries_for_node_like_cpp(100), &[1000]);
        assert_eq!(index.groups_for_node_like_cpp(100), &[200]);
        assert_eq!(index.parent_nodes_for_node_like_cpp(101), &[(100, 2)]);
        assert_eq!(index.cost_ids_for_tree_like_cpp(10), &[300]);
        assert_eq!(index.cost_ids_for_entry_like_cpp(1000), &[300]);
        assert_eq!(index.condition_ids_for_entry_like_cpp(1000), &[400]);
        assert_eq!(
            index.loadout_for_specialization_like_cpp(71)[0].num_points,
            1
        );
        assert!(index.loadout_selection_is_valid_like_cpp(
            &index.loadout_for_specialization_like_cpp(71)[0]
        ));
        assert!(index.entry_belongs_to_tree_set_like_cpp(&[10], 100, 1000));
        assert!(!index.entry_belongs_to_tree_set_like_cpp(&[10], 101, 1000));
    }

    #[test]
    fn trait_mgr_semantic_validation_matches_cpp_conditions_costs_and_grants() {
        let links = SkillLineXTraitTreeStore::from_entries([]);
        let trees = TraitTreeStore::from_entries([TraitTreeEntry {
            id: 10,
            trait_system_id: 7,
            unused1000_1: 0,
            first_trait_node_id: 100,
            player_condition_id: 0,
            flags: 0,
            unused1000_2: 0.0,
            unused1000_3: 0.0,
        }]);
        let nodes = TraitNodeStore::from_entries([
            TraitNodeEntry {
                id: 100,
                trait_tree_id: 10,
                pos_x: 0,
                pos_y: 0,
                node_type: 0,
                flags: 0,
            },
            TraitNodeEntry {
                id: 101,
                trait_tree_id: 10,
                pos_x: 1,
                pos_y: 0,
                node_type: 2,
                flags: 0,
            },
        ]);
        let node_entries = TraitNodeEntryStore::from_entries([
            TraitNodeEntryEntry {
                id: 1000,
                trait_definition_id: 0,
                max_ranks: 2,
                node_entry_type: 0,
            },
            TraitNodeEntryEntry {
                id: 1001,
                trait_definition_id: 0,
                max_ranks: 1,
                node_entry_type: 0,
            },
            TraitNodeEntryEntry {
                id: 1002,
                trait_definition_id: 0,
                max_ranks: 1,
                node_entry_type: 0,
            },
        ]);
        let costs = TraitCostStore::from_entries([TraitCostEntry {
            id: 300,
            internal_name: "point".into(),
            amount: 1,
            trait_currency_id: 4,
        }]);
        let conditions = TraitCondStore::from_entries([
            TraitCondEntry {
                id: 400,
                cond_type: 0,
                trait_tree_id: 10,
                granted_ranks: 0,
                quest_id: 42,
                achievement_id: 0,
                spec_set_id: 0,
                trait_node_group_id: 0,
                trait_node_id: 0,
                trait_currency_id: 0,
                spent_amount_required: 0,
                flags: 0,
                required_level: 0,
                free_shared_string_id: 0,
                spend_more_shared_string_id: 0,
            },
            TraitCondEntry {
                id: 401,
                cond_type: 2,
                trait_tree_id: 10,
                granted_ranks: 1,
                quest_id: 0,
                achievement_id: 7,
                spec_set_id: 0,
                trait_node_group_id: 0,
                trait_node_id: 0,
                trait_currency_id: 0,
                spent_amount_required: 0,
                flags: 0,
                required_level: 0,
                free_shared_string_id: 0,
                spend_more_shared_string_id: 0,
            },
            TraitCondEntry {
                id: 402,
                cond_type: 0,
                trait_tree_id: 10,
                granted_ranks: 0,
                quest_id: 0,
                achievement_id: 0,
                spec_set_id: 0,
                trait_node_group_id: 0,
                trait_node_id: 0,
                trait_currency_id: 0,
                spent_amount_required: 0,
                flags: 0,
                required_level: 60,
                free_shared_string_id: 0,
                spend_more_shared_string_id: 0,
            },
            TraitCondEntry {
                id: 403,
                cond_type: 0,
                trait_tree_id: 10,
                granted_ranks: 0,
                quest_id: 0,
                achievement_id: 0,
                spec_set_id: 9,
                trait_node_group_id: 0,
                trait_node_id: 0,
                trait_currency_id: 0,
                spent_amount_required: 0,
                flags: 0,
                required_level: 0,
                free_shared_string_id: 0,
                spend_more_shared_string_id: 0,
            },
            TraitCondEntry {
                id: 404,
                cond_type: 0,
                trait_tree_id: 10,
                granted_ranks: 0,
                quest_id: 0,
                achievement_id: 7,
                spec_set_id: 0,
                trait_node_group_id: 0,
                trait_node_id: 0,
                trait_currency_id: 0,
                spent_amount_required: 0,
                flags: 0,
                required_level: 0,
                free_shared_string_id: 0,
                spend_more_shared_string_id: 0,
            },
        ]);
        let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &links,
            &trees,
            |_| false,
            |_| Vec::new(),
        )
        .with_trait_graph_like_cpp(
            &trees,
            &nodes,
            &node_entries,
            &TraitNodeEntryXTraitCondStore::from_entries([
                TraitNodeEntryXTraitCondEntry {
                    id: 500,
                    trait_cond_id: 400,
                    trait_node_entry_id: 1001,
                },
                TraitNodeEntryXTraitCondEntry {
                    id: 501,
                    trait_cond_id: 401,
                    trait_node_entry_id: 1002,
                },
                TraitNodeEntryXTraitCondEntry {
                    id: 503,
                    trait_cond_id: 404,
                    trait_node_entry_id: 1000,
                },
            ]),
            &TraitNodeEntryXTraitCostStore::from_entries([TraitNodeEntryXTraitCostEntry {
                id: 502,
                trait_node_entry_id: 1000,
                trait_cost_id: 300,
            }]),
            &TraitNodeGroupStore::from_entries([TraitNodeGroupEntry {
                id: 200,
                trait_tree_id: 10,
                flags: 0,
            }]),
            &TraitNodeGroupXTraitCondStore::from_entries([TraitNodeGroupXTraitCondEntry {
                id: 504,
                trait_cond_id: 403,
                trait_node_group_id: 200,
            }]),
            &TraitNodeGroupXTraitCostStore::from_entries([]),
            &TraitNodeGroupXTraitNodeStore::from_entries([TraitNodeGroupXTraitNodeEntry {
                id: 505,
                trait_node_group_id: 200,
                trait_node_id: 101,
                index: 0,
            }]),
            &TraitNodeXTraitCondStore::from_entries([TraitNodeXTraitCondEntry {
                id: 506,
                trait_cond_id: 402,
                trait_node_id: 100,
            }]),
            &TraitNodeXTraitCostStore::from_entries([]),
            &TraitNodeXTraitNodeEntryStore::from_entries([
                TraitNodeXTraitNodeEntryEntry {
                    id: 600,
                    trait_node_id: 100,
                    trait_node_entry_id: 1000,
                    index: 0,
                },
                TraitNodeXTraitNodeEntryEntry {
                    id: 601,
                    trait_node_id: 101,
                    trait_node_entry_id: 1001,
                    index: 0,
                },
                TraitNodeXTraitNodeEntryEntry {
                    id: 602,
                    trait_node_id: 101,
                    trait_node_entry_id: 1002,
                    index: 1,
                },
            ]),
            &TraitEdgeStore::from_entries([TraitEdgeEntry {
                id: 700,
                visual_style: 0,
                left_trait_node_id: 100,
                right_trait_node_id: 101,
                edge_type: 2,
            }]),
            &costs,
            &conditions,
            &TraitTreeLoadoutStore::from_entries([]),
            &TraitTreeLoadoutEntryStore::from_entries([]),
            &TraitTreeXTraitCostStore::from_entries([]),
        )
        .with_trait_currency_data_like_cpp(
            &TraitCurrencyStore::from_entries([TraitCurrencyEntry {
                id: 4,
                currency_type: 2,
                currency_types_id: 0,
                flags: 0,
                icon: 0,
            }]),
            &TraitCurrencySourceStore::from_entries([TraitCurrencySourceEntry {
                id: 800,
                requirement: String::new(),
                trait_currency_id: 4,
                amount: 2,
                quest_id: 0,
                achievement_id: 0,
                player_level: 0,
                trait_node_entry_id: 1000,
                order_index: 0,
            }]),
            &TraitTreeXTraitCurrencyStore::from_entries([TraitTreeXTraitCurrencyEntry {
                id: 801,
                index: 0,
                trait_tree_id: 10,
                trait_currency_id: 4,
            }]),
            &crate::SpecSetMemberStore::from_entries([crate::SpecSetMemberEntry {
                id: 802,
                chr_specialization_id: 71,
                spec_set_id: 9,
            }]),
        );
        let currency_quantities = BTreeMap::new();
        let rewarded_quest_ids = BTreeSet::from([42]);
        let achievement_ids = BTreeSet::from([7]);
        let facts = TraitPlayerFactsLikeCpp {
            level: 80,
            primary_specialization_id: 71,
            money: 0,
            currency_quantities: &currency_quantities,
            rewarded_quest_ids: &rewarded_quest_ids,
            achievement_ids: &achievement_ids,
        };
        let valid = [
            TraitConfigEntryLikeCpp {
                trait_node_id: 100,
                trait_node_entry_id: 1000,
                rank: 2,
                granted_ranks: 0,
            },
            TraitConfigEntryLikeCpp {
                trait_node_id: 101,
                trait_node_entry_id: 1001,
                rank: 1,
                granted_ranks: 0,
            },
        ];
        assert_eq!(
            index.validate_config_like_cpp(&[10], 3, 0, &valid, &facts),
            TraitConfigValidationResultLikeCpp::Ok
        );
        let invalid_selection = [
            valid[0],
            valid[1],
            TraitConfigEntryLikeCpp {
                trait_node_id: 101,
                trait_node_entry_id: 1002,
                rank: 1,
                granted_ranks: 0,
            },
        ];
        assert_eq!(
            index.validate_config_like_cpp(&[10], 3, 0, &invalid_selection, &facts),
            TraitConfigValidationResultLikeCpp::Unknown
        );
        let granted =
            index.granted_entries_for_config_like_cpp(&[10], 3, 0, &invalid_selection, &facts);
        assert_eq!(
            granted,
            vec![TraitConfigEntryLikeCpp {
                trait_node_id: 101,
                trait_node_entry_id: 1002,
                rank: 0,
                granted_ranks: 1,
            }]
        );
        let no_quest = TraitPlayerFactsLikeCpp {
            rewarded_quest_ids: &BTreeSet::new(),
            ..facts
        };
        assert_eq!(
            index.validate_config_like_cpp(&[10], 3, 0, &valid, &no_quest),
            TraitConfigValidationResultLikeCpp::Unknown
        );
        let no_achievement = TraitPlayerFactsLikeCpp {
            achievement_ids: &BTreeSet::new(),
            ..facts
        };
        assert_eq!(
            index.validate_config_like_cpp(&[10], 3, 0, &valid, &no_achievement),
            TraitConfigValidationResultLikeCpp::Unknown
        );
        let no_level = TraitPlayerFactsLikeCpp { level: 59, ..facts };
        assert_eq!(
            index.validate_config_like_cpp(&[10], 3, 0, &valid, &no_level),
            TraitConfigValidationResultLikeCpp::Unknown
        );
        let no_specialization = TraitPlayerFactsLikeCpp {
            primary_specialization_id: 72,
            ..facts
        };
        assert_eq!(
            index.validate_config_like_cpp(&[10], 3, 0, &valid, &no_specialization),
            TraitConfigValidationResultLikeCpp::Unknown
        );
    }

    #[test]
    fn load_trait_tree_db2_subbatch_when_fixtures_exist() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
        if !dbc_dir.exists() {
            eprintln!(
                "Skipping test: DB2 fixture directory not found at {}",
                dbc_dir.display()
            );
            return;
        }

        macro_rules! load_if_exists {
            ($file:literal, $store:ty) => {
                if dbc_dir.join($file).exists() {
                    let _store = <$store>::load(data_dir, locale)
                        .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
                }
            };
        }

        load_if_exists!("TraitCond.db2", TraitCondStore);
        load_if_exists!("TraitCost.db2", TraitCostStore);
        load_if_exists!("TraitCurrency.db2", TraitCurrencyStore);
        load_if_exists!("TraitCurrencySource.db2", TraitCurrencySourceStore);
        load_if_exists!("TraitDefinition.db2", TraitDefinitionStore);
        load_if_exists!(
            "TraitDefinitionEffectPoints.db2",
            TraitDefinitionEffectPointsStore
        );
        load_if_exists!("TraitEdge.db2", TraitEdgeStore);
        load_if_exists!("TraitNode.db2", TraitNodeStore);
        load_if_exists!("TraitNodeEntry.db2", TraitNodeEntryStore);
        load_if_exists!(
            "TraitNodeEntryXTraitCond.db2",
            TraitNodeEntryXTraitCondStore
        );
        load_if_exists!(
            "TraitNodeEntryXTraitCost.db2",
            TraitNodeEntryXTraitCostStore
        );
        load_if_exists!("TraitNodeGroup.db2", TraitNodeGroupStore);
        load_if_exists!(
            "TraitNodeGroupXTraitCond.db2",
            TraitNodeGroupXTraitCondStore
        );
        load_if_exists!(
            "TraitNodeGroupXTraitCost.db2",
            TraitNodeGroupXTraitCostStore
        );
        load_if_exists!(
            "TraitNodeGroupXTraitNode.db2",
            TraitNodeGroupXTraitNodeStore
        );
        load_if_exists!("TraitNodeXTraitCond.db2", TraitNodeXTraitCondStore);
        load_if_exists!("TraitNodeXTraitCost.db2", TraitNodeXTraitCostStore);
        load_if_exists!(
            "TraitNodeXTraitNodeEntry.db2",
            TraitNodeXTraitNodeEntryStore
        );
        load_if_exists!("TraitTree.db2", TraitTreeStore);
        load_if_exists!("TraitTreeLoadout.db2", TraitTreeLoadoutStore);
        load_if_exists!("TraitTreeLoadoutEntry.db2", TraitTreeLoadoutEntryStore);
        load_if_exists!("TraitTreeXTraitCost.db2", TraitTreeXTraitCostStore);
        load_if_exists!("TraitTreeXTraitCurrency.db2", TraitTreeXTraitCurrencyStore);
    }
}
