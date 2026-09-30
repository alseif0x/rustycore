// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Deferred removal and grid-container switching in their original drain order.

use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SwitchGridContainersOutcomeLikeCpp {
    executed: bool,
    missing_or_stale: bool,
    unsupported_kind: bool,
    permanent_world_object: bool,
    invalid_or_unloaded_grid: bool,
}

impl SwitchGridContainersOutcomeLikeCpp {
    const fn executed() -> Self {
        Self {
            executed: true,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    const fn missing_or_stale() -> Self {
        Self {
            executed: false,
            missing_or_stale: true,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    const fn unsupported_kind() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: true,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    const fn permanent_world_object() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: true,
            invalid_or_unloaded_grid: false,
        }
    }

    const fn invalid_or_unloaded_grid() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: true,
        }
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
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
            let Some(kind) = self.map_object_record(guid).map(|record| record.kind()) else {
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
    pub(in crate::map) fn enqueue_object_to_remove_for_test(&mut self, guid: ObjectGuid) {
        self.objects_to_remove.insert(guid);
    }
}
