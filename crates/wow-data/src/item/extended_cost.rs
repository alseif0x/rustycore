// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! ItemExtendedCost.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_constants::ItemExtendedCostFlags;

use crate::wdc4::Wdc4Reader;

pub const MAX_ITEM_EXT_COST_ITEMS: usize = 5;
pub const MAX_ITEM_EXT_COST_CURRENCIES: usize = 5;

/// C++ `ItemExtendedCostEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemExtendedCostEntry {
    pub id: u32,
    pub required_arena_rating: u16,
    pub arena_bracket: i8,
    pub flags: ItemExtendedCostFlags,
    pub min_faction_id: u8,
    pub min_reputation: i32,
    pub required_achievement: u8,
    pub item_id: [i32; MAX_ITEM_EXT_COST_ITEMS],
    pub item_count: [u16; MAX_ITEM_EXT_COST_ITEMS],
    pub currency_id: [u16; MAX_ITEM_EXT_COST_CURRENCIES],
    pub currency_count: [u32; MAX_ITEM_EXT_COST_CURRENCIES],
}

impl ItemExtendedCostEntry {
    pub fn has_item_turnins(&self) -> bool {
        self.item_id.iter().any(|id| *id != 0)
    }

    pub fn has_currency_turnins(&self) -> bool {
        self.currency_id.iter().any(|id| *id != 0)
    }

    pub fn has_season_earned_currency_requirement(&self) -> bool {
        self.flags.intersects(
            ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1
                | ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2
                | ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3
                | ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4
                | ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5,
        )
    }

    pub fn requires_guild(&self) -> bool {
        self.flags.contains(ItemExtendedCostFlags::REQUIRE_GUILD)
    }
}

/// In-memory store for `ItemExtendedCost.db2`.
pub struct ItemExtendedCostStore {
    entries: HashMap<u32, ItemExtendedCostEntry>,
}

impl ItemExtendedCostStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemExtendedCostEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load ItemExtendedCost.db2 from `{data_dir}/dbc/{locale}/ItemExtendedCost.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemExtendedCostEntry`
    /// - `DB2LoadInfo.h::ItemExtendedCostLoadInfo`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemExtendedCost.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let record = ItemExtendedCostEntry {
                id,
                required_arena_rating: reader.get_field_u16(idx, 0),
                arena_bracket: reader.get_field_i8(idx, 1),
                flags: ItemExtendedCostFlags::from_bits_retain(u32::from(
                    reader.get_field_u8(idx, 2),
                )),
                min_faction_id: reader.get_field_u8(idx, 3),
                min_reputation: reader.get_field_i32(idx, 4),
                required_achievement: reader.get_field_u8(idx, 5),
                item_id: [
                    reader.get_array_element(idx, 6, 0, 32) as i32,
                    reader.get_array_element(idx, 6, 1, 32) as i32,
                    reader.get_array_element(idx, 6, 2, 32) as i32,
                    reader.get_array_element(idx, 6, 3, 32) as i32,
                    reader.get_array_element(idx, 6, 4, 32) as i32,
                ],
                item_count: [
                    reader.get_array_element(idx, 7, 0, 16) as u16,
                    reader.get_array_element(idx, 7, 1, 16) as u16,
                    reader.get_array_element(idx, 7, 2, 16) as u16,
                    reader.get_array_element(idx, 7, 3, 16) as u16,
                    reader.get_array_element(idx, 7, 4, 16) as u16,
                ],
                currency_id: [
                    reader.get_array_element(idx, 8, 0, 16) as u16,
                    reader.get_array_element(idx, 8, 1, 16) as u16,
                    reader.get_array_element(idx, 8, 2, 16) as u16,
                    reader.get_array_element(idx, 8, 3, 16) as u16,
                    reader.get_array_element(idx, 8, 4, 16) as u16,
                ],
                currency_count: [
                    reader.get_array_element(idx, 9, 0, 32),
                    reader.get_array_element(idx, 9, 1, 32),
                    reader.get_array_element(idx, 9, 2, 32),
                    reader.get_array_element(idx, 9, 3, 32),
                    reader.get_array_element(idx, 9, 4, 32),
                ],
            };
            entries.insert(id, record);
        }

        info!(
            "Loaded {} item extended costs from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&ItemExtendedCostEntry> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_extended_cost_helpers_match_cpp_fields() {
        let entry = ItemExtendedCostEntry {
            id: 1,
            required_arena_rating: 0,
            arena_bracket: -1,
            flags: ItemExtendedCostFlags::REQUIRE_GUILD
                | ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2,
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [100, 0, 0, 0, 0],
            item_count: [3, 0, 0, 0, 0],
            currency_id: [395, 0, 0, 0, 0],
            currency_count: [25, 0, 0, 0, 0],
        };

        assert!(entry.has_item_turnins());
        assert!(entry.has_currency_turnins());
        assert!(entry.has_season_earned_currency_requirement());
        assert!(entry.requires_guild());
    }

    #[test]
    fn load_item_extended_cost_store() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemExtendedCost.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: ItemExtendedCost.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store = ItemExtendedCostStore::load(data_dir, locale)
            .expect("failed to load ItemExtendedCostStore");
        assert!(!store.is_empty());
    }
}
