// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inert selected-owner construction for the complete App quest reward.

use super::*;

impl WorldSession {
    pub(in crate::handlers::quest) async fn reward_represented_quest_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut fixtures = crate::session::quest_reward_fixture_refs_like_cpp(
            &mut self.fixtures,
            &self.spell_state,
            &self.instances,
        );
        let player = self.core.quest_reward_player_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.race,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.class,
        );
        let mut operation = wow_world_application::QuestRewardCx::new(
            &mut self.inventory,
            &mut self.lifecycle,
            &mut self.quest_state,
            player,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.level,
            &self.catalogs,
            &self.config,
            &self.social,
            self.catalogs.currency_types_store.as_deref(),
            self.catalogs.quests.xp_store.as_deref(),
            cfg!(test),
        );
        operation
            .reward_quest_with_generator_like_cpp(
                item_guid_generator,
                quest,
                quest_giver_guid,
                choice.item_id,
                choice.loot_item_type,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
                #[cfg(any(test, feature = "test-fixtures"))]
                &mut fixtures.xp,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures.teleport,
                #[cfg(any(test, feature = "test-fixtures"))]
                &fixtures.planning,
                &self.loot,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures.transport,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures.vehicle_and_pet,
                #[cfg(any(test, feature = "test-fixtures"))]
                &mut fixtures.reputation,
            )
            .await
    }
}
