// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

mod default_skills;
mod reputation;
mod skills;
mod talents;

#[cfg(any(test, feature = "test-fixtures"))]
pub use reputation::{
    FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP,
    FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP,
    FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP,
};

#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::{BTreeSet, HashMap};
#[cfg(any(test, feature = "test-fixtures"))]
use super::RepresentedPlayerSkillLikeCpp;

pub const MAX_SPECIALIZATIONS_LIKE_CPP: usize = 4;

/// Handle-less test fixture for the canonical Player skill-state fallback.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct PlayerSkillTestFixtureLikeCpp {
    pub player_skill_values_like_cpp: HashMap<u16, u16>,
    pub player_skill_records_like_cpp: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    pub player_skill_non_durable_tombstones_like_cpp: BTreeSet<u16>,
    pub player_skill_records_loaded_like_cpp: bool,
    pub player_skill_records_complete_like_cpp: bool,
    pub player_skill_occupied_slots_like_cpp: Option<u16>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for PlayerSkillTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            player_skill_values_like_cpp: HashMap::new(),
            player_skill_records_like_cpp: HashMap::new(),
            player_skill_non_durable_tombstones_like_cpp: BTreeSet::new(),
            player_skill_records_loaded_like_cpp: false,
            player_skill_records_complete_like_cpp: false,
            player_skill_occupied_slots_like_cpp: None,
        }
    }
}
