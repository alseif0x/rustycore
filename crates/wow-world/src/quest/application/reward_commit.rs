// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::WorldSession;
use super::QuestRewardDurablePlanLikeCpp;

impl WorldSession {
    pub(crate) async fn commit_quest_reward_plan_like_cpp(
        &mut self,
        plan: QuestRewardDurablePlanLikeCpp,
        quest_id: u32,
    ) -> Option<Option<wow_persistence::PlayerQuestRewardMoneyLikeCpp>> {
        let player = self.core.quest_reward_player_access_like_cpp();
        let mut operation = wow_world_application::QuestRewardCommitCx::new(
            &mut self.inventory,
            &mut self.lifecycle,
            &mut self.quest_state,
            player,
            self.catalogs.currency_types_store.as_deref(),
            cfg!(test),
        );
        operation.commit_like_cpp(plan, quest_id).await
    }
}
