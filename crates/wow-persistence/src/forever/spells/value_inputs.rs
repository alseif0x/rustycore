//! Full source hotfix projections for spell-value dependency inputs.
//! No SQL, semantic indexes, CalcValue or mutable Player state.

pub struct ExpectedStatRow {
    pub id: u32,
    pub expansion_id: i32,
    pub creature_health: f32,
    pub player_health: f32,
    pub creature_auto_attack_dps: f32,
    pub creature_armor: f32,
    pub player_mana: f32,
    pub player_primary_stat: f32,
    pub player_secondary_stat: f32,
    pub armor_constant: f32,
    pub creature_spell_damage: f32,
    pub content_set_id: i32,
    pub lvl: u32,
}

pub struct ExpectedStatModRow {
    pub id: u32,
    pub creature_health_mod: f32,
    pub player_health_mod: f32,
    pub creature_auto_attack_dps_mod: f32,
    pub creature_armor_mod: f32,
    pub player_mana_mod: f32,
    pub player_primary_stat_mod: f32,
    pub player_secondary_stat_mod: f32,
    pub armor_constant_mod: f32,
    pub creature_spell_damage_mod: f32,
}

pub struct ContentTuningRow {
    pub id: u32,
    pub flags: i32,
    pub expansion_id: i32,
    pub health_item_level_curve_id: i32,
    pub damage_item_level_curve_id: i32,
    pub health_primary_stat_curve_id: i32,
    pub damage_primary_stat_curve_id: i32,
    pub primary_stat_scaling_mod_player_data_element_character_id: i32,
    pub primary_stat_scaling_mod_player_data_element_character_multiplier: f32,
    pub min_level: i32,
    pub max_level: i32,
    pub min_level_type: i32,
    pub max_level_type: i32,
    pub target_level_delta: i32,
    pub target_level_max_delta: i32,
    pub target_level_min: i32,
    pub target_level_max: i32,
    pub min_item_level: i32,
    pub quest_xp_multiplier: f32,
}

pub struct ContentTuningXExpectedRow {
    pub id: u32,
    pub expected_stat_mod_id: i32,
    pub min_mythic_plus_season_id: i32,
    pub max_mythic_plus_season_id: i32,
    pub content_tuning_id: u32,
}

pub struct RandPropPointsRow {
    pub id: u32,
    pub damage_replace_stat_f: f32,
    pub damage_secondary_f: f32,
    pub damage_replace_stat: i32,
    pub damage_secondary: i32,
    pub epic_f: [f32; 5],
    pub superior_f: [f32; 5],
    pub good_f: [f32; 5],
    pub epic: [u32; 5],
    pub superior: [u32; 5],
    pub good: [u32; 5],
}

pub struct MythicPlusSeasonRow {
    pub id: u32,
    pub milestone_season: i32,
    pub start_time_event: i32,
    pub expansion_level: i32,
    pub heroic_lfg_dungeon_min_gear: i32,
}
