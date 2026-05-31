//! Quest, reward, criteria, faction, curve and scaling DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

pub const QUEST_PACKAGE_FILTER_LOOT_SPECIALIZATION_LIKE_CPP: u8 = 0;
pub const QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP: u8 = 1;
pub const QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP: u8 = 2;
pub const QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP: u8 = 3;
pub const FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP: u16 = 0x0000_1000;
pub const FACTION_TEMPLATE_FLAG_HOSTILE_BY_DEFAULT_LIKE_CPP: u16 = 0x0000_2000;
pub const FACTION_MASK_PLAYER_LIKE_CPP: u8 = 0x01;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementCategoryEntry {
    pub id: u32,
    pub name: String,
    pub parent: i16,
    pub ui_order: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentTuningEntry {
    pub id: u32,
    pub min_level: i32,
    pub max_level: i32,
    pub flags: i32,
    pub expected_stat_mod_id: i32,
    pub difficulty_esm_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriteriaTreeEntry {
    pub id: u32,
    pub description: String,
    pub parent: u32,
    pub amount: u32,
    pub operator: i32,
    pub criteria_id: u32,
    pub order_index: i32,
    pub flags: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurveEntry {
    pub id: u32,
    pub curve_type: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurvePointEntry {
    pub id: u32,
    pub pos: [f32; 2],
    pub pre_sl_squish_pos: [f32; 2],
    pub curve_id: u32,
    pub order_index: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FactionEntry {
    pub id: u32,
    pub reputation_race_mask: [i64; 4],
    pub reputation_index: i16,
    pub parent_faction_id: u16,
    pub friendship_rep_id: u8,
    pub flags: i32,
    pub paragon_faction_id: u16,
    pub renown_faction_id: i32,
    pub renown_currency_id: i32,
    pub reputation_class_mask: [i16; 4],
    pub reputation_flags: [u16; 4],
    pub reputation_base: [i32; 4],
    pub reputation_max: [i32; 4],
    pub parent_faction_mod: [f32; 2],
    pub parent_faction_cap: [u8; 2],
}

impl FactionEntry {
    pub const fn for_test_like_cpp(id: u32, reputation_index: i16) -> Self {
        Self {
            id,
            reputation_race_mask: [0; 4],
            reputation_index,
            parent_faction_id: 0,
            friendship_rep_id: 0,
            flags: 0,
            paragon_faction_id: 0,
            renown_faction_id: 0,
            renown_currency_id: 0,
            reputation_class_mask: [0; 4],
            reputation_flags: [0; 4],
            reputation_base: [0; 4],
            reputation_max: [0; 4],
            parent_faction_mod: [0.0; 2],
            parent_faction_cap: [0; 2],
        }
    }

    pub const fn can_have_reputation_like_cpp(&self) -> bool {
        self.reputation_index >= 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactionTemplateEntry {
    pub id: u32,
    pub faction: u16,
    pub flags: u16,
    pub faction_group: u8,
    pub friend_group: u8,
    pub enemy_group: u8,
    pub enemies: [u16; 8],
    pub friend: [u16; 8],
}

impl FactionTemplateEntry {
    pub fn is_friendly_to_like_cpp(&self, entry: &Self) -> bool {
        if self.id == entry.id {
            return true;
        }
        if entry.faction != 0 {
            if self.enemies.contains(&entry.faction) {
                return false;
            }
            if self.friend.contains(&entry.faction) {
                return true;
            }
        }
        (self.friend_group & entry.faction_group) != 0
            || (self.faction_group & entry.friend_group) != 0
    }

    pub fn is_hostile_to_like_cpp(&self, entry: &Self) -> bool {
        if self.id == entry.id {
            return false;
        }
        if entry.faction != 0 {
            if self.enemies.contains(&entry.faction) {
                return true;
            }
            if self.friend.contains(&entry.faction) {
                return false;
            }
        }
        (self.enemy_group & entry.faction_group) != 0
    }

    pub fn is_hostile_to_players_like_cpp(&self) -> bool {
        (self.enemy_group & FACTION_MASK_PLAYER_LIKE_CPP) != 0
    }

    pub fn is_neutral_to_all_like_cpp(&self) -> bool {
        self.enemies.iter().all(|enemy| *enemy == 0)
            && self.enemy_group == 0
            && self.friend_group == 0
    }

    pub fn is_contested_guard_faction_like_cpp(&self) -> bool {
        (self.flags & FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP) != 0
    }

    pub fn is_hostile_by_default_like_cpp(&self) -> bool {
        (self.flags & FACTION_TEMPLATE_FLAG_HOSTILE_BY_DEFAULT_LIKE_CPP) != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendshipRepReactionEntry {
    pub id: u32,
    pub reaction: String,
    pub friendship_rep_id: u8,
    pub reaction_threshold: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendshipReputationEntry {
    pub id: u32,
    pub description: String,
    pub field_34146722002: i32,
    pub field_34146722003: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifierTreeEntry {
    pub id: u32,
    pub parent: u32,
    pub operator: i8,
    pub amount: i8,
    pub modifier_type: i32,
    pub asset: i32,
    pub secondary_asset: i32,
    pub tertiary_asset: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumTalentsAtLevelEntry {
    pub id: u32,
    pub num_talents: i32,
    pub num_talents_death_knight: i32,
    pub num_talents_demon_hunter: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagonReputationEntry {
    pub id: u32,
    pub faction_id: i32,
    pub level_threshold: i32,
    pub quest_id: i32,
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct RewardPackEntry {
    pub id: u32,
    pub char_title_id: i32,
    pub money: u32,
    pub artifact_xp_difficulty: i8,
    pub artifact_xp_multiplier: f32,
    pub artifact_xp_category_id: u8,
    pub treasure_picker_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardPackXCurrencyTypeEntry {
    pub id: u32,
    pub currency_type_id: u32,
    pub quantity: i32,
    pub reward_pack_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardPackXItemEntry {
    pub id: u32,
    pub item_id: i32,
    pub item_quantity: i32,
    pub reward_pack_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalingStatDistributionEntry {
    pub id: u32,
    pub player_level_to_item_level_curve_id: u16,
    pub min_level: i32,
    pub max_level: i32,
    pub bonus: [i32; 10],
    pub stat_id: [i32; 10],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalingStatValuesEntry {
    pub id: u32,
    pub char_level: i32,
    pub weapon_dps_1h: i32,
    pub weapon_dps_2h: i32,
    pub spellcaster_dps_1h: i32,
    pub spellcaster_dps_2h: i32,
    pub ranged_dps: i32,
    pub wand_dps: i32,
    pub spell_power: i32,
    pub shoulder_budget: i32,
    pub trinket_budget: i32,
    pub weapon_budget_1h: i32,
    pub primary_budget: i32,
    pub ranged_budget: i32,
    pub tertiary_budget: i32,
    pub cloth_shoulder_armor: i32,
    pub leather_shoulder_armor: i32,
    pub mail_shoulder_armor: i32,
    pub plate_shoulder_armor: i32,
    pub cloth_cloak_armor: i32,
    pub cloth_chest_armor: i32,
    pub leather_chest_armor: i32,
    pub mail_chest_armor: i32,
    pub plate_chest_armor: i32,
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
        }
    };
}

db2_store!(AchievementCategoryStore, AchievementCategoryEntry);
db2_store!(ContentTuningStore, ContentTuningEntry);
db2_store!(CriteriaTreeStore, CriteriaTreeEntry);
db2_store!(CurveStore, CurveEntry);
db2_store!(CurvePointStore, CurvePointEntry);
db2_store!(FactionStore, FactionEntry);
db2_store!(FactionTemplateStore, FactionTemplateEntry);
db2_store!(FriendshipRepReactionStore, FriendshipRepReactionEntry);
db2_store!(FriendshipReputationStore, FriendshipReputationEntry);
db2_store!(ModifierTreeStore, ModifierTreeEntry);
db2_store!(NumTalentsAtLevelStore, NumTalentsAtLevelEntry);
db2_store!(ParagonReputationStore, ParagonReputationEntry);
db2_store!(QuestFactionRewardStore, QuestFactionRewardEntry);
db2_store!(QuestInfoStore, QuestInfoEntry);
db2_store!(QuestLineXQuestStore, QuestLineXQuestEntry);
db2_store!(QuestMoneyRewardStore, QuestMoneyRewardEntry);
db2_store!(QuestPackageItemStore, QuestPackageItemEntry);
db2_store!(QuestSortStore, QuestSortEntry);
db2_store!(QuestV2Store, QuestV2Entry);
db2_store!(RewardPackStore, RewardPackEntry);
db2_store!(RewardPackXCurrencyTypeStore, RewardPackXCurrencyTypeEntry);
db2_store!(RewardPackXItemStore, RewardPackXItemEntry);
db2_store!(ScalingStatDistributionStore, ScalingStatDistributionEntry);
db2_store!(ScalingStatValuesStore, ScalingStatValuesEntry);

impl AchievementCategoryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "Achievement_Category.db2",
            |id, idx, r| AchievementCategoryEntry {
                id,
                name: r.get_field_string(idx, 0),
                parent: r.get_field_i16(idx, 2),
                ui_order: r.get_field_i8(idx, 3),
            },
        )
    }
}

impl ContentTuningStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ContentTuning.db2", |id, idx, r| {
            ContentTuningEntry {
                id,
                min_level: r.get_field_i32(idx, 1),
                max_level: r.get_field_i32(idx, 2),
                flags: r.get_field_i32(idx, 3),
                expected_stat_mod_id: r.get_field_i32(idx, 4),
                difficulty_esm_id: r.get_field_i32(idx, 5),
            }
        })
    }
}

impl CriteriaTreeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CriteriaTree.db2", |id, idx, r| {
            CriteriaTreeEntry {
                id,
                description: r.get_field_string(idx, 0),
                parent: r.get_field_u32(idx, 1),
                amount: r.get_field_u32(idx, 2),
                operator: r.get_field_i32(idx, 3),
                criteria_id: r.get_field_u32(idx, 4),
                order_index: r.get_field_i32(idx, 5),
                flags: r.get_field_i32(idx, 6),
            }
        })
    }
}

impl CurveStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Curve.db2", |id, idx, r| CurveEntry {
            id,
            curve_type: r.get_field_u8(idx, 1),
            flags: r.get_field_u8(idx, 2),
        })
    }
}

impl CurvePointStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CurvePoint.db2", |id, idx, r| {
            CurvePointEntry {
                id,
                pos: f32_array::<2>(r, idx, 0),
                pre_sl_squish_pos: f32_array::<2>(r, idx, 1),
                curve_id: r.get_relationship_id(idx).unwrap_or(0),
                order_index: r.get_field_u8(idx, 4),
            }
        })
    }
}

