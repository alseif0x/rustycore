// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! PlayerCondition.db2 store and C++-like evaluator.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;
use crate::{
    ChrSpecializationStore, WorldStateExpressionContextLikeCpp, WorldStateExpressionStore,
    is_meeting_world_state_expression_like_cpp,
};

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

#[derive(Debug, Clone, Default)]
pub struct PlayerConditionStore {
    entries: HashMap<u32, PlayerConditionEntry>,
}

impl PlayerConditionStore {
    pub fn from_entries(entries: impl IntoIterator<Item = PlayerConditionEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("PlayerCondition.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let entry = PlayerConditionEntry {
                race_mask: reader.get_field_i64(idx, 0),
                id,
                min_level: reader.get_field_u16(idx, 3),
                max_level: reader.get_field_u16(idx, 4),
                class_mask: reader.get_field_i32(idx, 5),
                skill_logic: reader.get_field_u32(idx, 6),
                language_id: reader.get_field_u8(idx, 7),
                min_language: reader.get_field_u8(idx, 8),
                max_language: reader.get_field_i32(idx, 9),
                max_faction_id: reader.get_field_u16(idx, 10),
                max_reputation: reader.get_field_u8(idx, 11),
                reputation_logic: reader.get_field_u32(idx, 12),
                current_pvp_faction: reader.get_field_i8(idx, 13),
                pvp_medal: reader.get_field_u8(idx, 14),
                prev_quest_logic: reader.get_field_u32(idx, 15),
                curr_quest_logic: reader.get_field_u32(idx, 16),
                current_completed_quest_logic: reader.get_field_u32(idx, 17),
                spell_logic: reader.get_field_u32(idx, 18),
                item_logic: reader.get_field_u32(idx, 19),
                item_flags: reader.get_field_u8(idx, 20),
                aura_spell_logic: reader.get_field_u32(idx, 21),
                world_state_expression_id: reader.get_field_u16(idx, 22),
                weather_id: reader.get_field_u8(idx, 23),
                party_status: reader.get_field_u8(idx, 24),
                lifetime_max_pvp_rank: reader.get_field_u8(idx, 25),
                achievement_logic: reader.get_field_u32(idx, 26),
                gender: reader.get_field_i8(idx, 27),
                native_gender: reader.get_field_i8(idx, 28),
                area_logic: reader.get_field_u32(idx, 29),
                lfg_logic: reader.get_field_u32(idx, 30),
                currency_logic: reader.get_field_u32(idx, 31),
                quest_kill_id: reader.get_field_u32(idx, 32),
                quest_kill_logic: reader.get_field_u32(idx, 33),
                min_expansion_level: reader.get_field_i8(idx, 34),
                max_expansion_level: reader.get_field_i8(idx, 35),
                min_avg_item_level: reader.get_field_i32(idx, 36),
                max_avg_item_level: reader.get_field_i32(idx, 37),
                min_avg_equipped_item_level: reader.get_field_u16(idx, 38),
                max_avg_equipped_item_level: reader.get_field_u16(idx, 39),
                phase_use_flags: reader.get_field_u8(idx, 40),
                phase_id: reader.get_field_u16(idx, 41),
                phase_group_id: reader.get_field_u32(idx, 42),
                flags: reader.get_field_u8(idx, 43),
                chr_specialization_index: reader.get_field_i8(idx, 44),
                chr_specialization_role: reader.get_field_i8(idx, 45),
                modifier_tree_id: reader.get_field_u32(idx, 46),
                power_type: reader.get_field_i8(idx, 47),
                power_type_comp: reader.get_field_u8(idx, 48),
                power_type_value: reader.get_field_u8(idx, 49),
                weapon_subclass_mask: reader.get_field_i32(idx, 50),
                max_guild_level: reader.get_field_u8(idx, 51),
                min_guild_level: reader.get_field_u8(idx, 52),
                max_expansion_tier: reader.get_field_i8(idx, 53),
                min_expansion_tier: reader.get_field_i8(idx, 54),
                min_pvp_rank: reader.get_field_u8(idx, 55),
                max_pvp_rank: reader.get_field_u8(idx, 56),
                // Trinity's load info expands these to fields 57..146, but
                // the WDC4 payload stores them as physical array fields 57..80.
                skill_id: read_u16_array(&reader, idx, 57),
                min_skill: read_u16_array(&reader, idx, 58),
                max_skill: read_u16_array(&reader, idx, 59),
                min_faction_id: read_u32_array3(&reader, idx, 60),
                min_reputation: read_u8_array3(&reader, idx, 61),
                prev_quest_id: read_u32_array4(&reader, idx, 62),
                curr_quest_id: read_u32_array4(&reader, idx, 63),
                current_completed_quest_id: read_u32_array4(&reader, idx, 64),
                spell_id: read_i32_array4(&reader, idx, 65),
                item_id: read_i32_array4(&reader, idx, 66),
                item_count: read_u32_array4(&reader, idx, 67),
                explored: [
                    reader.get_array_u16(idx, 68, 0),
                    reader.get_array_u16(idx, 68, 1),
                ],
                time: [
                    reader.get_array_element(idx, 69, 0, 32),
                    reader.get_array_element(idx, 69, 1, 32),
                ],
                aura_spell_id: read_i32_array4(&reader, idx, 70),
                aura_stacks: read_u8_array4(&reader, idx, 71),
                achievement: read_u16_array(&reader, idx, 72),
                area_id: read_u16_array(&reader, idx, 73),
                lfg_status: read_u8_array4(&reader, idx, 74),
                lfg_compare: read_u8_array4(&reader, idx, 75),
                lfg_value: read_u32_array4(&reader, idx, 76),
                currency_id: read_u32_array4(&reader, idx, 77),
                currency_count: read_u32_array4(&reader, idx, 78),
                quest_kill_monster: read_u32_array6(&reader, idx, 79),
                movement_flags: [
                    reader.get_array_i32(idx, 80, 0),
                    reader.get_array_i32(idx, 80, 1),
                ],
            };
            entries.insert(id, entry);
        }

