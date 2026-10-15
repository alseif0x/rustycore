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
//! visible-object queries live here too (gameobjects and creatures since
//! slice 10a, the other canonical-map objects since slice 11a); the
//! registry-backed other players, the viewer-dependent NPC and gameobject
//! flags, the far-sight sync and the quest-state refresh request stay with
//! the host.

use wow_core::{ObjectGuid, Position};
use wow_world_core::session::directory::PlayerVisibilityCreateSnapshot;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_visibility::VisibilityState;

use crate::RepresentedGameObjectUseState;
use crate::world_entities_host::WorldEntitiesHostLikeCpp;

mod canonical_objects;
mod dynamic_object_values;
mod nearby;
mod queries;
mod spawn;
mod update;

pub use canonical_objects::{
    dynamic_object_create_data_from_canonical_like_cpp,
    visible_dynamic_objects_from_canonical_map_like_cpp,
};
pub use dynamic_object_values::send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp;
pub use nearby::{send_nearby_creatures_with_catalogs_like_cpp, send_nearby_gameobjects};
pub use queries::{
    represented_gameobject_go_state_for_viewer_like_cpp,
    represented_visibility_source_combat_reach_like_cpp,
    visible_creatures_from_canonical_map_like_cpp, visible_gameobjects_from_canonical_map_like_cpp,
    visible_world_creatures_from_map_like_cpp,
};
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
    fn sync_represented_farsight_clear_from_canonical_like_cpp(&mut self) -> bool;
    fn represented_visibility_source_position_like_cpp(&self) -> Option<Position>;
    /// The World's `current_seer_guid_like_cpp` (`Player::m_seer` as a GUID).
    fn current_seer_guid_like_cpp(&self) -> Option<ObjectGuid>;
    fn visible_other_players_from_registry_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Vec<(ObjectGuid, PlayerVisibilityCreateSnapshot)>;
}
