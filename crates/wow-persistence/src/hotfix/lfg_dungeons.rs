//! SQLx-free Hotfix source contract for the `LFGDungeons.db2` authority.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub struct LfgDungeonsHotfixRowLikeCpp {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub min_level: u8,
    pub max_level: u16,
    pub type_id: u8,
    pub subtype: u8,
    pub faction: i8,
    pub icon_texture_file_id: i32,
    pub rewards_bg_texture_file_id: i32,
    pub popup_bg_texture_file_id: i32,
    pub expansion_level: u8,
    pub map_id: i16,
    pub difficulty_id: u8,
    pub min_gear: f32,
    pub group_id: u8,
    pub order_index: u8,
    pub required_player_condition_id: u32,
    pub target_level: u8,
    pub target_level_min: u8,
    pub target_level_max: u16,
    pub random_id: u16,
    pub scenario_id: u16,
    pub final_encounter_id: u16,
    pub count_tank: u8,
    pub count_healer: u8,
    pub count_damage: u8,
    pub min_count_tank: u8,
    pub min_count_healer: u8,
    pub min_count_damage: u8,
    pub bonus_reputation_amount: u16,
    pub mentor_item_level: u16,
    pub mentor_char_level: u8,
    pub flags: [i32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub enum LfgDungeonsHotfixLoadOutcomeLikeCpp {
    Loaded(Vec<LfgDungeonsHotfixRowLikeCpp>),
    Failed { reason: String },
}

/// The early Hotfix overlay is independent from the later World LFG template
/// and reward reads, which run at a different startup fence.
pub trait LfgDungeonsHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_lfg_dungeons_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, LfgDungeonsHotfixLoadOutcomeLikeCpp>;
}