        info!(
            "Loaded {} player conditions from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&PlayerConditionEntry> {
        self.entries.get(&id)
    }

    pub fn contains(&self, id: u32) -> bool {
        self.entries.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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

#[derive(Debug, Clone, Copy)]
pub struct PlayerConditionContextLikeCpp<'a> {
    pub race: u8,
    pub class_mask: u32,
    pub gender: u8,
    pub native_gender: u8,
    pub power_type: i8,
    pub power: i32,
    pub max_power: i32,
    pub primary_specialization_id: Option<u32>,
    pub skills: &'a [PlayerConditionSkillLikeCpp],
    pub language_skill: i32,
    pub reputations: &'a [PlayerConditionReputationLikeCpp],
    pub current_pvp_faction: i8,
    pub pvp_medals_mask: u32,
    pub lifetime_max_pvp_rank: u8,
    pub movement_flags: [i32; 2],
    pub mainhand_weapon_subclass: Option<u8>,
    pub party_status: PlayerConditionPartyStatusLikeCpp,
    pub completed_quests: &'a [u32],
    pub current_quests: &'a [u32],
    pub complete_quests: &'a [u32],
    pub spells: &'a [u32],
    pub items: &'a [PlayerConditionCountLikeCpp],
    pub currencies: &'a [PlayerConditionCountLikeCpp],
    pub explored_area_ids: &'a [u16],
    pub auras: &'a [PlayerConditionAuraLikeCpp],
    pub weather_id: u8,
    pub achievements: &'a [u16],
    pub lfg_values: &'a [PlayerConditionCountLikeCpp],
    pub area_id: u32,
    pub parent_area_ids: &'a [u32],
    pub expansion: i8,
    pub server_expansion: i8,
    pub is_game_master: bool,
    pub phase_satisfied: bool,
    pub quest_kill_id: u32,
    pub quest_kills: &'a [PlayerConditionQuestKillLikeCpp],
    pub avg_item_level: f32,
    pub avg_equipped_item_level: f32,
    pub modifier_tree_ids: &'a [u32],
    pub chr_specializations: Option<&'a ChrSpecializationStore>,
    pub world_state_expressions: Option<&'a WorldStateExpressionStore>,
    pub world_state_expression_context: Option<WorldStateExpressionContextLikeCpp<'a>>,
}

impl Default for PlayerConditionContextLikeCpp<'_> {
    fn default() -> Self {
        Self {
            race: 0,
            class_mask: 0,
            gender: 0,
            native_gender: 0,
            power_type: -1,
            power: 0,
            max_power: 0,
            primary_specialization_id: None,
            skills: &[],
            language_skill: 0,
            reputations: &[],
            current_pvp_faction: 0,
            pvp_medals_mask: 0,
            lifetime_max_pvp_rank: 0,
            movement_flags: [0, 0],
            mainhand_weapon_subclass: None,
            party_status: PlayerConditionPartyStatusLikeCpp::Solo,
            completed_quests: &[],
            current_quests: &[],
            complete_quests: &[],
            spells: &[],
            items: &[],
            currencies: &[],
            explored_area_ids: &[],
            auras: &[],
            weather_id: 0,
            achievements: &[],
            lfg_values: &[],
            area_id: 0,
            parent_area_ids: &[],
            expansion: 0,
            server_expansion: 0,
            is_game_master: false,
            phase_satisfied: true,
            quest_kill_id: 0,
            quest_kills: &[],
            avg_item_level: 0.0,
            avg_equipped_item_level: 0.0,
            modifier_tree_ids: &[],
            chr_specializations: None,
            world_state_expressions: None,
            world_state_expression_context: None,
        }
    }
}

