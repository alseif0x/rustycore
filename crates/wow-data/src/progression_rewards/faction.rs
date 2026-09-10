//! Faction packets.
//!
//! Separated from progression_rewards.rs under #691.

use super::*;

pub const FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP: u16 = 0x0000_1000;

pub const FACTION_TEMPLATE_FLAG_HOSTILE_BY_DEFAULT_LIKE_CPP: u16 = 0x0000_2000;

pub const FACTION_MASK_PLAYER_LIKE_CPP: u8 = 0x01;

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
pub struct ParagonReputationEntry {
    pub id: u32,
    pub faction_id: i32,
    pub level_threshold: i32,
    pub quest_id: i32,
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
            reputation_base: std::array::from_fn(|i| r.get_array_element(idx, 14, i, 32) as i32),
            reputation_max: std::array::from_fn(|i| r.get_array_element(idx, 15, i, 32) as i32),
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