impl FactionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Faction.db2", |id, idx, r| FactionEntry {
            id,
            reputation_race_mask: std::array::from_fn(|i| r.get_array_i64(idx, 0, i)),
            reputation_index: r.get_field_i16(idx, 4),
            parent_faction_id: r.get_field_u16(idx, 5),
            friendship_rep_id: r.get_field_u8(idx, 7),
            flags: r.get_field_i32(idx, 8),
            paragon_faction_id: r.get_field_u16(idx, 9),
            renown_faction_id: r.get_field_i32(idx, 10),
            renown_currency_id: r.get_field_i32(idx, 11),
            reputation_class_mask: std::array::from_fn(|i| r.get_array_i16(idx, 12, i)),
            reputation_flags: std::array::from_fn(|i| r.get_array_u16(idx, 13, i)),
            reputation_base: std::array::from_fn(|i| r.get_array_i32(idx, 14, i)),
            reputation_max: std::array::from_fn(|i| r.get_array_i32(idx, 15, i)),
            parent_faction_mod: [
                f32::from_bits(r.get_array_element(idx, 16, 0, 32)),
                f32::from_bits(r.get_array_element(idx, 16, 1, 32)),
            ],
            parent_faction_cap: [
                r.get_array_element(idx, 17, 0, 8) as u8,
                r.get_array_element(idx, 17, 1, 8) as u8,
            ],
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &FactionEntry> {
        self.entries.values()
    }

    pub fn faction_team_list_like_cpp(&self, faction_id: u32) -> Vec<u32> {
        let mut faction_ids = self
            .entries
            .values()
            .filter(|entry| u32::from(entry.parent_faction_id) == faction_id)
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        faction_ids.sort_unstable();
        faction_ids
    }
}

