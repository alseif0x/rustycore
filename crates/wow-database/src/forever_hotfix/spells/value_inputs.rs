//! Strict full SQL field order, 02245dcd HotfixDatabase/DB2LoadInfo.
use super::{LoadError, SpellRow};
use crate::SqlResult;
use wow_persistence::forever::spells::*;

pub(super) fn expected_stat(result: &SqlResult) -> Result<ExpectedStatRow, LoadError> {
    let mut r = SpellRow::new(result, 13)?;
    let row = ExpectedStatRow {
        id: r.read()?,
        expansion_id: r.read()?,
        creature_health: r.read()?,
        player_health: r.read()?,
        creature_auto_attack_dps: r.read()?,
        creature_armor: r.read()?,
        player_mana: r.read()?,
        player_primary_stat: r.read()?,
        player_secondary_stat: r.read()?,
        armor_constant: r.read()?,
        creature_spell_damage: r.read()?,
        content_set_id: r.read()?,
        lvl: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn expected_stat_mod(result: &SqlResult) -> Result<ExpectedStatModRow, LoadError> {
    let mut r = SpellRow::new(result, 10)?;
    let row = ExpectedStatModRow {
        id: r.read()?,
        creature_health_mod: r.read()?,
        player_health_mod: r.read()?,
        creature_auto_attack_dps_mod: r.read()?,
        creature_armor_mod: r.read()?,
        player_mana_mod: r.read()?,
        player_primary_stat_mod: r.read()?,
        player_secondary_stat_mod: r.read()?,
        armor_constant_mod: r.read()?,
        creature_spell_damage_mod: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn content_tuning(result: &SqlResult) -> Result<ContentTuningRow, LoadError> {
    let mut r = SpellRow::new(result, 19)?;
    let row = ContentTuningRow {
        id: r.read()?,
        flags: r.read()?,
        expansion_id: r.read()?,
        health_item_level_curve_id: r.read()?,
        damage_item_level_curve_id: r.read()?,
        health_primary_stat_curve_id: r.read()?,
        damage_primary_stat_curve_id: r.read()?,
        primary_stat_scaling_mod_player_data_element_character_id: r.read()?,
        primary_stat_scaling_mod_player_data_element_character_multiplier: r.read()?,
        min_level: r.read()?,
        max_level: r.read()?,
        min_level_type: r.read()?,
        max_level_type: r.read()?,
        target_level_delta: r.read()?,
        target_level_max_delta: r.read()?,
        target_level_min: r.read()?,
        target_level_max: r.read()?,
        min_item_level: r.read()?,
        quest_xp_multiplier: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn content_tuning_x_expected(
    result: &SqlResult,
) -> Result<ContentTuningXExpectedRow, LoadError> {
    let mut r = SpellRow::new(result, 5)?;
    let row = ContentTuningXExpectedRow {
        id: r.read()?,
        expected_stat_mod_id: r.read()?,
        min_mythic_plus_season_id: r.read()?,
        max_mythic_plus_season_id: r.read()?,
        content_tuning_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) const RAND_PROP_COLUMNS: usize = 35;

pub(super) fn rand_prop_points(result: &SqlResult) -> Result<RandPropPointsRow, LoadError> {
    let mut r = SpellRow::new(result, RAND_PROP_COLUMNS)?;
    let row = RandPropPointsRow {
        id: r.read()?,
        damage_replace_stat_f: r.read()?,
        damage_secondary_f: r.read()?,
        damage_replace_stat: r.read()?,
        damage_secondary: r.read()?,
        epic_f: r.array()?,
        superior_f: r.array()?,
        good_f: r.array()?,
        epic: r.array()?,
        superior: r.array()?,
        good: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn mythic_plus_season(result: &SqlResult) -> Result<MythicPlusSeasonRow, LoadError> {
    let mut r = SpellRow::new(result, 5)?;
    let row = MythicPlusSeasonRow {
        id: r.read()?,
        milestone_season: r.read()?,
        start_time_event: r.read()?,
        expansion_level: r.read()?,
        heroic_lfg_dungeon_min_gear: r.read()?,
    };
    r.finish()?;
    Ok(row)
}
