//! Effective SQL hotfix composition for the C++ TraitMgr catalog.
//!
//! This module deliberately owns only the startup data boundary.  It applies
//! WDC4, official SQL, custom SQL and final `RecordRemoved` tombstones to each
//! store before the graph projection is built.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::Db2HotfixRemovalStoreLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitCatalogOverlayTableLikeCpp {
    SpecSetMember,
    TraitCond,
    TraitCost,
    TraitCurrency,
    TraitCurrencySource,
    TraitDefinition,
    TraitDefinitionEffectPoints,
    TraitEdge,
    TraitNode,
    TraitNodeEntry,
    TraitNodeEntryXTraitCond,
    TraitNodeEntryXTraitCost,
    TraitNodeGroup,
    TraitNodeGroupXTraitCond,
    TraitNodeGroupXTraitCost,
    TraitNodeGroupXTraitNode,
    TraitNodeXTraitCond,
    TraitNodeXTraitCost,
    TraitNodeXTraitNodeEntry,
    TraitTree,
    TraitTreeLoadout,
    TraitTreeLoadoutEntry,
    TraitTreeXTraitCost,
    TraitTreeXTraitCurrency,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraitCatalogOverlayValueLikeCpp {
    Integer(i128),
    Real(f64),
    Text(String),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitCatalogOverlayRowLikeCpp {
    pub table: TraitCatalogOverlayTableLikeCpp,
    pub values: Vec<TraitCatalogOverlayValueLikeCpp>,
}

impl TraitCatalogOverlayRowLikeCpp {
    fn value(&self, index: usize) -> Result<&TraitCatalogOverlayValueLikeCpp> {
        self.values
            .get(index)
            .with_context(|| format!("missing Trait catalog SQL column {index}"))
    }
}

fn integer(row: &TraitCatalogOverlayRowLikeCpp, index: usize) -> Result<i128> {
    match row.value(index)? {
        TraitCatalogOverlayValueLikeCpp::Integer(value) => Ok(*value),
        TraitCatalogOverlayValueLikeCpp::Real(value) => Ok(*value as i128),
        other => bail!("Trait catalog SQL column {index} is not integer: {other:?}"),
    }
}

fn real(row: &TraitCatalogOverlayRowLikeCpp, index: usize) -> Result<f32> {
    match row.value(index)? {
        TraitCatalogOverlayValueLikeCpp::Real(value) => Ok(*value as f32),
        TraitCatalogOverlayValueLikeCpp::Integer(value) => Ok(*value as f32),
        other => bail!("Trait catalog SQL column {index} is not numeric: {other:?}"),
    }
}

fn text(row: &TraitCatalogOverlayRowLikeCpp, index: usize) -> Result<String> {
    match row.value(index)? {
        TraitCatalogOverlayValueLikeCpp::Text(value) => Ok(value.clone()),
        TraitCatalogOverlayValueLikeCpp::Null => Ok(String::new()),
        other => bail!("Trait catalog SQL column {index} is not text: {other:?}"),
    }
}

fn u32_value(row: &TraitCatalogOverlayRowLikeCpp, index: usize) -> Result<u32> {
    u32::try_from(integer(row, index)?)
        .with_context(|| format!("Trait catalog column {index} is not u32"))
}

fn i32_value(row: &TraitCatalogOverlayRowLikeCpp, index: usize) -> Result<i32> {
    i32::try_from(integer(row, index)?)
        .with_context(|| format!("Trait catalog column {index} is not i32"))
}

fn u8_value(row: &TraitCatalogOverlayRowLikeCpp, index: usize) -> Result<u8> {
    u8::try_from(integer(row, index)?)
        .with_context(|| format!("Trait catalog column {index} is not u8"))
}

fn compose_store<T>(
    entries: &mut HashMap<u32, T>,
    table_hash: Option<u32>,
    official: impl IntoIterator<Item = TraitCatalogOverlayRowLikeCpp>,
    custom: impl IntoIterator<Item = TraitCatalogOverlayRowLikeCpp>,
    table: TraitCatalogOverlayTableLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    convert: impl Fn(&TraitCatalogOverlayRowLikeCpp) -> Result<T>,
    id: impl Fn(&T) -> u32,
) -> Result<()> {
    let table_hash =
        table_hash.with_context(|| format!("{table:?} store is missing its WDC4 table hash"))?;
    for row in official.into_iter().chain(custom) {
        if row.table != table {
            continue;
        }
        let value = convert(&row)?;
        entries.insert(id(&value), value);
    }
    entries.retain(|record_id, _| {
        i32::try_from(*record_id)
            .map(|id| !removals.contains_like_cpp(table_hash, id))
            .unwrap_or(true)
    });
    Ok(())
}

macro_rules! store_overlay {
    ($store:path, $entry:ty, $table:ident, $convert:path) => {
        impl $store {
            pub fn apply_trait_catalog_hotfix_like_cpp(
                mut self,
                official: impl IntoIterator<Item = TraitCatalogOverlayRowLikeCpp>,
                custom: impl IntoIterator<Item = TraitCatalogOverlayRowLikeCpp>,
                removals: &Db2HotfixRemovalStoreLikeCpp,
            ) -> Result<Self> {
                compose_store(
                    &mut self.entries,
                    self.table_hash_like_cpp,
                    official,
                    custom,
                    TraitCatalogOverlayTableLikeCpp::$table,
                    removals,
                    $convert,
                    |entry: &$entry| entry.id,
                )?;
                Ok(self)
            }
        }
    };
}

fn spec_set_member(row: &TraitCatalogOverlayRowLikeCpp) -> Result<crate::SpecSetMemberEntry> {
    Ok(crate::SpecSetMemberEntry {
        id: u32_value(row, 0)?,
        chr_specialization_id: i32_value(row, 1)?,
        spec_set_id: u32_value(row, 2)?,
    })
}
fn trait_cond(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitCondEntry> {
    Ok(super::TraitCondEntry {
        id: u32_value(row, 0)?,
        cond_type: i32_value(row, 1)?,
        trait_tree_id: u32_value(row, 2)?,
        granted_ranks: i32_value(row, 3)?,
        quest_id: i32_value(row, 4)?,
        achievement_id: i32_value(row, 5)?,
        spec_set_id: i32_value(row, 6)?,
        trait_node_group_id: i32_value(row, 7)?,
        trait_node_id: i32_value(row, 8)?,
        trait_currency_id: i32_value(row, 9)?,
        spent_amount_required: i32_value(row, 10)?,
        flags: i32_value(row, 11)?,
        required_level: i32_value(row, 12)?,
        free_shared_string_id: i32_value(row, 13)?,
        spend_more_shared_string_id: i32_value(row, 14)?,
    })
}
fn trait_cost(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitCostEntry> {
    Ok(super::TraitCostEntry {
        internal_name: text(row, 0)?,
        id: u32_value(row, 1)?,
        amount: i32_value(row, 2)?,
        trait_currency_id: i32_value(row, 3)?,
    })
}
fn trait_currency(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitCurrencyEntry> {
    Ok(super::TraitCurrencyEntry {
        id: u32_value(row, 0)?,
        currency_type: i32_value(row, 1)?,
        currency_types_id: i32_value(row, 2)?,
        flags: i32_value(row, 3)?,
        icon: i32_value(row, 4)?,
    })
}
fn trait_currency_source(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitCurrencySourceEntry> {
    Ok(super::TraitCurrencySourceEntry {
        requirement: text(row, 0)?,
        id: u32_value(row, 1)?,
        trait_currency_id: u32_value(row, 2)?,
        amount: i32_value(row, 3)?,
        quest_id: i32_value(row, 4)?,
        achievement_id: i32_value(row, 5)?,
        player_level: i32_value(row, 6)?,
        trait_node_entry_id: i32_value(row, 7)?,
        order_index: i32_value(row, 8)?,
    })
}
fn trait_definition(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitDefinitionEntry> {
    Ok(super::TraitDefinitionEntry {
        override_name: text(row, 0)?,
        override_subtext: text(row, 1)?,
        override_description: text(row, 2)?,
        id: u32_value(row, 3)?,
        spell_id: i32_value(row, 4)?,
        override_icon: i32_value(row, 5)?,
        overrides_spell_id: i32_value(row, 6)?,
        visible_spell_id: i32_value(row, 7)?,
    })
}
fn trait_definition_effect_points(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitDefinitionEffectPointsEntry> {
    Ok(super::TraitDefinitionEffectPointsEntry {
        id: u32_value(row, 0)?,
        trait_definition_id: u32_value(row, 1)?,
        effect_index: i32_value(row, 2)?,
        operation_type: i32_value(row, 3)?,
        curve_id: i32_value(row, 4)?,
    })
}
fn trait_edge(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitEdgeEntry> {
    Ok(super::TraitEdgeEntry {
        id: u32_value(row, 0)?,
        visual_style: i32_value(row, 1)?,
        left_trait_node_id: u32_value(row, 2)?,
        right_trait_node_id: i32_value(row, 3)?,
        edge_type: i32_value(row, 4)?,
    })
}
fn trait_node(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitNodeEntry> {
    Ok(super::TraitNodeEntry {
        id: u32_value(row, 0)?,
        trait_tree_id: u32_value(row, 1)?,
        pos_x: i32_value(row, 2)?,
        pos_y: i32_value(row, 3)?,
        node_type: u8_value(row, 4)?,
        flags: i32_value(row, 5)?,
    })
}
fn trait_node_entry(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitNodeEntryEntry> {
    Ok(super::TraitNodeEntryEntry {
        id: u32_value(row, 0)?,
        trait_definition_id: i32_value(row, 1)?,
        max_ranks: i32_value(row, 2)?,
        node_entry_type: u8_value(row, 3)?,
    })
}
fn trait_node_entry_x_trait_cond(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitNodeEntryXTraitCondEntry> {
    Ok(super::TraitNodeEntryXTraitCondEntry {
        id: u32_value(row, 0)?,
        trait_cond_id: i32_value(row, 1)?,
        trait_node_entry_id: u32_value(row, 2)?,
    })
}
fn trait_node_entry_x_trait_cost(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitNodeEntryXTraitCostEntry> {
    Ok(super::TraitNodeEntryXTraitCostEntry {
        id: u32_value(row, 0)?,
        trait_node_entry_id: u32_value(row, 1)?,
        trait_cost_id: i32_value(row, 2)?,
    })
}
fn trait_node_group(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitNodeGroupEntry> {
    Ok(super::TraitNodeGroupEntry {
        id: u32_value(row, 0)?,
        trait_tree_id: u32_value(row, 1)?,
        flags: i32_value(row, 2)?,
    })
}
fn trait_node_group_x_trait_cond(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitNodeGroupXTraitCondEntry> {
    Ok(super::TraitNodeGroupXTraitCondEntry {
        id: u32_value(row, 0)?,
        trait_cond_id: i32_value(row, 1)?,
        trait_node_group_id: u32_value(row, 2)?,
    })
}
fn trait_node_group_x_trait_cost(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitNodeGroupXTraitCostEntry> {
    Ok(super::TraitNodeGroupXTraitCostEntry {
        id: u32_value(row, 0)?,
        trait_node_group_id: u32_value(row, 1)?,
        trait_cost_id: i32_value(row, 2)?,
    })
}
fn trait_node_group_x_trait_node(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitNodeGroupXTraitNodeEntry> {
    Ok(super::TraitNodeGroupXTraitNodeEntry {
        id: u32_value(row, 0)?,
        trait_node_group_id: u32_value(row, 1)?,
        trait_node_id: i32_value(row, 2)?,
        index: i32_value(row, 3)?,
    })
}
fn trait_node_x_trait_cond(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitNodeXTraitCondEntry> {
    Ok(super::TraitNodeXTraitCondEntry {
        id: u32_value(row, 0)?,
        trait_cond_id: i32_value(row, 1)?,
        trait_node_id: u32_value(row, 2)?,
    })
}
fn trait_node_x_trait_cost(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitNodeXTraitCostEntry> {
    Ok(super::TraitNodeXTraitCostEntry {
        id: u32_value(row, 0)?,
        trait_node_id: u32_value(row, 1)?,
        trait_cost_id: i32_value(row, 2)?,
    })
}
fn trait_node_x_trait_node_entry(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitNodeXTraitNodeEntryEntry> {
    Ok(super::TraitNodeXTraitNodeEntryEntry {
        id: u32_value(row, 0)?,
        trait_node_id: u32_value(row, 1)?,
        trait_node_entry_id: i32_value(row, 2)?,
        index: i32_value(row, 3)?,
    })
}
fn trait_tree(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitTreeEntry> {
    Ok(super::TraitTreeEntry {
        id: u32_value(row, 0)?,
        trait_system_id: u32_value(row, 1)?,
        unused1000_1: i32_value(row, 2)?,
        first_trait_node_id: i32_value(row, 3)?,
        player_condition_id: i32_value(row, 4)?,
        flags: i32_value(row, 5)?,
        unused1000_2: real(row, 6)?,
        unused1000_3: real(row, 7)?,
    })
}
fn trait_tree_loadout(row: &TraitCatalogOverlayRowLikeCpp) -> Result<super::TraitTreeLoadoutEntry> {
    Ok(super::TraitTreeLoadoutEntry {
        id: u32_value(row, 0)?,
        trait_tree_id: u32_value(row, 1)?,
        chr_specialization_id: i32_value(row, 2)?,
    })
}
fn trait_tree_loadout_entry(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitTreeLoadoutEntryEntry> {
    Ok(super::TraitTreeLoadoutEntryEntry {
        id: u32_value(row, 0)?,
        trait_tree_loadout_id: u32_value(row, 1)?,
        selected_trait_node_id: i32_value(row, 2)?,
        selected_trait_node_entry_id: i32_value(row, 3)?,
        num_points: i32_value(row, 4)?,
        order_index: i32_value(row, 5)?,
    })
}
fn trait_tree_x_trait_cost(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitTreeXTraitCostEntry> {
    Ok(super::TraitTreeXTraitCostEntry {
        id: u32_value(row, 0)?,
        trait_tree_id: u32_value(row, 1)?,
        trait_cost_id: i32_value(row, 2)?,
    })
}
fn trait_tree_x_trait_currency(
    row: &TraitCatalogOverlayRowLikeCpp,
) -> Result<super::TraitTreeXTraitCurrencyEntry> {
    Ok(super::TraitTreeXTraitCurrencyEntry {
        id: u32_value(row, 0)?,
        index: i32_value(row, 1)?,
        trait_tree_id: u32_value(row, 2)?,
        trait_currency_id: i32_value(row, 3)?,
    })
}

store_overlay!(
    super::TraitCondStore,
    super::TraitCondEntry,
    TraitCond,
    trait_cond
);
store_overlay!(
    super::TraitCostStore,
    super::TraitCostEntry,
    TraitCost,
    trait_cost
);
store_overlay!(
    super::TraitCurrencyStore,
    super::TraitCurrencyEntry,
    TraitCurrency,
    trait_currency
);
store_overlay!(
    super::TraitCurrencySourceStore,
    super::TraitCurrencySourceEntry,
    TraitCurrencySource,
    trait_currency_source
);
store_overlay!(
    super::TraitDefinitionStore,
    super::TraitDefinitionEntry,
    TraitDefinition,
    trait_definition
);
store_overlay!(
    super::TraitDefinitionEffectPointsStore,
    super::TraitDefinitionEffectPointsEntry,
    TraitDefinitionEffectPoints,
    trait_definition_effect_points
);
store_overlay!(
    super::TraitEdgeStore,
    super::TraitEdgeEntry,
    TraitEdge,
    trait_edge
);
store_overlay!(
    super::TraitNodeStore,
    super::TraitNodeEntry,
    TraitNode,
    trait_node
);
store_overlay!(
    super::TraitNodeEntryStore,
    super::TraitNodeEntryEntry,
    TraitNodeEntry,
    trait_node_entry
);
store_overlay!(
    super::TraitNodeEntryXTraitCondStore,
    super::TraitNodeEntryXTraitCondEntry,
    TraitNodeEntryXTraitCond,
    trait_node_entry_x_trait_cond
);
store_overlay!(
    super::TraitNodeEntryXTraitCostStore,
    super::TraitNodeEntryXTraitCostEntry,
    TraitNodeEntryXTraitCost,
    trait_node_entry_x_trait_cost
);
store_overlay!(
    super::TraitNodeGroupStore,
    super::TraitNodeGroupEntry,
    TraitNodeGroup,
    trait_node_group
);
store_overlay!(
    super::TraitNodeGroupXTraitCondStore,
    super::TraitNodeGroupXTraitCondEntry,
    TraitNodeGroupXTraitCond,
    trait_node_group_x_trait_cond
);
store_overlay!(
    super::TraitNodeGroupXTraitCostStore,
    super::TraitNodeGroupXTraitCostEntry,
    TraitNodeGroupXTraitCost,
    trait_node_group_x_trait_cost
);
store_overlay!(
    super::TraitNodeGroupXTraitNodeStore,
    super::TraitNodeGroupXTraitNodeEntry,
    TraitNodeGroupXTraitNode,
    trait_node_group_x_trait_node
);
store_overlay!(
    super::TraitNodeXTraitCondStore,
    super::TraitNodeXTraitCondEntry,
    TraitNodeXTraitCond,
    trait_node_x_trait_cond
);
store_overlay!(
    super::TraitNodeXTraitCostStore,
    super::TraitNodeXTraitCostEntry,
    TraitNodeXTraitCost,
    trait_node_x_trait_cost
);
store_overlay!(
    super::TraitNodeXTraitNodeEntryStore,
    super::TraitNodeXTraitNodeEntryEntry,
    TraitNodeXTraitNodeEntry,
    trait_node_x_trait_node_entry
);
store_overlay!(
    super::TraitTreeStore,
    super::TraitTreeEntry,
    TraitTree,
    trait_tree
);
store_overlay!(
    super::TraitTreeLoadoutStore,
    super::TraitTreeLoadoutEntry,
    TraitTreeLoadout,
    trait_tree_loadout
);
store_overlay!(
    super::TraitTreeLoadoutEntryStore,
    super::TraitTreeLoadoutEntryEntry,
    TraitTreeLoadoutEntry,
    trait_tree_loadout_entry
);
store_overlay!(
    super::TraitTreeXTraitCostStore,
    super::TraitTreeXTraitCostEntry,
    TraitTreeXTraitCost,
    trait_tree_x_trait_cost
);
store_overlay!(
    super::TraitTreeXTraitCurrencyStore,
    super::TraitTreeXTraitCurrencyEntry,
    TraitTreeXTraitCurrency,
    trait_tree_x_trait_currency
);

impl crate::SpecSetMemberStore {
    pub fn apply_trait_catalog_hotfix_like_cpp(
        mut self,
        official: impl IntoIterator<Item = TraitCatalogOverlayRowLikeCpp>,
        custom: impl IntoIterator<Item = TraitCatalogOverlayRowLikeCpp>,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        compose_store(
            &mut self.entries,
            self.table_hash_like_cpp,
            official,
            custom,
            TraitCatalogOverlayTableLikeCpp::SpecSetMember,
            removals,
            spec_set_member,
            |entry: &crate::SpecSetMemberEntry| entry.id,
        )?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integer(value: i128) -> TraitCatalogOverlayValueLikeCpp {
        TraitCatalogOverlayValueLikeCpp::Integer(value)
    }

    #[test]
    fn trait_store_applies_custom_precedence_and_final_tombstones() {
        let table_hash = 0xCAFE_BABEu32;
        let base = super::super::TraitCondStore::from_entries_with_table_hash_like_cpp(
            [super::super::TraitCondEntry {
                id: 7,
                cond_type: 1,
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
                required_level: 0,
                free_shared_string_id: 0,
                spend_more_shared_string_id: 0,
            }],
            table_hash,
        );
        let replacement = TraitCatalogOverlayRowLikeCpp {
            table: TraitCatalogOverlayTableLikeCpp::TraitCond,
            values: (0..15).map(integer).collect(),
        };
        let tombstone =
            Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(table_hash, 7, 2)]);
        let effective = base
            .apply_trait_catalog_hotfix_like_cpp([], [replacement], &tombstone)
            .expect("WDC4 hash must permit overlay composition");
        assert!(
            effective.get(7).is_none(),
            "RecordRemoved is applied after custom SQL"
        );
    }

    #[test]
    fn malformed_trait_row_fails_before_store_publication() {
        let store = super::super::TraitCostStore::from_entries_with_table_hash_like_cpp([], 1);
        let malformed = TraitCatalogOverlayRowLikeCpp {
            table: TraitCatalogOverlayTableLikeCpp::TraitCost,
            values: vec![TraitCatalogOverlayValueLikeCpp::Text("name".into())],
        };
        let error = match store.apply_trait_catalog_hotfix_like_cpp(
            [malformed],
            [],
            &Db2HotfixRemovalStoreLikeCpp::default(),
        ) {
            Ok(_) => panic!("short SQL rows must be rejected"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("missing Trait catalog SQL column")
        );
    }
}
