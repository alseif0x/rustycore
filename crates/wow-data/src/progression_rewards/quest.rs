//! Quest packets.
//!
//! Separated from progression_rewards.rs under #691.

use super::*;

pub const QUEST_PACKAGE_FILTER_LOOT_SPECIALIZATION_LIKE_CPP: u8 = 0;

pub const QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP: u8 = 1;

pub const QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP: u8 = 2;

pub const QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestFactionRewardEntry {
    pub id: u32,
    pub difficulty: [i16; 10],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestInfoEntry {
    pub id: u32,
    pub info_name: String,
    pub quest_type: i8,
    pub modifiers: i32,
    pub profession: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLineXQuestEntry {
    pub id: u32,
    pub quest_line_id: u32,
    pub quest_id: u32,
    pub order_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestMoneyRewardEntry {
    pub id: u32,
    pub difficulty: [u32; 10],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPackageItemEntry {
    pub id: u32,
    pub package_id: u16,
    pub item_id: i32,
    pub item_quantity: u32,
    pub display_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestSortEntry {
    pub id: u32,
    pub sort_name: String,
    pub ui_order_index: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestV2Entry {
    pub id: u32,
    pub unique_bit_flag: u16,
}

impl QuestFactionRewardStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "QuestFactionReward.db2", |id, idx, r| {
            QuestFactionRewardEntry {
                id,
                difficulty: std::array::from_fn(|i| r.get_array_i16(idx, 0, i)),
            }
        })
    }
}

impl QuestInfoStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "QuestInfo.db2", |id, idx, r| {
            QuestInfoEntry {
                id,
                info_name: r.get_field_string(idx, 0),
                quest_type: r.get_field_i8(idx, 1),
                modifiers: r.get_field_i32(idx, 2),
                profession: r.get_field_u16(idx, 3),
            }
        })
    }
}

impl QuestLineXQuestStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "QuestLineXQuest.db2", |id, idx, r| {
            QuestLineXQuestEntry {
                id,
                quest_line_id: r.get_relationship_id(idx).unwrap_or(0),
                quest_id: r.get_field_u32(idx, 1),
                order_index: r.get_field_u32(idx, 2),
            }
        })
    }
}

impl QuestMoneyRewardStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "QuestMoneyReward.db2", |id, idx, r| {
            QuestMoneyRewardEntry {
                id,
                difficulty: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 32)),
            }
        })
    }
}

impl QuestPackageItemStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "QuestPackageItem.db2", |id, idx, r| {
            QuestPackageItemEntry {
                id,
                package_id: r.get_field_u16(idx, 0),
                item_id: r.get_field_i32(idx, 1),
                item_quantity: r.get_field_u32(idx, 2),
                display_type: r.get_field_u8(idx, 3),
            }
        })
    }

    /// C++ `DB2Manager::GetQuestPackageItems`: package members whose
    /// `DisplayType` is not `QUEST_PACKAGE_FILTER_UNMATCHED`.
    pub fn quest_package_items_like_cpp(
        &self,
        package_id: u32,
    ) -> impl Iterator<Item = &QuestPackageItemEntry> {
        self.entries.values().filter(move |entry| {
            u32::from(entry.package_id) == package_id
                && entry.display_type != QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP
        })
    }

    /// C++ `DB2Manager::GetQuestPackageItemsFallback`: package members whose
    /// `DisplayType` is `QUEST_PACKAGE_FILTER_UNMATCHED`.
    pub fn quest_package_items_fallback_like_cpp(
        &self,
        package_id: u32,
    ) -> impl Iterator<Item = &QuestPackageItemEntry> {
        self.entries.values().filter(move |entry| {
            u32::from(entry.package_id) == package_id
                && entry.display_type == QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP
        })
    }
}

impl QuestSortStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "QuestSort.db2", |id, idx, r| {
            QuestSortEntry {
                id,
                sort_name: r.get_field_string(idx, 0),
                ui_order_index: r.get_field_i8(idx, 1),
            }
        })
    }
}

impl QuestV2Store {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "QuestV2.db2", |id, idx, r| QuestV2Entry {
            id,
            unique_bit_flag: r.get_field_u16(idx, 0),
        })
    }

    /// Mirrors TrinityCore `DB2Manager::GetQuestUniqueBitFlag`.
    pub fn get_quest_unique_bit_flag_like_cpp(&self, quest_id: u32) -> u32 {
        self.get(quest_id)
            .map_or(0, |entry| u32::from(entry.unique_bit_flag))
    }
}
