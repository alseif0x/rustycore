//! Store packets.
//!
//! Separated from player_condition.rs under #691.

use super::*;

#[derive(Debug, Clone, Default)]
pub struct PlayerConditionStore {
    pub(super) entries: HashMap<u32, PlayerConditionEntry>,
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
