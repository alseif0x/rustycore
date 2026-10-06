// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest interaction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ObjectGuid, RepresentedGameObjectAccessLikeCpp, RepresentedGameObjectUseState};
use super::{WorldSession, quest};

impl WorldSession {
    /// World facade over the single App provider so query/refresh paths and the
    /// ActivateToQuest/DynamicFlags consumers share one authority.
    pub(in crate::session) fn represented_has_quest_for_gameobject_like_cpp(
        &self,
        gameobject_entry: u32,
    ) -> bool {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_application::represented_has_quest_for_gameobject_like_cpp(
            &owner,
            &self.catalogs,
            &self.quest_state,
            gameobject_entry,
            cfg!(test),
        )
    }

    pub(in crate::session) fn represented_gameobject_is_for_quests_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> bool {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_application::represented_gameobject_is_for_quests_like_cpp(
            &self.catalogs,
            &owner,
            &self.quest_state,
            gameobject_entry,
            state,
            cfg!(test),
        )
    }

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

impl crate::session::QuestStateCx<'_> {
    pub(crate) fn record_represented_gameobject_template_quest_source_like_cpp(
        &mut self,
        guid: ObjectGuid,
        template: &wow_entities::GameObjectTemplateData,
    ) {
        let state = self
            .world_entities
            .ensure_represented_gameobject_use_state_like_cpp(guid);
        if let Some(source) = template.chest_loot_source_like_cpp() {
            state.chest_loot_source = Some(source);
        }
        if let Some(source) = template.gathering_node_use_source_like_cpp() {
            state.gathering_node_loot_id = Some(source.loot_id);
        }
        state.condition_id1 = (template.get_condition_id1_like_cpp() != 0)
            .then_some(template.get_condition_id1_like_cpp());
    }
}

impl crate::session::QuestStateCxRef<'_> {
    pub(crate) fn represented_gameobject_questgiver_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        // C++ anchor: Player::CanInteractWithQuestGiver(TYPEID_GAMEOBJECT)
        // delegates to GetGameObjectIfCanInteractWith(guid, GAMEOBJECT_TYPE_QUESTGIVER).
        // This represented guard consumes canonical map access plus locally recorded
        // template type/radius from the GO-use path. The C++ IconName == "Point"
        // rejection is represented earlier in handle_game_obj_use before runtime state
        // is registered/consumed here; standalone paths without represented type state
        // fail closed instead of treating canonical existence as interactability.
        let access = self.hub.core.canonical_gameobject_access_like_cpp(guid)?;
        let state = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid)?;
        if state.go_type.map(u32::from) != Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) {
            return None;
        }
        let player_position = self.hub.player_position_like_cpp()?;
        let interaction_distance = state
            .interact_radius_override
            .filter(|value| *value != 0)
            .map_or(5.5555553, |override_hundredths| {
                override_hundredths as f32 / 100.0
            });
        access
            .position
            .is_within_dist(&player_position, interaction_distance)
            .then_some(access)
    }
}
