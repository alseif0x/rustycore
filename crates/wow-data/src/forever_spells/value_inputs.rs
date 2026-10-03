//! Full 02245dcd DB2Structure/LoadInfo calculation dependencies.
//! Raw immutable inputs, not EvaluateExpectedStat/CalcValue readiness.

/// 02245dcd DBCEnums.h:988-1000. Not an item-stat or legacy enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ExpectedStatType {
    CreatureHealth = 0,
    PlayerHealth = 1,
    CreatureAutoAttackDps = 2,
    CreatureArmor = 3,
    PlayerMana = 4,
    PlayerPrimaryStat = 5,
    PlayerSecondaryStat = 6,
    ArmorConstant = 7,
    None = 8,
    CreatureSpellDamage = 9,
}

pub struct ExpectedStatRecord {
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

pub struct ExpectedStatModRecord {
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

pub struct ContentTuningRecord {
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

pub struct ContentTuningXExpectedRecord {
    pub id: u32,
    pub expected_stat_mod_id: i32,
    pub min_mythic_plus_season_id: i32,
    pub max_mythic_plus_season_id: i32,
    pub content_tuning_id: u32,
}

pub struct RandPropPointsRecord {
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

pub struct MythicPlusSeasonRecord {
    pub id: u32,
    pub milestone_season: i32,
    pub start_time_event: i32,
    pub expansion_level: i32,
    pub heroic_lfg_dungeon_min_gear: i32,
}