pub fn player_condition_compare_like_cpp(comparison_type: i32, value1: i32, value2: i32) -> bool {
    match comparison_type {
        1 => value1 == value2,
        2 => value1 != value2,
        3 => value1 > value2,
        4 => value1 >= value2,
        5 => value1 < value2,
        6 => value1 <= value2,
        _ => false,
    }
}

pub fn player_condition_logic_like_cpp<const N: usize>(logic: u32, mut results: [bool; N]) -> bool {
    debug_assert!(N < 16);
    for (i, result) in results.iter_mut().enumerate() {
        if ((logic >> (16 + i)) & 1) != 0 {
            *result = !*result;
        }
    }

    let mut result = results[0];
    for (i, value) in results.iter().enumerate().skip(1) {
        match (logic >> (2 * (i - 1))) & 3 {
            1 => result = result && *value,
            2 => result = result || *value,
            _ => {}
        }
    }
    result
}

pub fn is_player_meeting_condition_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.race_mask != 0
        && (condition.race_mask & (1i64 << (context.race.saturating_sub(1)))) == 0
    {
        return false;
    }
    if condition.class_mask != 0 && (context.class_mask & condition.class_mask as u32) == 0 {
        return false;
    }
    if condition.gender >= 0 && context.gender != condition.gender as u8 {
        return false;
    }
    if condition.native_gender >= 0 && context.native_gender != condition.native_gender as u8 {
        return false;
    }
    if condition.power_type != -1 && condition.power_type_comp != 0 {
        let required = if (condition.flags & 4) != 0 {
            context.max_power
        } else {
            i32::from(condition.power_type_value)
        };
        if condition.power_type != context.power_type
            || !player_condition_compare_like_cpp(
                i32::from(condition.power_type_comp),
                context.power,
                required,
            )
        {
            return false;
        }
    }

    if (condition.chr_specialization_index >= 0 || condition.chr_specialization_role >= 0)
        && let (Some(spec_id), Some(store)) = (
            context.primary_specialization_id,
            context.chr_specializations,
        )
        && let Some(spec) = store.get(spec_id)
    {
        if condition.chr_specialization_index >= 0
            && spec.order_index != condition.chr_specialization_index
        {
            return false;
        }
        if condition.chr_specialization_role >= 0 && spec.role != condition.chr_specialization_role
        {
            return false;
        }
    }

    if condition.skill_id.iter().any(|id| *id != 0) {
        let mut results = [true; 4];
        for (i, id) in condition.skill_id.iter().enumerate() {
            if *id != 0 {
                let value = skill_value(context, *id);
                results[i] =
                    value != 0 && value > condition.min_skill[i] && value < condition.max_skill[i];
            }
        }
        if !player_condition_logic_like_cpp(condition.skill_logic, results) {
            return false;
        }
    }

    if condition.language_id != 0 {
        if condition.min_language != 0 && context.language_skill < i32::from(condition.min_language)
        {
            return false;
        }
        if condition.max_language != 0 && context.language_skill > condition.max_language {
            return false;
        }
    }

    if !check_reputation_like_cpp(condition, context) {
        return false;
    }
    if condition.current_pvp_faction != 0
        && condition.current_pvp_faction - 1 != context.current_pvp_faction
    {
        return false;
    }
    if condition.pvp_medal != 0
        && ((1u32 << (condition.pvp_medal - 1)) & context.pvp_medals_mask) == 0
    {
        return false;
    }
    if condition.lifetime_max_pvp_rank != 0
        && context.lifetime_max_pvp_rank != condition.lifetime_max_pvp_rank
    {
        return false;
    }
    if condition.movement_flags[0] != 0
        && (context.movement_flags[0] & condition.movement_flags[0]) == 0
    {
        return false;
    }
    if condition.movement_flags[1] != 0
        && (context.movement_flags[1] & condition.movement_flags[1]) == 0
    {
        return false;
    }
    if condition.weapon_subclass_mask != 0 {
        let Some(subclass) = context.mainhand_weapon_subclass else {
            return false;
        };
        if ((1i32 << subclass) & condition.weapon_subclass_mask) == 0 {
            return false;
        }
    }

    if !check_party_status_like_cpp(condition.party_status, context.party_status) {
        return false;
    }
    if !check_id_array4_like_cpp(
        condition.prev_quest_id,
        condition.prev_quest_logic,
        context.completed_quests,
    ) {
        return false;
    }
    if !check_id_array4_like_cpp(
        condition.curr_quest_id,
        condition.curr_quest_logic,
        context.current_quests,
    ) {
        return false;
    }
    if !check_id_array4_like_cpp(
        condition.current_completed_quest_id,
        condition.current_completed_quest_logic,
        context.complete_quests,
    ) {
        return false;
    }
    if !check_signed_id_array4_like_cpp(condition.spell_id, condition.spell_logic, context.spells) {
        return false;
    }
    if !check_count_array4_like_cpp(
        condition.item_id,
        condition.item_count,
        condition.item_logic,
        context.items,
    ) {
        return false;
    }
    if !check_currency_array_like_cpp(condition, context) {
        return false;
    }
    if condition.explored.iter().any(|id| *id != 0)
        && condition
            .explored
            .iter()
            .filter(|id| **id != 0)
            .any(|id| !context.explored_area_ids.contains(id))
    {
        return false;
    }
    if !check_auras_like_cpp(condition, context) {
        return false;
    }
    if condition.world_state_expression_id != 0 {
        let Some(store) = context.world_state_expressions else {
            return false;
        };
        let Some(entry) = store.get(u32::from(condition.world_state_expression_id)) else {
            return false;
        };
        let Some(wse_context) = context.world_state_expression_context.as_ref() else {
            return false;
        };
        if !is_meeting_world_state_expression_like_cpp(entry, wse_context) {
            return false;
        }
    }
    if condition.weather_id != 0 && context.weather_id != condition.weather_id {
        return false;
    }
    if !check_u16_array4_like_cpp(
        condition.achievement,
        condition.achievement_logic,
        context.achievements,
    ) {
        return false;
    }
    if !check_lfg_like_cpp(condition, context) {
        return false;
    }
    if !check_area_like_cpp(condition, context) {
        return false;
    }
    if condition.min_expansion_level != -1 && context.expansion < condition.min_expansion_level {
        return false;
    }
    if condition.max_expansion_level != -1 && context.expansion > condition.max_expansion_level {
        return false;
    }
    if condition.min_expansion_level != -1
        && condition.min_expansion_tier != -1
        && !context.is_game_master
        && ((condition.min_expansion_level == context.server_expansion
            && condition.min_expansion_tier > 0)
            || condition.min_expansion_level > context.server_expansion)
    {
        return false;
    }
    if (condition.phase_id != 0 || condition.phase_group_id != 0 || condition.phase_use_flags != 0)
        && !context.phase_satisfied
    {
        return false;
    }
    if !check_quest_kills_like_cpp(condition, context) {
        return false;
    }
    if condition.min_avg_item_level != 0
        && (context.avg_item_level.floor() as i32) < condition.min_avg_item_level
    {
        return false;
    }
    if condition.max_avg_item_level != 0
        && context.avg_item_level.floor() as i32 > condition.max_avg_item_level
    {
        return false;
    }
    if condition.min_avg_equipped_item_level != 0
        && (context.avg_equipped_item_level.floor() as u32)
            < u32::from(condition.min_avg_equipped_item_level)
    {
        return false;
    }
    if condition.max_avg_equipped_item_level != 0
        && context.avg_equipped_item_level.floor() as u32
            > u32::from(condition.max_avg_equipped_item_level)
    {
        return false;
    }
    if condition.modifier_tree_id != 0
        && !context
            .modifier_tree_ids
            .contains(&condition.modifier_tree_id)
    {
        return false;
    }

    true
}

