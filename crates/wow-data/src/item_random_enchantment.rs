// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! item_random_enchantment_template loader.

use std::collections::HashMap;

use tracing::warn;

use crate::{ItemRandomPropertiesStore, ItemRandomSuffixStore};

/// C++ `item_random_enchantment_template` row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemRandomEnchantmentTemplateEntry {
    pub group_id: u32,
    pub enchantment_id: u32,
    pub chance: f64,
}

/// C++ `ItemEnchantmentMgr::_storage`.
pub struct ItemRandomEnchantmentTemplateStore {
    groups: HashMap<u32, Vec<ItemRandomEnchantmentTemplateEntry>>,
}

impl ItemRandomEnchantmentTemplateStore {
    pub fn from_entries(
        entries: impl IntoIterator<Item = ItemRandomEnchantmentTemplateEntry>,
    ) -> Self {
        let mut groups: HashMap<u32, Vec<ItemRandomEnchantmentTemplateEntry>> = HashMap::new();
        for entry in entries {
            groups.entry(entry.group_id).or_default().push(entry);
        }
        Self { groups }
    }

    pub fn from_entries_validated(
        entries: impl IntoIterator<Item = ItemRandomEnchantmentTemplateEntry>,
        random_properties: &ItemRandomPropertiesStore,
        random_suffixes: &ItemRandomSuffixStore,
    ) -> Self {
        Self::from_entries(entries.into_iter().filter(|entry| {
            let has_random_property = random_properties.get(entry.enchantment_id).is_some();
            let has_random_suffix = random_suffixes.get(entry.enchantment_id).is_some();
            if !has_random_property && !has_random_suffix {
                warn!(
                    enchantment_id = entry.enchantment_id,
                    group_id = entry.group_id,
                    "Skipping item random enchantment row without matching DB2 random property or suffix"
                );
                return false;
            }

            if !(0.000001..=100.0).contains(&entry.chance) {
                warn!(
                    enchantment_id = entry.enchantment_id,
                    group_id = entry.group_id,
                    chance = entry.chance,
                    "Skipping item random enchantment row with invalid chance"
                );
                return false;
            }

            true
        }))
    }

    pub fn group(&self, group_id: u32) -> Option<&[ItemRandomEnchantmentTemplateEntry]> {
        self.groups.get(&group_id).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ItemRandomPropertiesEntry, ItemRandomSuffixEntry, ItemRandomSuffixStore};

    #[test]
    fn validated_loader_filters_rows_like_cpp_load_random_enchantments_table() {
        let random_properties =
            ItemRandomPropertiesStore::from_entries([ItemRandomPropertiesEntry {
                id: 10,
                enchantments: [1, 0, 0, 0, 0],
            }]);
        let random_suffixes = ItemRandomSuffixStore::from_entries([ItemRandomSuffixEntry {
            id: 20,
            enchantments: [2, 0, 0, 0, 0],
            allocation_pct: [10000, 0, 0, 0, 0],
        }]);

        let store = ItemRandomEnchantmentTemplateStore::from_entries_validated(
            [
                ItemRandomEnchantmentTemplateEntry {
                    group_id: 1,
                    enchantment_id: 10,
                    chance: 100.0,
                },
                ItemRandomEnchantmentTemplateEntry {
                    group_id: 1,
                    enchantment_id: 20,
                    chance: 50.0,
                },
                ItemRandomEnchantmentTemplateEntry {
                    group_id: 1,
                    enchantment_id: 30,
                    chance: 50.0,
                },
                ItemRandomEnchantmentTemplateEntry {
                    group_id: 1,
                    enchantment_id: 10,
                    chance: 0.0,
                },
            ],
            &random_properties,
            &random_suffixes,
        );

        let group = store.group(1).unwrap();
        assert_eq!(group.len(), 2);
        assert_eq!(group[0].enchantment_id, 10);
        assert_eq!(group[1].enchantment_id, 20);
    }
}
