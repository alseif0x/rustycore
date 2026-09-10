//! Reward pack packets.
//!
//! Separated from progression_rewards.rs under #691.

use super::*;

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
