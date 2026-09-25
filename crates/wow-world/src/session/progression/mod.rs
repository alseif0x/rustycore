//! Represented character-progression responsibility, separated from the
//! Session root under #611. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

/// Handle-less test fixture for the canonical Player skill-state fallback.
#[cfg(test)]
pub(super) struct PlayerSkillTestFixtureLikeCpp {
    pub(super) player_skill_values_like_cpp: HashMap<u16, u16>,
    pub(super) player_skill_records_like_cpp: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    pub(super) player_skill_non_durable_tombstones_like_cpp: BTreeSet<u16>,
    pub(super) player_skill_records_loaded_like_cpp: bool,
    pub(super) player_skill_records_complete_like_cpp: bool,
    pub(super) player_skill_occupied_slots_like_cpp: Option<u16>,
}

#[cfg(test)]
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

mod reputation;
mod skills;
mod talents;
