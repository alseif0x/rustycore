// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Visibility notification and nearby-object resolution.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub const fn visibility_range(&self) -> f32 {
        self.visible_distance
    }

    pub fn nearby_cell_guids_like_cpp(&self, x: f32, y: f32, radius: f32) -> NearbyCellGuids {
        if !is_valid_map_coord_2d(x, y) {
            return NearbyCellGuids::default();
        }

        let area = calculate_cell_area_like_cpp(x, y, radius);
        let mut result = NearbyCellGuids::default();
        for cell_x in area.low_bound.x_coord..=area.high_bound.x_coord {
            for cell_y in area.low_bound.y_coord..=area.high_bound.y_coord {
                result.visited_cells += 1;
                let cell = Cell::from_cell_coord(CellCoord::new(cell_x, cell_y));
                let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
                else {
                    continue;
                };
                let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                    continue;
                };
                result.merge_world(&local_cell.world_objects);
                result.merge_grid(&local_cell.grid_objects);
            }
        }

        result
    }

    pub fn visit_nearby_cells_of_like_cpp(
        &self,
        centers: impl IntoIterator<Item = NearbyCellVisitCenter>,
    ) -> NearbyCellVisitPlan {
        let mut marked_cells = HashSet::new();
        let mut marked_cells_in_visit_order = Vec::new();
        let mut nearby = NearbyCellGuids::default();
        let mut skipped_missing_centers = Vec::new();
        let mut skipped_invalid_position_centers = Vec::new();

        for center in centers {
            let Some(object) = self.map_object(center.guid) else {
                skipped_missing_centers.push(center.guid);
                continue;
            };
            let position = object.position();
            if !is_valid_map_coord_2d(position.x, position.y) {
                skipped_invalid_position_centers.push(center.guid);
                continue;
            }

            let area =
                calculate_cell_area_like_cpp(position.x, position.y, center.activation_radius);
            for cell_x in area.low_bound.x_coord..=area.high_bound.x_coord {
                for cell_y in area.low_bound.y_coord..=area.high_bound.y_coord {
                    let cell_coord = CellCoord::new(cell_x, cell_y);
                    if !marked_cells.insert(cell_coord) {
                        continue;
                    }

                    marked_cells_in_visit_order.push(cell_coord);
                    nearby.visited_cells += 1;
                    let cell = Cell::from_cell_coord(cell_coord);
                    let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
                    else {
                        continue;
                    };
                    let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                        continue;
                    };
                    nearby.merge_world(&local_cell.world_objects);
                    nearby.merge_grid(&local_cell.grid_objects);
                }
            }
        }

        NearbyCellVisitPlan {
            marked_cells: marked_cells_in_visit_order,
            nearby,
            skipped_missing_centers,
            skipped_invalid_position_centers,
        }
    }

    pub fn object_update_plan_for_nearby_like_cpp(
        &self,
        nearby: &NearbyCellGuids,
        diff_ms: u32,
    ) -> ObjectUpdatePlan {
        let mut update_guids = Vec::new();
        for guid in nearby
            .world
            .creatures
            .iter()
            .chain(nearby.world.dynamic_objects.iter())
            .chain(nearby.grid.creatures.iter())
            .chain(nearby.grid.gameobjects.iter())
            .chain(nearby.grid.dynamic_objects.iter())
            .chain(nearby.grid.area_triggers.iter())
            .chain(nearby.grid.scene_objects.iter())
            .chain(nearby.grid.conversations.iter())
        {
            if self
                .map_object(*guid)
                .is_some_and(|object| object.object().is_in_world())
            {
                update_guids.push(*guid);
            }
        }

        update_guids.sort();
        update_guids.dedup();
        ObjectUpdatePlan {
            diff_ms,
            update_guids,
        }
    }

    pub fn reset_notify_flags_for_cells_like_cpp(
        &mut self,
        cells: impl IntoIterator<Item = CellCoord>,
    ) -> ResetNotifyFlagsOutcome {
        let mut reset_player_guids = Vec::new();
        let mut reset_creature_guids = Vec::new();
        let mut missing_guids = Vec::new();

        for cell_coord in cells {
            let cell = Cell::from_cell_coord(cell_coord);
            let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y())) else {
                continue;
            };
            let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                continue;
            };

            reset_player_guids.extend(local_cell.world_objects.players.iter().copied());
            reset_creature_guids.extend(local_cell.grid_objects.creatures.iter().copied());
            reset_creature_guids.extend(local_cell.world_objects.creatures.iter().copied());
        }

        sort_dedup(&mut reset_player_guids);
        sort_dedup(&mut reset_creature_guids);

        for guid in reset_player_guids
            .iter()
            .chain(reset_creature_guids.iter())
            .copied()
        {
            let Some(record) = self.entity_world.get_mut(&guid) else {
                missing_guids.push(guid);
                continue;
            };
            record.object_mut().object_mut().reset_all_notifies();
        }

        ResetNotifyFlagsOutcome {
            reset_player_guids,
            reset_creature_guids,
            missing_guids,
        }
    }

    pub(super) fn player_seer_needs_notify_visibility_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> bool {
        self.player_viewpoint_guid_like_cpp(player_guid)
            .is_some_and(|viewpoint_guid| self.object_needs_notify_visibility(viewpoint_guid))
    }

    pub(super) fn object_needs_notify_visibility(&self, guid: ObjectGuid) -> bool {
        self.map_object(guid).is_some_and(|object| {
            object
                .object()
                .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
        })
    }
}
