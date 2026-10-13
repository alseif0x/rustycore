// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player visibility at the session boundary (#1363 slice 8a): the nearby
//! creature and gameobject create paths on world entry and transfer, the
//! movement visibility update and the creature spawn-row materialization.
//!
//! C++ `Player::UpdateVisibilityForPlayer`, `Player::UpdateVisibilityOf` and
//! the `ObjectMgr::LoadCreatures`/`Creature::LoadFromDB` spawn path run on the
//! Player and its map. Their Rust bodies now live in this crate; each host
//! method below is the same-named `WorldSession` operation (or field), called
//! at the exact point the body called it before the move. The canonical
//! visible-object queries, the viewer-dependent NPC and gameobject flags, the
//! far-sight sync and the quest-state refresh request stay with the host.

use wow_core::{ObjectGuid, Position};
use wow_world_core::session::directory::PlayerVisibilityCreateSnapshot;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_visibility::VisibilityState;

use crate::world_entities_host::WorldEntitiesHostLikeCpp;
use crate::{
    CreatureCreateStatsLikeCpp, CreatureSpawnCatalogsLikeCpp, RepresentedGameObjectUseState,
};

mod nearby;
mod spawn;
mod update;

pub use nearby::{send_nearby_creatures_with_catalogs_like_cpp, send_nearby_gameobjects};
#[cfg(any(test, feature = "test-fixtures"))]
pub use spawn::{
    WAYPOINT_MOTION_TYPE_LIKE_CPP, choose_creature_flags_like_cpp,
    creature_create_movement_flags_like_cpp, creature_create_position_after_hover_offset_like_cpp,
    creature_movement_generator_type_from_db_like_cpp,
    normalize_creature_template_speed_run_like_cpp,
    normalize_creature_template_speed_walk_like_cpp, normalized_creature_wander_distance_like_cpp,
    spawn_difficulties_contains_spawn_mode_like_cpp,
};
pub use update::update_visibility_with_catalogs_like_cpp;

pub trait VisibilitySessionHostLikeCpp: WorldEntitiesHostLikeCpp {
    fn lifecycle(&self) -> &SessionLifecycleState;
    fn visibility(&self) -> &VisibilityState;
    fn visibility_mut(&mut self) -> &mut VisibilityState;
    /// The quest state's `consume_movement_visibility_refresh_request_like_cpp`.
    fn consume_movement_visibility_refresh_request_like_cpp(&mut self) -> bool;
    fn represented_viewer_dependent_creature_npc_flags_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        npc_flags: u64,
    ) -> u64;
    fn represented_gameobject_dynamic_flags_for_player_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32;
    fn record_represented_gameobject_db_phase_shift_like_cpp(
        &mut self,
        guid: ObjectGuid,
        map_id: u16,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
    );
    fn record_represented_gameobject_runtime_state_like_cpp(
        &mut self,
        map_id: u16,
        guid: ObjectGuid,
        entry: u32,
        position: Position,
        go_type: u8,
    );
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
    ) -> CreatureCreateStatsLikeCpp;
    fn creature_aggro_radius_for_faction_template_like_cpp(
        &self,
        faction_template_id: u32,
        default_radius: f32,
    ) -> f32;
    fn player_map_visibility_range_like_cpp(&self, map_id: u16) -> f32;
    fn sync_represented_farsight_clear_from_canonical_like_cpp(&mut self) -> bool;
    fn represented_visibility_source_position_like_cpp(&self) -> Option<Position>;
    fn visible_world_creatures_from_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
    ) -> Vec<wow_entities::Creature>;
    fn visible_gameobjects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::GameObjectCreateData>>;
    fn visible_dynamic_objects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::DynamicObjectCreateData>>;
    fn visible_area_triggers_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::AreaTriggerCreateData>>;
    fn visible_misc_objects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Option<(
        Vec<wow_packet::packets::update::CorpseCreateData>,
        Vec<wow_packet::packets::update::SceneObjectCreateData>,
        Vec<wow_packet::packets::update::ConversationCreateData>,
    )>;
    fn visible_transports_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
    ) -> Option<Vec<wow_packet::packets::update::GameObjectCreateData>>;
    fn visible_other_players_from_registry_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Vec<(ObjectGuid, PlayerVisibilityCreateSnapshot)>;
}
