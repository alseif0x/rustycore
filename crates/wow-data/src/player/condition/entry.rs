//! Entry packets.
//!
//! Separated from player_condition.rs under #691.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerConditionEntry {
    pub race_mask: i64,
    pub id: u32,
    pub min_level: u16,
    pub max_level: u16,
    pub class_mask: i32,
    pub skill_logic: u32,
    pub language_id: u8,
    pub min_language: u8,
    pub max_language: i32,
    pub max_faction_id: u16,
    pub max_reputation: u8,
    pub reputation_logic: u32,
    pub current_pvp_faction: i8,
    pub pvp_medal: u8,
    pub prev_quest_logic: u32,
    pub curr_quest_logic: u32,
    pub current_completed_quest_logic: u32,
    pub spell_logic: u32,
    pub item_logic: u32,
    pub item_flags: u8,
    pub aura_spell_logic: u32,
    pub world_state_expression_id: u16,
    pub weather_id: u8,
    pub party_status: u8,
    pub lifetime_max_pvp_rank: u8,
    pub achievement_logic: u32,
    pub gender: i8,
    pub native_gender: i8,
    pub area_logic: u32,
    pub lfg_logic: u32,
    pub currency_logic: u32,
    pub quest_kill_id: u32,
    pub quest_kill_logic: u32,
    pub min_expansion_level: i8,
    pub max_expansion_level: i8,
    pub min_avg_item_level: i32,
    pub max_avg_item_level: i32,
    pub min_avg_equipped_item_level: u16,
    pub max_avg_equipped_item_level: u16,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub flags: u8,
    pub chr_specialization_index: i8,
    pub chr_specialization_role: i8,
    pub modifier_tree_id: u32,
    pub power_type: i8,
    pub power_type_comp: u8,
    pub power_type_value: u8,
    pub weapon_subclass_mask: i32,
    pub max_guild_level: u8,
    pub min_guild_level: u8,
    pub max_expansion_tier: i8,
    pub min_expansion_tier: i8,
    pub min_pvp_rank: u8,
    pub max_pvp_rank: u8,
    pub skill_id: [u16; 4],
    pub min_skill: [u16; 4],
    pub max_skill: [u16; 4],
    pub min_faction_id: [u32; 3],
    pub min_reputation: [u8; 3],
    pub prev_quest_id: [u32; 4],
    pub curr_quest_id: [u32; 4],
    pub current_completed_quest_id: [u32; 4],
    pub spell_id: [i32; 4],
    pub item_id: [i32; 4],
    pub item_count: [u32; 4],
    pub explored: [u16; 2],
    pub time: [u32; 2],
    pub aura_spell_id: [i32; 4],
    pub aura_stacks: [u8; 4],
    pub achievement: [u16; 4],
    pub area_id: [u16; 4],
    pub lfg_status: [u8; 4],
    pub lfg_compare: [u8; 4],
    pub lfg_value: [u32; 4],
    pub currency_id: [u32; 4],
    pub currency_count: [u32; 4],
    pub quest_kill_monster: [u32; 6],
    pub movement_flags: [i32; 2],
}

