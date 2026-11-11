// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical power and pet-state decoding shared with World.

use wow_constants::PowerType;
use wow_entities::{PetType, ReactState};

pub const fn power_type_from_u8_like_cpp(power: u8) -> PowerType {
    match PowerType::from_u8_like_cpp(power) {
        Some(kind) => kind,
        None => PowerType::Mana,
    }
}

pub const fn primary_power_type_for_player_class_like_cpp(class_id: u8) -> PowerType {
    match class_id {
        1 => PowerType::Rage,
        4 => PowerType::Energy,
        6 => PowerType::RunicPower,
        _ => PowerType::Mana,
    }
}

pub const fn react_state_from_db_like_cpp(value: u8) -> ReactState {
    match value {
        0 => ReactState::Passive,
        1 => ReactState::Defensive,
        2 => ReactState::Aggressive,
        _ => ReactState::Passive,
    }
}

pub const fn pet_type_from_db_like_cpp(value: u8) -> PetType {
    match value {
        0 => PetType::Summon,
        1 => PetType::Hunter,
        _ => PetType::Max,
    }
}
