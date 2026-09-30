// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loaded corpse registration and grid activation/deactivation.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub const fn corpse_data_loaded_like_cpp(&self) -> bool {
        self.corpse_data_loaded_like_cpp
    }

    pub fn mark_corpse_data_loaded_like_cpp(&mut self) {
        self.corpse_data_loaded_like_cpp = true;
    }

    /// Register a corpse produced by C++ `Map::LoadCorpseData`.
    ///
    /// `Map::AddCorpse` retains every loaded corpse by cell, but
    /// `ObjectWorldLoader` only calls `AddToWorld` when that corpse's grid is
    /// loaded. Rust keeps the typed record dormant in `entity_world` and
    /// activates it immediately only when the destination grid is already
    /// loaded (the async login bridge may finish after the player's grid load).
    pub fn register_loaded_corpse_like_cpp(
        &mut self,
        corpse: Corpse,
    ) -> Result<bool, AddToMapError> {
        let record = MapObjectRecord::new_corpse(corpse).map_err(MapObjectStoreError::from)?;
        let guid = record.object().guid();
        let position = record.object().position();
        if !is_valid_map_coord_2d(position.x, position.y) {
            return Err(AddToMapError::InvalidCoordinates {
                guid,
                x: position.x,
                y: position.y,
            });
        }

        let grid = GridCoord::new(
            Cell::from_world(position.x, position.y).grid_x(),
            Cell::from_world(position.x, position.y).grid_y(),
        );
        self.insert_map_object_record(record)?;
        if self.is_grid_loaded(grid) {
            self.activate_registered_corpses_for_grid_like_cpp(grid);
        }

        Ok(self.object_is_in_world(guid))
    }

    pub(super) fn activate_registered_corpses_for_grid_like_cpp(&mut self, grid: GridCoord) -> usize {
        if !self.is_grid_loaded(grid) {
            return 0;
        }

        let corpses = self
            .entity_world
            .iter()
            .filter_map(|(guid, record)| {
                if record.kind() != AccessorObjectKind::Corpse
                    || record.object().object().is_in_world()
                {
                    return None;
                }
                let position = record.object().position();
                let cell = Cell::from_world(position.x, position.y);
                (GridCoord::new(cell.grid_x(), cell.grid_y()) == grid).then_some((
                    *guid,
                    cell,
                    record.object().is_world_object(),
                ))
            })
            .collect::<Vec<_>>();

        for (guid, cell, is_world_object) in &corpses {
            let ngrid = self
                .get_ngrid_mut(grid)
                .expect("registered corpse grid was checked as loaded");
            let local_cell = ngrid
                .get_grid_type_mut(cell.cell_x(), cell.cell_y())
                .expect("registered corpse coordinates must identify a local grid cell");
            insert_object_guid_in_cell_like_cpp(
                local_cell,
                AccessorObjectKind::Corpse,
                *is_world_object,
                *guid,
            );

            if let Some(corpse) = self
                .entity_world
                .get_mut(guid)
                .and_then(ObjectMut::corpse_mut)
            {
                corpse
                    .world_mut()
                    .set_current_cell(cell.cell_x(), cell.cell_y());
                corpse.world_mut().object_mut().add_to_world();
            }
        }

        corpses.len()
    }

    pub(super) fn deactivate_registered_corpses_for_grid_like_cpp(&mut self, grid: GridCoord) -> usize {
        let corpses = self
            .entity_world
            .iter()
            .filter_map(|(guid, record)| {
                if record.kind() != AccessorObjectKind::Corpse
                    || !record.object().object().is_in_world()
                {
                    return None;
                }
                let position = record.object().position();
                let cell = Cell::from_world(position.x, position.y);
                (GridCoord::new(cell.grid_x(), cell.grid_y()) == grid).then_some(*guid)
            })
            .collect::<Vec<_>>();

        for guid in &corpses {
            if let Some(corpse) = self
                .entity_world
                .get_mut(guid)
                .and_then(ObjectMut::corpse_mut)
            {
                corpse.world_mut().object_mut().remove_from_world();
                corpse.world_mut().clear_current_cell();
            }
        }

        corpses.len()
    }

}
