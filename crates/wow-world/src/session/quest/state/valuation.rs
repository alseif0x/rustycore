//! valuation operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    /// Set C++ `CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF`.
    pub fn set_quest_low_level_hide_diff_like_cpp(&mut self, value: u32) {
        self.quest_state.quest_low_level_hide_diff_like_cpp = value;
    }
    /// Set C++ `CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF`.
    pub fn set_quest_high_level_hide_diff_like_cpp(&mut self, value: u32) {
        self.quest_state.quest_high_level_hide_diff_like_cpp = value;
    }
    /// Set the QuestXP store (loaded from QuestXP.db2).
    pub fn set_quest_xp_store(&mut self, store: Arc<wow_data::quest_xp::QuestXpStore>) {
        self.quests.xp_store = Some(store);
    }
    pub fn set_min_quest_scaled_xp_ratio_like_cpp(&mut self, ratio: u32) {
        self.quest_state.min_quest_scaled_xp_ratio_like_cpp = if ratio > 100 { 0 } else { ratio };
    }
    /// C++ `Player::GetQuestLevel`.
    pub(crate) fn player_quest_level_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> i32 {
        wow_progression::effective_quest_level(
            &quest.reward_rules(),
            || self.player_level_like_cpp(),
        )
    }
    /// Calculate XP reward for a quest.
    /// C++ `Quest::XPValue(player, questLevel, xpDifficulty, xpMultiplier)`.
    pub(crate) fn calculate_quest_xp(
        &self,
        difficulty: u32,
        quest_level: i32,
        xp_multiplier: f32,
    ) -> u32 {
        if let Some(store) = &self.quests.xp_store {
            // Preserve argument evaluation: Player level is sampled even when
            // difficulty is invalid; the domain then gates before row lookup.
            wow_progression::calculate_quest_xp(
                quest_level,
                self.player_level_like_cpp(),
                difficulty,
                xp_multiplier,
                self.quest_state.min_quest_scaled_xp_ratio_like_cpp,
                |level| store.get(level).map(|row| &row.difficulty),
            )
        } else {
            wow_progression::fallback_quest_xp(difficulty)
        }
    }
}
