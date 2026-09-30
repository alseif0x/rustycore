//! publication operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(crate) fn send_represented_quest_giver_offer_reward_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        quest: &wow_data::quest::QuestTemplate,
        auto_launched: bool,
    ) {
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: source_guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(source_guid),
            quest_id: quest.id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards: quest_rewards_block_like_cpp(quest),
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched,
        });
    }
    #[cfg(test)]
    pub(crate) fn represented_confirm_barbers_choice_requests_like_cpp(
        &self,
    ) -> &[RepresentedConfirmBarbersChoiceLikeCpp] {
        &self.represented_confirm_barbers_choice_requests_like_cpp
    }
    pub(crate) fn set_represented_daily_quest_completed_like_cpp_for_test(
        &mut self,
        quest_id: u32,
        completed: bool,
    ) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            if completed {
                state.set_daily_like_cpp(quest_id, true);
            } else {
                state.set_daily_like_cpp(quest_id, false);
            }
        });
        self.sync_player_registry_state_like_cpp();
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_reward_skill_updates_like_cpp(&self) -> &[(u32, u32)] {
        &self
            .quest_test_fixture_like_cpp
            .represented_quest_reward_skill_updates_like_cpp
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_reward_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardSpellCastLikeCpp] {
        &self
            .quest_test_fixture_like_cpp
            .represented_quest_reward_spell_casts_like_cpp
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_reward_titles_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardTitleLikeCpp] {
        &self
            .quest_test_fixture_like_cpp
            .represented_quest_reward_titles_like_cpp
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_reward_talent_points_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardTalentPointsLikeCpp] {
        &self
            .quest_test_fixture_like_cpp
            .represented_quest_reward_talent_points_like_cpp
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_reward_mails_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardMailLikeCpp] {
        &self
            .quest_test_fixture_like_cpp
            .represented_quest_reward_mails_like_cpp
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_reward_reputations_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardReputationLikeCpp] {
        &self
            .quest_test_fixture_like_cpp
            .represented_quest_reward_reputations_like_cpp
    }
    pub(crate) fn represented_quest_complete_status_updates_like_cpp(
        &self,
    ) -> &[RepresentedQuestCompleteStatusUpdateLikeCpp] {
        &self
            .quest_state
            .represented_quest_complete_status_updates_like_cpp
    }
    pub(crate) fn record_represented_quest_complete_status_update_like_cpp(
        &mut self,
        evidence: RepresentedQuestCompleteStatusUpdateLikeCpp,
    ) {
        self.quest_state
            .represented_quest_complete_status_updates_like_cpp
            .push(evidence);
    }
}
