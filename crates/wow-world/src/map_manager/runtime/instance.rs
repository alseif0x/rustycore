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
            personal_phases: MultiPersonalPhaseTracker::default(),
            personal_phase_objects_to_remove: HashSet::new(),
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
            self.personal_phases
                .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
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

    pub fn load_personal_phase_grid_like_cpp(
        &mut self,
        phase_shift: &PhaseShift,
        x: i16,
        y: i16,
        has_personal_spawns: impl FnMut(u32) -> bool,
        load_phase: impl FnMut(ObjectGuid, u32),
    ) -> bool {
        self.get_or_create_grid(x, y);
        self.personal_phases.load_grid_like_cpp(
            phase_shift,
            GridCoord::new(x, y).personal_phase_grid_id_like_cpp(),
            has_personal_spawns,
            load_phase,
        )
    }

    pub fn update_personal_phases_for_owner_like_cpp(
        &mut self,
        phase_owner: ObjectGuid,
        phase_shift: &PhaseShift,
        grid: Option<GridCoord>,
        has_personal_spawns: impl FnMut(u32) -> bool,
        load_phase: impl FnMut(ObjectGuid, u32),
    ) -> bool {
        self.personal_phases.on_owner_phase_changed_like_cpp(
            phase_owner,
            phase_shift,
            grid.map(|coord| coord.personal_phase_grid_id_like_cpp()),
            has_personal_spawns,
            load_phase,
        )
    }

    pub fn register_personal_phase_object_like_cpp(
        &mut self,
        phase_id: u32,
        phase_owner: ObjectGuid,
        object: ObjectGuid,
    ) {
        self.personal_phases
            .register_tracked_object_like_cpp(phase_id, phase_owner, object);
    }

    pub fn unregister_personal_phase_object_like_cpp(
        &mut self,
        phase_owner: ObjectGuid,
        object: ObjectGuid,
    ) {
        self.personal_phases
            .unregister_tracked_object_like_cpp(phase_owner, object);
    }

    pub fn mark_personal_phases_for_deletion_like_cpp(&mut self, phase_owner: ObjectGuid) {
        self.personal_phases
            .mark_all_phases_for_deletion_like_cpp(phase_owner);
    }

    pub fn update_personal_phases_like_cpp(&mut self, diff: Duration) {
        let mut objects_to_remove = Vec::new();
        self.personal_phases
            .update_like_cpp(diff, |guid| objects_to_remove.push(guid));
        self.personal_phase_objects_to_remove
            .extend(objects_to_remove);
    }
}