fn skill_value(context: &PlayerConditionContextLikeCpp<'_>, id: u16) -> u16 {
    context
        .skills
        .iter()
        .find(|skill| skill.id == id)
        .map(|skill| skill.value)
        .unwrap_or(0)
}

fn reputation_rank(context: &PlayerConditionContextLikeCpp<'_>, id: u32) -> u8 {
    context
        .reputations
        .iter()
        .find(|rep| rep.faction_id == id)
        .map(|rep| rep.rank)
        .unwrap_or(0)
}

fn count_for(values: &[PlayerConditionCountLikeCpp], id: u32) -> u32 {
    values
        .iter()
        .find(|value| value.id == id)
        .map(|value| value.count)
        .unwrap_or(0)
}

fn check_reputation_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.min_faction_id.iter().all(|id| *id == 0) && condition.max_faction_id == 0 {
        return true;
    }
    if condition.min_faction_id.iter().all(|id| *id == 0) {
        return reputation_rank(context, u32::from(condition.max_faction_id))
            <= condition.max_reputation;
    }

    let mut results = [true; 4];
    for (i, id) in condition.min_faction_id.iter().enumerate() {
        if *id != 0 {
            results[i] = reputation_rank(context, *id) >= condition.min_reputation[i];
        }
    }
    if condition.max_faction_id != 0 {
        results[3] = reputation_rank(context, u32::from(condition.max_faction_id))
            <= condition.max_reputation;
    }
    player_condition_logic_like_cpp(condition.reputation_logic, results)
}

