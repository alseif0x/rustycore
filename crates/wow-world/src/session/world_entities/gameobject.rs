//! Represented gameobject state owned by the Session boundary.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_gameobject_dynamic_flags_for_player_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32 {
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
        let eligibility = wow_world_quest::QuestEligibilityCx::new(
            player,
            &self.quest_state,
            &self.catalogs,
            &conditions,
            cfg!(test),
        );
        let player_is_game_master =
            crate::session::hub_ref(self).player_is_game_master_like_cpp() == Some(true);
        wow_world_quest::represented_gameobject_dynamic_flags_for_player_like_cpp(
            &owner,
            &self.catalogs,
            &self.quest_state,
            &self.inventory,
            &inventory_access,
            &eligibility,
            self.player_guid(),
            player_is_game_master,
            gameobject_entry,
            state,
            cfg!(test),
        )
    }
    pub(crate) async fn kill_credit_gameobject_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        gameobject_entry: u32,
        gameobject_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_GAMEOBJECT.
        self.quest_objective_operation_like_cpp(
            item_guid_generator,
            crate::session::quest::QuestObjectiveOperationLikeCpp::StoringValue {
                objective_type: 2,
                object_id: gameobject_entry as i32,
                add_count: 1,
                credit_guid: gameobject_guid,
            },
        )
        .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/gameobject/f3_shims.rs"]
mod f3_shims;
