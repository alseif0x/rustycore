use super::*;
// Reuse the existing narrow synthetic item builder without widening the
// creation module's production/test visibility or copying its 80 fields.
#[allow(dead_code)]
#[path = "../../../../creation/item_templates/tests/fixtures.rs"]
mod item_rows;

pub(super) fn no_draw(_: f32, _: f32) -> Result<f32, SpellValueError> {
    panic!("zero variance must not consume randomness")
}
pub(super) fn catalog(rows: SpellRecords) -> SpellCatalog {
    rows.finish(
        Default::default(),
        Default::default(),
        6,
        Default::default(),
        Default::default(),
        &Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .unwrap()
}
pub(super) fn items(inventory: Option<u8>) -> ItemCatalog {
    let mut rows = ItemRecords::default();
    if let Some(inventory) = inventory {
        let mut row = item_rows::sparse(0);
        row.inventory_type = inventory as i8;
        rows.sparse.push(row);
    }
    rows.finish(
        Default::default(),
        Default::default(),
        &Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .unwrap()
}
pub(super) fn scaling_text(levels: u32) -> String {
    let mut text = format!(
        "id\t{}\n",
        (0..24)
            .map(|c| format!("C{c}"))
            .collect::<Vec<_>>()
            .join("\t")
    );
    for level in 1..=levels {
        text.push_str(&format!(
            "{}\t{}\n",
            900 + level,
            (1..=24)
                .map(|c| (level * 100 + c).to_string())
                .collect::<Vec<_>>()
                .join("\t")
        ));
    }
    text
}
pub(super) fn tables(levels: u32) -> SpellValueGameTables {
    SpellValueGameTables::parse_strs(
        &scaling_text(levels),
        "id\tA\tB\tC\tD\n1\t2\t3\t5\t7\n",
        "id\tA\tB\tC\tD\n1\t11\t13\t17\t19\n",
    )
    .unwrap()
}
pub(super) fn random_points(id: u32, damage: f32) -> RandPropPointsRecord {
    RandPropPointsRecord {
        id,
        damage_replace_stat_f: damage,
        damage_secondary_f: damage + 1.0,
        damage_replace_stat: 0,
        damage_secondary: 0,
        epic_f: [0.0; 5],
        superior_f: [0.0; 5],
        good_f: [0.0; 5],
        epic: [0; 5],
        superior: [0; 5],
        good: [0; 5],
    }
}
pub(super) fn stat(id: u32, expansion_id: i32, health: f32) -> ExpectedStatRecord {
    ExpectedStatRecord {
        id,
        expansion_id,
        lvl: 1,
        creature_health: 10.0,
        player_health: health,
        creature_auto_attack_dps: 7.0,
        creature_armor: 40.0,
        player_mana: 80.0,
        player_primary_stat: 20.0,
        player_secondary_stat: 30.0,
        armor_constant: 50.0,
        creature_spell_damage: 100.0,
        content_set_id: 0,
    }
}
fn modifier(id: u32, value: f32) -> ExpectedStatModRecord {
    ExpectedStatModRecord {
        id,
        creature_health_mod: value,
        player_health_mod: value,
        creature_auto_attack_dps_mod: value,
        creature_armor_mod: value,
        player_mana_mod: value,
        player_primary_stat_mod: value,
        player_secondary_stat_mod: value,
        armor_constant_mod: value,
        creature_spell_damage_mod: value,
    }
}
pub(super) fn expected_catalog() -> SpellCatalog {
    let tuning = ContentTuningRecord {
        id: 7,
        expansion_id: 3,
        flags: 0,
        health_item_level_curve_id: 0,
        damage_item_level_curve_id: 0,
        health_primary_stat_curve_id: 0,
        damage_primary_stat_curve_id: 0,
        primary_stat_scaling_mod_player_data_element_character_id: 0,
        primary_stat_scaling_mod_player_data_element_character_multiplier: 0.0,
        min_level: 0,
        max_level: 0,
        min_level_type: 0,
        max_level_type: 0,
        target_level_delta: 0,
        target_level_max_delta: 0,
        target_level_min: 0,
        target_level_max: 0,
        min_item_level: 0,
        quest_xp_multiplier: 0.0,
    };
    catalog(SpellRecords {
        expected_stats: vec![stat(1, -2, 100.0), stat(2, 3, 200.0)],
        content_tunings: vec![tuning],
        expected_stat_mods: vec![modifier(8, 3.0), modifier(9, 7.0)],
        content_tuning_x_expected: vec![
            ContentTuningXExpectedRecord {
                id: 1,
                expected_stat_mod_id: 8,
                content_tuning_id: 0,
                min_mythic_plus_season_id: 0,
                max_mythic_plus_season_id: 0,
            },
            ContentTuningXExpectedRecord {
                id: 2,
                expected_stat_mod_id: 9,
                content_tuning_id: 7,
                min_mythic_plus_season_id: 0,
                max_mythic_plus_season_id: 0,
            },
        ],
        ..Default::default()
    })
}
pub(super) fn difficulty(id: i16, fallback: i16) -> DifficultyRecord {
    DifficultyRecord {
        id: id as u32,
        name: SpellText::default(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id: fallback,
        min_players: 0,
        max_players: 0,
        flags: 0,
        item_context: 0,
        toggle_difficulty_id: 0,
        group_size_health_curve_id: 0,
        group_size_dmg_curve_id: 0,
        group_size_spell_points_curve_id: 0,
        unknown1105: 0,
    }
}
