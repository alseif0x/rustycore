//! Trait tree DB2 readers.

use std::collections::{BTreeMap, BTreeSet, HashMap};

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

/// Immutable selection from `TraitTreeLoadoutEntry.db2`, retained in the process-owned
/// TraitMgr projection. The runtime does not expose DB2 rows directly to Player state.
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
            table_hash_like_cpp: Option<u32>,
        }

        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                    table_hash_like_cpp: None,
                }
            }

            fn from_entries_with_table_hash_like_cpp(
                entries: impl IntoIterator<Item = $entry>,
                table_hash: u32,
            ) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                    table_hash_like_cpp: Some(table_hash),
                }
            }

            pub fn table_hash_like_cpp(&self) -> Option<u32> {
                self.table_hash_like_cpp
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

#[path = "trait_tree_hotfix.rs"]
mod trait_tree_hotfix;
#[path = "trait_tree_locale.rs"]
mod trait_tree_locale;
#[path = "trait_tree_semantics.rs"]
mod trait_tree_semantics;
pub use trait_tree_hotfix::{
    TraitCatalogLocaleOverlayRowLikeCpp, TraitCatalogOverlayRowLikeCpp,
    TraitCatalogOverlayTableLikeCpp, TraitCatalogOverlayValueLikeCpp,
    compose_trait_currency_source_locale_like_cpp, compose_trait_definition_locale_like_cpp,
};
pub use trait_tree_locale::{
    TraitCurrencySourceLocaleEntry, TraitCurrencySourceLocaleStore, TraitDefinitionLocaleEntry,
    TraitDefinitionLocaleStore,
};
pub use trait_tree_semantics::{
    TraitConfigEntryLikeCpp, TraitConfigValidationResultLikeCpp, TraitPlayerFactsLikeCpp,
};

/// The startup projection built by C++ `TraitMgr::Load` from `SkillLineXTraitTree`.
/// It deliberately stores only validated immutable IDs; trait rules, costs and conditions remain owned by their respective stores.
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

#[path = "trait_tree/index.rs"]
mod index;
#[path = "trait_tree/loaders.rs"]
mod loaders;
#[cfg(test)]
#[path = "trait_tree/tests.rs"]
mod tests;
