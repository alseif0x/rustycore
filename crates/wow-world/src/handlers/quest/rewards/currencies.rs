// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World construction seam for the App-owned quest currency operation.

use super::*;

impl WorldSession {
    pub(super) async fn grant_quest_reward_currencies_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let player = self.core.quest_reward_player_access_like_cpp(
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let player = self.core.quest_reward_player_access_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut operation = wow_world_application::QuestRewardCx::new(
            &mut self.inventory,
            &mut self.lifecycle,
            &mut self.quest_state,
            player,
            &mut self.fixtures.identity.player_level,
            &self.catalogs,
            &self.config,
            &self.social,
            self.catalogs.currency_types_store.as_deref(),
            self.catalogs.quests.xp_store.as_deref(),
            cfg!(test),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let mut operation = wow_world_application::QuestRewardCx::new(
            &mut self.inventory,
            &mut self.lifecycle,
            &mut self.quest_state,
            player,
            &self.catalogs,
            &self.config,
            &self.social,
            self.catalogs.currency_types_store.as_deref(),
            self.catalogs.quests.xp_store.as_deref(),
            cfg!(test),
        );
        operation
            .grant_quest_reward_currencies_like_cpp(
                quest,
                choice.item_id,
                choice.loot_item_type,
                QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
            )
            .await
    }
}