impl FactionTemplateStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "FactionTemplate.db2", |id, idx, r| {
            FactionTemplateEntry {
                id,
                faction: r.get_field_u16(idx, 0),
                flags: r.get_field_u16(idx, 1),
                faction_group: r.get_field_u8(idx, 2),
                friend_group: r.get_field_u8(idx, 3),
                enemy_group: r.get_field_u8(idx, 4),
                enemies: std::array::from_fn(|i| r.get_array_u16(idx, 5, i)),
                friend: std::array::from_fn(|i| r.get_array_u16(idx, 6, i)),
            }
        })
    }
}

impl FriendshipRepReactionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "FriendshipRepReaction.db2",
            |id, idx, r| FriendshipRepReactionEntry {
                id,
                reaction: r.get_field_string(idx, 0),
                friendship_rep_id: r.get_relationship_id(idx).unwrap_or(0) as u8,
                reaction_threshold: r.get_field_u16(idx, 2),
            },
        )
    }

    pub fn reactions_for_friendship_rep_like_cpp(
        &self,
        friendship_rep_id: u8,
    ) -> Vec<&FriendshipRepReactionEntry> {
        let mut reactions: Vec<_> = self
            .entries
            .values()
            .filter(|entry| entry.friendship_rep_id == friendship_rep_id)
            .collect();
        reactions.sort_by_key(|entry| entry.reaction_threshold);
        reactions
    }
}

