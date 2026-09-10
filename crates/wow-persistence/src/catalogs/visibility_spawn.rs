//! SQLx-free source contract for Rust's transitional visibility spawn reads.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisibilitySpawnCatalogRequestLikeCpp {
    pub map_id: u16,
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureVisibilityPersistenceRowLikeCpp {
    pub spawn_guid: u64,
    pub entry: u32,
    pub position: [f32; 4],
    pub current_health: u32,
    pub current_mana: u32,
    pub model_id: u32,
    pub min_level: u8,
    pub faction: i32,
    pub template_npc_flags: u64,
    pub template_unit_flags: [u32; 3],
    pub speed_walk: f32,
    pub speed_run: f32,
    pub scale: f32,
    pub unit_class: u8,
    pub flags_extra: u32,
    pub attack_time: [u32; 2],
    pub template_display_id: u32,
    pub template_display_scale: f32,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold: [u32; 2],
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub terrain_swap_map: i32,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub wander_distance: f32,
    pub effective_movement_type: u8,
    pub waypoint_path_id: u32,
    pub classification: u32,
    pub regen_health: bool,
    pub spawn_npc_flags_override: Option<u64>,
    pub spawn_unit_flags_override: [Option<u32>; 3],
    pub equipment_id: i16,
    pub respawn_delay_secs: u32,
    pub spawn_difficulties: String,
    pub script_name: String,
    pub string_id: Option<String>,
    pub vehicle_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectVisibilityPersistenceRowLikeCpp {
    pub spawn_guid: u64,
    pub entry: u32,
    pub position: [f32; 4],
    pub rotation: [f32; 4],
    pub anim_progress: u8,
    pub state: i8,
    pub go_type: u8,
    pub display_id: u32,
    pub scale: f32,
    pub template_data: [i32; 35],
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub terrain_swap_map: i32,
    pub effective_flags: u32,
    pub effective_faction: u32,
    pub override_source_known: bool,
    pub parent_rotation: [f32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub enum VisibilitySpawnCatalogOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

pub trait VisibilitySpawnCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_creatures_in_bounds_like_cpp(
        &self,
        request: VisibilitySpawnCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VisibilitySpawnCatalogOutcomeLikeCpp<CreatureVisibilityPersistenceRowLikeCpp>,
    >;

    fn load_gameobjects_in_bounds_like_cpp(
        &self,
        request: VisibilitySpawnCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VisibilitySpawnCatalogOutcomeLikeCpp<GameObjectVisibilityPersistenceRowLikeCpp>,
    >;
}
