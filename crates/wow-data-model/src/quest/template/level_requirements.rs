//! Quest level bounds shared by availability and application admission gates.
//! TrinityCore a5f8da2e, Player.cpp:15038–15067; existing Rust row semantics retained.

use super::QuestTemplate;

impl QuestTemplate {
    pub fn meets_min_level(&self, level: u8) -> bool {
        self.min_level <= 0 || i32::from(level) >= self.min_level
    }

    pub fn meets_max_level(&self, level: u8) -> bool {
        self.max_level == 0 || level <= self.max_level
    }
}
