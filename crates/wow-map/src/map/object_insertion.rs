// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map object insertion and activation lifecycle.

use super::*;

mod creature;
mod game_object;
mod other_objects;
mod membership;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{

    pub fn add_to_map_like_cpp(
        &mut self,
        kind: AccessorObjectKind,
        object: WorldObject,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        let record = MapObjectRecord::new(kind, object).map_err(MapObjectStoreError::from)?;
        self.add_map_object_record_to_map_like_cpp(record)
    }

    pub fn add_map_object_record_to_map_like_cpp(
        &mut self,
        record: MapObjectRecord,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        self.add_object_entry_to_map(ObjectEntry::Record(record))
    }

    pub(super) fn add_object_entry_to_map(
        &mut self,
        entry: ObjectEntry,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        let kind = entry.as_ref().kind();
        let guid = entry.as_ref().object().guid();
        let position = entry.as_ref().object().position();
        let is_world_object = entry.as_ref().object().is_world_object();

        if entry.as_ref().object().object().is_in_world() {
            let cell = Cell::from_world(position.x, position.y);
            let previous = self.insert_object_entry(entry)?;
            if kind == AccessorObjectKind::Transport {
                self.mark_transport_players_for_visibility_like_cpp(guid);
            } else {
                self.mark_nearby_players_for_visibility_like_cpp(guid);
            }
            return Ok(AddToMapOutcome {
                guid,
                cell: cell.cell_coord(),
                grid: GridCoord::new(cell.grid_x(), cell.grid_y()),
                inserted: previous.is_none(),
                already_in_world: true,
                grid_created: false,
                grid_loaded: false,
                inserted_into_cell: false,
                gameobject_model_insert: None,
                gameobject_collision_enable: None,
                gameobject_zone_script_create: None,
                gameobject_store_inserted_before_add_to_world: None,
                gameobject_spawn_indexed_before_add_to_world: None,
                creature_store_inserted_before_add_to_world: None,
                creature_spawn_indexed_before_add_to_world: None,
                creature_unit_add_to_world: None,
                creature_search_formation: None,
                creature_aim_initialize: None,
                creature_vehicle_reset: None,
                creature_vehicle_install: None,
                creature_zone_script_create: None,
                add_to_map_tail: None,
            });
        }

        self.validate_map_object(entry.as_ref().object())?;

        if !is_valid_map_coord_2d(position.x, position.y) {
            return Err(AddToMapError::InvalidCoordinates {
                guid,
                x: position.x,
                y: position.y,
            });
        }

        let cell = Cell::from_world(position.x, position.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        let active_object = is_active_object_like_cpp(kind, entry.as_ref().object());
        let grid_loaded = if active_object {
            self.ensure_grid_loaded_for_active_object(&cell, kind.into())
        } else {
            false
        };
        let grid_created = if active_object {
            false
        } else {
            self.ensure_grid_created(grid)
        };

        {
            let ngrid = self
                .get_ngrid_mut(grid)
                .expect("Map::AddToMap must have created or loaded the target grid");
            let local_cell = ngrid
                .get_grid_type_mut(cell.cell_x(), cell.cell_y())
                .expect("cell coordinates must be local to target grid");
            insert_object_guid_in_cell_like_cpp(local_cell, kind, is_world_object, guid);
        }

        if kind == AccessorObjectKind::Creature && entry.as_ref().creature().is_some() {
            return self.add_creature_entry(entry, kind, guid, cell, grid, active_object, grid_loaded, grid_created);
        }

        if kind == AccessorObjectKind::GameObject && entry.as_ref().game_object().is_some() {
            return self.add_game_object_entry(entry, kind, guid, cell, grid, active_object, grid_loaded, grid_created);
        }

        self.add_other_object_entry(entry, kind, guid, cell, grid, active_object, grid_loaded, grid_created)
    }

}
