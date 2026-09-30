//! Complete signed previous-Quest eligibility against existing membership facts.
//!
//! TrinityCore a5f8da2ebf5424bf0450ca4e08843ecbf72577bd,
//! Player.cpp:15088–15112, Player::SatisfyQuestPreviousQuest.
//! Rust's unsigned_abs behavior for i32::MIN remains unchanged.

use super::PlayerQuestGameplayState;
use wow_constants::quest::QUEST_STATUS_INCOMPLETE_LIKE_CPP;

#[cfg(test)]
mod tests;

impl PlayerQuestGameplayState {
    /// Zero reads neither callback; positive links require rewarded membership,
    /// while negative links require the exact active INCOMPLETE status.
    pub fn previous_quest_requirement_satisfied_from_membership(
        previous_quest_id: i32,
        mut is_rewarded: impl FnMut(u32) -> bool,
        mut status_by_id: impl FnMut(u32) -> Option<u8>,
    ) -> bool {
        if previous_quest_id == 0 {
            return true;
        }
        let previous_id = previous_quest_id.unsigned_abs();
        if previous_quest_id > 0 {
            is_rewarded(previous_id)
        } else {
            status_by_id(previous_id) == Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        }
    }
}
