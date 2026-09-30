//! valuation operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    /// Set the QuestFactionReward store used by C++ quest reputation reward lookup.
    pub fn set_quest_faction_reward_store(&mut self, store: Arc<QuestFactionRewardStore>) {
        self.quests.faction_reward_store = Some(store);
    }
    /// Set the QuestMoneyReward store (loaded from QuestMoneyReward.db2).
    pub fn set_quest_money_reward_store(&mut self, store: Arc<QuestMoneyRewardStore>) {
        self.quests.money_reward_store = Some(store);
    }
    /// C++ `Player::GetQuestXPReward`.
    pub(crate) fn quest_xp_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32 {
        let Some(already_rewarded) = self.represented_player_has_rewarded_quest_like_cpp(quest.id)
        else {
            return 0;
        };
        let rules = quest.reward_rules();
        if wow_progression::quest_xp_is_blocked(
            already_rewarded,
            rules.is_dungeon_finder(),
        ) {
            return 0;
        }

        self.calculate_quest_xp(
            rules.xp_difficulty(),
            self.player_quest_level_like_cpp(quest),
            rules.xp_multiplier(),
        )
    }
    /// C++ `Player::GetQuestMoneyReward` -> `Quest::MoneyValue`.
    pub(crate) fn quest_money_reward_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> u32 {
        let Some(store) = &self.quests.money_reward_store else {
            return 0;
        };
        let quest_level = self.player_quest_level_like_cpp(quest).max(0) as u32;
        let Some(row) = store.get(quest_level) else {
            return 0;
        };
        let rules = quest.reward_rules();
        wow_progression::quest_money_value(
            &row.difficulty,
            rules.money_difficulty(),
            rules.money_multiplier(),
        )
    }
}
