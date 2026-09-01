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
            .map_objects
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
        let Some(record) = self.map_objects.get_mut(&guid) else {
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
    /// Scope: source-of-truth is this canonical `Map::map_objects` store. This
    /// does not model the exact C++ `Unit::m_areaTrigger` vector ordering,
    /// destroy-packet fanout, ObjectAccessor/session mirrors, AI target list
    /// exits, scripts, DB, or cross-map lookup beyond this map.
    pub fn remove_all_area_triggers_for_caster_like_cpp(
        &mut self,
        caster_guid: ObjectGuid,
    ) -> RemoveAllAreaTriggersForCasterOutcomeLikeCpp {
        let mut guids = self
            .map_objects
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
                if let Some(record) = self.map_objects.get_mut(&guid) {
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
    /// Scope: source-of-truth is this canonical `Map::map_objects` store. This
    /// does not model the C++ `Unit::m_dynObj` vector ordering, session fanout,
    /// destroy packets, ObjectAccessor mirrors, scripts, DB, or cross-map
    /// instance lookup beyond this map.
    pub fn remove_all_dynamic_objects_for_caster_like_cpp(
        &mut self,
        caster_guid: ObjectGuid,
    ) -> RemoveAllDynamicObjectsForCasterOutcomeLikeCpp {
        let mut guids = self
            .map_objects
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

        if let Some(record) = self.map_objects.get_mut(&guid) {
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

    pub(super) fn remove_spawn_id_index_entry_like_cpp(
        index: &mut HashMap<SpawnId, HashSet<ObjectGuid>>,
        spawn_id: SpawnId,
        guid: ObjectGuid,
    ) {
        if spawn_id == 0 {
            return;
        }

        if let Some(guids) = index.get_mut(&spawn_id) {
            guids.remove(&guid);
            if guids.is_empty() {
                index.remove(&spawn_id);
            }
        }
    }

    fn remove_creature_from_formation_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<CreatureRemoveFormationOutcomeLikeCpp> {
        let (spawn_id, leader_spawn_id) = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::Creature)
            .and_then(MapObjectRecord::creature)
            .filter(|creature| creature.unit().world().object().is_in_world())
            .and_then(|creature| {
                let leader_spawn_id = creature.formation_info_like_cpp()?.leader_spawn_id;
                Some((creature.spawn_id(), leader_spawn_id))
            })?;

        let Some(group) = self
            .creature_group_holder_like_cpp
            .get_mut(&leader_spawn_id)
        else {
            return Some(CreatureRemoveFormationOutcomeLikeCpp {
                guid,
                spawn_id,
                leader_spawn_id: Some(leader_spawn_id),
                had_group: false,
                removed_member: false,
                removed_group: false,
                remaining_members: 0,
            });
        };

        let removed_member = group.remove(&guid);
        let remaining_members = group.len();
        let removed_group = remaining_members == 0;
        if removed_group {
            self.creature_group_holder_like_cpp.remove(&leader_spawn_id);
        }

        Some(CreatureRemoveFormationOutcomeLikeCpp {
            guid,
            spawn_id,
            leader_spawn_id: Some(leader_spawn_id),
            had_group: true,
            removed_member,
            removed_group,
            remaining_members,
        })
    }

    pub fn remove_from_active_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveFromActiveOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::MissingRecord,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        };
        if record.kind() == AccessorObjectKind::Player {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::PlayerUnsupported,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }
        if !is_active_object_like_cpp(record.kind(), record.object()) {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::NotActiveObject,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }

        let location = self.active_respawn_location_like_cpp(guid);
        let removed_from_active_set = self.active_non_players_like_cpp.remove(&guid);
        let unload_lock = location.map(|location| {
            self.mutate_unload_active_lock_for_respawn_location_like_cpp(location, false)
        });
        RemoveFromActiveOutcomeLikeCpp {
            guid,
            status: ActiveNonPlayerMutationStatusLikeCpp::Mutated,
            inserted_in_active_set: false,
            removed_from_active_set,
            spawn_id_zero_or_unsupported: unload_lock.is_none(),
            unload_lock,
        }
    }

    pub fn remove_map_object(&mut self, guid: ObjectGuid) -> Option<MapObjectRecord> {
        let record = self.map_objects.remove(&guid)?;
        self.unindex_map_object_record_by_spawn_id_like_cpp(&record);
        Some(record)
    }

    /// Bounded map-owned representation of C++ `Unit::RemoveGameObject(uint32
    /// spellid, bool del)`.
    ///
    /// C++ anchors:
    /// - `Unit.cpp:5253-5274`: iterates `m_gameObj`, matches all when
    ///   `spellid == 0` or only objects with the requested spell id, clears
    ///   `CreatedBy`, optionally `SetRespawnTime(0); Delete();`, then erases
    ///   the list entry.
    /// - `Spell.cpp:3621-3625`: channeled spell cancellation uses this overload
    ///   with `del=true`.
    ///
    /// Scope: this overload intentionally does not clear `m_ObjectSlot`, remove
    /// auras, send cooldown events, or dispatch Creature AI despawn callbacks;
    /// those belong to the pointer overload represented by
    /// `gameobject_remove_from_owner_like_cpp`.
    pub fn unit_remove_gameobjects_by_spell_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        spell_id: u32,
        delete: bool,
    ) -> UnitRemoveGameObjectsBySpellOutcomeLikeCpp {
        let owner_found_as_unit_like = self
            .map_object_record(owner_guid)
            .is_some_and(Self::map_record_is_unit_like_gameobject_owner_like_cpp);
        let owned_guids_before = self
            .map_object_record(owner_guid)
            .and_then(Self::map_record_unit_like_cpp)
            .map(|owner| owner.subsystems().control.owned_gameobjects.clone())
            .unwrap_or_default();

        let matched_guids: Vec<ObjectGuid> = owned_guids_before
            .iter()
            .copied()
            .filter(|guid| {
                if spell_id == 0 {
                    return true;
                }
                self.map_object_record(*guid)
                    .and_then(MapObjectRecord::game_object)
                    .is_some_and(|game_object| game_object.spell_id() == spell_id)
            })
            .collect();

        let mut owner_guid_cleared = 0;
        let mut respawn_time_cleared = 0;
        for guid in &matched_guids {
            if let Some(game_object) = self
                .map_objects
                .get_mut(guid)
                .and_then(MapObjectRecord::game_object_mut)
            {
                game_object.clear_owner_guid_like_cpp();
                owner_guid_cleared += 1;
                if delete {
                    game_object.set_respawn_time(0);
                    respawn_time_cleared += 1;
                }
            }
        }

        let mut owner_list_entries_removed = 0;
        if let Some(owner) = self
            .map_objects
            .get_mut(&owner_guid)
            .and_then(Self::map_record_unit_mut_like_cpp)
        {
            let before = owner.subsystems().control.owned_gameobjects.len();
            owner
                .subsystems_mut()
                .control
                .owned_gameobjects
                .retain(|guid| !matched_guids.contains(guid));
            owner_list_entries_removed =
                before.saturating_sub(owner.subsystems().control.owned_gameobjects.len());
        }

        let mut delete_outcomes = 0;
        if delete {
            for guid in &matched_guids {
                if self.gameobject_delete_like_cpp(*guid).is_some() {
                    delete_outcomes += 1;
                }
            }
        }

        UnitRemoveGameObjectsBySpellOutcomeLikeCpp {
            owner_guid,
            spell_id,
            delete_requested: delete,
            owner_found_as_unit_like,
            owned_entries_before: owned_guids_before.len(),
            matched_entries: matched_guids.len(),
            owner_guid_cleared,
            respawn_time_cleared,
            owner_list_entries_removed,
            delete_outcomes,
            object_slot_cleanup_represented: false,
            aura_cleanup_represented: false,
            cooldown_event_represented: false,
            creature_ai_callback_represented: false,
        }
    }

    /// Bounded map-owned representation of C++ `GameObject::RemoveFromOwner()`
    /// during `GameObject::RemoveFromWorld()`.
    ///
    /// C++ anchors:
    /// - `GameObject.cpp:880-897`: empty owner returns; resolved Unit calls
    ///   `Unit::RemoveGameObject(this, false)`; missing owner falls back to
    ///   `SetOwnerGUID(ObjectGuid::Empty)`.
    /// - `GameObject.cpp:926-948`: this runs after ZoneScript remove and before
    ///   model removal, linked trap despawn, `WorldObject::RemoveFromWorld`,
    ///   spawn-id unindex, and map store removal.
    /// - `Unit.cpp:5213-5250`: real owner-side list/slot/aura/cooldown/AI effects
    ///   remain explicit gaps here.
    pub(super) fn gameobject_remove_from_owner_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<GameObjectRemoveFromOwnerOutcomeLikeCpp> {
        let (owner_guid_before, spell_id) = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(MapObjectRecord::game_object)
            .filter(|game_object| game_object.world().object().is_in_world())
            .map(|game_object| (game_object.owner_guid(), game_object.spell_id()))?;

        let owner_found_as_unit_like = !owner_guid_before.is_empty()
            && self
                .map_object_record(owner_guid_before)
                .is_some_and(Self::map_record_is_unit_like_gameobject_owner_like_cpp);
        let cleared_owner = !owner_guid_before.is_empty();

        if cleared_owner {
            if let Some(game_object) = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::game_object_mut)
            {
                game_object.clear_owner_guid_like_cpp();
            }
        }

        let (
            unit_owned_gameobject_list_removed,
            unit_object_slot_cleared,
            aura_cleanup_removed_count,
            creature_ai_callback_represented,
        ) = if owner_found_as_unit_like {
            self.map_objects
                .get_mut(&owner_guid_before)
                .map(|record| {
                    let creature_ai_callback_represented = match record.kind() {
                        AccessorObjectKind::Creature => record
                            .creature_mut()
                            .map(|creature| {
                                creature
                                    .unit_mut()
                                    .subsystems_mut()
                                    .ai
                                    .summoned_gameobject_despawn_like_cpp()
                            })
                            .unwrap_or(false),
                        AccessorObjectKind::Pet => record
                            .pet_mut()
                            .map(|pet| {
                                pet.creature_mut()
                                    .unit_mut()
                                    .subsystems_mut()
                                    .ai
                                    .summoned_gameobject_despawn_like_cpp()
                            })
                            .unwrap_or(false),
                        _ => false,
                    };
                    let Some(owner) = Self::map_record_unit_mut_like_cpp(record) else {
                        return (false, false, 0, creature_ai_callback_represented);
                    };
                    let subsystems = owner.subsystems_mut();
                    let control = &mut subsystems.control;
                    let unit_owned_gameobject_list_removed =
                        control.remove_owned_gameobject_like_cpp(guid);
                    let unit_object_slot_cleared =
                        control.clear_gameobject_slot_for_guid_like_cpp(guid);
                    let aura_cleanup_removed_count = (spell_id != 0)
                        .then(|| {
                            subsystems
                                .auras
                                .remove_auras_due_to_spell_like_cpp(spell_id, ObjectGuid::EMPTY, 0)
                                .len()
                        })
                        .unwrap_or(0);
                    (
                        unit_owned_gameobject_list_removed,
                        unit_object_slot_cleared,
                        aura_cleanup_removed_count,
                        creature_ai_callback_represented,
                    )
                })
                .unwrap_or((false, false, 0, false))
        } else {
            (false, false, 0, false)
        };

        Some(GameObjectRemoveFromOwnerOutcomeLikeCpp {
            guid,
            owner_guid_before,
            owner_guid_after: if cleared_owner {
                ObjectGuid::EMPTY
            } else {
                owner_guid_before
            },
            owner_found_as_unit_like,
            cleared_owner,
            spell_id,
            unit_side_effects_represented: owner_found_as_unit_like,
            unit_owned_gameobject_list_removed,
            unit_object_slot_cleared,
            aura_cleanup_represented: spell_id != 0 && owner_found_as_unit_like,
            aura_cleanup_removed_count,
            cooldown_event_represented: false,
            creature_ai_callback_represented,
        })
    }

    /// Bounded map-owned representation of C++ `GameObject::RemoveFromWorld()`
    /// linked-trap cleanup.
    ///
    /// C++ anchors:
    /// - `GameObject.cpp:926-948`: after ZoneScript remove, `RemoveFromOwner`,
    ///   and represented model removal, `GetLinkedTrap()->DespawnOrUnsummon()`
    ///   runs before `WorldObject::RemoveFromWorld()` and before ObjectsStore
    ///   removal.
    /// - `Map.cpp:933-951`: `Map::RemoveFromMap<T>` calls
    ///   `obj->RemoveFromWorld()` before active/grid/reset/delete tail.
    fn gameobject_remove_linked_trap_like_cpp(
        &mut self,
        guid: ObjectGuid,
        remove_from_map_in_progress: &mut HashSet<ObjectGuid>,
    ) -> Option<GameObjectRemoveLinkedTrapOutcomeLikeCpp> {
        let linked_trap_guid = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(MapObjectRecord::game_object)
            .filter(|game_object| game_object.world().object().is_in_world())
            .map(GameObject::linked_trap_guid_like_cpp)?;

        let owner_present_before_linked_trap_remove = self.map_object_record(guid).is_some();
        let linked_trap_guid = (!linked_trap_guid.is_empty()).then_some(linked_trap_guid);
        let linked_trap_cycle_guarded = linked_trap_guid.is_some_and(|linked_guid| {
            linked_guid != guid && remove_from_map_in_progress.contains(&linked_guid)
        });
        let linked_trap_missing_or_self = linked_trap_guid.is_none_or(|linked_guid| {
            linked_guid == guid
                || (!linked_trap_cycle_guarded && self.map_object_record(linked_guid).is_none())
        });
        let linked_trap_delete = if let Some(linked_guid) = linked_trap_guid {
            if linked_guid == guid
                || linked_trap_cycle_guarded
                || self.map_object_record(linked_guid).is_none()
            {
                None
            } else {
                self.gameobject_delete_like_cpp(linked_guid)
            }
        } else {
            None
        };
        let linked_trap_remove_queued = linked_trap_delete.as_ref().is_some_and(|delete| {
            delete
                .remove_list
                .as_ref()
                .is_some_and(|remove| remove.queued || remove.duplicate)
        });

        Some(GameObjectRemoveLinkedTrapOutcomeLikeCpp {
            guid,
            linked_trap_guid,
            owner_present_before_linked_trap_remove,
            linked_trap_removed: false,
            linked_trap_remove_queued,
            linked_trap_missing_or_self,
            linked_trap_cycle_guarded,
            despawn_or_unsummon_scheduler_represented: linked_trap_delete.is_some(),
            object_accessor_fanout_represented: false,
        })
    }

    /// Bounded map-owned cleanup for the late C++ `Player::RemoveFromWorld()`
    /// `GetViewpoint()` -> `SetViewpoint(viewpoint, false)` branch.
    ///
    /// Source-of-truth anchors:
    /// - `Player.cpp:1567-1585` runs this after `Unit::RemoveFromWorld()` and
    ///   item cleanup while the Player still exists.
    /// - `Player.cpp:25344-25387` clears `FarsightObject`, removes Unit shared
    ///   vision for Unit targets, requests `SetSeer(this)`, and does not request
    ///   `UpdateVisibilityOf` on remove.
    /// - `Player.cpp:25389-25395` resolves `GetViewpoint()` from
    ///   `FarsightObject` through `TYPEMASK_SEER`.
    ///
    /// Ownership: only canonical same-map `Map::map_objects` typed records are
    /// consulted/mutated. DynamicObject targets clear only the removing Player's
    /// `FarsightObject` when it still equals the target GUID; this branch never
    /// resolves `DynamicObject::bound_caster()` or toggles DynamicObject caster
    /// viewpoint state because that lifecycle belongs to DynamicObject removal.
    /// There is no ObjectAccessor/session fallback, no packet fanout, and no real
    /// SetSeer implementation in this seam. Vehicle-base skipping stays open
    /// because this map-owned cleanup has no Player vehicle base runtime; the Unit
    /// helper is called with `vehicle_base_guid: None`.
    fn cleanup_player_remove_from_world_viewpoint_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> Option<PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp> {
        let player_record = self.map_object_record(player_guid)?;
        if player_record.kind() != AccessorObjectKind::Player
            || !player_record.object().object().is_in_world()
        {
            return None;
        }

        let viewpoint_guid = player_record
            .player()
            .map(|player| player.active_data().farsight_object)?;
        if viewpoint_guid.is_empty() {
            return None;
        }

        let outcome = |status,
                       player_set_viewpoint: Option<PlayerSetViewpointOutcomeLikeCpp>,
                       dynamic_object_caster_viewpoint: Option<
            DynamicObjectCasterViewpointOutcomeLikeCpp,
        >,
                       update_visibility_requested,
                       set_seer_requested| {
            PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp {
                player_guid,
                viewpoint_guid,
                status,
                player_set_viewpoint,
                dynamic_object_caster_viewpoint,
                update_visibility_requested,
                set_seer_requested,
                object_accessor_fanout_represented: false,
            }
        };

        let Some(target_record) = self.map_object_record(viewpoint_guid) else {
            return Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::MissingTarget,
                None,
                None,
                false,
                false,
            ));
        };
        let target_kind = target_record.kind();
        if !target_record.object().object().is_in_world() {
            return Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::TargetNotInWorld,
                None,
                None,
                false,
                false,
            ));
        }

        match target_kind {
            AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
                let player_set_viewpoint = self.apply_player_set_viewpoint_unit_like_cpp(
                    player_guid,
                    viewpoint_guid,
                    false,
                    None,
                );
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedUnitViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            AccessorObjectKind::DynamicObject => {
                let player_set_viewpoint = match self.get_typed_player_mut(player_guid) {
                    Some(player) if player.active_data().farsight_object == viewpoint_guid => {
                        player.set_farsight_object_like_cpp(ObjectGuid::EMPTY);
                        Self::player_set_viewpoint_outcome_like_cpp(
                            player_guid,
                            viewpoint_guid,
                            false,
                            PlayerSetViewpointStatusLikeCpp::Removed,
                            None,
                            false,
                            true,
                        )
                    }
                    Some(_) => Self::player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                        None,
                        false,
                        false,
                    ),
                    None => Self::player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                        None,
                        false,
                        false,
                    ),
                };
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            AccessorObjectKind::Player => {
                let player_set_viewpoint = match self.get_typed_player_mut(player_guid) {
                    Some(player) if player.active_data().farsight_object == viewpoint_guid => {
                        player.set_farsight_object_like_cpp(ObjectGuid::EMPTY);
                        Self::player_set_viewpoint_outcome_like_cpp(
                            player_guid,
                            viewpoint_guid,
                            false,
                            PlayerSetViewpointStatusLikeCpp::Removed,
                            None,
                            false,
                            true,
                        )
                    }
                    _ => Self::player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                        None,
                        false,
                        false,
                    ),
                };
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedPlayerViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            _ => Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::TargetNotSeer,
                None,
                None,
                false,
                false,
            )),
        }
    }

    pub fn remove_from_map_like_cpp(
        &mut self,
        guid: ObjectGuid,
        delete_from_world: bool,
    ) -> Result<RemoveFromMapOutcome, RemoveFromMapError> {
        let mut remove_from_map_in_progress = HashSet::new();
        self.remove_from_map_like_cpp_inner(
            guid,
            delete_from_world,
            &mut remove_from_map_in_progress,
        )
    }

    fn remove_from_map_like_cpp_inner(
        &mut self,
        guid: ObjectGuid,
        delete_from_world: bool,
        remove_from_map_in_progress: &mut HashSet<ObjectGuid>,
    ) -> Result<RemoveFromMapOutcome, RemoveFromMapError> {
        if !remove_from_map_in_progress.insert(guid) {
            return Err(RemoveFromMapError::ObjectNotFound { guid });
        }

        let outcome = (|| {
            let should_cleanup_dynamic_object_caster_viewpoint = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::dynamic_object)
                .is_some_and(|dynamic_object| {
                    dynamic_object.world().object().is_in_world()
                        && dynamic_object.is_caster_viewpoint()
                });
            let dynamic_object_caster_viewpoint = should_cleanup_dynamic_object_caster_viewpoint
                .then(|| self.apply_dynamic_object_caster_viewpoint_like_cpp(guid, false));
            let dynamic_object_remove_cleanup = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::dynamic_object_mut)
                .and_then(|dynamic_object| {
                    if !dynamic_object.world().object().is_in_world() {
                        return None;
                    }

                    let had_aura = dynamic_object.has_aura();
                    if had_aura {
                        dynamic_object.remove_aura();
                    }

                    let unbound_caster = dynamic_object.bound_caster();
                    if unbound_caster.is_some() {
                        dynamic_object.unbind_from_caster();
                    }

                    Some(DynamicObjectRemoveCleanupOutcomeLikeCpp {
                        had_aura,
                        removed_aura_pending_delete: dynamic_object
                            .has_removed_aura_pending_delete(),
                        unbound_caster,
                    })
                });
            let gameobject_model_key = self
                .map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                .and_then(MapObjectRecord::game_object)
                .filter(|game_object| game_object.world().object().is_in_world())
                .filter(|game_object| game_object.has_represented_gameobject_model_like_cpp())
                .map(|_| RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid });
            let gameobject_model_remove_pending_before_callback = gameobject_model_key
                .is_some_and(|key| self.contains_gameobject_model_like_cpp(key));
            let gameobject_zone_script_remove = self
                .map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                .and_then(MapObjectRecord::game_object)
                .filter(|game_object| game_object.world().object().is_in_world())
                .map(|game_object| {
                    let spawn_id = game_object.spawn_id();
                    GameObjectZoneScriptRemoveOutcomeLikeCpp {
                        guid,
                        represented_callback_boundary: true,
                        script_dispatch_represented: false,
                        model_remove_pending_before_callback:
                            gameobject_model_remove_pending_before_callback,
                        spawn_index_present_before_callback: spawn_id != 0
                            && self
                                .gameobject_spawn_id_store_guids_like_cpp(spawn_id)
                                .contains(&guid),
                    }
                });
            let gameobject_remove_from_owner = self.gameobject_remove_from_owner_like_cpp(guid);
            let gameobject_model_remove = gameobject_model_key.and_then(|key| {
                self.contains_gameobject_model_like_cpp(key)
                    .then(|| self.remove_gameobject_model_like_cpp(key))
            });
            let gameobject_linked_trap_remove =
                self.gameobject_remove_linked_trap_like_cpp(guid, remove_from_map_in_progress);
            let remove_from_map_was_in_world = self
                .map_object_record(guid)
                .is_some_and(|record| record.object().object().is_in_world());
            let creature_zone_script_remove = self
                .map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::Creature)
                .and_then(MapObjectRecord::creature)
                .filter(|creature| creature.unit().world().object().is_in_world())
                .map(|_| CreatureZoneScriptRemoveOutcomeLikeCpp {
                    guid,
                    represented_callback: true,
                    script_dispatch_represented: false,
                });
            let creature_remove_formation = self.remove_creature_from_formation_like_cpp(guid);
            let creature_unit_remove_from_world = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::creature_mut)
                .and_then(|creature| creature.unit_mut().remove_from_world_like_cpp());
            let creature_vehicle_remove = creature_unit_remove_from_world
                .as_ref()
                .and_then(|outcome| outcome.vehicle_remove);
            let player_viewpoint_cleanup =
                self.cleanup_player_remove_from_world_viewpoint_like_cpp(guid);
            let (kind, was_active) = self
                .map_object_record(guid)
                .map(|record| {
                    (
                        record.kind(),
                        is_active_object_like_cpp(record.kind(), record.object()),
                    )
                })
                .ok_or(RemoveFromMapError::ObjectNotFound { guid })?;
            let remove_from_active = was_active.then(|| self.remove_from_active_like_cpp(guid));
            let mut record = self
                .remove_map_object(guid)
                .ok_or(RemoveFromMapError::ObjectNotFound { guid })?;
            // Rust's non-delete outcome retains only the erased
            // `WorldObject`; `MapObjectRecord::into_object` still destroys the
            // typed Creature/GameObject that owns its Loot. Until this API can
            // return the full typed record like C++ retains the object pointer,
            // both paths must terminally detach that otherwise orphaned
            // authority. A stale lease may finish only if it already crossed
            // the protected durable boundary.
            detach_typed_loot_authority_like_cpp(&mut record);
            let was_world_object_like_cpp = map_record_is_world_object_like_cpp(&record);
            let was_in_world = remove_from_map_was_in_world;
            let cxx_in_world =
                was_in_world && remove_from_map_in_world_eligible_type_like_cpp(kind);
            let personal_phase_owner = record.object().phase_shift().personal_guid_like_cpp();
            let cell = Cell::from_world(record.object().position().x, record.object().position().y);
            let grid = GridCoord::new(cell.grid_x(), cell.grid_y());

            record.object_mut().object_mut().remove_from_world();
            let personal_phase_unregister = self
                .personal_phase_tracker
                .unregister_tracked_object_for_phase_owner_like_cpp(personal_phase_owner, guid);
            let visibility_on_destroy = RemoveFromMapVisibilityOnDestroyOutcomeLikeCpp {
                guid,
                cxx_in_world,
                update_object_visibility_on_destroy_represented: !cxx_in_world,
                update_object_visibility_on_destroy_runtime_gap: !cxx_in_world,
            };
            let removed_from_cell = remove_object_guid_from_cell_like_cpp(
                self,
                grid,
                &cell,
                kind,
                was_world_object_like_cpp,
                guid,
            );

            record.object_mut().clear_current_cell();
            record
                .object_mut()
                .reset_map()
                .map_err(RemoveFromMapError::ResetMap)?;

            // Preserve the typed Player for MapManager's detached/far-teleport
            // owner. The `WorldObject` is only an immutable compatibility
            // projection in the outcome; no second mutable Player is created.
            let object = record.object().clone();
            let player = if !delete_from_world && kind == AccessorObjectKind::Player {
                record.into_player().ok()
            } else {
                None
            };

            Ok(RemoveFromMapOutcome {
                guid,
                cell: cell.cell_coord(),
                grid,
                was_in_world,
                cxx_in_world,
                was_active,
                remove_from_active,
                removed_from_cell,
                delete_from_world,
                dynamic_object_caster_viewpoint,
                dynamic_object_remove_cleanup,
                gameobject_zone_script_remove,
                gameobject_remove_from_owner,
                gameobject_model_remove,
                gameobject_linked_trap_remove,
                creature_zone_script_remove,
                creature_vehicle_remove,
                player_viewpoint_cleanup,
                creature_unit_remove_from_world,
                creature_remove_formation,
                personal_phase_unregister,
                visibility_on_destroy,
                player,
                object: if delete_from_world {
                    None
                } else {
                    Some(object)
                },
            })
        })();
        remove_from_map_in_progress.remove(&guid);
        outcome
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
    /// Source of truth stays map-owned canonical `map_objects`: typed in-world
    /// Players become player sources, typed in-world active non-Players become
    /// active object sources, and the existing visit/relocation helpers consume
    /// marked cells and reset notify flags. Unsupported far combat/aura/summon
    /// source ownership remains a gap and is represented by empty source lists;
    /// no session, ObjectAccessor, packet, AI, dynamic-tree, or fanout side
    /// effects are claimed here.
    pub fn process_live_relocation_notifies_like_cpp(
        &mut self,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
    ) -> ProcessRelocationNotifiesOutcome {
        let mut player_sources = Vec::new();

        for (guid, record) in &self.map_objects {
            let object = record.object();
            if !object.object().is_in_world() || record.kind() != AccessorObjectKind::Player {
                continue;
            }

            let viewpoint_guid = record.player().and_then(|player| {
                let farsight = player.active_data().farsight_object;
                (!farsight.is_empty()).then_some(farsight)
            });
            player_sources.push(MapUpdatePlayerSources {
                player_guid: *guid,
                viewpoint_guid,
                far_combat_unit_guids: Vec::new(),
                far_aura_caster_guids: Vec::new(),
                far_summon_guids: Vec::new(),
            });
        }

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
                    activation_radius: MAX_VISIBILITY_DISTANCE,
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

    pub fn process_map_object_move_list_like_cpp(
        &mut self,
        entries: impl IntoIterator<Item = MapObjectMoveListEntry>,
    ) -> MapObjectMoveListPlan {
        let mut plan = MapObjectMoveListPlan::default();

        for entry in entries {
            let Some(record) = self.map_object_record(entry.guid) else {
                plan.skipped_other_map_or_missing.push(entry.guid);
                continue;
            };
            if record.kind() != entry.kind {
                plan.skipped_kind_mismatch.push(entry.guid);
                continue;
            }

            if entry.move_state != MapObjectCellMoveState::Active {
                plan.reset_inactive_or_none.push(entry.guid);
                continue;
            }

            if !record.object().object().is_in_world() {
                plan.skipped_not_in_world.push(entry.guid);
                continue;
            }

            match self.relocate_map_object_like_cpp(entry.guid, entry.new_position) {
                Ok(outcome) if outcome.relocated => {
                    plan.relocated.push(entry.guid);
                    continue;
                }
                Ok(outcome) if outcome.blocked_by_unloaded_grid => {}
                Ok(_) => {}
                Err(MapObjectRelocationError::InvalidCoordinates { .. }) => {
                    plan.failed_invalid_position.push(entry.guid);
                    continue;
                }
                Err(MapObjectRelocationError::ObjectNotFound { .. }) => {
                    plan.skipped_other_map_or_missing.push(entry.guid);
                    continue;
                }
                Err(MapObjectRelocationError::Record(_) | MapObjectRelocationError::Store(_)) => {
                    plan.failed_store.push(entry.guid);
                    continue;
                }
            }

            match entry.kind {
                AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
                    if let Some(respawn_position) = entry.respawn_position
                        && self
                            .relocate_map_object_like_cpp(entry.guid, respawn_position)
                            .is_ok_and(|outcome| outcome.relocated)
                    {
                        plan.respawn_relocated.push(entry.guid);
                        continue;
                    }

                    if entry.kind == AccessorObjectKind::Pet || entry.is_pet {
                        plan.pet_removed.push(entry.guid);
                    } else {
                        plan.remove_from_world.push(entry.guid);
                    }
                }
                AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
                    if let Some(respawn_position) = entry.respawn_position
                        && self
                            .relocate_map_object_like_cpp(entry.guid, respawn_position)
                            .is_ok_and(|outcome| outcome.relocated)
                    {
                        plan.respawn_relocated.push(entry.guid);
                        continue;
                    }

                    plan.remove_from_world.push(entry.guid);
                }
                AccessorObjectKind::DynamicObject | AccessorObjectKind::AreaTrigger => {
                    plan.blocked_unloaded_grid.push(entry.guid);
                }
                AccessorObjectKind::Player
                | AccessorObjectKind::Corpse
                | AccessorObjectKind::SceneObject
                | AccessorObjectKind::Conversation => {
                    plan.unsupported_kind.push(entry.guid);
                }
            }
        }

        plan
    }

    /// C++ `Map::AddCreatureToMoveList` (`Map.cpp:1163-1176`) seam.
    pub fn add_creature_to_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        self.add_to_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid, position)
    }

    /// C++ `Map::RemoveCreatureFromMoveList` (`Map.cpp:1178-1187`) seam.
    pub fn remove_creature_from_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        self.remove_from_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid)
    }

    /// C++ `Map::AddGameObjectToMoveList` (`Map.cpp:1189-1202`) seam.
    pub fn add_game_object_to_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        self.add_to_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject, guid, position)
    }

    /// C++ `Map::RemoveGameObjectFromMoveList` (`Map.cpp:1204-1213`) seam.
    pub fn remove_game_object_from_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        self.remove_from_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject, guid)
    }

    /// C++ `Map::AddDynamicObjectToMoveList` (`Map.cpp:1215-1226`) seam.
    pub fn add_dynamic_object_to_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        self.add_to_move_list_like_cpp(
            MapObjectMoveListFamilyLikeCpp::DynamicObject,
            guid,
            position,
        )
    }

    /// C++ `Map::RemoveDynamicObjectFromMoveList` (`Map.cpp:1228-1237`) seam.
    pub fn remove_dynamic_object_from_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        self.remove_from_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::DynamicObject, guid)
    }

    /// C++ `Map::AddAreaTriggerToMoveList` (`Map.h:566-579`, `Map.cpp:1163-1237`) seam.
    pub fn add_area_trigger_to_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        self.add_to_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger, guid, position)
    }

    /// C++ `Map::RemoveAreaTriggerFromMoveList` (`Map.h:566-579`, `Map.cpp:1163-1237`) seam.
    pub fn remove_area_trigger_from_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        self.remove_from_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger, guid)
    }

    pub fn move_all_creatures_in_move_list_like_cpp(&mut self) -> MoveListDrainSummaryLikeCpp {
        self.drain_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature)
    }

    pub fn move_all_game_objects_in_move_list_like_cpp(&mut self) -> MoveListDrainSummaryLikeCpp {
        self.drain_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject)
    }

    pub fn move_all_dynamic_objects_in_move_list_like_cpp(
        &mut self,
    ) -> MoveListDrainSummaryLikeCpp {
        self.drain_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::DynamicObject)
    }

    pub fn move_all_area_triggers_in_move_list_like_cpp(&mut self) -> MoveListDrainSummaryLikeCpp {
        self.drain_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger)
    }

    pub fn pending_cell_move_like_cpp(
        &self,
        family: MapObjectMoveListFamilyLikeCpp,
        guid: ObjectGuid,
    ) -> Option<PendingCellMoveLikeCpp> {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_states.get(&guid),
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobject_move_states.get(&guid),
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                self.dynamic_object_move_states.get(&guid)
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => self.area_trigger_move_states.get(&guid),
        }
        .copied()
    }

    pub fn move_list_len_like_cpp(&self, family: MapObjectMoveListFamilyLikeCpp) -> usize {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creatures_to_move.len(),
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobjects_to_move.len(),
            MapObjectMoveListFamilyLikeCpp::DynamicObject => self.dynamic_objects_to_move.len(),
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => self.area_triggers_to_move.len(),
        }
    }

    fn add_to_move_list_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        if self.move_list_locked_like_cpp(family) {
            return AddObjectToMoveListOutcomeLikeCpp::LockedIgnored;
        }
        let Some(record) = self.map_object_record(guid) else {
            return AddObjectToMoveListOutcomeLikeCpp::MissingOrStale;
        };
        let actual = record.kind();
        if !move_list_family_accepts_kind_like_cpp(family, actual) {
            return AddObjectToMoveListOutcomeLikeCpp::WrongKind { actual };
        }

        let pending = PendingCellMoveLikeCpp {
            state: MapObjectCellMoveStateLikeCpp::Active,
            new_position: position,
        };
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => {
                let existed = self.creature_move_states.insert(guid, pending).is_some();
                if !existed {
                    self.creatures_to_move.push(guid);
                }
                if existed {
                    AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
                } else {
                    AddObjectToMoveListOutcomeLikeCpp::Queued
                }
            }
            MapObjectMoveListFamilyLikeCpp::GameObject => {
                let existed = self.gameobject_move_states.insert(guid, pending).is_some();
                if !existed {
                    self.gameobjects_to_move.push(guid);
                }
                if existed {
                    AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
                } else {
                    AddObjectToMoveListOutcomeLikeCpp::Queued
                }
            }
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                let existed = self
                    .dynamic_object_move_states
                    .insert(guid, pending)
                    .is_some();
                if !existed {
                    self.dynamic_objects_to_move.push(guid);
                }
                if existed {
                    AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
                } else {
                    AddObjectToMoveListOutcomeLikeCpp::Queued
                }
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => {
                let existed = self
                    .area_trigger_move_states
                    .insert(guid, pending)
                    .is_some();
                if !existed {
                    self.area_triggers_to_move.push(guid);
                }
                if existed {
                    AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
                } else {
                    AddObjectToMoveListOutcomeLikeCpp::Queued
                }
            }
        }
    }

    fn remove_from_move_list_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        if self.move_list_locked_like_cpp(family) {
            return RemoveObjectFromMoveListOutcomeLikeCpp::LockedIgnored;
        }
        let Some(record) = self.map_object_record(guid) else {
            return RemoveObjectFromMoveListOutcomeLikeCpp::MissingOrStale;
        };
        let actual = record.kind();
        if !move_list_family_accepts_kind_like_cpp(family, actual) {
            return RemoveObjectFromMoveListOutcomeLikeCpp::WrongKind { actual };
        }
        let state = match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_states.get_mut(&guid),
            MapObjectMoveListFamilyLikeCpp::GameObject => {
                self.gameobject_move_states.get_mut(&guid)
            }
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                self.dynamic_object_move_states.get_mut(&guid)
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => {
                self.area_trigger_move_states.get_mut(&guid)
            }
        };
        let Some(pending) = state else {
            return RemoveObjectFromMoveListOutcomeLikeCpp::NotQueued;
        };
        if pending.state == MapObjectCellMoveStateLikeCpp::Active {
            pending.state = MapObjectCellMoveStateLikeCpp::Inactive;
            RemoveObjectFromMoveListOutcomeLikeCpp::MarkedInactive
        } else {
            RemoveObjectFromMoveListOutcomeLikeCpp::AlreadyInactive
        }
    }

    fn move_list_locked_like_cpp(&self, family: MapObjectMoveListFamilyLikeCpp) -> bool {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_lock,
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobject_move_lock,
            MapObjectMoveListFamilyLikeCpp::DynamicObject => self.dynamic_object_move_lock,
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => self.area_trigger_move_lock,
        }
    }

    fn set_move_list_lock_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
        locked: bool,
    ) {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_lock = locked,
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobject_move_lock = locked,
            MapObjectMoveListFamilyLikeCpp::DynamicObject => self.dynamic_object_move_lock = locked,
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => self.area_trigger_move_lock = locked,
        }
    }

    fn take_move_list_queue_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
    ) -> Vec<ObjectGuid> {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => std::mem::take(&mut self.creatures_to_move),
            MapObjectMoveListFamilyLikeCpp::GameObject => {
                std::mem::take(&mut self.gameobjects_to_move)
            }
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                std::mem::take(&mut self.dynamic_objects_to_move)
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => {
                std::mem::take(&mut self.area_triggers_to_move)
            }
        }
    }

    fn remove_pending_move_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
        guid: ObjectGuid,
    ) -> Option<PendingCellMoveLikeCpp> {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_states.remove(&guid),
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobject_move_states.remove(&guid),
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                self.dynamic_object_move_states.remove(&guid)
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => {
                self.area_trigger_move_states.remove(&guid)
            }
        }
    }

    fn drain_move_list_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
    ) -> MoveListDrainSummaryLikeCpp {
        let mut summary = MoveListDrainSummaryLikeCpp {
            family: Some(family),
            ..Default::default()
        };
        if self.move_list_locked_like_cpp(family) {
            summary.locked_ignored = 1;
            return summary;
        }

        self.set_move_list_lock_like_cpp(family, true);
        let queued = self.take_move_list_queue_like_cpp(family);
        for guid in queued {
            summary.processed += 1;
            let Some(pending) = self.remove_pending_move_like_cpp(family, guid) else {
                summary.inactive_reset += 1;
                continue;
            };
            if pending.state != MapObjectCellMoveStateLikeCpp::Active {
                summary.inactive_reset += 1;
                continue;
            }

            let Some(record) = self.map_object_record(guid) else {
                summary.missing_or_stale += 1;
                continue;
            };
            let actual = record.kind();
            if !move_list_family_accepts_kind_like_cpp(family, actual) {
                summary.wrong_kind += 1;
                continue;
            }
            if !record.object().object().is_in_world() {
                summary.not_in_world += 1;
                continue;
            }

            match self.relocate_map_object_like_cpp(guid, pending.new_position) {
                Ok(outcome) if outcome.relocated => summary.relocated += 1,
                Ok(outcome) if outcome.blocked_by_unloaded_grid => {
                    summary.blocked_by_unloaded_grid += 1;
                    if matches!(
                        family,
                        MapObjectMoveListFamilyLikeCpp::Creature
                            | MapObjectMoveListFamilyLikeCpp::GameObject
                    ) {
                        summary.respawn_relocation_unsupported += 1;
                    }
                }
                Ok(_) => summary.blocked_by_unloaded_grid += 1,
                Err(MapObjectRelocationError::InvalidCoordinates { .. }) => {
                    summary.failed_invalid_position += 1;
                }
                Err(MapObjectRelocationError::ObjectNotFound { .. }) => {
                    summary.missing_or_stale += 1;
                }
                Err(MapObjectRelocationError::Record(_) | MapObjectRelocationError::Store(_)) => {
                    summary.failed_store += 1;
                }
            }
        }
        self.set_move_list_lock_like_cpp(family, false);
        summary
    }
}
