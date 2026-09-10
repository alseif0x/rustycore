//! SQLx-free staged source contracts for canonical spawn startup metadata.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureSpawnPersistenceRowLikeCpp {
    pub spawn_id: u64,
    pub entry: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
    pub model_id: u32,
    pub equipment_id: i8,
    pub spawn_time_secs: i32,
    pub wander_distance: f32,
    pub curhealth: u32,
    pub curmana: u32,
    pub movement_type: u8,
    pub npc_flags: Option<u64>,
    pub unit_flags: Option<u32>,
    pub unit_flags2: Option<u32>,
    pub unit_flags3: Option<u32>,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub spawn_difficulties: String,
    pub event_entry: i16,
    pub pool_id: u32,
    pub phase_use_flags: u8,
    pub phase_id: u32,
    pub phase_group: u32,
    pub terrain_swap_map: i32,
    pub script_name: String,
    pub string_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaypointPathPersistenceRowLikeCpp {
    pub path_id: u32,
    pub move_type: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaypointPathNodePersistenceRowLikeCpp {
    pub path_id: u32,
    pub node_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: Option<f32>,
    pub delay: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaypointPathCatalogLikeCpp {
    pub paths: Vec<WaypointPathPersistenceRowLikeCpp>,
    pub nodes: Vec<WaypointPathNodePersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureFormationPersistenceRowLikeCpp {
    pub leader_spawn_id: u64,
    pub member_spawn_id: u64,
    pub dist: f32,
    pub angle_degrees: f32,
    pub group_ai: u32,
    pub point_1: u32,
    pub point_2: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectSpawnPersistenceRowLikeCpp {
    pub spawn_id: u64,
    pub entry: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
    pub rotation: [f32; 4],
    pub spawn_time_secs: i32,
    pub anim_progress: u8,
    pub state: u8,
    pub spawn_difficulties: String,
    pub event_entry: i16,
    pub pool_id: u32,
    pub phase_use_flags: u8,
    pub phase_id: u32,
    pub phase_group: u32,
    pub terrain_swap_map: i32,
    pub script_name: String,
    pub string_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerSpawnPersistenceRowLikeCpp {
    pub spawn_id: u64,
    pub create_properties_id: u32,
    pub is_custom: bool,
    pub map_id: u32,
    pub spawn_difficulties: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
    pub phase_use_flags: u8,
    pub phase_id: u32,
    pub phase_group: u32,
    pub spell_for_visuals: Option<i32>,
    pub script_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedRespawnPersistenceRowLikeCpp {
    pub guid: u64,
    pub linked_guid: u64,
    pub link_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolTemplatePersistenceRowLikeCpp {
    pub entry: u32,
    pub max_limit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolMemberPersistenceRowLikeCpp {
    pub spawn_id: u64,
    pub pool_spawn_id: u32,
    pub chance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolMemberKindPersistenceLikeCpp {
    Creature,
    GameObject,
    Pool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolAutospawnCandidatePersistenceRowLikeCpp {
    pub pool_entry: u32,
    pub child_pool_id: u64,
    pub mother_pool_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnGroupMemberPersistenceRowLikeCpp {
    pub group_id: u32,
    pub spawn_type: u8,
    pub spawn_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalSpawnCatalogLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// Ordered World reads whose domain application remains in `world-server`.
pub trait CanonicalSpawnCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_creature_spawns_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<CreatureSpawnPersistenceRowLikeCpp>>,
    >;
    fn load_waypoint_paths_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<WaypointPathCatalogLikeCpp>,
    >;
    fn load_creature_formations_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<CreatureFormationPersistenceRowLikeCpp>>,
    >;
    fn load_gameobject_spawns_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<GameObjectSpawnPersistenceRowLikeCpp>>,
    >;
    fn load_area_trigger_spawns_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<AreaTriggerSpawnPersistenceRowLikeCpp>>,
    >;
    fn load_linked_respawns_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<LinkedRespawnPersistenceRowLikeCpp>>,
    >;
    fn load_pool_templates_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<PoolTemplatePersistenceRowLikeCpp>>,
    >;
    fn load_pool_members_like_cpp(
        &self,
        kind: PoolMemberKindPersistenceLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<PoolMemberPersistenceRowLikeCpp>>,
    >;
    fn load_pool_autospawn_candidates_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<PoolAutospawnCandidatePersistenceRowLikeCpp>>,
    >;
    fn load_spawn_group_members_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<SpawnGroupMemberPersistenceRowLikeCpp>>,
    >;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateTemplatePersistenceRowLikeCpp {
    pub id: i32,
    pub default_value: i32,
    pub map_ids_csv: String,
    pub area_ids_csv: String,
    pub script_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldStateSavedValuePersistenceRowLikeCpp {
    pub id: i32,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateStartupCatalogLikeCpp {
    pub templates: Vec<WorldStateTemplatePersistenceRowLikeCpp>,
    pub saved_values: Vec<WorldStateSavedValuePersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldStateStartupLoadOutcomeLikeCpp {
    Loaded(WorldStateStartupCatalogLikeCpp),
    Failed { reason: String },
}

/// World templates followed by the Characters saved-value overlay.
pub trait WorldStateStartupPersistencePortLikeCpp: Send + Sync {
    fn load_world_then_character_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, WorldStateStartupLoadOutcomeLikeCpp>;
}
