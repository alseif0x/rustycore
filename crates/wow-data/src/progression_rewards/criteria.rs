//! Criteria packets.
//!
//! Separated from progression_rewards.rs under #691.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementCategoryEntry {
    pub id: u32,
    pub name: String,
    pub parent: i16,
    pub ui_order: i8,
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
