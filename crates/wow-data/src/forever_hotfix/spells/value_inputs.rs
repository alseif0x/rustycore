//! DB2StorageBase::WriteRecord metadata order: external ID excluded,
//! inline IDs and extra parents included. Full floats/arrays retain raw bits.
use crate::forever_spells::*;

pub(super) fn expected_stat(row: &ExpectedStatRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.expansion_id.to_le_bytes());
    bytes.extend_from_slice(&row.creature_health.to_le_bytes());
    bytes.extend_from_slice(&row.player_health.to_le_bytes());
    bytes.extend_from_slice(&row.creature_auto_attack_dps.to_le_bytes());
    bytes.extend_from_slice(&row.creature_armor.to_le_bytes());
    bytes.extend_from_slice(&row.player_mana.to_le_bytes());
    bytes.extend_from_slice(&row.player_primary_stat.to_le_bytes());
    bytes.extend_from_slice(&row.player_secondary_stat.to_le_bytes());
    bytes.extend_from_slice(&row.armor_constant.to_le_bytes());
    bytes.extend_from_slice(&row.creature_spell_damage.to_le_bytes());
    bytes.extend_from_slice(&row.content_set_id.to_le_bytes());
    bytes.extend_from_slice(&row.lvl.to_le_bytes());
    bytes
}

pub(super) fn expected_stat_mod(row: &ExpectedStatModRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.creature_health_mod.to_le_bytes());
    bytes.extend_from_slice(&row.player_health_mod.to_le_bytes());
    bytes.extend_from_slice(&row.creature_auto_attack_dps_mod.to_le_bytes());
    bytes.extend_from_slice(&row.creature_armor_mod.to_le_bytes());
    bytes.extend_from_slice(&row.player_mana_mod.to_le_bytes());
    bytes.extend_from_slice(&row.player_primary_stat_mod.to_le_bytes());
    bytes.extend_from_slice(&row.player_secondary_stat_mod.to_le_bytes());
    bytes.extend_from_slice(&row.armor_constant_mod.to_le_bytes());
    bytes.extend_from_slice(&row.creature_spell_damage_mod.to_le_bytes());
    bytes
}

pub(super) fn content_tuning(row: &ContentTuningRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.id.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.expansion_id.to_le_bytes());
    bytes.extend_from_slice(&row.health_item_level_curve_id.to_le_bytes());
    bytes.extend_from_slice(&row.damage_item_level_curve_id.to_le_bytes());
    bytes.extend_from_slice(&row.health_primary_stat_curve_id.to_le_bytes());
    bytes.extend_from_slice(&row.damage_primary_stat_curve_id.to_le_bytes());
    bytes.extend_from_slice(
        &row.primary_stat_scaling_mod_player_data_element_character_id
            .to_le_bytes(),
    );
    bytes.extend_from_slice(
        &row.primary_stat_scaling_mod_player_data_element_character_multiplier
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&row.min_level.to_le_bytes());
    bytes.extend_from_slice(&row.max_level.to_le_bytes());
    bytes.extend_from_slice(&row.min_level_type.to_le_bytes());
    bytes.extend_from_slice(&row.max_level_type.to_le_bytes());
    bytes.extend_from_slice(&row.target_level_delta.to_le_bytes());
    bytes.extend_from_slice(&row.target_level_max_delta.to_le_bytes());
    bytes.extend_from_slice(&row.target_level_min.to_le_bytes());
    bytes.extend_from_slice(&row.target_level_max.to_le_bytes());
    bytes.extend_from_slice(&row.min_item_level.to_le_bytes());
    bytes.extend_from_slice(&row.quest_xp_multiplier.to_le_bytes());
    bytes
}

pub(super) fn content_tuning_x_expected(
    row: &ContentTuningXExpectedRecord,
    _locale: u8,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.expected_stat_mod_id.to_le_bytes());
    bytes.extend_from_slice(&row.min_mythic_plus_season_id.to_le_bytes());
    bytes.extend_from_slice(&row.max_mythic_plus_season_id.to_le_bytes());
    bytes.extend_from_slice(&row.content_tuning_id.to_le_bytes());
    bytes
}

pub(super) fn rand_prop_points(row: &RandPropPointsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.damage_replace_stat_f.to_le_bytes());
    bytes.extend_from_slice(&row.damage_secondary_f.to_le_bytes());
    bytes.extend_from_slice(&row.damage_replace_stat.to_le_bytes());
    bytes.extend_from_slice(&row.damage_secondary.to_le_bytes());
    for value in &row.epic_f {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.superior_f {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.good_f {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.epic {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.superior {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &row.good {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(super) fn mythic_plus_season(row: &MythicPlusSeasonRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.id.to_le_bytes());
    bytes.extend_from_slice(&row.milestone_season.to_le_bytes());
    bytes.extend_from_slice(&row.start_time_event.to_le_bytes());
    bytes.extend_from_slice(&row.expansion_level.to_le_bytes());
    bytes.extend_from_slice(&row.heroic_lfg_dungeon_min_gear.to_le_bytes());
    bytes
}
