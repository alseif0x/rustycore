// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature movement: splines, pathfinding and relocation.

use super::*;

impl MapInstance {
    pub fn remove_grid(&mut self, x: i16, y: i16) -> bool {
        let coord = GridCoord::new(x, y);
        let removed = self.grids.remove(&coord).is_some();
        if removed {
            self.personal_phases
                .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
        }
        removed
    }

    pub fn remove_creature(&mut self, x: i16, y: i16, guid: ObjectGuid) -> bool {
        if let Some(grid) = self.get_grid_mut(x, y) {
            grid.remove_creature(guid)
        } else {
            false
        }
    }

    pub fn remove_personal_phase_objects_like_cpp(&mut self) -> usize {
        let objects_to_remove = std::mem::take(&mut self.personal_phase_objects_to_remove);
        let removed = objects_to_remove.len();
        for object in objects_to_remove {
            for grid in self.grids.values_mut() {
                grid.remove_creature(object);
            }
        }
        removed
    }

    pub fn queued_personal_phase_remove_count_like_cpp(&self) -> usize {
        self.personal_phase_objects_to_remove.len()
    }

    // ── Respawn queue (Slice 4A.2a) ───────────────────────────────────────────
    //
    // Mirrors `Map::_respawnTimes` (Map.h:748-750) ownership model.
    // The queue is a plain `Vec`; heap/SpawnId convergence is deferred.

    pub fn remove_persisted_respawn_time_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<PersistedRespawnRowLikeCpp> {
        self.persisted_respawn_times
            .remove(&(object_type, spawn_id))
    }
}

impl MapManager {
    pub fn remove_creature(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> bool {
        if let Some(map) = self.get_map_mut(map_id, instance_id) {
            map.remove_creature(x, y, guid)
        } else {
            false
        }
    }

    pub fn remove_creature_any(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<WorldCreature> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.grids
            .values_mut()
            .find_map(|grid| grid.creatures.remove(&guid))
    }

    pub fn remove_persisted_respawn_time_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<RespawnPersistenceMutationLikeCpp> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.remove_persisted_respawn_time_like_cpp(object_type, spawn_id)?;
        Some(respawn_delete_mutation_like_cpp(
            object_type,
            spawn_id,
            map_id,
            instance_id,
        ))
    }

    pub fn player_move(
        &mut self,
        map_id: u16,
        instance_id: u32,
        from: (i16, i16),
        to: (i16, i16),
        player_guid: ObjectGuid,
        pos: Position,
    ) {
        let (from_x, from_y) = from;
        let (to_x, to_y) = to;

        // Leave old grid
        self.player_leave_grid(map_id, instance_id, from_x, from_y, player_guid);

        // Enter new grid
        self.player_enter_grid(map_id, instance_id, to_x, to_y, player_guid, pos);
    }
}

mod corpse_loot;
mod home_and_chase;
mod motion_master;
mod point_and_effects;
mod random_and_waypoint;
mod spline;
mod terrain;
