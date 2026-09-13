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

    /// Resolve the activation radius used by TrinityCore's
    /// `WorldObject::GetGridActivationRange` for a map-owned source.
    ///
    /// C++ (`Entities/Object/Object.cpp:1433-1450`) gives active objects and
    /// Players the map visibility range (with the `CinematicMgr::IsOnCinematic`
    /// instance-distance override), while an ordinary Creature uses its
    /// canonical `m_SightDistance`. Pets follow the Creature branch through
    /// their embedded Creature. Unsupported or missing records are fail-closed
    /// with a zero radius; callers still validate the source position before
    /// visiting cells. The represented cinematic state has the same active
    /// camera cursor as C++; resolving a FlyByCamera row is still outside this
    /// map-owned selector.
    pub(crate) fn grid_activation_range_for_guid_like_cpp(&self, guid: ObjectGuid) -> f32 {
        let Some(record) = self.map_object_record(guid) else {
            return 0.0;
        };

        if record.kind() == AccessorObjectKind::Player {
            let cinematic_active = record.player().is_some_and(|player| {
                player.gameplay_state().cinematic.camera_index_like_cpp() >= 0
            });
            if cinematic_active {
                return self
                    .visible_distance
                    .max(wow_entities::DEFAULT_VISIBILITY_INSTANCE);
            }
            return self.visible_distance;
        }

        if record.object().is_active() {
            return self.visible_distance;
        }

        match record.kind() {
            AccessorObjectKind::Creature => record
                .creature()
                .map(|creature| creature.sight_distance())
                .unwrap_or(0.0),
            AccessorObjectKind::Pet => record
                .pet()
                .map(|pet| pet.creature().sight_distance())
                .unwrap_or(0.0),
            _ => 0.0,
        }
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

    /// Build the production `ObjectUpdater` selection for the current map
    /// incarnation. This keeps source discovery and nearby-cell marking on the
    /// canonical map owner; the caller may then consume the owned GUID plan
    /// without retaining any map borrow across delivery or I/O.
    ///
    /// C++ source categories are taken from `Map.cpp:701-754`: in-world
    /// players, viewpoints, far PvE combat creatures, out-of-range aura
    /// casters, summons and active non-Players. Unsupported runtime references
    /// remain absent rather than being fabricated. Players and Corpses are not
    /// included in the resulting ObjectUpdater GUID set.
    pub fn object_update_plan_for_current_tick_like_cpp(&self, diff_ms: u32) -> ObjectUpdatePlan {
        let mut sources = Vec::new();
        for (guid, record) in self.entity_world.iter() {
            if record.kind() != AccessorObjectKind::Player
                || !record.object().object().is_in_world()
            {
                continue;
            }

            let player_object = record.object();
            let Some(player) = record.player() else {
                continue;
            };
            let is_far_in_world_unit = |target_guid: ObjectGuid| {
                let Some(target) = self.map_object(target_guid) else {
                    return false;
                };
                target.object().is_in_world()
                    && !player_object.is_within_dist_in_map(target, self.visible_distance, false)
            };

            let far_combat_unit_guids = player
                .unit()
                .subsystems()
                .combat
                .pve_refs
                .keys()
                .copied()
                .filter(|target_guid| {
                    is_far_in_world_unit(*target_guid)
                        && self.map_object_record(*target_guid).is_some_and(|target| {
                            matches!(
                                target.kind(),
                                AccessorObjectKind::Creature | AccessorObjectKind::Pet
                            )
                        })
                })
                .collect();
            let far_aura_caster_guids = player
                .unit()
                .subsystems()
                .auras
                .applied_auras
                .iter()
                .map(|aura| aura.caster_guid)
                .filter(|target_guid| {
                    is_far_in_world_unit(*target_guid)
                        && self.map_object_record(*target_guid).is_some_and(|target| {
                            matches!(
                                target.kind(),
                                AccessorObjectKind::Creature | AccessorObjectKind::Pet
                            )
                        })
                })
                .collect();
            let far_summon_guids = player
                .unit()
                .subsystems()
                .control
                .summon_slots
                .iter()
                .copied()
                .filter(|target_guid| {
                    !target_guid.is_empty()
                        && is_far_in_world_unit(*target_guid)
                        && self.map_object_record(*target_guid).is_some_and(|target| {
                            matches!(
                                target.kind(),
                                AccessorObjectKind::Creature | AccessorObjectKind::Pet
                            )
                        })
                })
                .collect();

            sources.push(MapUpdatePlayerSources {
                player_guid: *guid,
                viewpoint_guid: (!player.active_data().farsight_object.is_empty())
                    .then_some(player.active_data().farsight_object),
                far_combat_unit_guids,
                far_aura_caster_guids,
                far_summon_guids,
            });
        }

        let visit_plan = self.map_update_visit_plan_like_cpp(
            sources,
            self.represented_active_non_player_sources_like_cpp(),
            std::iter::empty(),
            diff_ms,
        );
        let centers =
            visit_plan
                .nearby_visit_centers
                .into_iter()
                .map(|guid| NearbyCellVisitCenter {
                    guid,
                    activation_radius: self.grid_activation_range_for_guid_like_cpp(guid),
                });
        let nearby = self.visit_nearby_cells_of_like_cpp(centers);
        self.object_update_plan_for_nearby_like_cpp(&nearby.nearby, diff_ms)
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
