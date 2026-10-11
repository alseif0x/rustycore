// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `quest_interaction` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_gameobject_activate_to_quest_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> bool {
        let owner = self.core.quest_objective_access_like_cpp();
        let inventory_access = self.core.owned_inventory_access_like_cpp();
        let player = self.core.quest_eligibility_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.reputation_state_like_cpp,
        );
        let conditions = self.player_condition_projection_cx_like_cpp();
        let eligibility = wow_world_application::QuestEligibilityCx::new(
            player,
            &self.quest_state,
            &self.catalogs,
            &conditions,
            cfg!(test),
        );
        wow_world_application::represented_gameobject_activate_to_quest_like_cpp(
            &owner,
            &self.catalogs,
            &self.quest_state,
            &self.inventory,
            &inventory_access,
            &eligibility,
            gameobject_entry,
            state,
            cfg!(test),
        )
    }
}
