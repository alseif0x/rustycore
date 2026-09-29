// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Object relocation and grid/cell transitions.

use super::*;

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
            .and_then(MapObjectRecord::creature_mut)
        {
            creature.unit_mut().remove_from_world_like_cpp();
        }
    }

    /// C++ `Map::AddObjectToRemoveList` represented over canonical map records.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2547-2555` asserts same map/instance, marks destroyed, runs
    ///   `CleanupsBeforeDelete(false)`, and inserts into `i_objectsToRemove`.
    /// - `Object.cpp:1826-1835` delegates `WorldObject::AddObjectToRemoveList` to
    ///   the owning map when present.
    ///
    /// Divergence note: the C++ `std::set` insert is deduplicated, but the
    /// cleanup call happens before insertion; this Rust seam preserves that order
    /// and reports `duplicate=true` while still incrementing represented cleanup.
    pub fn add_object_to_remove_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> AddObjectToRemoveListOutcomeLikeCpp {
        let Some(record) = self.entity_world.get_mut(&guid) else {
            return AddObjectToRemoveListOutcomeLikeCpp {
                guid,
                queued: false,
                duplicate: false,
                missing_or_stale: true,
                unsupported_kind: None,
                cleanup_before_delete_count: 0,
            };
        };

        let kind = record.kind();
        debug_assert_eq!(record.object().map_id(), self.map_id);
        debug_assert_eq!(record.object().instance_id(), self.instance_id);

        let cleanup_before_delete_count =
            cleanup_map_object_record_before_delete_like_cpp(record, kind, false);
        let inserted = self.objects_to_remove.insert(guid);
        AddObjectToRemoveListOutcomeLikeCpp {
            guid,
            queued: inserted,
            duplicate: !inserted,
            missing_or_stale: false,
            unsupported_kind: remove_list_grid_kind_like_cpp(kind)
                .is_none()
                .then_some(kind),
            cleanup_before_delete_count,
        }
    }

    /// C++ `Unit::RemoveAllAreaTriggers` represented over map-owned AreaTriggers.
    ///
    /// C++ anchors:
    /// - `Player.cpp:1421-1422` calls `RemoveAllAreaTriggers()` during accepted
    ///   inter-map `Player::TeleportTo`, immediately after `RemoveAllDynObjects()`.
    /// - `Unit.cpp:5347-5351` repeatedly removes every AreaTrigger owned by the
    ///   Unit (`m_areaTrigger.back()->Remove()`).
    /// - `AreaTrigger.cpp:366-372` routes `Remove()` through the owning map
    ///   remove list only while the object is in world; Rust reuses
    ///   `remove_from_map_like_cpp(..., true)` to keep physical removal in one
    ///   canonical map path.
    ///
    /// Scope: source-of-truth is this canonical `Map::entity_world` store. This
    /// does not model the exact C++ `Unit::m_areaTrigger` vector ordering,
    /// destroy-packet fanout, ObjectAccessor/session mirrors, AI target list
    /// exits, scripts, DB, or cross-map lookup beyond this map.
    pub fn remove_all_area_triggers_for_caster_like_cpp(
        &mut self,
        caster_guid: ObjectGuid,
    ) -> RemoveAllAreaTriggersForCasterOutcomeLikeCpp {
        let mut guids = self
            .entity_world
            .iter()
            .filter_map(|(guid, record)| {
                if record.kind() != AccessorObjectKind::AreaTrigger {
                    return None;
                }
                let area_trigger = record.area_trigger()?;
                (area_trigger.caster_guid() == caster_guid).then_some(*guid)
            })
            .collect::<Vec<_>>();
        guids.sort_by_key(ObjectGuid::to_raw_bytes);

        let mut outcome = RemoveAllAreaTriggersForCasterOutcomeLikeCpp {
            caster_guid,
            candidates: guids.len(),
            removed: 0,
            missing_or_stale: 0,
            remove_errors: 0,
        };

        for guid in guids {
            match self.remove_from_map_like_cpp(guid, true) {
                Ok(_) => {
                    outcome.removed += 1;
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => {
                    outcome.missing_or_stale += 1;
                }
                Err(_) => {
                    outcome.remove_errors += 1;
                }
            }
        }

        outcome
    }

    /// C++ `Map::RemoveAllObjectsInRemoveList` physical map-local drain.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2574-2594` drains `i_objectsToSwitch` first and calls
    ///   `SwitchGridContainers<Creature>` for non-permanent Unit objects.
    /// - `Map.cpp:2596-2646` then drains `i_objectsToRemove`; supported grid
    ///   object types call `RemoveFromMap(..., true)`, Creature runs a second
    ///   `CleanupsBeforeDelete()` immediately before removal, and non-grid types
    ///   are logged/ignored.
    /// - `Map.cpp:933-951` shows `RemoveFromMap(T*, true)` does the physical map
    ///   removal/reset/delete path.
    pub fn remove_all_objects_in_remove_list_like_cpp(
        &mut self,
    ) -> RemoveAllObjectsInRemoveListOutcomeLikeCpp {
        let mut switches = self.objects_to_switch.drain().collect::<Vec<_>>();
        switches.sort_by_key(|(guid, _)| guid.to_raw_bytes());
        let mut outcome = RemoveAllObjectsInRemoveListOutcomeLikeCpp {
            switch_processed: switches.len(),
            ..Default::default()
        };

        for (guid, on) in switches {
            let switch = self.switch_grid_containers_like_cpp(guid, on);
            if switch.executed {
                outcome.switch_executed += 1;
            } else if switch.missing_or_stale {
                outcome.switch_missing_or_stale += 1;
            } else if switch.unsupported_kind {
                outcome.switch_unsupported_kinds += 1;
            } else if switch.permanent_world_object {
                outcome.switch_permanent_world_objects += 1;
            } else if switch.invalid_or_unloaded_grid {
                outcome.switch_invalid_or_unloaded_grid += 1;
            }
        }

        while let Some(guid) = self.objects_to_remove.iter().next().copied() {
            self.objects_to_remove.remove(&guid);
            outcome.processed += 1;
            let Some(kind) = self.map_object_record(guid).map(MapObjectRecord::kind) else {
                outcome.missing_or_stale += 1;
                continue;
            };

            if remove_list_grid_kind_like_cpp(kind).is_none() {
                outcome.unsupported_kinds += 1;
                continue;
            }

            if matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet) {
                if let Some(record) = self.entity_world.get_mut(&guid) {
                    outcome.creature_second_cleanup_count +=
                        cleanup_map_object_record_before_delete_like_cpp(record, kind, true);
                }
            }

            match self.remove_from_map_like_cpp(guid, true) {
                Ok(removed) => {
                    outcome.removed += 1;
                    if let Some(cleanup) = removed.dynamic_object_remove_cleanup {
                        if cleanup.removed_aura_pending_delete {
                            outcome.dynamic_object_remove_aura_cleanup_count += 1;
                        }
                        if cleanup.unbound_caster.is_some() {
                            outcome.dynamic_object_unbound_caster_count += 1;
                        }
                    }
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => outcome.missing_or_stale += 1,
                Err(_) => outcome.remove_errors += 1,
            }
        }

        outcome
    }

    /// C++ `Unit::RemoveAllDynObjects` represented over map-owned DynamicObjects.
    ///
    /// C++ anchors:
    /// - `Player.cpp:1418-1419` calls `RemoveAllDynObjects()` during accepted
    ///   inter-map `Player::TeleportTo`.
    /// - `Unit.cpp:5169-5174` repeatedly removes every DynamicObject owned by
    ///   the Unit (`m_dynObj.back()->Remove()`).
    /// - `DynamicObject.cpp:167-171` routes `Remove()` through the owning
    ///   map remove list; Rust reuses `remove_from_map_like_cpp(..., true)` so
    ///   aura and caster-unbind cleanup stays in the canonical remove path.
    ///
    /// Scope: source-of-truth is this canonical `Map::entity_world` store. This
    /// does not model the C++ `Unit::m_dynObj` vector ordering, session fanout,
    /// destroy packets, ObjectAccessor mirrors, scripts, DB, or cross-map
    /// instance lookup beyond this map.
    pub fn remove_all_dynamic_objects_for_caster_like_cpp(
        &mut self,
        caster_guid: ObjectGuid,
    ) -> RemoveAllDynamicObjectsForCasterOutcomeLikeCpp {
        let mut guids = self
            .entity_world
            .iter()
            .filter_map(|(guid, record)| {
                if record.kind() != AccessorObjectKind::DynamicObject {
                    return None;
                }
                let dynamic_object = record.dynamic_object()?;
                (dynamic_object.caster_guid() == caster_guid).then_some(*guid)
            })
            .collect::<Vec<_>>();
        guids.sort_by_key(ObjectGuid::to_raw_bytes);

        let mut outcome = RemoveAllDynamicObjectsForCasterOutcomeLikeCpp {
            caster_guid,
            candidates: guids.len(),
            removed: 0,
            missing_or_stale: 0,
            remove_errors: 0,
            dynamic_object_remove_aura_cleanup_count: 0,
            dynamic_object_unbound_caster_count: 0,
        };

        for guid in guids {
            match self.remove_from_map_like_cpp(guid, true) {
                Ok(removed) => {
                    outcome.removed += 1;
                    if let Some(cleanup) = removed.dynamic_object_remove_cleanup {
                        if cleanup.removed_aura_pending_delete {
                            outcome.dynamic_object_remove_aura_cleanup_count += 1;
                        }
                        if cleanup.unbound_caster.is_some() {
                            outcome.dynamic_object_unbound_caster_count += 1;
                        }
                    }
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => {
                    outcome.missing_or_stale += 1;
                }
                Err(_) => {
                    outcome.remove_errors += 1;
                }
            }
        }

        outcome
    }

    /// C++ `Map::SwitchGridContainers<Creature>` represented for Creature/Pet.
    ///
    /// C++ anchors:
    /// - `Map.cpp:260-305` computes the current cell, returns on invalid coords or
    ///   unloaded grid, moves Unit GUID between `grid_objects.creatures` and
    ///   `world_objects.creatures`, then writes `Creature::m_isTempWorldObject`.
    /// - `Object.cpp:918-925` makes `WorldObject::IsWorldObject` true for a
    ///   Creature with `m_isTempWorldObject`, while `Object.h:723-724` keeps
    ///   permanent world-object state in base `m_isWorldObject`.
    fn switch_grid_containers_like_cpp(
        &mut self,
        guid: ObjectGuid,
        on: bool,
    ) -> SwitchGridContainersOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return SwitchGridContainersOutcomeLikeCpp::missing_or_stale();
        };
        let kind = record.kind();
        if !switch_list_unit_kind_like_cpp(kind) {
            return SwitchGridContainersOutcomeLikeCpp::unsupported_kind();
        }
        if record.object().is_world_object() {
            return SwitchGridContainersOutcomeLikeCpp::permanent_world_object();
        }

        let position = record.object().position();
        if !is_valid_map_coord_2d(position.x, position.y) {
            return SwitchGridContainersOutcomeLikeCpp::invalid_or_unloaded_grid();
        }

        let cell = Cell::from_world(position.x, position.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        if !self.is_grid_loaded(grid) {
            return SwitchGridContainersOutcomeLikeCpp::invalid_or_unloaded_grid();
        }

        let Some(ngrid) = self.get_ngrid_mut(grid) else {
            return SwitchGridContainersOutcomeLikeCpp::invalid_or_unloaded_grid();
        };
        let Some(local_cell) = ngrid.get_grid_type_mut(cell.cell_x(), cell.cell_y()) else {
            return SwitchGridContainersOutcomeLikeCpp::invalid_or_unloaded_grid();
        };

        if on {
            local_cell.grid_objects.creatures.remove(&guid);
            local_cell.world_objects.creatures.insert(guid);
        } else {
            local_cell.world_objects.creatures.remove(&guid);
            local_cell.grid_objects.creatures.insert(guid);
        }

        if let Some(record) = self.entity_world.get_mut(&guid) {
            set_record_temp_world_object_like_cpp(record, on);
        }

        SwitchGridContainersOutcomeLikeCpp::executed()
    }

    pub fn objects_to_remove_count_like_cpp(&self) -> usize {
        self.objects_to_remove.len()
    }

    #[cfg(test)]
    pub(super) fn enqueue_object_to_remove_for_test(&mut self, guid: ObjectGuid) {
        self.objects_to_remove.insert(guid);
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
            let mut record = self
                .remove_map_object(guid)
                .expect("record was just observed");
            record.object_mut().relocate(new_position);
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
        record.object_mut().relocate(new_position);
        record
            .object_mut()
            .set_current_cell(new_cell.cell_x(), new_cell.cell_y());
        self.insert_map_object_record(record)
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

    /// Live represented C++ `Map::Update` source selection for
    /// `ProcessRelocationNotifies(t_diff)` (`Map.cpp:692-717,797-805,830-905`).
    ///
    /// Source of truth stays map-owned canonical `entity_world`: the same typed
    /// player sources used by `ObjectUpdater` include viewpoints, far combat
    /// creatures, aura casters and summons. Active non-Players remain separate
    /// sources. The existing visit/relocation helpers consume marked cells and
    /// reset notify flags; packet, ObjectAccessor and AI side effects remain
    /// outside this map phase.
    pub fn process_live_relocation_notifies_like_cpp(
        &mut self,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
    ) -> ProcessRelocationNotifiesOutcome {
        let mut player_sources = self.map_update_player_sources_for_current_tick_like_cpp();
        let active_non_player_guids = self.represented_active_non_player_sources_like_cpp();
        player_sources.sort_by_key(|source| source.player_guid);
        player_sources.dedup_by_key(|source| source.player_guid);

        let visit_plan = self.map_update_visit_plan_like_cpp(
            player_sources,
            active_non_player_guids,
            std::iter::empty(),
            diff_ms,
        );
        if !visit_plan.process_relocation_notifies {
            return ProcessRelocationNotifiesOutcome::default();
        }

        let centers =
            visit_plan
                .nearby_visit_centers
                .into_iter()
                .map(|guid| NearbyCellVisitCenter {
                    guid,
                    activation_radius: self.grid_activation_range_for_guid_like_cpp(guid),
                });
        let nearby_plan = self.visit_nearby_cells_of_like_cpp(centers);
        self.process_relocation_notifies_like_cpp(
            nearby_plan.marked_cells,
            diff_ms,
            visibility_notify_period_ms,
            std::iter::empty(),
        )
    }

    pub fn process_relocation_notifies_plan_like_cpp(
        &mut self,
        marked_cells: impl IntoIterator<Item = CellCoord>,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
    ) -> RelocationNotifyProcessPlan {
        let marked_cells: HashSet<_> = marked_cells.into_iter().collect();
        let mut delayed_relocation_cells = Vec::new();
        let mut reset_notify_cells = Vec::new();
        let mut reset_timer_grids = Vec::new();
        let mut expired_active_grids = Vec::new();

        for grid_x in 0..MAX_NUMBER_OF_GRIDS {
            for grid_y in 0..MAX_NUMBER_OF_GRIDS {
                let coord = GridCoord::new(grid_x, grid_y);
                let Some(grid) = self.get_ngrid_mut(coord) else {
                    continue;
                };
                if grid.state() != GridStateKind::Active {
                    continue;
                }

                grid.info_mut()
                    .relocation_timer_mut()
                    .tracker_update(diff_ms);
                if !grid.info().relocation_timer().tracker_passed() {
                    continue;
                }

                expired_active_grids.push(coord);
                delayed_relocation_cells
                    .extend(marked_cells_in_grid_like_cpp(coord, &marked_cells));
            }
        }

        for coord in &expired_active_grids {
            let Some(grid) = self.get_ngrid_mut(*coord) else {
                continue;
            };
            if grid.state() != GridStateKind::Active {
                continue;
            }
            if !grid.info().relocation_timer().tracker_passed() {
                continue;
            }

            grid.info_mut()
                .relocation_timer_mut()
                .tracker_reset(diff_ms, visibility_notify_period_ms);
            reset_timer_grids.push(*coord);
            reset_notify_cells.extend(marked_cells_in_grid_like_cpp(*coord, &marked_cells));
        }

        RelocationNotifyProcessPlan {
            diff_ms,
            delayed_relocation_cells,
            reset_notify_cells,
            reset_timer_grids,
        }
    }

    pub fn process_relocation_notifies_like_cpp(
        &mut self,
        marked_cells: impl IntoIterator<Item = CellCoord>,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
        invalid_non_self_viewpoints: impl IntoIterator<Item = ObjectGuid>,
    ) -> ProcessRelocationNotifiesOutcome {
        let process_plan = self.process_relocation_notifies_plan_like_cpp(
            marked_cells,
            diff_ms,
            visibility_notify_period_ms,
        );
        let delayed_plan = self.delayed_unit_relocation_for_cells_like_cpp(
            process_plan.delayed_relocation_cells.iter().copied(),
            invalid_non_self_viewpoints,
        );
        // C++ runs DelayedUnitRelocation's CreatureRelocationNotifier and
        // PlayerRelocationNotifier while NOTIFY_VISIBILITY_CHANGED is still set,
        // before ResetNotifier clears the cell. Rust exposes only represented
        // visibility/AI evidence here: no packets, sessions, ObjectAccessor fanout,
        // real UpdateObjectVisibility, or SendObjectUpdates are executed.
        let visibility_plans = self.delayed_unit_relocation_visibility_plans_like_cpp(
            &delayed_plan,
            self.delayed_player_relocation_contexts_from_plan_like_cpp(&delayed_plan),
            self.delayed_creature_relocation_contexts_from_plan_like_cpp(&delayed_plan),
        );
        let reset_outcome = self
            .reset_notify_flags_for_cells_like_cpp(process_plan.reset_notify_cells.iter().copied());

        ProcessRelocationNotifiesOutcome {
            process_plan,
            delayed_plan,
            visibility_plans,
            reset_outcome,
        }
    }

    pub fn delayed_unit_relocation_for_cells_like_cpp(
        &self,
        cells: impl IntoIterator<Item = CellCoord>,
        invalid_non_self_viewpoints: impl IntoIterator<Item = ObjectGuid>,
    ) -> DelayedUnitRelocationForCellsPlan {
        let invalid_non_self_viewpoints: HashSet<_> =
            invalid_non_self_viewpoints.into_iter().collect();
        let mut cell_plans = Vec::new();

        for cell_coord in cells {
            let nearby = self.exact_cell_guids_like_cpp(cell_coord);
            let creatures_needing_notify = nearby
                .world
                .creatures
                .iter()
                .chain(nearby.grid.creatures.iter())
                .copied()
                .filter(|guid| self.object_needs_notify_visibility(*guid));
            let mut plan = DelayedUnitRelocationPlan::from_nearby_like_cpp(
                &nearby,
                creatures_needing_notify,
                std::iter::empty::<ObjectGuid>(),
                std::iter::empty::<ObjectGuid>(),
            );
            let mut players: Vec<_> = nearby.world.players.iter().copied().collect();
            players.sort();
            for player_guid in players {
                let Some(viewpoint_guid) = self.player_viewpoint_guid_like_cpp(player_guid) else {
                    continue;
                };
                if !self.object_needs_notify_visibility(viewpoint_guid) {
                    continue;
                }
                if player_guid != viewpoint_guid
                    && (invalid_non_self_viewpoints.contains(&player_guid)
                        || invalid_non_self_viewpoints.contains(&viewpoint_guid)
                        || self.viewpoint_has_invalid_position_like_cpp(viewpoint_guid))
                {
                    plan.skipped_invalid_viewpoints.push(player_guid);
                    continue;
                }
                plan.player_relocations.push(player_guid);
            }
            sort_dedup(&mut plan.player_relocations);
            sort_dedup(&mut plan.skipped_invalid_viewpoints);
            if !plan.creature_relocations.is_empty()
                || !plan.player_relocations.is_empty()
                || !plan.skipped_invalid_viewpoints.is_empty()
            {
                cell_plans.push(DelayedUnitRelocationCellPlan { cell_coord, plan });
            }
        }

        DelayedUnitRelocationForCellsPlan { cell_plans }
    }

    pub fn delayed_unit_relocation_visibility_plans_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
        player_contexts: impl IntoIterator<Item = DelayedPlayerRelocationContext>,
        creature_contexts: impl IntoIterator<Item = DelayedCreatureRelocationContext>,
    ) -> DelayedUnitRelocationVisibilityPlans {
        let player_contexts: HashMap<_, _> = player_contexts
            .into_iter()
            .map(|context| (context.player_guid, context))
            .collect();
        let creature_contexts: HashMap<_, _> = creature_contexts
            .into_iter()
            .map(|context| (context.creature_guid, context))
            .collect();
        let mut creature_plans = Vec::new();
        let mut player_plans = Vec::new();
        let mut skipped_missing_sources = Vec::new();
        let mut skipped_invalid_source_positions = Vec::new();
        let mut missing_player_contexts = Vec::new();

        for cell_plan in &delayed_plan.cell_plans {
            for creature_guid in &cell_plan.plan.creature_relocations {
                let Some(creature) = self.map_object(*creature_guid) else {
                    skipped_missing_sources.push(*creature_guid);
                    continue;
                };
                let position = creature.position();
                if !is_valid_map_coord_2d(position.x, position.y) {
                    skipped_invalid_source_positions.push(*creature_guid);
                    continue;
                }

                let nearby = self.nearby_cell_guids_like_cpp(
                    position.x,
                    position.y,
                    MAX_VISIBILITY_DISTANCE + creature.combat_reach(),
                );
                let player_seers_needing_notify = nearby
                    .world
                    .players
                    .iter()
                    .copied()
                    .filter(|guid| self.player_seer_needs_notify_visibility_like_cpp(*guid));
                let creatures_needing_notify = nearby
                    .world
                    .creatures
                    .iter()
                    .chain(nearby.grid.creatures.iter())
                    .copied()
                    .filter(|guid| self.object_needs_notify_visibility(*guid));
                let Some(creature_context) = creature_contexts.get(creature_guid) else {
                    skipped_missing_sources.push(*creature_guid);
                    continue;
                };
                let source_creature_alive = creature_context.source_creature_alive;
                let visibility_plan = CreatureRelocationVisibilityPlan::from_nearby_like_cpp(
                    *creature_guid,
                    source_creature_alive,
                    &nearby,
                    player_seers_needing_notify,
                    creatures_needing_notify,
                );
                creature_plans.push(CreatureDelayedRelocationVisibilityPlan {
                    creature_guid: *creature_guid,
                    cell_coord: cell_plan.cell_coord,
                    nearby,
                    visibility_plan,
                });
            }

            for player_guid in &cell_plan.plan.player_relocations {
                let Some(context) = player_contexts.get(player_guid) else {
                    missing_player_contexts.push(*player_guid);
                    continue;
                };
                let Some(viewpoint) = self.map_object(context.viewpoint_guid) else {
                    skipped_missing_sources.push(context.viewpoint_guid);
                    continue;
                };
                let position = viewpoint.position();
                if !is_valid_map_coord_2d(position.x, position.y) {
                    skipped_invalid_source_positions.push(context.viewpoint_guid);
                    continue;
                }

                let nearby = self.nearby_cell_guids_like_cpp(
                    position.x,
                    position.y,
                    MAX_VISIBILITY_DISTANCE + viewpoint.combat_reach(),
                );
                let player_seers_needing_notify = nearby
                    .world
                    .players
                    .iter()
                    .copied()
                    .filter(|guid| self.player_seer_needs_notify_visibility_like_cpp(*guid));
                let creatures_needing_notify = nearby
                    .world
                    .creatures
                    .iter()
                    .chain(nearby.grid.creatures.iter())
                    .copied()
                    .filter(|guid| self.object_needs_notify_visibility(*guid));
                let visibility_plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
                    *player_guid,
                    context.previous_client_guids.iter().copied(),
                    &nearby,
                    context.relocated_for_ai,
                    player_seers_needing_notify,
                    creatures_needing_notify,
                );
                player_plans.push(PlayerDelayedRelocationVisibilityPlan {
                    player_guid: *player_guid,
                    viewpoint_guid: context.viewpoint_guid,
                    cell_coord: cell_plan.cell_coord,
                    nearby,
                    visibility_plan,
                });
            }
        }

        sort_dedup(&mut skipped_missing_sources);
        sort_dedup(&mut skipped_invalid_source_positions);
        sort_dedup(&mut missing_player_contexts);

        DelayedUnitRelocationVisibilityPlans {
            creature_plans,
            player_plans,
            skipped_missing_sources,
            skipped_invalid_source_positions,
            missing_player_contexts,
        }
    }

    pub(super) fn delayed_player_relocation_contexts_from_plan_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
    ) -> Vec<DelayedPlayerRelocationContext> {
        let mut player_guids: Vec<_> = delayed_plan
            .cell_plans
            .iter()
            .flat_map(|cell_plan| cell_plan.plan.player_relocations.iter().copied())
            .collect();
        sort_dedup(&mut player_guids);

        player_guids
            .into_iter()
            .filter_map(|player_guid| {
                let viewpoint_guid = self.player_viewpoint_guid_like_cpp(player_guid)?;
                Some(DelayedPlayerRelocationContext {
                    player_guid,
                    viewpoint_guid,
                    // Map-owned live relocation currently has no canonical client
                    // object-list source; keep this empty as an explicit visibility
                    // fanout gap rather than inventing session state.
                    previous_client_guids: Vec::new(),
                    relocated_for_ai: viewpoint_guid == player_guid,
                })
            })
            .collect()
    }

    fn delayed_creature_relocation_contexts_from_plan_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
    ) -> Vec<DelayedCreatureRelocationContext> {
        let mut creature_guids: Vec<_> = delayed_plan
            .cell_plans
            .iter()
            .flat_map(|cell_plan| cell_plan.plan.creature_relocations.iter().copied())
            .collect();
        sort_dedup(&mut creature_guids);

        creature_guids
            .into_iter()
            .filter_map(|creature_guid| {
                let creature = self.get_typed_creature(creature_guid)?;
                Some(DelayedCreatureRelocationContext {
                    creature_guid,
                    source_creature_alive: creature.is_alive(),
                })
            })
            .collect()
    }
}
