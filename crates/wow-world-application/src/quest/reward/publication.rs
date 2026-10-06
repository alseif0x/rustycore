// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Registry publication at the closing quest-reward phase.

use super::QuestRewardCx;

impl QuestRewardCx<'_> {
    pub(super) fn send_quest_reward_log_slot_update_like_cpp(&self, slot: u8) {
        let owner = self.player.quest_objective_access_like_cpp();
        let publication = self.player.packet_publication_access_like_cpp();
        super::super::quest_log::send_represented_quest_log_slot_update_like_cpp(
            &owner,
            self.quest_state,
            self.catalogs,
            &publication,
            slot,
            self.world_test_consumer,
        );
    }

    pub(super) fn sync_quest_reward_registry_like_cpp(
        &self,
        loot: &wow_world_loot::LootState,
        #[cfg(any(test, feature = "test-fixtures"))]
        planning: &super::item_planning::QuestRewardItemPlanningFixtureRefsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] vitals: (&u32, &u32, &bool),
        #[cfg(any(test, feature = "test-fixtures"))] transport: &Option<
            Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>,
        >,
        #[cfg(any(test, feature = "test-fixtures"))] vehicle_and_pet: (
            &Option<wow_entities::Vehicle>,
            &Option<i32>,
            &Option<u32>,
            &Option<wow_core::ObjectGuid>,
        ),
    ) {
        let owner = self.player.quest_objective_access_like_cpp();
        super::super::completion::sync_quest_completion_registry_like_cpp(
            &owner,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            planning.registry_spell_state_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            vehicle_and_pet,
            #[cfg(any(test, feature = "test-fixtures"))]
            planning.registry_position_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            vitals.0,
            #[cfg(any(test, feature = "test-fixtures"))]
            vitals.1,
            #[cfg(any(test, feature = "test-fixtures"))]
            vitals.2,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            transport,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.world_test_consumer,
        );
    }
}
