// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical power and pet-state decoding shared with World.

use wow_constants::PowerType;
use wow_entities::{PetType, ReactState};

pub const fn power_type_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
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
