// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Existing durable reward commit bridge.

use super::super::QuestRewardDurablePlanLikeCpp;
use super::super::reward_commit::QuestRewardCommitCx;
use super::QuestRewardCx;

impl QuestRewardCx<'_> {
    pub async fn commit_quest_reward_plan_like_cpp(
        &mut self,
        plan: QuestRewardDurablePlanLikeCpp,
        quest_id: u32,
    ) -> Option<Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>> {
        let mut commit = QuestRewardCommitCx::new(
            self.inventory,
            self.lifecycle,
            self.quest_state,
            &mut self.player,
            self.currency_types,
            self.world_test_consumer,
        );
        commit.commit_like_cpp(plan, quest_id).await
    }
}
