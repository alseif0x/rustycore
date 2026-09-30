use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub(in crate::map) fn active_respawn_location_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<ActiveNonPlayerRespawnLocationLikeCpp> {
        let record = self.map_object_record(guid)?;
        match record.kind() {
            AccessorObjectKind::Creature => {
                let creature = record.creature()?;
                let spawn_id = creature.spawn_id();
                (spawn_id != 0).then_some(ActiveNonPlayerRespawnLocationLikeCpp {
                    spawn_id,
                    position: creature.ai_home_position(),
                })
            }
            AccessorObjectKind::GameObject => {
                let game_object = record.game_object()?;
                let spawn_id = game_object.spawn_id();
                (spawn_id != 0).then_some(ActiveNonPlayerRespawnLocationLikeCpp {
                    spawn_id,
                    position: game_object.stationary_position(),
                })
            }
            _ => None,
        }
    }

    pub(in crate::map) fn mutate_unload_active_lock_for_respawn_location_like_cpp(
        &mut self,
        location: ActiveNonPlayerRespawnLocationLikeCpp,
        increment: bool,
    ) -> ActiveNonPlayerUnloadLockOutcomeLikeCpp {
        if !is_valid_map_coord_2d(location.position.x, location.position.y) {
            return ActiveNonPlayerUnloadLockOutcomeLikeCpp {
                spawn_id: location.spawn_id,
                respawn_grid: None,
                respawn_grid_missing: true,
                invalid_respawn_position: true,
                lock_incremented: false,
                lock_decremented: false,
            };
        }

        let cell = Cell::from_world(location.position.x, location.position.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        let Some(ngrid) = self.get_ngrid_mut(grid) else {
            return ActiveNonPlayerUnloadLockOutcomeLikeCpp {
                spawn_id: location.spawn_id,
                respawn_grid: Some(grid),
                respawn_grid_missing: true,
                invalid_respawn_position: false,
                lock_incremented: false,
                lock_decremented: false,
            };
        };

        if increment {
            ngrid.info_mut().inc_unload_active_lock();
        } else {
            ngrid.info_mut().dec_unload_active_lock();
        }

        ActiveNonPlayerUnloadLockOutcomeLikeCpp {
            spawn_id: location.spawn_id,
            respawn_grid: Some(grid),
            respawn_grid_missing: false,
            invalid_respawn_position: false,
            lock_incremented: increment,
            lock_decremented: !increment,
        }
    }
}
