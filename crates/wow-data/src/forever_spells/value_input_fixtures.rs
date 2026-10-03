//! Synthetic complete records; no private client cells are used.
use super::*;

pub(crate) fn expected_stat(id: u32) -> ExpectedStatRecord {
    ExpectedStatRecord {
        id: id,
        expansion_id: 0x80000010u32 as i32,
        creature_health: f32::from_bits(0x7fc00020),
        player_health: f32::from_bits(0x7fc00030),
        creature_auto_attack_dps: f32::from_bits(0x7fc00040),
        creature_armor: f32::from_bits(0x7fc00050),
        player_mana: f32::from_bits(0x7fc00060),
        player_primary_stat: f32::from_bits(0x7fc00070),
        player_secondary_stat: f32::from_bits(0x7fc00080),
        armor_constant: f32::from_bits(0x7fc00090),
        creature_spell_damage: f32::from_bits(0x7fc000a0),
        content_set_id: 0x800000b0u32 as i32,
        lvl: 0x800000c0,
    }
}

pub(crate) fn expected_stat_mod(id: u32) -> ExpectedStatModRecord {
    ExpectedStatModRecord {
        id: id,
        creature_health_mod: f32::from_bits(0x7fc00010),
        player_health_mod: f32::from_bits(0x7fc00020),
        creature_auto_attack_dps_mod: f32::from_bits(0x7fc00030),
        creature_armor_mod: f32::from_bits(0x7fc00040),
        player_mana_mod: f32::from_bits(0x7fc00050),
        player_primary_stat_mod: f32::from_bits(0x7fc00060),
        player_secondary_stat_mod: f32::from_bits(0x7fc00070),
        armor_constant_mod: f32::from_bits(0x7fc00080),
        creature_spell_damage_mod: f32::from_bits(0x7fc00090),
    }
}

pub(crate) fn content_tuning(id: u32) -> ContentTuningRecord {
    ContentTuningRecord {
        id: id,
        flags: 0x80000020u32 as i32,
        expansion_id: 0x80000030u32 as i32,
        health_item_level_curve_id: 0x80000040u32 as i32,
        damage_item_level_curve_id: 0x80000050u32 as i32,
        health_primary_stat_curve_id: 0x80000060u32 as i32,
        damage_primary_stat_curve_id: 0x80000070u32 as i32,
        primary_stat_scaling_mod_player_data_element_character_id: 0x80000080u32 as i32,
        primary_stat_scaling_mod_player_data_element_character_multiplier: f32::from_bits(
            0x7fc00090,
        ),
        min_level: 0x800000a0u32 as i32,
        max_level: 0x800000b0u32 as i32,
        min_level_type: 0x800000c0u32 as i32,
        max_level_type: 0x800000d0u32 as i32,
        target_level_delta: 0x800000e0u32 as i32,
        target_level_max_delta: 0x800000f0u32 as i32,
        target_level_min: 0x80000100u32 as i32,
        target_level_max: 0x80000110u32 as i32,
        min_item_level: 0x80000120u32 as i32,
        quest_xp_multiplier: f32::from_bits(0x7fc00130),
    }
}

pub(crate) fn content_tuning_x_expected(id: u32) -> ContentTuningXExpectedRecord {
    ContentTuningXExpectedRecord {
        id: id,
        expected_stat_mod_id: 0x80000010u32 as i32,
        min_mythic_plus_season_id: 0x80000020u32 as i32,
        max_mythic_plus_season_id: 0x80000030u32 as i32,
        content_tuning_id: 0x80000040,
    }
}

pub(crate) fn rand_prop_points(id: u32) -> RandPropPointsRecord {
    RandPropPointsRecord {
        id: id,
        damage_replace_stat_f: f32::from_bits(0x7fc00010),
        damage_secondary_f: f32::from_bits(0x7fc00020),
        damage_replace_stat: 0x80000030u32 as i32,
        damage_secondary: 0x80000040u32 as i32,
        epic_f: [
            f32::from_bits(0x7fc00050),
            f32::from_bits(0x7fc00051),
            f32::from_bits(0x7fc00052),
            f32::from_bits(0x7fc00053),
            f32::from_bits(0x7fc00054),
        ],
        superior_f: [
            f32::from_bits(0x7fc00060),
            f32::from_bits(0x7fc00061),
            f32::from_bits(0x7fc00062),
            f32::from_bits(0x7fc00063),
            f32::from_bits(0x7fc00064),
        ],
        good_f: [
            f32::from_bits(0x7fc00070),
            f32::from_bits(0x7fc00071),
            f32::from_bits(0x7fc00072),
            f32::from_bits(0x7fc00073),
            f32::from_bits(0x7fc00074),
        ],
        epic: [0x80000080, 0x80000081, 0x80000082, 0x80000083, 0x80000084],
        superior: [0x80000090, 0x80000091, 0x80000092, 0x80000093, 0x80000094],
        good: [0x800000a0, 0x800000a1, 0x800000a2, 0x800000a3, 0x800000a4],
    }
}

pub(crate) fn mythic_plus_season(id: u32) -> MythicPlusSeasonRecord {
    MythicPlusSeasonRecord {
        id: id,
        milestone_season: 0x80000020u32 as i32,
        start_time_event: 0x80000030u32 as i32,
        expansion_level: 0x80000040u32 as i32,
        heroic_lfg_dungeon_min_gear: 0x80000050u32 as i32,
    }
}
