// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-creation defaults and restored-stat support.

use super::{PowerType, primary_power_type_for_class_like_cpp};

/// Default start position for a race.
/// Returns (map_id, x, y, z, orientation).
pub(super) fn start_position(race: u8) -> (i32, f32, f32, f32, f32) {
    match race {
        1 => (0, -8949.95, -132.493, 83.5312, 0.0),       // Human
        2 => (1, -618.518, -4251.67, 38.718, 0.0),        // Orc
        3 => (0, -6240.32, 331.033, 382.758, 6.17716),    // Dwarf
        4 => (1, 10311.3, 832.463, 1326.41, 5.69632),     // NightElf
        5 => (0, 1676.71, 1678.31, 121.67, 2.70526),      // Undead
        6 => (1, -2917.58, -257.98, 52.9968, 0.0),        // Tauren
        7 => (0, -6240.32, 331.033, 382.758, 0.0),        // Gnome
        8 => (1, -618.518, -4251.67, 38.718, 0.0),        // Troll
        10 => (530, 10349.6, -6357.29, 33.4026, 5.31605), // BloodElf
        11 => (530, -3961.64, -13931.2, 100.615, 2.08364), // Draenei
        22 => (0, -8949.95, -132.493, 83.5312, 0.0),      // Worgen → Human
        _ => (0, -8949.95, -132.493, 83.5312, 0.0),       // Default: Human
    }
}

/// Default display ID for a race/sex combination.
pub(crate) fn default_display_id(race: u8, sex: u8) -> u32 {
    match (race, sex) {
        (1, 0) => 49,
        (1, 1) => 50, // Human M/F
        (2, 0) => 51,
        (2, 1) => 52, // Orc
        (3, 0) => 53,
        (3, 1) => 54, // Dwarf
        (4, 0) => 55,
        (4, 1) => 56, // NightElf
        (5, 0) => 57,
        (5, 1) => 58, // Undead
        (6, 0) => 59,
        (6, 1) => 60, // Tauren
        (7, 0) => 1563,
        (7, 1) => 1564, // Gnome
        (8, 0) => 1478,
        (8, 1) => 1479, // Troll
        (10, 0) => 15476,
        (10, 1) => 15475, // BloodElf
        (11, 0) => 16125,
        (11, 1) => 16126, // Draenei
        _ => 49,          // Default: Human Male
    }
}

/// Default zone ID for a starting position.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn start_zone(race: u8) -> i32 {
    match race {
        1 | 22 => 12, // Human / Worgen: Elwynn Forest
        2 | 8 => 14,  // Orc / Troll: Durotar
        3 | 7 => 1,   // Dwarf / Gnome: Dun Morogh
        4 => 141,     // NightElf: Teldrassil
        5 => 85,      // Undead: Tirisfal Glades
        6 => 215,     // Tauren: Mulgore
        10 => 3430,   // BloodElf: Eversong Woods
        11 => 3524,   // Draenei: Azuremyst Isle
        _ => 12,
    }
}

/// Default starting health and mana for a level 1 character by class.
pub(super) fn default_health_mana(class: u8) -> (u32, u32) {
    match class {
        1 => (50, 0),   // Warrior — no mana
        2 => (52, 79),  // Paladin
        3 => (46, 85),  // Hunter (uses focus at high level, mana at 1)
        4 => (45, 0),   // Rogue — no mana
        5 => (52, 160), // Priest
        6 => (130, 0),  // Death Knight — no mana (runic power)
        7 => (47, 73),  // Shaman
        8 => (42, 200), // Mage
        9 => (43, 200), // Warlock
        11 => (54, 60), // Druid
        _ => (50, 100), // Default
    }
}

pub(super) fn max_health_u32_like_cpp(max_health: i64) -> u32 {
    max_health.max(1).min(i64::from(u32::MAX)) as u32
}

pub(super) fn restored_saved_health_like_cpp(saved_health: Option<u32>, max_health: i64) -> i64 {
    let max_health = max_health_u32_like_cpp(max_health);
    saved_health
        .map(|health| i64::from(health.min(max_health)))
        .unwrap_or(i64::from(max_health))
}

pub(super) fn default_character_power1_like_cpp(class: u8, mana: u32) -> u32 {
    match primary_power_type_for_class_like_cpp(class) {
        PowerType::Energy => 100,
        _ => mana,
    }
}
