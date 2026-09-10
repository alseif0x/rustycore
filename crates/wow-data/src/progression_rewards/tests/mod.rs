//! Progression-reward store regressions.
//!
//! Separated from progression_rewards.rs under #685.

use super::*;

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}

fn curve(id: u32, curve_type: u8) -> CurveEntry {
    CurveEntry {
        id,
        curve_type,
        flags: 0,
    }
}

fn point(id: u32, curve_id: u32, order_index: u8, x: f32, y: f32) -> CurvePointEntry {
    CurvePointEntry {
        id,
        pos: [x, y],
        pre_sl_squish_pos: [0.0, 0.0],
        curve_id,
        order_index,
    }
}

fn faction_template_for_test(
    id: u32,
    faction: u16,
    faction_group: u8,
    friend_group: u8,
    enemy_group: u8,
) -> FactionTemplateEntry {
    FactionTemplateEntry {
        id,
        faction,
        flags: 0,
        faction_group,
        friend_group,
        enemy_group,
        enemies: [0; 8],
        friend: [0; 8],
    }
}

fn scaling_stat_values_for_test(id: u32, char_level: i32) -> ScalingStatValuesEntry {
    ScalingStatValuesEntry {
        id,
        char_level,
        weapon_dps_1h: 101,
        weapon_dps_2h: 102,
        spellcaster_dps_1h: 103,
        spellcaster_dps_2h: 104,
        ranged_dps: 105,
        wand_dps: 106,
        spell_power: 107,
        shoulder_budget: 201,
        trinket_budget: 202,
        weapon_budget_1h: 203,
        primary_budget: 204,
        ranged_budget: 205,
        tertiary_budget: 206,
        cloth_shoulder_armor: 301,
        leather_shoulder_armor: 302,
        mail_shoulder_armor: 303,
        plate_shoulder_armor: 304,
        cloth_cloak_armor: 305,
        cloth_chest_armor: 306,
        leather_chest_armor: 307,
        mail_chest_armor: 308,
        plate_chest_armor: 309,
    }
}

mod scenarios;