fn check_party_status_like_cpp(required: u8, current: PlayerConditionPartyStatusLikeCpp) -> bool {
    match required {
        0 => true,
        1 => current == PlayerConditionPartyStatusLikeCpp::Solo,
        2 => current != PlayerConditionPartyStatusLikeCpp::Solo,
        3 => current == PlayerConditionPartyStatusLikeCpp::InParty,
        4 => current == PlayerConditionPartyStatusLikeCpp::InRaid,
        5 => current != PlayerConditionPartyStatusLikeCpp::InRaid,
        _ => true,
    }
}

fn check_id_array4_like_cpp(ids: [u32; 4], logic: u32, owned: &[u32]) -> bool {
    if ids[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            results[i] = owned.contains(id);
        }
    }
    player_condition_logic_like_cpp(logic, results)
}

fn check_signed_id_array4_like_cpp(ids: [i32; 4], logic: u32, owned: &[u32]) -> bool {
    if ids[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            results[i] = u32::try_from(*id)
                .ok()
                .map(|id| owned.contains(&id))
                .unwrap_or(false);
        }
    }
    player_condition_logic_like_cpp(logic, results)
}

fn check_count_array4_like_cpp(
    ids: [i32; 4],
    counts: [u32; 4],
    logic: u32,
    owned: &[PlayerConditionCountLikeCpp],
) -> bool {
    if ids[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            results[i] = u32::try_from(*id)
                .ok()
                .map(|id| count_for(owned, id) >= counts[i])
                .unwrap_or(false);
        }
    }
    player_condition_logic_like_cpp(logic, results)
}

fn check_currency_array_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.currency_id[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in condition.currency_id.iter().enumerate() {
        if *id != 0 {
            results[i] = count_for(context.currencies, *id) >= condition.currency_count[i];
        }
    }
    player_condition_logic_like_cpp(condition.currency_logic, results)
}

fn check_auras_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.aura_spell_id[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in condition.aura_spell_id.iter().enumerate() {
        if *id != 0 {
            results[i] = u32::try_from(*id)
                .ok()
                .and_then(|id| context.auras.iter().find(|aura| aura.spell_id == id))
                .map(|aura| {
                    condition.aura_stacks[i] == 0 || aura.stacks >= condition.aura_stacks[i]
                })
                .unwrap_or(false);
        }
    }
    player_condition_logic_like_cpp(condition.aura_spell_logic, results)
}

