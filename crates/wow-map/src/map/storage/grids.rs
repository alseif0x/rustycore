// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Grid admission, loaded-cell storage and ordered unload lifecycle.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub const fn grid_expiry_ms(&self) -> i64 {
        self.grid_expiry_ms
    }

    pub const fn grid_unload(&self) -> bool {
        self.grid_unload
    }

    pub fn terrain(&self) -> &Terrain {
        &self.terrain
    }

    /// Bridge for C++ `Map::ShouldBeSpawnedOnGridLoad` callers while `Map` does
    /// not yet own the ObjectMgr spawn metadata. The canonical toggle state,
    /// respawn timers, and `SpawnedPoolData` are map-owned; spawn metadata remains
    /// caller-supplied.
    pub fn spawn_grid_load_state_like_cpp<'a>(
        &'a self,
        spawn_store: &'a SpawnStore,
    ) -> SpawnGridLoadStateLikeCpp<'a> {
        SpawnGridLoadStateLikeCpp::new(spawn_store, &self.spawn_group_state)
            .with_respawn_timers(self.respawn_store.respawn_timer_keys_like_cpp())
            .with_pool_spawned_objects(self.pool_data.spawned_objects_like_cpp())
    }

    pub fn mark_active_cell(&mut self, cell: CellCoord) {
        assert!(cell.is_coord_valid());
        self.active_cells.insert(cell);
    }

    pub fn unmark_active_cell(&mut self, cell: CellCoord) {
        self.active_cells.remove(&cell);
    }

    pub fn get_ngrid(&self, coord: GridCoord) -> Option<&NGrid> {
        let index = grid_index(coord)?;
        self.grids[index].as_deref()
    }

    pub fn get_ngrid_mut(&mut self, coord: GridCoord) -> Option<&mut NGrid> {
        let index = grid_index(coord)?;
        self.grids[index].as_deref_mut()
    }

    pub fn set_ngrid(&mut self, coord: GridCoord, grid: Option<NGrid>) {
        let index = checked_grid_index(coord);
        self.grids[index] = grid.map(Box::new);
    }

    pub fn is_grid_loaded(&self, coord: GridCoord) -> bool {
        self.get_ngrid(coord)
            .is_some_and(NGrid::grid_object_data_loaded)
    }

    pub fn loaded_grid_coords_like_cpp(&self) -> Vec<GridCoord> {
        self.grids
            .iter()
            .enumerate()
            .filter_map(|(index, grid)| {
                grid.as_ref()
                    .filter(|grid| grid.grid_object_data_loaded())
                    .map(|_| {
                        GridCoord::new(
                            (index as u32) / MAX_NUMBER_OF_GRIDS,
                            (index as u32) % MAX_NUMBER_OF_GRIDS,
                        )
                    })
            })
            .collect()
    }

    pub fn ensure_grid_created(&mut self, coord: GridCoord) -> bool {
        let index = checked_grid_index(coord);
        if self.grids[index].is_some() {
            return false;
        }

        let mut grid = NGrid::from_coords(
            coord.x_coord as i32,
            coord.y_coord as i32,
            self.grid_expiry_ms,
            self.grid_unload,
        );
        grid.set_state(GridStateKind::Idle);
        self.grids[index] = Some(Box::new(grid));

        let (terrain_x, terrain_y) = terrain_grid_coords(coord);
        self.terrain.load_map_and_vmap(terrain_x, terrain_y);
        true
    }

    pub fn ensure_grid_loaded(&mut self, cell: &Cell) -> bool {
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        self.ensure_grid_created(coord);
        let index = checked_grid_index(coord);
        {
            let grid = self.grids[index].as_mut().expect("grid was just created");
            if grid.grid_object_data_loaded() {
                return false;
            }

            grid.set_grid_object_data_loaded(true);
            self.lifecycle.load_grid_objects(grid, cell);
        }
        self.activate_registered_corpses_for_grid_like_cpp(coord);
        true
    }

    pub fn ensure_grid_loaded_for_active_object(
        &mut self,
        cell: &Cell,
        kind: ActiveObjectKind,
    ) -> bool {
        let loaded_now = self.ensure_grid_loaded(cell);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        self.mark_active_cell(cell.cell_coord());

        if matches!(kind, ActiveObjectKind::Player) {
            // Use `ensure_grid_loaded_for_player_phase` when phase-shift state
            // is available; this entry point only has the object kind.
        }

        let active_expiry_ms = (self.grid_expiry_ms as f32 * 0.1) as i64;
        let grid = self.get_ngrid_mut(coord).expect("grid was just loaded");
        if grid.state() != GridStateKind::Active {
            grid.info_mut().reset_time_tracker(active_expiry_ms);
            grid.set_state(GridStateKind::Active);
        }

        loaded_now
    }

    pub fn load_grid(&mut self, x: f32, y: f32) -> bool {
        self.ensure_grid_loaded(&Cell::from_world(x, y))
    }

    pub fn load_grid_for_active_object(&mut self, x: f32, y: f32, kind: ActiveObjectKind) -> bool {
        self.ensure_grid_loaded_for_active_object(&Cell::from_world(x, y), kind)
    }

    pub fn reset_grid_expiry(&self, grid: &mut NGrid, factor: f32) {
        grid.info_mut()
            .reset_time_tracker((self.grid_expiry_ms as f32 * factor) as i64);
    }

    pub fn active_objects_near_grid(&self, grid: &NGrid) -> bool {
        if active_cells_near_grid(&self.active_cells, self.visible_distance, grid) {
            return true;
        }

        let active_non_player_cells: HashSet<_> = self
            .active_non_players_like_cpp
            .iter()
            .filter_map(|guid| {
                let record = self.map_object_record(*guid)?;
                record.object().object().is_in_world().then(|| {
                    compute_cell_coord(record.object().position().x, record.object().position().y)
                })
            })
            .collect();
        active_cells_near_grid(&active_non_player_cells, self.visible_distance, grid)
    }

    pub fn unload_grid_at(&mut self, coord: GridCoord, unload_all: bool) -> bool {
        let index = checked_grid_index(coord);
        let Some(mut grid) = self.grids[index].take() else {
            return false;
        };

        if !self.can_unload_grid(&grid, unload_all) {
            self.grids[index] = Some(grid);
            return false;
        }

        self.run_unload_lifecycle(&mut grid, unload_all);
        true
    }

    pub(in crate::map) fn can_unload_grid(&self, grid: &NGrid, unload_all: bool) -> bool {
        unload_all
            || (grid.world_creature_count_in_ngrid() == 0 && !self.active_objects_near_grid(grid))
    }

    pub(in crate::map) fn run_unload_lifecycle(&mut self, grid: &mut NGrid, unload_all: bool) {
        // C++ `Map::UnloadGrid` drains Creature/GameObject/AreaTrigger move lists
        // only in the `!unloadAll` branch, before and after the evacuator
        // (`Map.cpp:1579-1596`). `UnloadGrid(..., true)` does not drain or
        // relocate move-lists; `Map::UnloadAll` only clears Creature/GameObject
        // delayed moves before entering that loop (`Map.cpp:1646-1651`). Rust
        // still keeps the rest of this unload lifecycle represented: no
        // DynamicObject drain in this path, no full visibility/fanout/scripts/DB.
        if !unload_all {
            self.move_all_creatures_in_move_list_like_cpp();
            self.move_all_game_objects_in_move_list_like_cpp();
            self.move_all_area_triggers_in_move_list_like_cpp();
            self.lifecycle.evacuate_grid(grid);
            self.drain_grid_unload_actions_like_cpp();
            self.move_all_creatures_in_move_list_like_cpp();
            self.move_all_game_objects_in_move_list_like_cpp();
            self.move_all_area_triggers_in_move_list_like_cpp();
        }

        self.lifecycle.clean_grid(grid);
        self.drain_grid_unload_actions_like_cpp();
        self.personal_phase_tracker.unload_grid(grid);
        self.lifecycle.unload_grid_objects(grid);
        self.drain_grid_unload_actions_like_cpp();

        let coord = GridCoord::new(grid.x() as u32, grid.y() as u32);
        self.deactivate_registered_corpses_for_grid_like_cpp(coord);
        let (terrain_x, terrain_y) = terrain_grid_coords(coord);
        self.terrain.unload_map(terrain_x, terrain_y);
    }

    pub(in crate::map) fn drain_grid_unload_actions_like_cpp(
        &mut self,
    ) -> Vec<GridUnloadApplyOutcome> {
        let actions = self.lifecycle.take_unload_actions_like_cpp();
        if actions.is_empty() {
            return Vec::new();
        }

        apply_grid_unload_actions(self, actions)
    }
}
