// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item.db2 reader — loads item metadata from WDC4 format into memory.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// A minimal item record from Item.db2.
///
/// Only fields currently needed by the server are extracted.
/// Additional fields can be added later by reading more columns.
#[derive(Debug, Clone)]
pub struct ItemRecord {
    pub id: u32,
    /// Item class (0=Consumable, 1=Container, 2=Weapon, 4=Armor, …)
    pub class_id: u8,
    /// Item subclass within the class (e.g. 7=Swords for class=2)
    pub subclass_id: u8,
    /// Material type
    pub material: u8,
    /// Inventory type (1=Head, 5=Chest, 13=TwoHandWeapon, 21=MainHand, …)
    /// Signed because -1 means non-equippable.
    pub inventory_type: i8,
    /// Sheathe display type
    pub sheathe_type: u8,
    /// C++ `ItemEntry::RandomSelect`.
    pub random_select: u16,
    /// C++ `ItemEntry::ItemRandomSuffixGroupID`.
    pub random_suffix_group_id: u16,
    /// C++ `ItemEntry::ScalingStatDistributionID`.
    pub scaling_stat_distribution_id: u16,
    /// C++ `ItemEntry::ScalingStatValue`.
    pub scaling_stat_value: i32,
}

/// In-memory store of all items loaded from Item.db2.
pub struct ItemStore {
    items: HashMap<u32, ItemRecord>,
}

impl ItemStore {
    /// Build an item store from already parsed records.
    pub fn from_records(records: impl IntoIterator<Item = ItemRecord>) -> Self {
        Self {
            items: records
                .into_iter()
                .map(|record| (record.id, record))
                .collect(),
        }
    }

    /// Load Item.db2 from `{data_dir}/dbc/{locale}/Item.db2`.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Item.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut items = HashMap::with_capacity(reader.total_count());

        for (id, idx) in reader.iter_records() {
            let record = ItemRecord {
                id,
                class_id: reader.get_field_u8(idx, 0),
                subclass_id: reader.get_field_u8(idx, 1),
                material: reader.get_field_u8(idx, 2),
                inventory_type: reader.get_field_i8(idx, 3),
                sheathe_type: reader.get_field_u8(idx, 5),
                random_select: reader.get_field_u16(idx, 6),
                random_suffix_group_id: reader.get_field_u16(idx, 7),
                scaling_stat_distribution_id: reader.get_field_u16(idx, 9),
                scaling_stat_value: reader.get_field_i32(idx, 15),
            };
            items.insert(id, record);
        }

        info!("Loaded {} items from {}", items.len(), path.display());
        Ok(Self { items })
    }

    /// Look up an item record by entry ID.
    pub fn get(&self, entry_id: u32) -> Option<&ItemRecord> {
        self.items.get(&entry_id)
    }

    /// Get the equip/storage inventory type for an item (convenience method).
    ///
    /// C++ stores `ItemEntry::InventoryType` as `int8` and later exposes it
    /// through the unsigned `InventoryType` enum where 0 is non-equippable.
    /// Rust keeps negative and zero values out of equipment-slot mapping so
    /// `-1` cannot wrap to the `INVENTORY_SLOT_BAG_0=255` sentinel.
    pub fn inventory_type(&self, entry_id: u32) -> Option<u8> {
        self.items
            .get(&entry_id)
            .and_then(|r| u8::try_from(r.inventory_type).ok())
            .filter(|&inventory_type| inventory_type != 0)
    }

    pub fn random_select(&self, entry_id: u32) -> u16 {
        self.items
            .get(&entry_id)
            .map(|record| record.random_select)
            .unwrap_or(0)
    }

    pub fn random_suffix_group_id(&self, entry_id: u32) -> u16 {
        self.items
            .get(&entry_id)
            .map(|record| record.random_suffix_group_id)
            .unwrap_or(0)
    }

    pub fn scaling_stat_distribution_id(&self, entry_id: u32) -> u16 {
        self.items
            .get(&entry_id)
            .map(|record| record.scaling_stat_distribution_id)
            .unwrap_or(0)
    }

    pub fn scaling_stat_value(&self, entry_id: u32) -> i32 {
        self.items
            .get(&entry_id)
            .map(|record| record.scaling_stat_value)
            .unwrap_or(0)
    }

    /// Number of items in the store.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_item_store() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Item.db2");
        if !path.exists() {
            eprintln!("Skipping test: Item.db2 not found at {}", path.display());
            return;
        }

        let store = ItemStore::load(data_dir, locale).expect("failed to load ItemStore");
        assert!(
            store.len() > 20000,
            "expected >20k items, got {}",
            store.len()
        );

        // Thunderfury, Blessed Blade of the Windseeker (entry 19019)
        if let Some(tf) = store.get(19019) {
            assert_eq!(tf.class_id, 2, "Thunderfury should be Weapon class");
            assert_eq!(
                tf.inventory_type, 13,
                "Thunderfury should be One-Hand Weapon"
            );
        }

        // Hearthstone (entry 6948) — not equippable
        if let Some(hs) = store.get(6948) {
            assert_eq!(hs.class_id, 15, "Hearthstone should be class 15 (Misc)");
        }
    }

    #[test]
    fn inventory_type_does_not_wrap_non_equippable_values() {
        let store = ItemStore {
            items: std::collections::HashMap::from([
                (
                    1,
                    ItemRecord {
                        id: 1,
                        class_id: 15,
                        subclass_id: 0,
                        material: 0,
                        inventory_type: -1,
                        sheathe_type: 0,
                        random_select: 0,
                        random_suffix_group_id: 0,
                        scaling_stat_distribution_id: 0,
                        scaling_stat_value: 0,
                    },
                ),
                (
                    2,
                    ItemRecord {
                        id: 2,
                        class_id: 15,
                        subclass_id: 0,
                        material: 0,
                        inventory_type: 0,
                        sheathe_type: 0,
                        random_select: 0,
                        random_suffix_group_id: 0,
                        scaling_stat_distribution_id: 0,
                        scaling_stat_value: 0,
                    },
                ),
                (
                    3,
                    ItemRecord {
                        id: 3,
                        class_id: 2,
                        subclass_id: 7,
                        material: 1,
                        inventory_type: 13,
                        sheathe_type: 3,
                        random_select: 0,
                        random_suffix_group_id: 0,
                        scaling_stat_distribution_id: 77,
                        scaling_stat_value: 0x200,
                    },
                ),
            ]),
        };

        assert_eq!(store.inventory_type(1), None);
        assert_eq!(store.inventory_type(2), None);
        assert_eq!(store.inventory_type(3), Some(13));
        assert_eq!(store.scaling_stat_distribution_id(3), 77);
        assert_eq!(store.scaling_stat_value(3), 0x200);
        assert_eq!(store.scaling_stat_distribution_id(4), 0);
        assert_eq!(store.scaling_stat_value(4), 0);
    }
}
