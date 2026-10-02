//! Grid items of mod.
//!
//! Separated from mod.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

/// Size of a grid cell in yards (64x64 yards like TrinityCore).
pub const GRID_SIZE: f32 = 64.0;

/// Visibility radius in yards (how far a player can see).
pub const VISIBILITY_RADIUS: f32 = 100.0;

pub(super) const MAX_NUMBER_OF_CELLS_LIKE_CPP: i32 = 8;

pub(super) const TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP: i32 =
    MAX_NUMBER_OF_GRIDS_LIKE_CPP * MAX_NUMBER_OF_CELLS_LIKE_CPP;

pub(super) const SIZE_OF_GRID_CELL_LIKE_CPP: f32 =
    SIZE_OF_GRIDS_LIKE_CPP / MAX_NUMBER_OF_CELLS_LIKE_CPP as f32;

pub(super) const CENTER_GRID_CELL_ID_LIKE_CPP: i32 = TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP / 2;

pub(super) const CENTER_GRID_CELL_OFFSET_LIKE_CPP: f32 = SIZE_OF_GRID_CELL_LIKE_CPP / 2.0;

/// Default time before a grid unloads if no players are nearby (5 minutes).
pub const DEFAULT_GRID_UNLOAD_TIME: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CellCoordLikeCpp {
    pub(super) x: i32,
    pub(super) y: i32,
}

pub(super) fn compute_cell_coord_like_cpp(x: f32, y: f32) -> CellCoordLikeCpp {
    let x_offset = (f64::from(x) - f64::from(CENTER_GRID_CELL_OFFSET_LIKE_CPP))
        / f64::from(SIZE_OF_GRID_CELL_LIKE_CPP);
    let y_offset = (f64::from(y) - f64::from(CENTER_GRID_CELL_OFFSET_LIKE_CPP))
        / f64::from(SIZE_OF_GRID_CELL_LIKE_CPP);
    let x_coord = (x_offset + f64::from(CENTER_GRID_CELL_ID_LIKE_CPP) + 0.5) as i32;
    let y_coord = (y_offset + f64::from(CENTER_GRID_CELL_ID_LIKE_CPP) + 0.5) as i32;

    CellCoordLikeCpp {
        x: x_coord.clamp(0, TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP - 1),
        y: y_coord.clamp(0, TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP - 1),
    }
}

pub(super) fn calculate_cell_area_like_cpp(
    position: Position,
    radius: f32,
) -> (CellCoordLikeCpp, CellCoordLikeCpp) {
    if radius <= 0.0 {
        let center = compute_cell_coord_like_cpp(position.x, position.y);
        return (center, center);
    }

    (
        compute_cell_coord_like_cpp(position.x - radius, position.y - radius),
        compute_cell_coord_like_cpp(position.x + radius, position.y + radius),
    )
}

pub(super) fn cell_area_contains_position_like_cpp(
    low: CellCoordLikeCpp,
    high: CellCoordLikeCpp,
    position: Position,
) -> Option<CellCoordLikeCpp> {
    let coord = compute_cell_coord_like_cpp(position.x, position.y);
    (coord.x >= low.x && coord.x <= high.x && coord.y >= low.y && coord.y <= high.y)
        .then_some(coord)
}

pub(super) fn position_to_i32_tuple(position: Position) -> (i32, i32, i32) {
    (position.x as i32, position.y as i32, position.z as i32)
}

/// Coordinate of a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCoord {
    pub x: i16,
    pub y: i16,
}

impl GridCoord {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    pub fn personal_phase_grid_id_like_cpp(&self) -> u16 {
        (i32::from(self.x) * MAX_NUMBER_OF_GRIDS_LIKE_CPP + i32::from(self.y)) as u16
    }

    /// Get surrounding coordinates in a 3x3 area (including self).
    pub fn surrounding(&self) -> Vec<GridCoord> {
        let mut coords = Vec::with_capacity(9);
        for dx in -1..=1 {
            for dy in -1..=1 {
                coords.push(GridCoord::new(self.x + dx, self.y + dy));
            }
        }
        coords
    }

    /// Check if another coordinate is within a given range.
    pub fn distance_squared(&self, other: &GridCoord) -> i32 {
        let dx = (self.x - other.x) as i32;
        let dy = (self.y - other.y) as i32;
        dx * dx + dy * dy
    }
}

/// A grid cell containing creatures and player references.
#[derive(Debug)]
pub struct Grid {
    pub coord: GridCoord,
    pub creatures: HashMap<ObjectGuid, WorldCreature>,
    pub player_guids: HashSet<ObjectGuid>,
    pub last_player_time: Instant,
    pub loaded: bool,
}

impl Grid {
    pub fn new(x: i16, y: i16) -> Self {
        Self {
            coord: GridCoord::new(x, y),
            creatures: HashMap::new(),
            player_guids: HashSet::new(),
            last_player_time: Instant::now(),
            loaded: true,
        }
    }

    pub fn add_creature(&mut self, creature: WorldCreature) -> bool {
        if self.creatures.contains_key(&creature.guid()) {
            warn!(
                "Creature {:?} already exists in grid {:?}",
                creature.guid(),
                self.coord
            );
            return false;
        }
        self.creatures.insert(creature.guid(), creature);
        true
    }

    pub fn remove_creature(&mut self, guid: ObjectGuid) -> bool {
        self.creatures.remove(&guid).is_some()
    }

    pub fn get_creature(&self, guid: ObjectGuid) -> Option<&WorldCreature> {
        self.creatures.get(&guid)
    }

    pub fn get_creature_mut(&mut self, guid: ObjectGuid) -> Option<&mut WorldCreature> {
        self.creatures.get_mut(&guid)
    }

    pub fn player_enter(&mut self, guid: ObjectGuid) {
        self.player_guids.insert(guid);
        self.last_player_time = Instant::now();
    }

    pub fn player_leave(&mut self, guid: ObjectGuid) {
        self.player_guids.remove(&guid);
    }

    pub fn should_unload(&self, timeout: Duration) -> bool {
        self.player_guids.is_empty() && self.last_player_time.elapsed() > timeout
    }

    pub fn creature_count(&self) -> usize {
        self.creatures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.creatures.is_empty() && self.player_guids.is_empty()
    }
}

/// Convert world X coordinate to grid X coordinate.
/// Uses floor() to handle negative coordinates correctly.
pub fn world_to_grid_x(world_x: f32) -> i16 {
    (world_x / GRID_SIZE).floor() as i16
}

/// Convert world Y coordinate to grid Y coordinate.
/// Uses floor() to handle negative coordinates correctly.
pub fn world_to_grid_y(world_y: f32) -> i16 {
    (world_y / GRID_SIZE).floor() as i16
}

/// Convert world coordinates to grid coordinates (x, y).
/// Convenience function that returns both coordinates at once.
pub fn world_to_grid_coords(world_x: f32, world_y: f32) -> (i16, i16) {
    (world_to_grid_x(world_x), world_to_grid_y(world_y))
}

/// Convert grid coordinate to world coordinate (center of grid).
pub fn grid_to_world(grid: i16) -> f32 {
    (grid as f32 * GRID_SIZE) + (GRID_SIZE / 2.0)
}

/// Get the world coordinates of a grid's corner.
pub fn grid_corner(grid_x: i16, grid_y: i16) -> (f32, f32) {
    (grid_x as f32 * GRID_SIZE, grid_y as f32 * GRID_SIZE)
}
