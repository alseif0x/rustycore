//! Full metadata field/array/parent decoding, 02245dcd.
use crate::forever_spells::*;
use crate::wdc4::creation::CreationDb2;
use anyhow::Result;

pub(super) fn expected_stat(table: &CreationDb2, id: u32) -> Result<ExpectedStatRecord> {
    Ok(ExpectedStatRecord {
        id: id,
        expansion_id: table.bits(id, 0, 0)? as i32,
        creature_health: f32::from_bits(table.bits(id, 1, 0)?),
        player_health: f32::from_bits(table.bits(id, 2, 0)?),
        creature_auto_attack_dps: f32::from_bits(table.bits(id, 3, 0)?),
        creature_armor: f32::from_bits(table.bits(id, 4, 0)?),
        player_mana: f32::from_bits(table.bits(id, 5, 0)?),
        player_primary_stat: f32::from_bits(table.bits(id, 6, 0)?),
        player_secondary_stat: f32::from_bits(table.bits(id, 7, 0)?),
        armor_constant: f32::from_bits(table.bits(id, 8, 0)?),
        creature_spell_damage: f32::from_bits(table.bits(id, 9, 0)?),
        content_set_id: table.bits(id, 10, 0)? as i32,
        lvl: table.bits(id, 11, 0)?,
    })
}

pub(super) fn expected_stat_mod(table: &CreationDb2, id: u32) -> Result<ExpectedStatModRecord> {
    Ok(ExpectedStatModRecord {
        id: id,
        creature_health_mod: f32::from_bits(table.bits(id, 0, 0)?),
        player_health_mod: f32::from_bits(table.bits(id, 1, 0)?),
        creature_auto_attack_dps_mod: f32::from_bits(table.bits(id, 2, 0)?),
        creature_armor_mod: f32::from_bits(table.bits(id, 3, 0)?),
        player_mana_mod: f32::from_bits(table.bits(id, 4, 0)?),
        player_primary_stat_mod: f32::from_bits(table.bits(id, 5, 0)?),
        player_secondary_stat_mod: f32::from_bits(table.bits(id, 6, 0)?),
        armor_constant_mod: f32::from_bits(table.bits(id, 7, 0)?),
        creature_spell_damage_mod: f32::from_bits(table.bits(id, 8, 0)?),
    })
}

pub(super) fn content_tuning(table: &CreationDb2, id: u32) -> Result<ContentTuningRecord> {
    Ok(ContentTuningRecord {
        id: id,
        flags: table.bits(id, 1, 0)? as i32,
        expansion_id: table.bits(id, 2, 0)? as i32,
        health_item_level_curve_id: table.bits(id, 3, 0)? as i32,
        damage_item_level_curve_id: table.bits(id, 4, 0)? as i32,
        health_primary_stat_curve_id: table.bits(id, 5, 0)? as i32,
        damage_primary_stat_curve_id: table.bits(id, 6, 0)? as i32,
        primary_stat_scaling_mod_player_data_element_character_id: table.bits(id, 7, 0)? as i32,
        primary_stat_scaling_mod_player_data_element_character_multiplier: f32::from_bits(
            table.bits(id, 8, 0)?,
        ),
        min_level: table.bits(id, 9, 0)? as i32,
        max_level: table.bits(id, 10, 0)? as i32,
        min_level_type: table.bits(id, 11, 0)? as i32,
        max_level_type: table.bits(id, 12, 0)? as i32,
        target_level_delta: table.bits(id, 13, 0)? as i32,
        target_level_max_delta: table.bits(id, 14, 0)? as i32,
        target_level_min: table.bits(id, 15, 0)? as i32,
        target_level_max: table.bits(id, 16, 0)? as i32,
        min_item_level: table.bits(id, 17, 0)? as i32,
        quest_xp_multiplier: f32::from_bits(table.bits(id, 18, 0)?),
    })
}

pub(super) fn content_tuning_x_expected(
    table: &CreationDb2,
    id: u32,
) -> Result<ContentTuningXExpectedRecord> {
    Ok(ContentTuningXExpectedRecord {
        id: id,
        expected_stat_mod_id: table.bits(id, 0, 0)? as i32,
        min_mythic_plus_season_id: table.bits(id, 1, 0)? as i32,
        max_mythic_plus_season_id: table.bits(id, 2, 0)? as i32,
        content_tuning_id: table.bits(id, 3, 0)?,
    })
}

pub(super) fn rand_prop_points(table: &CreationDb2, id: u32) -> Result<RandPropPointsRecord> {
    Ok(RandPropPointsRecord {
        id: id,
        damage_replace_stat_f: f32::from_bits(table.bits(id, 0, 0)?),
        damage_secondary_f: f32::from_bits(table.bits(id, 1, 0)?),
        damage_replace_stat: table.bits(id, 2, 0)? as i32,
        damage_secondary: table.bits(id, 3, 0)? as i32,
        epic_f: [
            f32::from_bits(table.bits(id, 4, 0)?),
            f32::from_bits(table.bits(id, 4, 1)?),
            f32::from_bits(table.bits(id, 4, 2)?),
            f32::from_bits(table.bits(id, 4, 3)?),
            f32::from_bits(table.bits(id, 4, 4)?),
        ],
        superior_f: [
            f32::from_bits(table.bits(id, 5, 0)?),
            f32::from_bits(table.bits(id, 5, 1)?),
            f32::from_bits(table.bits(id, 5, 2)?),
            f32::from_bits(table.bits(id, 5, 3)?),
            f32::from_bits(table.bits(id, 5, 4)?),
        ],
        good_f: [
            f32::from_bits(table.bits(id, 6, 0)?),
            f32::from_bits(table.bits(id, 6, 1)?),
            f32::from_bits(table.bits(id, 6, 2)?),
            f32::from_bits(table.bits(id, 6, 3)?),
            f32::from_bits(table.bits(id, 6, 4)?),
        ],
        epic: [
            table.bits(id, 7, 0)?,
            table.bits(id, 7, 1)?,
            table.bits(id, 7, 2)?,
            table.bits(id, 7, 3)?,
            table.bits(id, 7, 4)?,
        ],
        superior: [
            table.bits(id, 8, 0)?,
            table.bits(id, 8, 1)?,
            table.bits(id, 8, 2)?,
            table.bits(id, 8, 3)?,
            table.bits(id, 8, 4)?,
        ],
        good: [
            table.bits(id, 9, 0)?,
            table.bits(id, 9, 1)?,
            table.bits(id, 9, 2)?,
            table.bits(id, 9, 3)?,
            table.bits(id, 9, 4)?,
        ],
    })
}

pub(super) fn mythic_plus_season(table: &CreationDb2, id: u32) -> Result<MythicPlusSeasonRecord> {
    Ok(MythicPlusSeasonRecord {
        id: id,
        milestone_season: table.bits(id, 1, 0)? as i32,
        start_time_event: table.bits(id, 2, 0)? as i32,
        expansion_level: table.bits(id, 3, 0)? as i32,
        heroic_lfg_dungeon_min_gear: table.bits(id, 4, 0)? as i32,
    })
}
