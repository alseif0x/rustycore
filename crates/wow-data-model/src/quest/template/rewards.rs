//! Scalar projection for quest reward valuation; the catalog owns the full row.

use super::{QuestRewardRules, QuestTemplate};

impl QuestTemplate {
    pub fn reward_rules(&self) -> QuestRewardRules {
        QuestRewardRules::new(
            self.quest_level,
            self.quest_max_scaling_level,
            self.is_df_quest_like_cpp(),
            self.reward_xp_difficulty,
            self.reward_xp_multiplier,
            self.reward_money_difficulty,
            self.reward_money_multiplier,
        )
    }
}