impl Default for PlayerConditionEntry {
    fn default() -> Self {
        Self {
            race_mask: 0,
            id: 0,
            min_level: 0,
            max_level: 0,
            class_mask: 0,
            skill_logic: 0,
            language_id: 0,
            min_language: 0,
            max_language: 0,
            max_faction_id: 0,
            max_reputation: 0,
            reputation_logic: 0,
            current_pvp_faction: 0,
            pvp_medal: 0,
            prev_quest_logic: 0,
            curr_quest_logic: 0,
            current_completed_quest_logic: 0,
            spell_logic: 0,
            item_logic: 0,
            item_flags: 0,
            aura_spell_logic: 0,
            world_state_expression_id: 0,
            weather_id: 0,
            party_status: 0,
            lifetime_max_pvp_rank: 0,
            achievement_logic: 0,
            gender: -1,
            native_gender: -1,
            area_logic: 0,
            lfg_logic: 0,
            currency_logic: 0,
            quest_kill_id: 0,
            quest_kill_logic: 0,
            min_expansion_level: -1,
            max_expansion_level: -1,
            min_avg_item_level: 0,
            max_avg_item_level: 0,
            min_avg_equipped_item_level: 0,
            max_avg_equipped_item_level: 0,
            phase_use_flags: 0,
            phase_id: 0,
            phase_group_id: 0,
            flags: 0,
            chr_specialization_index: -1,
            chr_specialization_role: -1,
            modifier_tree_id: 0,
            power_type: -1,
            power_type_comp: 0,
            power_type_value: 0,
            weapon_subclass_mask: 0,
            max_guild_level: 0,
            min_guild_level: 0,
            max_expansion_tier: -1,
            min_expansion_tier: -1,
            min_pvp_rank: 0,
            max_pvp_rank: 0,
            skill_id: [0; 4],
            min_skill: [0; 4],
            max_skill: [0; 4],
            min_faction_id: [0; 3],
            min_reputation: [0; 3],
            prev_quest_id: [0; 4],
            curr_quest_id: [0; 4],
            current_completed_quest_id: [0; 4],
            spell_id: [0; 4],
            item_id: [0; 4],
            item_count: [0; 4],
            explored: [0; 2],
            time: [0; 2],
            aura_spell_id: [0; 4],
            aura_stacks: [0; 4],
            achievement: [0; 4],
            area_id: [0; 4],
            lfg_status: [0; 4],
            lfg_compare: [0; 4],
            lfg_value: [0; 4],
            currency_id: [0; 4],
            currency_count: [0; 4],
            quest_kill_monster: [0; 6],
            movement_flags: [0; 2],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerConditionPartyStatusLikeCpp {
    Solo,
    InGroup,
    InParty,
    InRaid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerConditionSkillLikeCpp {
    pub id: u16,
    pub value: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerConditionReputationLikeCpp {
    pub faction_id: u32,
    pub rank: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerConditionCountLikeCpp {
    pub id: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerConditionAuraLikeCpp {
    pub spell_id: u32,
    pub stacks: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerConditionQuestKillLikeCpp {
    pub monster_id: u32,
    pub done: bool,
}

pub(super) fn read_u16_array(reader: &Wdc4Reader, idx: usize, start: usize) -> [u16; 4] {
    [
        reader.get_array_u16(idx, start, 0),
        reader.get_array_u16(idx, start, 1),
        reader.get_array_u16(idx, start, 2),
        reader.get_array_u16(idx, start, 3),
    ]
}

pub(super) fn read_u8_array3(reader: &Wdc4Reader, idx: usize, start: usize) -> [u8; 3] {
    [
        reader.get_array_element(idx, start, 0, 8) as u8,
        reader.get_array_element(idx, start, 1, 8) as u8,
        reader.get_array_element(idx, start, 2, 8) as u8,
    ]
}

pub(super) fn read_u8_array4(reader: &Wdc4Reader, idx: usize, start: usize) -> [u8; 4] {
    [
        reader.get_array_element(idx, start, 0, 8) as u8,
        reader.get_array_element(idx, start, 1, 8) as u8,
        reader.get_array_element(idx, start, 2, 8) as u8,
        reader.get_array_element(idx, start, 3, 8) as u8,
    ]
}

pub(super) fn read_u32_array3(reader: &Wdc4Reader, idx: usize, start: usize) -> [u32; 3] {
    [
        reader.get_array_element(idx, start, 0, 32),
        reader.get_array_element(idx, start, 1, 32),
        reader.get_array_element(idx, start, 2, 32),
    ]
}

pub(super) fn read_u32_array4(reader: &Wdc4Reader, idx: usize, start: usize) -> [u32; 4] {
    [
        reader.get_array_element(idx, start, 0, 32),
        reader.get_array_element(idx, start, 1, 32),
        reader.get_array_element(idx, start, 2, 32),
        reader.get_array_element(idx, start, 3, 32),
    ]
}

pub(super) fn read_u32_array6(reader: &Wdc4Reader, idx: usize, start: usize) -> [u32; 6] {
    [
        reader.get_array_element(idx, start, 0, 32),
        reader.get_array_element(idx, start, 1, 32),
        reader.get_array_element(idx, start, 2, 32),
        reader.get_array_element(idx, start, 3, 32),
        reader.get_array_element(idx, start, 4, 32),
        reader.get_array_element(idx, start, 5, 32),
    ]
}

pub(super) fn read_i32_array4(reader: &Wdc4Reader, idx: usize, start: usize) -> [i32; 4] {
    [
        reader.get_array_i32(idx, start, 0),
        reader.get_array_i32(idx, start, 1),
        reader.get_array_i32(idx, start, 2),
        reader.get_array_i32(idx, start, 3),
    ]
}