impl FriendshipReputationStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "FriendshipReputation.db2",
            |id, idx, r| FriendshipReputationEntry {
                id,
                description: r.get_field_string(idx, 0),
                field_34146722002: r.get_field_i32(idx, 2),
                field_34146722003: r.get_field_i32(idx, 3),
            },
        )
    }
}

impl ModifierTreeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ModifierTree.db2", |id, idx, r| {
            ModifierTreeEntry {
                id,
                parent: r.get_field_u32(idx, 0),
                operator: r.get_field_i8(idx, 1),
                amount: r.get_field_i8(idx, 2),
                modifier_type: r.get_field_i32(idx, 3),
                asset: r.get_field_i32(idx, 4),
                secondary_asset: r.get_field_i32(idx, 5),
                tertiary_asset: r.get_field_i8(idx, 6),
            }
        })
    }
}

impl NumTalentsAtLevelStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NumTalentsAtLevel.db2", |id, idx, r| {
            NumTalentsAtLevelEntry {
                id,
                num_talents: r.get_field_i32(idx, 1),
                num_talents_death_knight: r.get_field_i32(idx, 2),
                num_talents_demon_hunter: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl ParagonReputationStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ParagonReputation.db2", |id, idx, r| {
            ParagonReputationEntry {
                id,
                faction_id: r.get_field_i32(idx, 0),
                level_threshold: r.get_field_i32(idx, 1),
                quest_id: r.get_field_i32(idx, 2),
            }
        })
    }

    pub fn get_by_faction_id_like_cpp(&self, faction_id: u32) -> Option<&ParagonReputationEntry> {
        self.entries
            .values()
            .find(|entry| entry.faction_id == faction_id as i32)
    }
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

impl RewardPackStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "RewardPack.db2", |id, idx, r| {
            RewardPackEntry {
                id,
                char_title_id: r.get_field_i32(idx, 0),
                money: r.get_field_u32(idx, 1),
                artifact_xp_difficulty: r.get_field_i8(idx, 2),
                artifact_xp_multiplier: f32_field(r, idx, 3),
                artifact_xp_category_id: r.get_field_u8(idx, 4),
                treasure_picker_id: r.get_field_u32(idx, 5),
            }
        })
    }
}

impl RewardPackXCurrencyTypeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "RewardPackXCurrencyType.db2",
            |id, idx, r| RewardPackXCurrencyTypeEntry {
                id,
                currency_type_id: r.get_field_u32(idx, 0),
                quantity: r.get_field_i32(idx, 1),
                reward_pack_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl RewardPackXItemStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "RewardPackXItem.db2", |id, idx, r| {
            RewardPackXItemEntry {
                id,
                item_id: r.get_field_i32(idx, 0),
                item_quantity: r.get_field_i32(idx, 1),
                reward_pack_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl ScalingStatDistributionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ScalingStatDistribution.db2",
            |id, idx, r| ScalingStatDistributionEntry {
                id,
                player_level_to_item_level_curve_id: r.get_field_u16(idx, 0),
                min_level: r.get_field_i32(idx, 1),
                max_level: r.get_field_i32(idx, 2),
                bonus: std::array::from_fn(|i| r.get_array_element(idx, 3, i, 32) as i32),
                stat_id: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 32) as i32),
            },
        )
    }
}

impl ScalingStatValuesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ScalingStatValues.db2", |id, idx, r| {
            ScalingStatValuesEntry {
                id,
                char_level: r.get_field_i32(idx, 0),
                weapon_dps_1h: r.get_field_i32(idx, 1),
                weapon_dps_2h: r.get_field_i32(idx, 2),
                spellcaster_dps_1h: r.get_field_i32(idx, 3),
                spellcaster_dps_2h: r.get_field_i32(idx, 4),
                ranged_dps: r.get_field_i32(idx, 5),
                wand_dps: r.get_field_i32(idx, 6),
                spell_power: r.get_field_i32(idx, 7),
                shoulder_budget: r.get_field_i32(idx, 8),
                trinket_budget: r.get_field_i32(idx, 9),
                weapon_budget_1h: r.get_field_i32(idx, 10),
                primary_budget: r.get_field_i32(idx, 11),
                ranged_budget: r.get_field_i32(idx, 12),
                tertiary_budget: r.get_field_i32(idx, 13),
                cloth_shoulder_armor: r.get_field_i32(idx, 14),
                leather_shoulder_armor: r.get_field_i32(idx, 15),
                mail_shoulder_armor: r.get_field_i32(idx, 16),
                plate_shoulder_armor: r.get_field_i32(idx, 17),
                cloth_cloak_armor: r.get_field_i32(idx, 18),
                cloth_chest_armor: r.get_field_i32(idx, 19),
                leather_chest_armor: r.get_field_i32(idx, 20),
                mail_chest_armor: r.get_field_i32(idx, 21),
                plate_chest_armor: r.get_field_i32(idx, 22),
            }
        })
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

