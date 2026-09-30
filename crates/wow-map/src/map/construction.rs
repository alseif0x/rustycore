use super::{
    Cell, Difficulty, EntityWorld, GridLifecycle, GridUnloadAction, LineOfSightQuery, Map,
    MapGuidSequenceGeneratorLikeCpp, MapWorldObjectEnvironment, NGrid,
    MultiPersonalPhaseTracker, Position, RespawnStoreLikeCpp, SpawnGroupRuntimeState,
    SpawnedPoolDataLikeCpp, TerrainGridLoader, WorldObject, WorldObjectHeightQuery,
    DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP, GRID_SLOT_COUNT, INVALID_HEIGHT,
    WEATHER_UPDATE_INTERVAL_MS_LIKE_CPP,
};
use rand::{SeedableRng, rngs::StdRng};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

pub trait TerrainGridLoader {
    fn load_map_and_vmap(&mut self, grid_x: u32, grid_y: u32);
    fn unload_map(&mut self, grid_x: u32, grid_y: u32);
}

/// Terrain/dynamic-tree hook used by `Map` when it acts as a
/// `WorldObjectEnvironment` for `WorldObject` helpers.
///
/// This is the explicit ownership seam for C++ `Map::isInLineOfSight`,
/// `Map::GetHeight`, and `Map::GetGameObjectFloor`. Implementations may be a
/// noop while real terrain/vmap/dynamic-tree runtime is not ported, but callers
/// must still flow through `WorldObject -> WorldObjectEnvironment -> Map -> terrain`.
pub trait MapWorldObjectEnvironment {
    fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool;

    fn map_height(
        &self,
        object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32;

    fn floor_z(&self, object: &WorldObject, position: Position, max_search_dist: f32) -> f32;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopTerrainGridLoader;

impl TerrainGridLoader for NoopTerrainGridLoader {
    fn load_map_and_vmap(&mut self, _grid_x: u32, _grid_y: u32) {}
    fn unload_map(&mut self, _grid_x: u32, _grid_y: u32) {}
}

impl MapWorldObjectEnvironment for NoopTerrainGridLoader {
    fn line_of_sight(&self, _query: LineOfSightQuery<'_>) -> bool {
        true
    }

    fn map_height(
        &self,
        _object: &WorldObject,
        _x: f32,
        _y: f32,
        _z: f32,
        _query: WorldObjectHeightQuery,
    ) -> f32 {
        INVALID_HEIGHT
    }

    fn floor_z(&self, _object: &WorldObject, _position: Position, _max_search_dist: f32) -> f32 {
        INVALID_HEIGHT
    }
}

pub trait GridLifecycle {
    fn load_grid_objects(&mut self, grid: &mut NGrid, cell: &Cell);
    fn stop_grid_objects(&mut self, grid: &NGrid);
    fn evacuate_grid(&mut self, grid: &mut NGrid);
    fn clean_grid(&mut self, grid: &mut NGrid);
    fn unload_grid_objects(&mut self, grid: &mut NGrid);
    fn take_unload_actions_like_cpp(&mut self) -> Vec<GridUnloadAction> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopGridLifecycle;

impl GridLifecycle for NoopGridLifecycle {
    fn load_grid_objects(&mut self, _grid: &mut NGrid, _cell: &Cell) {}
    fn stop_grid_objects(&mut self, _grid: &NGrid) {}
    fn evacuate_grid(&mut self, _grid: &mut NGrid) {}
    fn clean_grid(&mut self, _grid: &mut NGrid) {}
    fn unload_grid_objects(&mut self, _grid: &mut NGrid) {}
}

impl Map<NoopTerrainGridLoader, NoopGridLifecycle> {
    pub fn new(map_id: u32, instance_id: u32, spawn_mode: Difficulty, grid_expiry_ms: i64) -> Self {
        Self::with_hooks(
            map_id,
            instance_id,
            spawn_mode,
            grid_expiry_ms,
            true,
            100.0,
            NoopTerrainGridLoader,
            NoopGridLifecycle,
        )
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    #[allow(clippy::too_many_arguments)]
    pub fn with_hooks(
        map_id: u32,
        instance_id: u32,
        spawn_mode: Difficulty,
        grid_expiry_ms: i64,
        grid_unload: bool,
        visible_distance: f32,
        terrain: Terrain,
        lifecycle: Lifecycle,
    ) -> Self {
        Self {
            map_id,
            instance_id,
            spawn_mode,
            grid_expiry_ms,
            grid_unload,
            visible_distance,
            grids: std::iter::repeat_with(|| None)
                .take(GRID_SLOT_COUNT)
                .collect(),
            terrain,
            lifecycle,
            active_cells: HashSet::new(),
            active_non_players_like_cpp: HashSet::new(),
            personal_phase_tracker: MultiPersonalPhaseTracker::default(),
            spawn_group_state: SpawnGroupRuntimeState::new(),
            respawn_store: RespawnStoreLikeCpp::new(),
            pool_data: SpawnedPoolDataLikeCpp::new(),
            grid_state_unloaded: false,
            corpse_data_loaded_like_cpp: false,
            creatures_by_spawn_id: HashMap::new(),
            gameobjects_by_spawn_id: HashMap::new(),
            area_triggers_by_spawn_id: HashMap::new(),
            entity_world: EntityWorld::default(),
            map_reference_order_like_cpp: Vec::new(),
            creature_group_holder_like_cpp: HashMap::new(),
            dynamic_tree_model_keys_like_cpp: HashSet::new(),
            dynamic_tree_rebalance_timer_remaining_ms_like_cpp:
                DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP,
            dynamic_tree_unbalanced_times_like_cpp: 0,
            objects_to_remove: HashSet::new(),
            pending_object_visibility_destroy_recipients_like_cpp: Vec::new(),
            objects_to_switch: HashMap::new(),
            far_spell_callbacks_like_cpp: VecDeque::new(),
            represented_far_spell_callback_execution_log_like_cpp: Vec::new(),
            creatures_to_move: Vec::new(),
            gameobjects_to_move: Vec::new(),
            dynamic_objects_to_move: Vec::new(),
            area_triggers_to_move: Vec::new(),
            creature_move_states: HashMap::new(),
            gameobject_move_states: HashMap::new(),
            dynamic_object_move_states: HashMap::new(),
            area_trigger_move_states: HashMap::new(),
            creature_move_lock: false,
            gameobject_move_lock: false,
            dynamic_object_move_lock: false,
            area_trigger_move_lock: false,
            script_schedule_like_cpp: BTreeMap::new(),
            script_schedule_lock_like_cpp: false,
            represented_executed_script_actions_like_cpp: Vec::new(),
            zone_dynamic_info_like_cpp: BTreeMap::new(),
            weather_update_timer_current_ms_like_cpp: 0,
            weather_update_timer_interval_ms_like_cpp: WEATHER_UPDATE_INTERVAL_MS_LIKE_CPP,
            guid_generators: HashMap::new(),
            creature_level_rng_like_cpp: StdRng::from_entropy(),
        }
    }
}
