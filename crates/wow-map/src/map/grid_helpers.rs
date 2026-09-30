// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Grid coordinate helpers and stable cell lookup API.

use super::*;

pub(super) fn grid_index(coord: GridCoord) -> Option<usize> {
    coord
        .is_coord_valid()
        .then_some((coord.x_coord * MAX_NUMBER_OF_GRIDS + coord.y_coord) as usize)
}

pub(super) fn checked_grid_index(coord: GridCoord) -> usize {
    grid_index(coord).expect("grid coordinates must be within MAX_NUMBER_OF_GRIDS")
}

pub(super) fn terrain_grid_coords(coord: GridCoord) -> (u32, u32) {
    (
        (MAX_NUMBER_OF_GRIDS - 1) - coord.x_coord,
        (MAX_NUMBER_OF_GRIDS - 1) - coord.y_coord,
    )
}

pub(super) fn active_cells_near_grid(
    active_cells: &HashSet<CellCoord>,
    visible_distance: f32,
    grid: &NGrid,
) -> bool {
    let mut cell_min = CellCoord::new(
        grid.x() as u32 * MAX_NUMBER_OF_CELLS,
        grid.y() as u32 * MAX_NUMBER_OF_CELLS,
    );
    let mut cell_max = CellCoord::new(
        cell_min.x_coord + MAX_NUMBER_OF_CELLS,
        cell_min.y_coord + MAX_NUMBER_OF_CELLS,
    );
    let cell_range = (visible_distance / SIZE_OF_GRID_CELL).ceil() as u32 + 1;

    cell_min.dec_x(cell_range);
    cell_min.dec_y(cell_range);
    cell_max.inc_x(cell_range);
    cell_max.inc_y(cell_range);

    active_cells.iter().any(|cell| {
        cell_min.x_coord <= cell.x_coord
            && cell.x_coord <= cell_max.x_coord
            && cell_min.y_coord <= cell.y_coord
            && cell.y_coord <= cell_max.y_coord
    })
}

pub fn is_grid_id_loaded<Terrain, Lifecycle>(map: &Map<Terrain, Lifecycle>, grid_id: u32) -> bool
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    map.is_grid_loaded(GridCoord::new(
        grid_id % MAX_NUMBER_OF_GRIDS,
        grid_id / MAX_NUMBER_OF_GRIDS,
    ))
}

pub fn cell_from_grid_center(coord: GridCoord) -> Cell {
    let cell = CellCoord::new(
        coord.x_coord * MAX_NUMBER_OF_CELLS,
        coord.y_coord * MAX_NUMBER_OF_CELLS,
    );
    Cell::from_cell_coord(cell)
}

pub fn cell_from_world(x: f32, y: f32) -> Cell {
    Cell::from_cell_coord(compute_cell_coord(x, y))
}

pub const fn total_cell_count() -> u32 {
    TOTAL_NUMBER_OF_CELLS_PER_MAP * TOTAL_NUMBER_OF_CELLS_PER_MAP
}
