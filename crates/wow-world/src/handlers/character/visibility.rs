// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter for the visibility bodies moved to
//! `wow_world_entities::visibility_session` (#1363 slice 8a). Every method is a
//! pure forward to the same-named inherent `WorldSession` method or field.

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
    fn record_represented_gameobject_db_phase_shift_like_cpp(
        &mut self,
        guid: ObjectGuid,
        map_id: u16,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
    ) {
        WorldSession::record_represented_gameobject_db_phase_shift_like_cpp(
            self,
            guid,
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        )
    }
    fn record_represented_gameobject_runtime_state_like_cpp(
        &mut self,
        map_id: u16,
        guid: ObjectGuid,
        entry: u32,
        position: Position,
        go_type: u8,
    ) {
        WorldSession::record_represented_gameobject_runtime_state_like_cpp(
            self, map_id, guid, entry, position, go_type,
        )
    }
    fn creature_create_stats_with_catalogs_like_cpp(
        &self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        entry: u32,
        level: u8,
        unit_class: u8,
        classification: u32,
        regen_health: bool,
        db_cur_health: u32,
        db_cur_mana: u32,
    ) -> wow_world_entities::CreatureCreateStatsLikeCpp {
        WorldSession::creature_create_stats_with_catalogs_like_cpp(
            self,
            catalogs,
            entry,
            level,
            unit_class,
            classification,
            regen_health,
            db_cur_health,
            db_cur_mana,
        )
    }
    fn creature_aggro_radius_for_faction_template_like_cpp(
        &self,
        faction_template_id: u32,
        default_radius: f32,
    ) -> f32 {
        WorldSession::creature_aggro_radius_for_faction_template_like_cpp(
            self,
            faction_template_id,
            default_radius,
        )
    }
    fn player_map_visibility_range_like_cpp(&self, map_id: u16) -> f32 {
        WorldSession::player_map_visibility_range_like_cpp(self, map_id)
    }
    fn sync_represented_farsight_clear_from_canonical_like_cpp(&mut self) -> bool {
        WorldSession::sync_represented_farsight_clear_from_canonical_like_cpp(self)
    }
    fn represented_visibility_source_position_like_cpp(&self) -> Option<Position> {
        WorldSession::represented_visibility_source_position_like_cpp(self)
    }
    fn visible_world_creatures_from_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
    ) -> Vec<wow_entities::Creature> {
        WorldSession::visible_world_creatures_from_map_like_cpp(self, map_id, position)
    }
    fn visible_gameobjects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::GameObjectCreateData>> {
        WorldSession::visible_gameobjects_from_canonical_map_like_cpp(
            self,
            map_id,
            position,
            visibility_radius,
        )
    }
    fn visible_dynamic_objects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::DynamicObjectCreateData>> {
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
    ) -> Option<Vec<wow_packet::packets::update::AreaTriggerCreateData>> {
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
        Vec<wow_packet::packets::update::CorpseCreateData>,
        Vec<wow_packet::packets::update::SceneObjectCreateData>,
        Vec<wow_packet::packets::update::ConversationCreateData>,
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
    ) -> Option<Vec<wow_packet::packets::update::GameObjectCreateData>> {
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
