//! Consuming numeric SQL DTO conversion, no second record authority.
use anyhow::Result;
use wow_data::forever_spells::*;
use wow_persistence::forever::spells::*;

pub(super) fn expected_stat(row: ExpectedStatRow) -> Result<ExpectedStatRecord> {
    Ok(ExpectedStatRecord {
        id: row.id,
        expansion_id: row.expansion_id,
        creature_health: row.creature_health,
        player_health: row.player_health,
        creature_auto_attack_dps: row.creature_auto_attack_dps,
        creature_armor: row.creature_armor,
        player_mana: row.player_mana,
        player_primary_stat: row.player_primary_stat,
        player_secondary_stat: row.player_secondary_stat,
        armor_constant: row.armor_constant,
        creature_spell_damage: row.creature_spell_damage,
        content_set_id: row.content_set_id,
        lvl: row.lvl,
    })
}

pub(super) fn expected_stat_mod(row: ExpectedStatModRow) -> Result<ExpectedStatModRecord> {
    Ok(ExpectedStatModRecord {
        id: row.id,
        creature_health_mod: row.creature_health_mod,
        player_health_mod: row.player_health_mod,
        creature_auto_attack_dps_mod: row.creature_auto_attack_dps_mod,
        creature_armor_mod: row.creature_armor_mod,
        player_mana_mod: row.player_mana_mod,
        player_primary_stat_mod: row.player_primary_stat_mod,
        player_secondary_stat_mod: row.player_secondary_stat_mod,
        armor_constant_mod: row.armor_constant_mod,
        creature_spell_damage_mod: row.creature_spell_damage_mod,
    })
}

pub(super) fn content_tuning(row: ContentTuningRow) -> Result<ContentTuningRecord> {
    Ok(ContentTuningRecord {
        id: row.id,
        flags: row.flags,
        expansion_id: row.expansion_id,
        health_item_level_curve_id: row.health_item_level_curve_id,
        damage_item_level_curve_id: row.damage_item_level_curve_id,
        health_primary_stat_curve_id: row.health_primary_stat_curve_id,
        damage_primary_stat_curve_id: row.damage_primary_stat_curve_id,
        primary_stat_scaling_mod_player_data_element_character_id: row
            .primary_stat_scaling_mod_player_data_element_character_id,
        primary_stat_scaling_mod_player_data_element_character_multiplier: row
            .primary_stat_scaling_mod_player_data_element_character_multiplier,
        min_level: row.min_level,
        max_level: row.max_level,
        min_level_type: row.min_level_type,
        max_level_type: row.max_level_type,
        target_level_delta: row.target_level_delta,
        target_level_max_delta: row.target_level_max_delta,
        target_level_min: row.target_level_min,
        target_level_max: row.target_level_max,
        min_item_level: row.min_item_level,
        quest_xp_multiplier: row.quest_xp_multiplier,
    })
}

pub(super) fn content_tuning_x_expected(
    row: ContentTuningXExpectedRow,
) -> Result<ContentTuningXExpectedRecord> {
    Ok(ContentTuningXExpectedRecord {
        id: row.id,
        expected_stat_mod_id: row.expected_stat_mod_id,
        min_mythic_plus_season_id: row.min_mythic_plus_season_id,
        max_mythic_plus_season_id: row.max_mythic_plus_season_id,
        content_tuning_id: row.content_tuning_id,
    })
}

pub(super) fn rand_prop_points(row: RandPropPointsRow) -> Result<RandPropPointsRecord> {
    Ok(RandPropPointsRecord {
        id: row.id,
        damage_replace_stat_f: row.damage_replace_stat_f,
        damage_secondary_f: row.damage_secondary_f,
        damage_replace_stat: row.damage_replace_stat,
        damage_secondary: row.damage_secondary,
        epic_f: row.epic_f,
        superior_f: row.superior_f,
        good_f: row.good_f,
        epic: row.epic,
        superior: row.superior,
        good: row.good,
    })
}

pub(super) fn mythic_plus_season(row: MythicPlusSeasonRow) -> Result<MythicPlusSeasonRecord> {
    Ok(MythicPlusSeasonRecord {
        id: row.id,
        milestone_season: row.milestone_season,
        start_time_event: row.start_time_event,
        expansion_level: row.expansion_level,
        heroic_lfg_dungeon_min_gear: row.heroic_lfg_dungeon_min_gear,
    })
}
