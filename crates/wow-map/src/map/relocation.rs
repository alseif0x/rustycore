// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Object relocation and grid/cell transitions.

use super::*;

mod notifications;
mod removal_queue;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Represents C++ `Map::RemoveGameObjectModel` -> `DynamicMapTree::remove`.
    ///
    /// C++ GameObject callers check containment before removal. Rust exposes a
    /// safe missing-key no-op at the facade so represented count cannot underflow.
    pub fn remove_gameobject_model_like_cpp(
        &mut self,
        key: RepresentedGameObjectModelKeyLikeCpp,
    ) -> DynamicMapTreeModelMutationOutcomeLikeCpp {
        let model_count_before = self.dynamic_tree_model_keys_like_cpp.len();
        let unbalanced_before = self.dynamic_tree_unbalanced_times_like_cpp;
        let removed = self.dynamic_tree_model_keys_like_cpp.remove(&key);

        if removed {
            self.dynamic_tree_unbalanced_times_like_cpp = self
                .dynamic_tree_unbalanced_times_like_cpp
                .saturating_add(1);
        }

        DynamicMapTreeModelMutationOutcomeLikeCpp {
            key,
            status: if removed {
                DynamicMapTreeModelMutationStatusLikeCpp::Removed
            } else {
                DynamicMapTreeModelMutationStatusLikeCpp::Missing
            },
            model_count_before,
            model_count_after: self.dynamic_tree_model_keys_like_cpp.len(),
            unbalanced_before,
            unbalanced_after: self.dynamic_tree_unbalanced_times_like_cpp,
        }
    }

    /// Test seam: flip a cell-resident creature to not-in-world (post C++
    /// `RemoveFromWorld`) while leaving its record in the cell/store, so the
    /// cell-anchored `ObjectUpdater` still visits it and exercises the
    /// `NotInWorld` skip branch.
    #[cfg(test)]
    pub(crate) fn test_remove_creature_from_world_keep_cell_like_cpp(&mut self, guid: ObjectGuid) {
        if let Some(creature) = self
            .entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::creature_mut)
        {
            creature.unit_mut().remove_from_world_like_cpp();
        }
    }

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
            let mut entry = self
                .take_object_entry(guid)
                .expect("record was just observed");
            entry.as_mut().object_mut().relocate(new_position);
            self.insert_object_entry(entry)
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
        let loaded_grid = if diff_grid && active_object {
            self.ensure_grid_loaded_for_active_object(&new_cell, kind.into())
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

        let mut entry = self
            .take_object_entry(guid)
            .expect("record was just observed");
        let object_is_world_object = entry.as_ref().object().is_world_object();
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
                self.insert_object_entry(entry)
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
                self.insert_object_entry(entry)
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
        entry.as_mut().object_mut().relocate(new_position);
        entry
            .as_mut()
            .object_mut()
            .set_current_cell(new_cell.cell_x(), new_cell.cell_y());
        self.insert_object_entry(entry)
            .map_err(MapObjectRelocationError::Store)?;

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
