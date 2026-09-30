//! Periodic Quest admission against the caller's existing membership facts.
//!
//! TrinityCore a5f8da2ebf5424bf0450ca4e08843ecbf72577bd,
//! Player.cpp:15387–15401, Player::SatisfyQuestDay.
//! Weekly/monthly predicates and recurrence writes remain separate operations.

use super::PlayerQuestGameplayState;
use wow_data_model::quest::{QuestDayCooldownBlock, QuestEligibilityRules};

#[cfg(test)]
mod tests;

impl PlayerQuestGameplayState {
    /// DF is the outer branch: even a DF+daily quest reads only DF membership.
    /// Non-DF daily quests read only daily membership; other quests read neither.
    pub fn quest_day_cooldown_block_from_membership(
        quest: &QuestEligibilityRules<'_>,
        mut df_completed: impl FnMut() -> bool,
        mut daily_completed: impl FnMut() -> bool,
    ) -> Option<QuestDayCooldownBlock> {
        if quest.is_dungeon_finder() {
            if df_completed() {
                return Some(QuestDayCooldownBlock::DungeonFinder);
            }
        } else if quest.is_daily() && daily_completed() {
            return Some(QuestDayCooldownBlock::Daily);
        }
        None
    }
}
