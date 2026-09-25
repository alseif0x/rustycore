//! Test-only detached instance state owned by the Session fixture.

use super::super::{
    DIFFICULTY_10_N_LIKE_CPP, DIFFICULTY_NORMAL_LIKE_CPP, DIFFICULTY_NORMAL_RAID_LIKE_CPP,
};
use std::collections::HashMap;

/// Handle-less test inputs for Player-owned difficulty and recent-instance state.
pub(crate) struct InstanceTestFixtureLikeCpp {
    pub(crate) represented_dungeon_difficulty_id_like_cpp: u32,
    pub(in crate::session) represented_raid_difficulty_id_like_cpp: u32,
    pub(in crate::session) represented_legacy_raid_difficulty_id_like_cpp: u32,
    pub(in crate::session) represented_player_recent_instances_like_cpp: HashMap<u32, u32>,
}

impl Default for InstanceTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_dungeon_difficulty_id_like_cpp: DIFFICULTY_NORMAL_LIKE_CPP,
            represented_raid_difficulty_id_like_cpp: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            represented_legacy_raid_difficulty_id_like_cpp: DIFFICULTY_10_N_LIKE_CPP,
            represented_player_recent_instances_like_cpp: HashMap::new(),
        }
    }
}
