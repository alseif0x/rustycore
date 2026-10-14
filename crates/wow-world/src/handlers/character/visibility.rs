// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter for the visibility bodies moved to
//! `wow_world_entities::visibility_session` (#1363 slice 8a). Every method is a
//! pure forward to the same-named inherent `WorldSession` method or field.

use wow_packet::packets::update::{
    AreaTriggerCreateData, ConversationCreateData, CorpseCreateData, DynamicObjectCreateData,
    GameObjectCreateData, SceneObjectCreateData,
};
use wow_world_core::session::directory::PlayerVisibilityCreateSnapshot;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_visibility::VisibilityState;

use super::*;

impl wow_world_entities::VisibilitySessionHostLikeCpp for WorldSession {
    fn lifecycle(&self) -> &SessionLifecycleState {
        &self.lifecycle
    }
    fn visibility(&self) -> &VisibilityState {
        &self.visibility
    }
    fn visibility_mut(&mut self) -> &mut VisibilityState {
        &mut self.visibility
    }
    fn consume_movement_visibility_refresh_request_like_cpp(&mut self) -> bool {
        self.quest_state
            .consume_movement_visibility_refresh_request_like_cpp()
    }
    fn represented_viewer_dependent_creature_npc_flags_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        npc_flags: u64,
    ) -> u64 {
        WorldSession::represented_viewer_dependent_creature_npc_flags_like_cpp(
            self,
            creature_guid,
            npc_flags,
        )
    }
    fn represented_gameobject_dynamic_flags_for_player_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32 {
        WorldSession::represented_gameobject_dynamic_flags_for_player_like_cpp(
            self,
            gameobject_entry,
            state,
        )
    }
    fn sync_represented_farsight_clear_from_canonical_like_cpp(&mut self) -> bool {
        WorldSession::sync_represented_farsight_clear_from_canonical_like_cpp(self)
    }
    fn represented_visibility_source_position_like_cpp(&self) -> Option<Position> {
        WorldSession::represented_visibility_source_position_like_cpp(self)
    }
    fn current_seer_guid_like_cpp(&self) -> Option<ObjectGuid> {
        WorldSession::current_seer_guid_like_cpp(self)
    }
    fn visible_dynamic_objects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<Vec<DynamicObjectCreateData>> {
        WorldSession::visible_dynamic_objects_from_canonical_map_like_cpp(
            self,
            map_id,
            position,
            visibility_radius,
        )
    }
    fn visible_area_triggers_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<Vec<AreaTriggerCreateData>> {
        WorldSession::visible_area_triggers_from_canonical_map_like_cpp(
            self,
            map_id,
            position,
            visibility_radius,
        )
    }
    fn visible_misc_objects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<(
        Vec<CorpseCreateData>,
        Vec<SceneObjectCreateData>,
        Vec<ConversationCreateData>,
    )> {
        WorldSession::visible_misc_objects_from_canonical_map_like_cpp(
            self,
            map_id,
            position,
            visibility_radius,
        )
    }
    fn visible_transports_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
    ) -> Option<Vec<GameObjectCreateData>> {
        WorldSession::visible_transports_from_canonical_map_like_cpp(self, map_id)
    }
    fn visible_other_players_from_registry_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Vec<(ObjectGuid, PlayerVisibilityCreateSnapshot)> {
        WorldSession::visible_other_players_from_registry_like_cpp(
            self,
            map_id,
            position,
            visibility_radius,
        )
    }
}
