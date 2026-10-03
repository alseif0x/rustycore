use super::super::value_inputs;
use wow_persistence::forever::spells::*;

#[test]
fn consuming_sql_batch_conversion_preserves_the_separate_storage_size_observation() {
    for size in [None, Some(0), Some(1), Some(u32::MAX)] {
        let result = super::super::records(SpellRows {
            rand_prop_points_sql_index_size: size,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(result.rand_prop_points_sql_index_size, size);
        assert!(result.rand_prop_points_storage_last_index.is_none());
    }
}

#[test]
fn expected_stat_sql_conversion_preserves_every_cell_and_array_bit() {
    let id = 0xf1234567;
    let row = ExpectedStatRow {
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
    };
    let result = value_inputs::expected_stat(row).unwrap();
    assert_eq!(result.id, id);
    assert_eq!(result.expansion_id, 0x80000010u32 as i32);
    assert_eq!(result.creature_health.to_bits(), 0x7fc00020);
    assert_eq!(result.player_health.to_bits(), 0x7fc00030);
    assert_eq!(result.creature_auto_attack_dps.to_bits(), 0x7fc00040);
    assert_eq!(result.creature_armor.to_bits(), 0x7fc00050);
    assert_eq!(result.player_mana.to_bits(), 0x7fc00060);
    assert_eq!(result.player_primary_stat.to_bits(), 0x7fc00070);
    assert_eq!(result.player_secondary_stat.to_bits(), 0x7fc00080);
    assert_eq!(result.armor_constant.to_bits(), 0x7fc00090);
    assert_eq!(result.creature_spell_damage.to_bits(), 0x7fc000a0);
    assert_eq!(result.content_set_id, 0x800000b0u32 as i32);
    assert_eq!(result.lvl, 0x800000c0);
}

#[test]
fn expected_stat_mod_sql_conversion_preserves_every_cell_and_array_bit() {
    let id = 0xf1234567;
    let row = ExpectedStatModRow {
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
    };
    let result = value_inputs::expected_stat_mod(row).unwrap();
    assert_eq!(result.id, id);
    assert_eq!(result.creature_health_mod.to_bits(), 0x7fc00010);
    assert_eq!(result.player_health_mod.to_bits(), 0x7fc00020);
    assert_eq!(result.creature_auto_attack_dps_mod.to_bits(), 0x7fc00030);
    assert_eq!(result.creature_armor_mod.to_bits(), 0x7fc00040);
    assert_eq!(result.player_mana_mod.to_bits(), 0x7fc00050);
    assert_eq!(result.player_primary_stat_mod.to_bits(), 0x7fc00060);
    assert_eq!(result.player_secondary_stat_mod.to_bits(), 0x7fc00070);
    assert_eq!(result.armor_constant_mod.to_bits(), 0x7fc00080);
    assert_eq!(result.creature_spell_damage_mod.to_bits(), 0x7fc00090);
}

#[test]
fn content_tuning_sql_conversion_preserves_every_cell_and_array_bit() {
    let id = 0xf1234567;
    let row = ContentTuningRow {
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
    };
    let result = value_inputs::content_tuning(row).unwrap();
    assert_eq!(result.id, id);
    assert_eq!(result.flags, 0x80000020u32 as i32);
    assert_eq!(result.expansion_id, 0x80000030u32 as i32);
    assert_eq!(result.health_item_level_curve_id, 0x80000040u32 as i32);
    assert_eq!(result.damage_item_level_curve_id, 0x80000050u32 as i32);
    assert_eq!(result.health_primary_stat_curve_id, 0x80000060u32 as i32);
    assert_eq!(result.damage_primary_stat_curve_id, 0x80000070u32 as i32);
    assert_eq!(
        result.primary_stat_scaling_mod_player_data_element_character_id,
        0x80000080u32 as i32
    );
    assert_eq!(
        result
            .primary_stat_scaling_mod_player_data_element_character_multiplier
            .to_bits(),
        0x7fc00090
    );
    assert_eq!(result.min_level, 0x800000a0u32 as i32);
    assert_eq!(result.max_level, 0x800000b0u32 as i32);
    assert_eq!(result.min_level_type, 0x800000c0u32 as i32);
    assert_eq!(result.max_level_type, 0x800000d0u32 as i32);
    assert_eq!(result.target_level_delta, 0x800000e0u32 as i32);
    assert_eq!(result.target_level_max_delta, 0x800000f0u32 as i32);
    assert_eq!(result.target_level_min, 0x80000100u32 as i32);
    assert_eq!(result.target_level_max, 0x80000110u32 as i32);
    assert_eq!(result.min_item_level, 0x80000120u32 as i32);
    assert_eq!(result.quest_xp_multiplier.to_bits(), 0x7fc00130);
}

#[test]
fn content_tuning_x_expected_sql_conversion_preserves_every_cell_and_array_bit() {
    let id = 0xf1234567;
    let row = ContentTuningXExpectedRow {
        id: id,
        expected_stat_mod_id: 0x80000010u32 as i32,
        min_mythic_plus_season_id: 0x80000020u32 as i32,
        max_mythic_plus_season_id: 0x80000030u32 as i32,
        content_tuning_id: 0x80000040,
    };
    let result = value_inputs::content_tuning_x_expected(row).unwrap();
    assert_eq!(result.id, id);
    assert_eq!(result.expected_stat_mod_id, 0x80000010u32 as i32);
    assert_eq!(result.min_mythic_plus_season_id, 0x80000020u32 as i32);
    assert_eq!(result.max_mythic_plus_season_id, 0x80000030u32 as i32);
    assert_eq!(result.content_tuning_id, 0x80000040);
}

#[test]
fn rand_prop_points_sql_conversion_preserves_every_cell_and_array_bit() {
    let id = 0xf1234567;
    let row = RandPropPointsRow {
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
    };
    let result = value_inputs::rand_prop_points(row).unwrap();
    assert_eq!(result.id, id);
    assert_eq!(result.damage_replace_stat_f.to_bits(), 0x7fc00010);
    assert_eq!(result.damage_secondary_f.to_bits(), 0x7fc00020);
    assert_eq!(result.damage_replace_stat, 0x80000030u32 as i32);
    assert_eq!(result.damage_secondary, 0x80000040u32 as i32);
    assert_eq!(
        result.epic_f.map(f32::to_bits),
        [0x7fc00050, 0x7fc00051, 0x7fc00052, 0x7fc00053, 0x7fc00054]
    );
    assert_eq!(
        result.superior_f.map(f32::to_bits),
        [0x7fc00060, 0x7fc00061, 0x7fc00062, 0x7fc00063, 0x7fc00064]
    );
    assert_eq!(
        result.good_f.map(f32::to_bits),
        [0x7fc00070, 0x7fc00071, 0x7fc00072, 0x7fc00073, 0x7fc00074]
    );
    assert_eq!(
        result.epic,
        [0x80000080, 0x80000081, 0x80000082, 0x80000083, 0x80000084]
    );
    assert_eq!(
        result.superior,
        [0x80000090, 0x80000091, 0x80000092, 0x80000093, 0x80000094]
    );
    assert_eq!(
        result.good,
        [0x800000a0, 0x800000a1, 0x800000a2, 0x800000a3, 0x800000a4]
    );
}

#[test]
fn mythic_plus_season_sql_conversion_preserves_every_cell_and_array_bit() {
    let id = 0xf1234567;
    let row = MythicPlusSeasonRow {
        id: id,
        milestone_season: 0x80000020u32 as i32,
        start_time_event: 0x80000030u32 as i32,
        expansion_level: 0x80000040u32 as i32,
        heroic_lfg_dungeon_min_gear: 0x80000050u32 as i32,
    };
    let result = value_inputs::mythic_plus_season(row).unwrap();
    assert_eq!(result.id, id);
    assert_eq!(result.milestone_season, 0x80000020u32 as i32);
    assert_eq!(result.start_time_event, 0x80000030u32 as i32);
    assert_eq!(result.expansion_level, 0x80000040u32 as i32);
    assert_eq!(result.heroic_lfg_dungeon_min_gear, 0x80000050u32 as i32);
}
