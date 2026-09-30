//! Immutable TraitMgr skill-line and graph projection.

use super::*;
use crate::skill_talent::SkillLineXTraitTreeStore;

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

    /// Extend the skill-line projection with the immutable graph assembled by C++ `TraitMgr::Load`.
    /// Relation rows are accepted only when both sides resolve in the effective stores; this preserves C++'s fail-closed lookup behaviour while giving production consumers one canonical index.
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
