// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_skill_updates_like_cpp(&self) -> &[(u32, u32)] {
        self.quest_state
            .represented_quest_reward_skill_updates_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardSpellCastLikeCpp] {
        self.quest_state
            .represented_quest_reward_spell_casts_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_titles_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardTitleLikeCpp] {
        self.quest_state.represented_quest_reward_titles_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_talent_points_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardTalentPointsLikeCpp] {
        self.quest_state
            .represented_quest_reward_talent_points_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_mails_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardMailLikeCpp] {
        self.quest_state.represented_quest_reward_mails_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_reputations_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardReputationLikeCpp] {
        self.quest_state
            .represented_quest_reward_reputations_like_cpp()
    }
}
