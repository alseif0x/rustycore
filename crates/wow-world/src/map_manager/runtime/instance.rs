//! Instance packets.
//!
//! Separated from runtime.rs under #701.

use super::*;

impl MapInstance {
    pub fn new(map_id: u16, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id,
            grids: HashMap::new(),
            grid_unload_timeout: DEFAULT_GRID_UNLOAD_TIME,
            persisted_respawn_times: HashMap::new(),
            respawn_queue: Vec::new(),
        }
    }

    pub fn get_or_create_grid(&mut self, x: i16, y: i16) -> &mut Grid {
        let coord = GridCoord::new(x, y);
        if !self.grids.contains_key(&coord) {
            let grid = Grid::new(x, y);
            self.grids.insert(coord, grid);
            debug!(
                "Created new grid ({}, {}) for map {} instance {}",
                x, y, self.map_id, self.instance_id
            );
        }
        self.grids.get_mut(&coord).unwrap()
    }

    pub fn get_grid(&self, x: i16, y: i16) -> Option<&Grid> {
        self.grids.get(&GridCoord::new(x, y))
    }

    pub fn get_grid_mut(&mut self, x: i16, y: i16) -> Option<&mut Grid> {
        self.grids.get_mut(&GridCoord::new(x, y))
    }

    pub fn add_creature(&mut self, x: i16, y: i16, creature: WorldCreature) -> bool {
        self.get_or_create_grid(x, y).add_creature(creature)
    }

    pub fn get_creature(&self, x: i16, y: i16, guid: ObjectGuid) -> Option<&WorldCreature> {
        self.get_grid(x, y)?.get_creature(guid)
    }

    pub fn get_creature_mut(
        &mut self,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        self.get_grid_mut(x, y)?.get_creature_mut(guid)
    }

    pub fn unload_empty_grids(&mut self) {
        let to_remove: Vec<GridCoord> = self
            .grids
            .iter()
            .filter(|(_, grid)| grid.should_unload(self.grid_unload_timeout))
            .map(|(coord, _)| *coord)
            .collect();

        for coord in to_remove {
            info!(
                "Unloading grid {:?} from map {} (timeout)",
                coord, self.map_id
            );
            self.grids.remove(&coord);
        }
    }

    pub fn creature_count(&self) -> usize {
        self.grids.values().map(|g| g.creature_count()).sum()
    }

    pub fn is_grid_loaded(&self, x: i16, y: i16) -> bool {
        self.get_grid(x, y).is_some()
    }

    pub fn min_height_like_cpp(&self, _x: f32, _y: f32) -> f32 {
        DEFAULT_MIN_HEIGHT_LIKE_CPP
    }
}
