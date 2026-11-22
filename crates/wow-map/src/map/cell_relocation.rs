//! Cell relocation and the associated move-list results.

use super::*;

/// Drain summary for C++ `Map::MoveAll*InMoveList` (`Map.cpp:1239-1416`).
/// This is a map-owned seam only: it does not claim UpdatePositionData,
/// visibility fanout, AfterRelocation, Pet::Remove,
/// dynamic tree, scripts/AI, ObjectAccessor, or session packet runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveListDrainSummaryLikeCpp {
    pub family: Option<MapObjectMoveListFamilyLikeCpp>,
    pub processed: usize,
    pub relocated: usize,
    pub inactive_reset: usize,
    pub not_in_world: usize,
    pub missing_or_stale: usize,
    pub wrong_kind: usize,
    pub blocked_by_unloaded_grid: usize,
    pub remove_list_queued: usize,
    pub pet_remove_requested: usize,
    pub respawn_relocated: usize,
    pub respawn_relocation_unsupported: usize,
    pub failed_invalid_position: usize,
    pub failed_store: usize,
    pub locked_ignored: usize,
}

fn commit_relocated_position(record: &mut MapObjectRecord, position: Position) {
    let is_creature = matches!(
        record.kind(),
        AccessorObjectKind::Creature | AccessorObjectKind::Pet
    );
    let world = record.object_mut();
    let from = world.position();
    world.relocate(position);
    // Unit::UpdatePosition treats a turn separately. A successful creature
    // move marks the source for the map's next ProcessRelocationNotifies.
    if is_creature
        && ((from.x - position.x).abs() > 0.001
            || (from.y - position.y).abs() > 0.001
            || (from.z - position.z).abs() > 0.001)
    {
        world
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn relocate_map_object_like_cpp(
        &mut self,
        guid: ObjectGuid,
        new_position: Position,
    ) -> Result<MapObjectRelocationOutcome, MapObjectRelocationError> {
        if !is_valid_map_coord_2d(new_position.x, new_position.y) {
            return Err(MapObjectRelocationError::InvalidCoordinates {
                guid,
                x: new_position.x,
                y: new_position.y,
            });
        }

        let record = self
            .map_object_record(guid)
            .ok_or(MapObjectRelocationError::ObjectNotFound { guid })?;
        let kind = record.kind();
        let old_position = record.object().position();
        let old_cell = Cell::from_world(old_position.x, old_position.y);
        let new_cell = Cell::from_world(new_position.x, new_position.y);
        let old_grid = GridCoord::new(old_cell.grid_x(), old_cell.grid_y());
        let new_grid = GridCoord::new(new_cell.grid_x(), new_cell.grid_y());
        let diff_cell = old_cell.diff_cell(&new_cell);
        let diff_grid = old_cell.diff_grid(&new_cell);

        if !diff_cell && !diff_grid {
            let mut record = self
                .remove_map_object(guid)
                .expect("record was just observed");
            commit_relocated_position(&mut record, new_position);
            self.insert_map_object_record(record)
                .map_err(MapObjectRelocationError::Store)?;
            return Ok(MapObjectRelocationOutcome {
                guid,
                old_cell: old_cell.cell_coord(),
                new_cell: new_cell.cell_coord(),
                old_grid,
                new_grid,
                moved_between_cells: false,
                loaded_grid: false,
                created_grid: false,
                relocated: true,
                blocked_by_unloaded_grid: false,
            });
        }

        let active_object = is_active_object_like_cpp(kind, record.object());
        let player_controlled = record
            .creature()
            .or_else(|| record.pet().map(|pet| pet.creature()))
            .and_then(|creature| creature.unit().subsystems().control.charmer_or_owner_guid())
            .is_some_and(|owner| owner.is_player());
        let loaded_grid = if diff_grid && active_object {
            self.ensure_grid_loaded_for_active_object(&new_cell, kind.into())
        } else if diff_grid && player_controlled {
            self.ensure_grid_loaded(&new_cell)
        } else {
            false
        };
        let created_grid = if diff_grid && !active_object {
            if !self.is_grid_loaded(new_grid) {
                return Ok(MapObjectRelocationOutcome {
                    guid,
                    old_cell: old_cell.cell_coord(),
                    new_cell: new_cell.cell_coord(),
                    old_grid,
                    new_grid,
                    moved_between_cells: false,
                    loaded_grid: false,
                    created_grid: false,
                    relocated: false,
                    blocked_by_unloaded_grid: true,
                });
            }
            self.ensure_grid_created(new_grid)
        } else {
            false
        };

        if self.get_ngrid(new_grid).is_none() {
            return Ok(MapObjectRelocationOutcome {
                guid,
                old_cell: old_cell.cell_coord(),
                new_cell: new_cell.cell_coord(),
                old_grid,
                new_grid,
                moved_between_cells: false,
                loaded_grid,
                created_grid: false,
                relocated: false,
                blocked_by_unloaded_grid: true,
            });
        }

        // The destination visitor cannot find stationary observers left behind
        // by a long relocation. Their session rechecks its actual visible set.
        let previous_observers = if kind == AccessorObjectKind::Creature {
            self.nearby_player_guids_for_visibility_like_cpp(guid)
        } else {
            Vec::new()
        };
        let mut record = self
            .remove_map_object(guid)
            .expect("record was just observed");
        let object_is_world_object = record.object().is_world_object();
        let removed = remove_object_guid_from_cell_like_cpp(
            self,
            old_grid,
            &old_cell,
            kind,
            object_is_world_object,
            guid,
        );
        let _removed_from_old_cell = removed;
        {
            let Some(ngrid) = self.get_ngrid_mut(new_grid) else {
                self.insert_map_object_record(record)
                    .map_err(MapObjectRelocationError::Store)?;
                return Ok(MapObjectRelocationOutcome {
                    guid,
                    old_cell: old_cell.cell_coord(),
                    new_cell: new_cell.cell_coord(),
                    old_grid,
                    new_grid,
                    moved_between_cells: false,
                    loaded_grid,
                    created_grid,
                    relocated: false,
                    blocked_by_unloaded_grid: true,
                });
            };
            let Some(local_cell) = ngrid.get_grid_type_mut(new_cell.cell_x(), new_cell.cell_y())
            else {
                self.insert_map_object_record(record)
                    .map_err(MapObjectRelocationError::Store)?;
                return Ok(MapObjectRelocationOutcome {
                    guid,
                    old_cell: old_cell.cell_coord(),
                    new_cell: new_cell.cell_coord(),
                    old_grid,
                    new_grid,
                    moved_between_cells: false,
                    loaded_grid,
                    created_grid,
                    relocated: false,
                    blocked_by_unloaded_grid: true,
                });
            };
            insert_object_guid_in_cell_like_cpp(local_cell, kind, object_is_world_object, guid);
        }
        commit_relocated_position(&mut record, new_position);
        record
            .object_mut()
            .set_current_cell(new_cell.cell_x(), new_cell.cell_y());
        self.insert_map_object_record(record)
            .map_err(MapObjectRelocationError::Store)?;
        self.mark_player_visibility_guids_like_cpp(&previous_observers);

        Ok(MapObjectRelocationOutcome {
            guid,
            old_cell: old_cell.cell_coord(),
            new_cell: new_cell.cell_coord(),
            old_grid,
            new_grid,
            moved_between_cells: true,
            loaded_grid,
            created_grid,
            relocated: true,
            blocked_by_unloaded_grid: false,
        })
    }
}