fn check_u16_array4_like_cpp(ids: [u16; 4], logic: u32, owned: &[u16]) -> bool {
    if ids[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            results[i] = owned.contains(id);
        }
    }
    player_condition_logic_like_cpp(logic, results)
}

fn check_lfg_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.lfg_status[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, status) in condition.lfg_status.iter().enumerate() {
        if *status != 0 {
            results[i] = player_condition_compare_like_cpp(
                i32::from(condition.lfg_compare[i]),
                count_for(context.lfg_values, u32::from(*status)) as i32,
                condition.lfg_value[i] as i32,
            );
        }
    }
    player_condition_logic_like_cpp(condition.lfg_logic, results)
}

fn check_area_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.area_id[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in condition.area_id.iter().enumerate() {
        if *id != 0 {
            let area = u32::from(*id);
            results[i] = context.area_id == area || context.parent_area_ids.contains(&area);
        }
    }
    player_condition_logic_like_cpp(condition.area_logic, results)
}

fn check_quest_kills_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.quest_kill_id == 0
        || context.quest_kill_id != condition.quest_kill_id
        || condition.quest_kill_monster.iter().all(|id| *id == 0)
    {
        return true;
    }
    let mut results = [true; 6];
    for (i, id) in condition.quest_kill_monster.iter().enumerate() {
        if *id != 0 {
            results[i] = context
                .quest_kills
                .iter()
                .find(|kill| kill.monster_id == *id)
                .map(|kill| kill.done)
                .unwrap_or(false);
        }
    }
    player_condition_logic_like_cpp(condition.quest_kill_logic, results)
}

fn read_u16_array(reader: &Wdc4Reader, idx: usize, start: usize) -> [u16; 4] {
    [
        reader.get_array_u16(idx, start, 0),
        reader.get_array_u16(idx, start, 1),
        reader.get_array_u16(idx, start, 2),
        reader.get_array_u16(idx, start, 3),
    ]
}

fn read_u8_array3(reader: &Wdc4Reader, idx: usize, start: usize) -> [u8; 3] {
    [
        reader.get_array_element(idx, start, 0, 8) as u8,
        reader.get_array_element(idx, start, 1, 8) as u8,
        reader.get_array_element(idx, start, 2, 8) as u8,
    ]
}

fn read_u8_array4(reader: &Wdc4Reader, idx: usize, start: usize) -> [u8; 4] {
    [
        reader.get_array_element(idx, start, 0, 8) as u8,
        reader.get_array_element(idx, start, 1, 8) as u8,
        reader.get_array_element(idx, start, 2, 8) as u8,
        reader.get_array_element(idx, start, 3, 8) as u8,
    ]
}

fn read_u32_array3(reader: &Wdc4Reader, idx: usize, start: usize) -> [u32; 3] {
    [
        reader.get_array_element(idx, start, 0, 32),
        reader.get_array_element(idx, start, 1, 32),
        reader.get_array_element(idx, start, 2, 32),
    ]
}

fn read_u32_array4(reader: &Wdc4Reader, idx: usize, start: usize) -> [u32; 4] {
    [
        reader.get_array_element(idx, start, 0, 32),
        reader.get_array_element(idx, start, 1, 32),
        reader.get_array_element(idx, start, 2, 32),
        reader.get_array_element(idx, start, 3, 32),
    ]
}

fn read_u32_array6(reader: &Wdc4Reader, idx: usize, start: usize) -> [u32; 6] {
    [
        reader.get_array_element(idx, start, 0, 32),
        reader.get_array_element(idx, start, 1, 32),
        reader.get_array_element(idx, start, 2, 32),
        reader.get_array_element(idx, start, 3, 32),
        reader.get_array_element(idx, start, 4, 32),
        reader.get_array_element(idx, start, 5, 32),
    ]
}

