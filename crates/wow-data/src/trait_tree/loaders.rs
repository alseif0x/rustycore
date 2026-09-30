//! Trait DB2 store loaders and shared WDC4 row ingestion.

use super::*;
use std::path::Path;
use anyhow::{Context, Result};
use tracing::info;
use crate::wdc4::Wdc4Reader;

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

    let store = S::from_entries_with_table_hash_like_cpp(entries, reader.table_hash());
    info!("Loaded {} rows from {}", store.len(), path.display());
    Ok(store)
}

fn f32_field(reader: &Wdc4Reader, record_idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(record_idx, field))
}

trait FromEntries<T> {
    fn from_entries(entries: impl IntoIterator<Item = T>) -> Self;
    fn from_entries_with_table_hash_like_cpp(
        entries: impl IntoIterator<Item = T>,
        table_hash: u32,
    ) -> Self;
    fn len(&self) -> usize;
}

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }

            fn from_entries_with_table_hash_like_cpp(
                entries: impl IntoIterator<Item = $entry>,
                table_hash: u32,
            ) -> Self {
                Self::from_entries_with_table_hash_like_cpp(entries, table_hash)
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

