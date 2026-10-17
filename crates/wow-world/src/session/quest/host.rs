//! World-side adapter for the quest bodies that moved to `wow_world_quest`
//! (#1363 slices 6a and 6b). Each method forwards to the same-named session
//! field, split or inherent operation.

use std::collections::HashMap;

use wow_world_core::session::{HubMut, HubRef, RepresentedGameObjectAccessLikeCpp};
use wow_world_entities::WorldEntitiesState;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_player_conditions::PlayerConditionProjectionCxLikeCpp;
use wow_world_quest::{QuestSessionHostLikeCpp, SessionQuestState};

use super::*;

impl QuestSessionHostLikeCpp for WorldSession {
    fn quest_state(&self) -> &SessionQuestState {
        &self.quest_state
    }
    fn split_quest_state_ref(&self) -> (&SessionQuestState, HubRef<'_>) {
        crate::session::split_quest_state_ref(self)
    }
    fn split_quest_state_mut(&mut self) -> (&mut SessionQuestState, HubMut<'_>) {
        crate::session::split_quest_state_mut(self)
    }
    fn lifecycle(&self) -> &SessionLifecycleState {
        &self.lifecycle
    }
    fn world_entities_mut(&mut self) -> &mut WorldEntitiesState {
        &mut self.world_entities
    }
    fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState> {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        wow_world_quest::player_quest_gameplay_snapshot_like_cpp(hub, state)
    }
    fn mutate_player_quest_gameplay_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut PlayerQuestGameplayState) -> R,
    ) -> Option<R> {
        let (state, hub) = crate::session::split_quest_state_mut(self);
        wow_world_quest::mutate_player_quest_gameplay_like_cpp(hub, state, mutate)
    }
    fn represented_can_see_spell_click_on_creature_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> wow_world_entities::spell_click_values::RepresentedCanSeeSpellClickOutcomeLikeCpp {
        WorldSession::represented_can_see_spell_click_on_creature_like_cpp(self, creature_guid)
    }
    fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        self.inventory
            .represented_inventory_item_counts_like_cpp(crate::session::hub_ref(self))
    }
    fn represented_gameobject_questgiver_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        crate::session::cx_quest_state_ref(self)
            .represented_gameobject_questgiver_can_interact_with_like_cpp(guid)
    }
    fn complete_represented_quest_after_objective_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
    ) -> impl Future<Output = bool> + Send {
        self.quest_objective_operation_like_cpp(
            item_guid_generator,
            crate::session::quest::QuestObjectiveOperationLikeCpp::Complete {
                quest,
                ignored_objective_id,
            },
        )
    }
    fn sync_player_registry_state_like_cpp(&self) {
        WorldSession::sync_player_registry_state_like_cpp(self)
    }
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize {
        WorldSession::update_visible_gameobjects_or_spell_clicks_like_cpp(self)
    }
    fn send_represented_quest_log_slot_update_like_cpp(&mut self, slot: u8) {
        WorldSession::send_represented_quest_log_slot_update_like_cpp(self, slot)
    }
    fn player_condition_projection_cx_like_cpp(&self) -> PlayerConditionProjectionCxLikeCpp<'_> {
        WorldSession::player_condition_projection_cx_like_cpp(self)
    }
    fn player_target_spell_is_hit_inert_like_cpp(&self, spell_id: u32, difficulty_id: u8) -> bool {
        wow_world_spell::player_target_spell_is_hit_inert_like_cpp(self, spell_id, difficulty_id)
    }
}