fn read_i32_array4(reader: &Wdc4Reader, idx: usize, start: usize) -> [i32; 4] {
    [
        reader.get_array_i32(idx, start, 0),
        reader.get_array_i32(idx, start, 1),
        reader.get_array_i32(idx, start, 2),
        reader.get_array_i32(idx, start, 3),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pc(id: u32) -> PlayerConditionEntry {
        PlayerConditionEntry {
            id,
            ..PlayerConditionEntry::default()
        }
    }

    #[test]
    fn load_player_condition_uses_physical_wdc4_array_fields_like_cpp_meta() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "enUS";
        let path = std::path::Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("PlayerCondition.db2");
        if !path.exists() {
            eprintln!("Skipping test: PlayerCondition.db2 fixture not found at {path:?}");
            return;
        }

        let reader = Wdc4Reader::open(&path).expect("open PlayerCondition.db2 fixture");
        assert_eq!(
            reader.field_count(),
            81,
            "C++ PlayerConditionMeta stores 147 logical fields in 81 physical WDC4 fields"
        );

        let store = PlayerConditionStore::load(data_dir, locale).expect("load PlayerCondition.db2");
        assert!(!store.is_empty());
        assert!(
            store.entries.values().any(|entry| {
                entry.skill_id.iter().any(|value| *value != 0)
                    || entry.prev_quest_id.iter().any(|value| *value != 0)
                    || entry.quest_kill_monster.iter().any(|value| *value != 0)
            }),
            "fixture should exercise at least one compacted physical array field"
        );
    }

    #[test]
    fn player_condition_compare_matches_cpp_table() {
        assert!(player_condition_compare_like_cpp(1, 7, 7));
        assert!(player_condition_compare_like_cpp(2, 7, 8));
        assert!(player_condition_compare_like_cpp(3, 8, 7));
        assert!(player_condition_compare_like_cpp(4, 7, 7));
        assert!(player_condition_compare_like_cpp(5, 6, 7));
        assert!(player_condition_compare_like_cpp(6, 7, 7));
        assert!(!player_condition_compare_like_cpp(0, 7, 7));
    }

    #[test]
    fn player_condition_logic_applies_invert_and_and_or_like_cpp() {
        assert!(player_condition_logic_like_cpp(1, [true, true]));
        assert!(player_condition_logic_like_cpp(2, [false, true]));
        assert!(!player_condition_logic_like_cpp(
            1 | (1 << 17),
            [true, true]
        ));
    }

    #[test]
    fn player_condition_filters_race_class_gender_like_cpp() {
        let condition = PlayerConditionEntry {
            race_mask: 1 << 0,
            class_mask: 1 << 1,
            gender: 1,
            ..pc(1)
        };
        let context = PlayerConditionContextLikeCpp {
            race: 1,
            class_mask: 1 << 1,
            gender: 1,
            native_gender: 0,
            ..Default::default()
        };

        assert!(is_player_meeting_condition_like_cpp(&condition, &context));
        assert!(!is_player_meeting_condition_like_cpp(
            &PlayerConditionEntry {
                race_mask: 1 << 1,
                ..condition.clone()
            },
            &context
        ));
    }

    #[test]
    fn player_condition_filters_spells_items_currency_and_auras_like_cpp() {
        let condition = PlayerConditionEntry {
            spell_id: [133, 0, 0, 0],
            item_id: [6948, 0, 0, 0],
            item_count: [1, 0, 0, 0],
            currency_id: [61, 0, 0, 0],
            currency_count: [3, 0, 0, 0],
            aura_spell_id: [21562, 0, 0, 0],
            aura_stacks: [2, 0, 0, 0],
            ..pc(1)
        };
        let context = PlayerConditionContextLikeCpp {
            spells: &[133],
            items: &[PlayerConditionCountLikeCpp { id: 6948, count: 1 }],
            currencies: &[PlayerConditionCountLikeCpp { id: 61, count: 3 }],
            auras: &[PlayerConditionAuraLikeCpp {
                spell_id: 21562,
                stacks: 2,
            }],
            ..Default::default()
        };

        assert!(is_player_meeting_condition_like_cpp(&condition, &context));
    }

    #[test]
    fn player_condition_filters_party_and_area_like_cpp() {
        let condition = PlayerConditionEntry {
            party_status: 3,
            area_id: [1519, 0, 0, 0],
            ..pc(1)
        };
        let context = PlayerConditionContextLikeCpp {
            party_status: PlayerConditionPartyStatusLikeCpp::InParty,
            area_id: 42,
            parent_area_ids: &[1519],
            ..Default::default()
        };

        assert!(is_player_meeting_condition_like_cpp(&condition, &context));
    }
}