fn f32_array<const N: usize>(reader: &Wdc4Reader, record_idx: usize, field: usize) -> [f32; N] {
    std::array::from_fn(|i| f32::from_bits(reader.get_array_element(record_idx, field, i, 32)))
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

impl_from_entries!(AchievementCategoryStore, AchievementCategoryEntry);
impl_from_entries!(ContentTuningStore, ContentTuningEntry);
impl_from_entries!(CriteriaTreeStore, CriteriaTreeEntry);
impl_from_entries!(CurveStore, CurveEntry);
impl_from_entries!(CurvePointStore, CurvePointEntry);
impl_from_entries!(FactionStore, FactionEntry);
impl_from_entries!(FactionTemplateStore, FactionTemplateEntry);
impl_from_entries!(FriendshipRepReactionStore, FriendshipRepReactionEntry);
impl_from_entries!(FriendshipReputationStore, FriendshipReputationEntry);
impl_from_entries!(ModifierTreeStore, ModifierTreeEntry);
impl_from_entries!(NumTalentsAtLevelStore, NumTalentsAtLevelEntry);
impl_from_entries!(ParagonReputationStore, ParagonReputationEntry);
impl_from_entries!(QuestFactionRewardStore, QuestFactionRewardEntry);
impl_from_entries!(QuestInfoStore, QuestInfoEntry);
impl_from_entries!(QuestLineXQuestStore, QuestLineXQuestEntry);
impl_from_entries!(QuestMoneyRewardStore, QuestMoneyRewardEntry);
impl_from_entries!(QuestPackageItemStore, QuestPackageItemEntry);
impl_from_entries!(QuestSortStore, QuestSortEntry);
impl_from_entries!(QuestV2Store, QuestV2Entry);
impl_from_entries!(RewardPackStore, RewardPackEntry);
impl_from_entries!(RewardPackXCurrencyTypeStore, RewardPackXCurrencyTypeEntry);
impl_from_entries!(RewardPackXItemStore, RewardPackXItemEntry);
impl_from_entries!(ScalingStatDistributionStore, ScalingStatDistributionEntry);
impl_from_entries!(ScalingStatValuesStore, ScalingStatValuesEntry);

#[cfg(test)]
mod tests {
    use super::*;

    fn faction_template_for_test(
        id: u32,
        faction: u16,
        faction_group: u8,
        friend_group: u8,
        enemy_group: u8,
    ) -> FactionTemplateEntry {
        FactionTemplateEntry {
            id,
            faction,
            flags: 0,
            faction_group,
            friend_group,
            enemy_group,
            enemies: [0; 8],
            friend: [0; 8],
        }
    }

    #[test]
    fn reward_pack_currency_store_uses_cpp_parent_relationship() {
        let store = RewardPackXCurrencyTypeStore::from_entries([RewardPackXCurrencyTypeEntry {
            id: 1,
            currency_type_id: 2,
            quantity: 3,
            reward_pack_id: 4,
        }]);

        assert_eq!(store.get(1).unwrap().reward_pack_id, 4);
    }

    #[test]
    fn faction_template_friendly_relation_matches_cpp_precedence() {
        let mut faction = faction_template_for_test(1, 100, 0x02, 0x04, 0);
        let other = faction_template_for_test(2, 200, 0x04, 0, 0);

        assert!(faction.is_friendly_to_like_cpp(&faction));
        assert!(faction.is_friendly_to_like_cpp(&other));

        faction.enemies[0] = 200;
        faction.friend[0] = 200;
        assert!(!faction.is_friendly_to_like_cpp(&other));
    }

    #[test]
    fn faction_template_hostile_relation_matches_cpp_precedence() {
        let mut faction = faction_template_for_test(1, 100, 0, 0, 0x04);
        let other = faction_template_for_test(2, 200, 0x04, 0, 0);

        assert!(!faction.is_hostile_to_like_cpp(&faction));
        assert!(faction.is_hostile_to_like_cpp(&other));

        faction.friend[0] = 200;
        assert!(!faction.is_hostile_to_like_cpp(&other));
    }

    #[test]
    fn faction_template_misc_helpers_match_cpp_flags_and_groups() {
        let mut faction = faction_template_for_test(1, 100, 0, 0, FACTION_MASK_PLAYER_LIKE_CPP);
        faction.flags = FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP
            | FACTION_TEMPLATE_FLAG_HOSTILE_BY_DEFAULT_LIKE_CPP;

        assert!(faction.is_hostile_to_players_like_cpp());
        assert!(faction.is_contested_guard_faction_like_cpp());
        assert!(faction.is_hostile_by_default_like_cpp());
        assert!(!faction.is_neutral_to_all_like_cpp());

        faction.enemy_group = 0;
        faction.flags = 0;
        assert!(faction.is_neutral_to_all_like_cpp());

        faction.enemies[7] = 200;
        assert!(!faction.is_neutral_to_all_like_cpp());
    }

    #[test]
    fn quest_v2_unique_bit_flag_missing_quest_returns_zero_like_cpp() {
        let store = QuestV2Store::from_entries([]);

        assert_eq!(store.get_quest_unique_bit_flag_like_cpp(12_345), 0);
    }

    #[test]
    fn quest_v2_unique_bit_flag_existing_nonzero_returns_exact_flag_like_cpp() {
        let store = QuestV2Store::from_entries([QuestV2Entry {
            id: 12_345,
            unique_bit_flag: 77,
        }]);

        assert_eq!(store.get_quest_unique_bit_flag_like_cpp(12_345), 77);
    }

    #[test]
    fn quest_v2_unique_bit_flag_existing_zero_stays_zero_like_cpp() {
        let store = QuestV2Store::from_entries([QuestV2Entry {
            id: 12_345,
            unique_bit_flag: 0,
        }]);

        assert_eq!(store.get_quest_unique_bit_flag_like_cpp(12_345), 0);
    }

    #[test]
    fn quest_v2_unique_bit_flag_duplicate_ids_preserve_from_entries_last_wins() {
        let store = QuestV2Store::from_entries([
            QuestV2Entry {
                id: 12_345,
                unique_bit_flag: 11,
            },
            QuestV2Entry {
                id: 12_345,
                unique_bit_flag: 99,
            },
        ]);

        assert_eq!(store.len(), 1);
        assert_eq!(store.get_quest_unique_bit_flag_like_cpp(12_345), 99);
    }

    #[test]
    fn quest_package_items_split_primary_and_fallback_like_cpp() {
        let store = QuestPackageItemStore::from_entries([
            QuestPackageItemEntry {
                id: 1,
                package_id: 10,
                item_id: 100,
                item_quantity: 1,
                display_type: QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP,
            },
            QuestPackageItemEntry {
                id: 2,
                package_id: 10,
                item_id: 200,
                item_quantity: 1,
                display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
            },
            QuestPackageItemEntry {
                id: 3,
                package_id: 11,
                item_id: 300,
                item_quantity: 1,
                display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
            },
        ]);

        let primary: Vec<i32> = store
            .quest_package_items_like_cpp(10)
            .map(|entry| entry.item_id)
            .collect();
        let fallback: Vec<i32> = store
            .quest_package_items_fallback_like_cpp(10)
            .map(|entry| entry.item_id)
            .collect();

        assert_eq!(primary, vec![100]);
        assert_eq!(fallback, vec![200]);
    }

    #[test]
    fn load_progression_rewards_db2_subbatch_when_fixtures_exist() {
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

        load_if_exists!("Achievement_Category.db2", AchievementCategoryStore);
        load_if_exists!("ContentTuning.db2", ContentTuningStore);
        load_if_exists!("CriteriaTree.db2", CriteriaTreeStore);
        load_if_exists!("Curve.db2", CurveStore);
        load_if_exists!("CurvePoint.db2", CurvePointStore);
        load_if_exists!("Faction.db2", FactionStore);
        load_if_exists!("FactionTemplate.db2", FactionTemplateStore);
        load_if_exists!("FriendshipRepReaction.db2", FriendshipRepReactionStore);
        load_if_exists!("FriendshipReputation.db2", FriendshipReputationStore);
        load_if_exists!("ModifierTree.db2", ModifierTreeStore);
        load_if_exists!("NumTalentsAtLevel.db2", NumTalentsAtLevelStore);
        load_if_exists!("ParagonReputation.db2", ParagonReputationStore);
        load_if_exists!("QuestFactionReward.db2", QuestFactionRewardStore);
        load_if_exists!("QuestInfo.db2", QuestInfoStore);
        load_if_exists!("QuestLineXQuest.db2", QuestLineXQuestStore);
        load_if_exists!("QuestMoneyReward.db2", QuestMoneyRewardStore);
        load_if_exists!("QuestPackageItem.db2", QuestPackageItemStore);
        load_if_exists!("QuestSort.db2", QuestSortStore);
        load_if_exists!("QuestV2.db2", QuestV2Store);
        load_if_exists!("RewardPack.db2", RewardPackStore);
        load_if_exists!("RewardPackXCurrencyType.db2", RewardPackXCurrencyTypeStore);
        load_if_exists!("RewardPackXItem.db2", RewardPackXItemStore);
        load_if_exists!("ScalingStatDistribution.db2", ScalingStatDistributionStore);
        load_if_exists!("ScalingStatValues.db2", ScalingStatValuesStore);
    }
}
