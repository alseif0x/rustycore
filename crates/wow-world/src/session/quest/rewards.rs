//! Represented quest completion, turn-in and reward choice.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_game_event_quest_complete_sender_like_cpp(
        &mut self,
        sender: flume::Sender<GameEventQuestCompleteCommandLikeCpp>,
    ) {
        self.core.directory.game_event_quest_complete_tx = Some(sender);
    }
    /// Set the QuestFactionReward store used by C++ quest reputation reward lookup.
    pub fn set_quest_faction_reward_store(&mut self, store: Arc<QuestFactionRewardStore>) {
        self.catalogs.quests.faction_reward_store = Some(store);
    }
    #[cfg(test)]
    pub(in crate::session) fn represented_quest_rewarded_talent_points_like_cpp(
        &self,
    ) -> Option<u32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.gameplay_state().quest_rewarded_talent_points
        });
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.quest_state
                    .fixture_represented_quest_reward_talent_points_like_cpp()
                    .iter()
                    .map(|reward| reward.points)
                    .sum(),
            );
        }
        canonical
    }
    /// Set the QuestMoneyReward store (loaded from QuestMoneyReward.db2).
    pub fn set_quest_money_reward_store(&mut self, store: Arc<QuestMoneyRewardStore>) {
        self.catalogs.quests.money_reward_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn represented_confirm_barbers_choice_requests_like_cpp(
        &self,
    ) -> &[RepresentedConfirmBarbersChoiceLikeCpp] {
        &self
            .fixtures
            .presentation
            .represented_confirm_barbers_choice_requests_like_cpp
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/quest/rewards/f3_shims.rs"]
mod f3_shims;
