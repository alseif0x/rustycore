//! World-side adapter for the quest bodies that moved to `wow_world_quest`
//! (#1363 slice 6a). Each method forwards to the same-named session field,
//! split or inherent operation; `world_test_consumer_like_cpp` is this
//! crate's `cfg!(test)`.

use std::collections::HashMap;

use wow_world_core::session::{HubMut, HubRef, RepresentedGameObjectAccessLikeCpp};
use wow_world_entities::WorldEntitiesState;
use wow_world_lifecycle::SessionLifecycleState;
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
    fn world_test_consumer_like_cpp(&self) -> bool {
        cfg!(test)
    }
    fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState> {
        WorldSession::player_quest_gameplay_snapshot_like_cpp(self)
    }
    fn mutate_player_quest_gameplay_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut PlayerQuestGameplayState) -> R,
    ) -> Option<R> {
        WorldSession::mutate_player_quest_gameplay_like_cpp(self, mutate)
    }
    fn invalidate_player_quest_status_authority_like_cpp(&mut self) {
        WorldSession::invalidate_player_quest_status_authority_like_cpp(self)
    }
    fn can_take_quest(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        WorldSession::can_take_quest(self, quest)
    }
    fn represented_quest_is_important_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        WorldSession::represented_quest_is_important_like_cpp(self, quest)
    }
    fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        WorldSession::represented_inventory_item_counts_like_cpp(self)
    }
    fn represented_gameobject_questgiver_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        crate::session::cx_quest_state_ref(self)
            .represented_gameobject_questgiver_can_interact_with_like_cpp(guid)
    }
}
