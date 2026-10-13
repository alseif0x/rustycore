// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player load values: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Gender, Team, player_team_for_race_cpp};

pub(in crate::session) fn gender_from_u8(value: u8) -> Gender {
    match value {
        1 => Gender::Female,
        2 => Gender::None,
        _ => Gender::Male,
    }
}

pub(in crate::session) fn player_team_id_for_race_cpp(race: u8) -> u32 {
    match player_team_for_race_cpp(race) {
        Team::Horde => 1,
        _ => 0,
    }
}
