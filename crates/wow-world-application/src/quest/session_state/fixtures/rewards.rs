// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private quest reward evidence fixture operations for quest state.

use super::super::{contracts, SessionQuestState};

impl SessionQuestState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_timed_quest_removals_like_cpp(&self) -> &[u32] {
        &self.fixtures.represented_timed_quest_removals_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_reward_skill_updates_like_cpp(&self) -> &[(u32, u32)] {
        &self.fixtures.represented_quest_reward_skill_updates_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_reward_skill_update_like_cpp(&mut self, skill: u32, value: u32) {
        self.fixtures
            .represented_quest_reward_skill_updates_like_cpp
            .push((skill, value));
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_reward_spell_casts_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestRewardSpellCastLikeCpp] {
        &self.fixtures.represented_quest_reward_spell_casts_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_reward_spell_cast_like_cpp(
        &mut self,
        cast: contracts::RepresentedQuestRewardSpellCastLikeCpp,
    ) {
        self.fixtures
            .represented_quest_reward_spell_casts_like_cpp
            .push(cast);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_reward_titles_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestRewardTitleLikeCpp] {
        &self.fixtures.represented_quest_reward_titles_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_reward_title_like_cpp(
        &mut self,
        title: contracts::RepresentedQuestRewardTitleLikeCpp,
    ) {
        self.fixtures.represented_quest_reward_titles_like_cpp.push(title);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_reward_talent_points_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestRewardTalentPointsLikeCpp] {
        &self.fixtures.represented_quest_reward_talent_points_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_reward_talent_points_like_cpp(
        &mut self,
        points: contracts::RepresentedQuestRewardTalentPointsLikeCpp,
    ) {
        self.fixtures
            .represented_quest_reward_talent_points_like_cpp
            .push(points);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_reward_mails_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestRewardMailLikeCpp] {
        &self.fixtures.represented_quest_reward_mails_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_reward_mail_like_cpp(
        &mut self,
        mail: contracts::RepresentedQuestRewardMailLikeCpp,
    ) {
        self.fixtures.represented_quest_reward_mails_like_cpp.push(mail);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_represented_quest_reward_reputations_like_cpp(
        &self,
    ) -> &[contracts::RepresentedQuestRewardReputationLikeCpp] {
        &self.fixtures.represented_quest_reward_reputations_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_record_quest_reward_reputation_like_cpp(
        &mut self,
        reputation: contracts::RepresentedQuestRewardReputationLikeCpp,
    ) {
        self.fixtures
            .represented_quest_reward_reputations_like_cpp
            .push(reputation);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_timed_quest_removals_like_cpp(&self) -> &[u32] {
        self.fixture_represented_timed_quest_removals_like_cpp()
    }
}

