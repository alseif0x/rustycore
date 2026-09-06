// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Grid and cell storage, terrain loading and object lookup.

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

    fn activate_registered_corpses_for_grid_like_cpp(&mut self, grid: GridCoord) -> usize {
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
                .and_then(MapObjectRecord::corpse_mut)
            {
                corpse
                    .world_mut()
                    .set_current_cell(cell.cell_x(), cell.cell_y());
                corpse.world_mut().object_mut().add_to_world();
            }
        }

        corpses.len()
    }

    fn deactivate_registered_corpses_for_grid_like_cpp(&mut self, grid: GridCoord) -> usize {
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
                .and_then(MapObjectRecord::corpse_mut)
            {
                corpse.world_mut().object_mut().remove_from_world();
                corpse.world_mut().clear_current_cell();
            }
        }

        corpses.len()
    }

    pub fn generate_low_guid_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> Result<i64, MapGuidSequenceErrorLikeCpp> {
        Self::ensure_map_guid_sequence_source_like_cpp(high)?;
        Ok(self
            .guid_sequence_generator_like_cpp(high)
            .generator
            .generate())
    }

    pub fn get_max_low_guid_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> Result<i64, MapGuidSequenceErrorLikeCpp> {
        Self::ensure_map_guid_sequence_source_like_cpp(high)?;
        Ok(self
            .guid_sequence_generator_like_cpp(high)
            .generator
            .next_after_max_used())
    }

    pub fn set_guid_sequence_like_cpp(
        &mut self,
        high: HighGuid,
        next: i64,
    ) -> Result<(), MapGuidSequenceErrorLikeCpp> {
        Self::ensure_map_guid_sequence_source_like_cpp(high)?;
        self.guid_sequence_generator_like_cpp(high)
            .generator
            .set(next);
        Ok(())
    }

    fn guid_sequence_generator_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> &mut MapGuidSequenceGeneratorLikeCpp {
        self.guid_generators
            .entry(high)
            .or_insert_with(|| MapGuidSequenceGeneratorLikeCpp::new(high))
    }

    fn ensure_map_guid_sequence_source_like_cpp(
        high: HighGuid,
    ) -> Result<(), MapGuidSequenceErrorLikeCpp> {
        match high {
            HighGuid::WorldTransaction
            | HighGuid::StaticDoor
            | HighGuid::Transport
            | HighGuid::Conversation
            | HighGuid::Creature
            | HighGuid::Vehicle
            | HighGuid::Pet
            | HighGuid::GameObject
            | HighGuid::DynamicObject
            | HighGuid::AreaTrigger
            | HighGuid::Corpse
            | HighGuid::LootObject
            | HighGuid::SceneObject
            | HighGuid::Scenario
            | HighGuid::AIGroup
            | HighGuid::DynamicDoor
            | HighGuid::Vignette
            | HighGuid::CallForHelp
            | HighGuid::AIResource
            | HighGuid::AILock
            | HighGuid::AILockTicket
            | HighGuid::Cast => Ok(()),
            _ => Err(MapGuidSequenceErrorLikeCpp::UnsupportedSequenceSource { high }),
        }
    }

    pub const fn grid_expiry_ms(&self) -> i64 {
        self.grid_expiry_ms
    }

    pub const fn grid_unload(&self) -> bool {
        self.grid_unload
    }

    pub fn terrain(&self) -> &Terrain {
        &self.terrain
    }

    /// Bridge for C++ `Map::ShouldBeSpawnedOnGridLoad` callers while `Map` does
    /// not yet own the ObjectMgr spawn metadata. The canonical toggle state,
    /// respawn timers, and `SpawnedPoolData` are map-owned; spawn metadata remains
    /// caller-supplied.
    pub fn spawn_grid_load_state_like_cpp<'a>(
        &'a self,
        spawn_store: &'a SpawnStore,
    ) -> SpawnGridLoadStateLikeCpp<'a> {
        SpawnGridLoadStateLikeCpp::new(spawn_store, &self.spawn_group_state)
            .with_respawn_timers(self.respawn_store.respawn_timer_keys_like_cpp())
            .with_pool_spawned_objects(self.pool_data.spawned_objects_like_cpp())
    }

    /// C++ `Map::AddFarSpellCallback` represented as a map-owned FIFO action queue.
    ///
    /// This helper only accepts explicit represented actions; it does not expose a
    /// general closure/callback runtime or real Spell/Aura side effects.
    pub fn add_far_spell_callback_like_cpp(
        &mut self,
        callback: RepresentedFarSpellCallbackLikeCpp,
    ) {
        self.far_spell_callbacks_like_cpp.push_back(callback);
    }

    /// C++ `Trinity::ObjectUpdater` visits creature containers reachable from
    /// the map's loaded grids, not the global object accessor/store. Keeping the
    /// visitation anchored to cells prevents unloaded-grid records from being
    /// updated after `Map::UnloadGrid` has removed their NGrid.
    pub(super) fn object_updater_creature_guids_like_cpp(&self) -> Vec<ObjectGuid> {
        let mut creature_guids = Vec::new();
        for grid in self.grids.iter().filter_map(|grid| grid.as_deref()) {
            grid.visit_all_grids(|cell| {
                creature_guids.extend(cell.grid_objects.creatures.iter().copied());
                creature_guids.extend(cell.world_objects.creatures.iter().copied());
            });
        }
        sort_dedup(&mut creature_guids);
        creature_guids
    }

    /// C++ `Map::AddObjectToSwitchList` represented over canonical map records.
    ///
    /// C++ anchors:
    /// - `Map.h:345-346` declares `AddObjectToRemoveList` beside
    ///   `AddObjectToSwitchList`; `Map.h:651-652` owns both queues.
    /// - `Map.cpp:2557-2572` accepts only `TYPEID_UNIT`, inserts first toggle,
    ///   cancels an opposite pending toggle, and aborts on duplicate direction.
    /// - `Object.cpp:910-915` shows `WorldObject::SetWorldObject(on)` enqueues
    ///   through the owning map only when the object is already in world.
    pub fn add_object_to_switch_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        on: bool,
    ) -> AddObjectToSwitchListOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::MissingOrStale,
            };
        };

        debug_assert_eq!(record.object().map_id(), self.map_id);
        debug_assert_eq!(record.object().instance_id(), self.instance_id);

        if !switch_list_unit_kind_like_cpp(record.kind()) {
            return AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit,
            };
        }

        match self.objects_to_switch.get(&guid).copied() {
            None => {
                self.objects_to_switch.insert(guid, on);
                AddObjectToSwitchListOutcomeLikeCpp {
                    guid,
                    on,
                    status: AddObjectToSwitchListStatusLikeCpp::Queued,
                }
            }
            Some(existing) if existing != on => {
                self.objects_to_switch.remove(&guid);
                AddObjectToSwitchListOutcomeLikeCpp {
                    guid,
                    on,
                    status: AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle,
                }
            }
            Some(_) => AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::DuplicateSameDirectionAbort,
            },
        }
    }

    pub fn creature_spawn_id_store_count_like_cpp(&self, spawn_id: SpawnId) -> usize {
        self.creatures_by_spawn_id
            .get(&spawn_id)
            .map_or(0, HashSet::len)
    }

    pub fn creature_group_holder_contains_like_cpp(
        &self,
        leader_spawn_id: SpawnId,
        member_guid: ObjectGuid,
    ) -> bool {
        self.creature_group_holder_like_cpp
            .get(&leader_spawn_id)
            .is_some_and(|members| members.contains(&member_guid))
    }

    pub fn creature_spawn_id_store_guids_like_cpp(&self, spawn_id: SpawnId) -> Vec<ObjectGuid> {
        self.creatures_by_spawn_id
            .get(&spawn_id)
            .map(|guids| {
                let mut guids: Vec<_> = guids.iter().copied().collect();
                guids.sort();
                guids
            })
            .unwrap_or_default()
    }

    pub fn get_creature_by_spawn_id_like_cpp(&self, spawn_id: SpawnId) -> Option<&Creature> {
        let mut fallback_guid = None;
        let mut alive_guid = None;
        for guid in self.creature_spawn_id_store_guids_like_cpp(spawn_id) {
            let Some(creature) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
            else {
                continue;
            };
            if creature.spawn_id() != spawn_id {
                continue;
            }
            fallback_guid.get_or_insert(guid);
            if creature.is_alive() {
                alive_guid = Some(guid);
                break;
            }
        }

        alive_guid
            .or(fallback_guid)
            .and_then(|guid| self.map_object_record(guid)?.creature())
    }

    pub fn get_world_object_by_spawn_id_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<&WorldObject> {
        match object_type {
            SpawnObjectType::Creature => self
                .get_creature_by_spawn_id_like_cpp(spawn_id)
                .map(|creature| creature.unit().world()),
            SpawnObjectType::GameObject => self
                .get_gameobject_by_spawn_id_like_cpp(spawn_id)
                .map(GameObject::world),
            SpawnObjectType::AreaTrigger => self
                .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
                .map(AreaTrigger::world),
        }
    }

    pub fn insert_map_object(
        &mut self,
        kind: AccessorObjectKind,
        object: WorldObject,
    ) -> Result<Option<MapObjectRecord>, MapObjectStoreError> {
        let record = MapObjectRecord::new(kind, object)?;
        self.insert_map_object_record(record)
    }

    pub fn insert_map_object_record(
        &mut self,
        record: MapObjectRecord,
    ) -> Result<Option<MapObjectRecord>, MapObjectStoreError> {
        self.validate_map_object(record.object())?;
        let guid = record.object().guid();
        let mut previous = self.entity_world.remove(&guid);
        if let Some(previous_record) = previous.as_mut() {
            if !typed_loot_authorities_share_storage_like_cpp(previous_record, &record) {
                detach_typed_loot_authority_like_cpp(previous_record);
            }
            self.unindex_map_object_record_by_spawn_id_like_cpp(previous_record);
        }
        self.index_map_object_record_by_spawn_id_like_cpp(&record);
        let displaced = self.entity_world.insert(record);
        debug_assert!(displaced.is_none());
        Ok(previous)
    }

    pub fn add_to_active_like_cpp(&mut self, guid: ObjectGuid) -> AddToActiveOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::MissingRecord,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        };
        if record.kind() == AccessorObjectKind::Player {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::PlayerUnsupported,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }
        if !is_active_object_like_cpp(record.kind(), record.object()) {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::NotActiveObject,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }

        let location = self.active_respawn_location_like_cpp(guid);
        let inserted_in_active_set = self.active_non_players_like_cpp.insert(guid);
        let unload_lock = location.map(|location| {
            self.mutate_unload_active_lock_for_respawn_location_like_cpp(location, true)
        });
        AddToActiveOutcomeLikeCpp {
            guid,
            status: ActiveNonPlayerMutationStatusLikeCpp::Mutated,
            inserted_in_active_set,
            removed_from_active_set: false,
            spawn_id_zero_or_unsupported: unload_lock.is_none(),
            unload_lock,
        }
    }

    fn represent_add_to_map_post_add_to_world_tail_like_cpp(
        &mut self,
        kind: AccessorObjectKind,
        guid: ObjectGuid,
        active_object: bool,
    ) -> Option<AddToMapPostAddToWorldOutcomeLikeCpp> {
        let pending_move_state = match kind {
            AccessorObjectKind::Creature => {
                if self
                    .map_object_record(guid)
                    .is_some_and(|record| record.creature().is_some())
                {
                    self.creature_move_states.remove(&guid)
                } else {
                    return None;
                }
            }
            AccessorObjectKind::GameObject => {
                if self
                    .map_object_record(guid)
                    .is_some_and(|record| record.game_object().is_some())
                {
                    self.gameobject_move_states.remove(&guid)
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        if pending_move_state.is_some() {
            match kind {
                AccessorObjectKind::Creature => {
                    self.creatures_to_move.retain(|queued| *queued != guid)
                }
                AccessorObjectKind::GameObject => {
                    self.gameobjects_to_move.retain(|queued| *queued != guid);
                }
                _ => {}
            }
        }

        let add_to_active = active_object.then(|| self.add_to_active_like_cpp(guid));

        let mut set_true = false;
        let mut set_false = false;
        let final_is_new_object = if let Some(record) = self.entity_world.get_mut(&guid) {
            record.object_mut().object_mut().set_is_new_object(true);
            set_true = true;
            record.object_mut().object_mut().set_is_new_object(false);
            set_false = true;
            record.object().object().is_new_object()
        } else {
            false
        };

        Some(AddToMapPostAddToWorldOutcomeLikeCpp {
            initialize_object_represented: true,
            pending_move_state_cleared: pending_move_state.is_some(),
            no_pending_move_state: pending_move_state.is_none(),
            add_to_active_represented: add_to_active.is_some(),
            add_to_active_skipped_runtime_gap: false,
            add_to_active,
            set_is_new_object_true: set_true,
            update_object_visibility_on_create_represented: true,
            update_object_visibility_on_create_runtime_gap: true,
            set_is_new_object_false: set_false,
            final_is_new_object,
        })
    }

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
        mut record: MapObjectRecord,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        let kind = record.kind();
        let guid = record.object().guid();
        let position = record.object().position();
        let is_world_object = record.object().is_world_object();

        if record.object().object().is_in_world() {
            let cell = Cell::from_world(position.x, position.y);
            let previous = self.insert_map_object_record(record)?;
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

        self.validate_map_object(record.object())?;

        if !is_valid_map_coord_2d(position.x, position.y) {
            return Err(AddToMapError::InvalidCoordinates {
                guid,
                x: position.x,
                y: position.y,
            });
        }

        let cell = Cell::from_world(position.x, position.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        let active_object = is_active_object_like_cpp(kind, record.object());
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

        if kind == AccessorObjectKind::Creature && record.creature().is_some() {
            record
                .object_mut()
                .set_current_cell(cell.cell_x(), cell.cell_y());
            let previous = self.insert_map_object_record(record)?;

            let creature_store_inserted_before_add_to_world = self
                .map_object_record(guid)
                .is_some_and(|record| record.creature().is_some());
            let creature_spawn_indexed_before_add_to_world = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .is_some_and(|creature| {
                    let spawn_id = creature.spawn_id();
                    spawn_id != 0
                        && self
                            .creature_spawn_id_store_guids_like_cpp(spawn_id)
                            .contains(&guid)
                });

            let creature_unit_add_to_world = self
                .entity_world
                .get_mut(&guid)
                .and_then(MapObjectRecord::creature_mut)
                .map(|creature| creature.unit_mut().add_to_world_like_cpp());
            let creature_search_formation = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .map(Creature::search_formation_like_cpp);
            if let Some(outcome) = creature_search_formation {
                self.apply_creature_search_formation_like_cpp(guid, outcome);
            }

            let creature_aim_initialize = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .map(Creature::aim_initialize_like_cpp);

            let creature_vehicle_reset = self
                .entity_world
                .get_mut(&guid)
                .and_then(MapObjectRecord::creature_mut)
                .and_then(|creature| {
                    let context = creature
                        .add_to_world_vehicle_reset_context_like_cpp()?
                        .clone();
                    let base_is_alive = creature.is_alive();
                    creature
                        .unit_mut()
                        .subsystems_mut()
                        .vehicle
                        .reset_vehicle_kit_for_creature_add_to_world_like_cpp(
                            &context,
                            base_is_alive,
                        )
                });

            let creature_vehicle_install = self
                .entity_world
                .get_mut(&guid)
                .and_then(MapObjectRecord::creature_mut)
                .and_then(|creature| {
                    let install = creature
                        .unit_mut()
                        .subsystems_mut()
                        .vehicle
                        .install_vehicle_kit_like_cpp();
                    install.had_kit.then_some(install)
                });

            let creature_zone_script_create = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .is_some()
                .then_some(CreatureZoneScriptCreateOutcomeLikeCpp {
                    guid,
                    represented_callback: true,
                    script_dispatch_represented: false,
                });
            let add_to_map_tail = self.represent_add_to_map_post_add_to_world_tail_like_cpp(
                kind,
                guid,
                active_object,
            );

            return Ok(AddToMapOutcome {
                guid,
                cell: cell.cell_coord(),
                grid,
                inserted: previous.is_none(),
                already_in_world: false,
                grid_created,
                grid_loaded,
                inserted_into_cell: true,
                gameobject_model_insert: None,
                gameobject_collision_enable: None,
                gameobject_zone_script_create: None,
                gameobject_store_inserted_before_add_to_world: None,
                gameobject_spawn_indexed_before_add_to_world: None,
                creature_store_inserted_before_add_to_world: Some(
                    creature_store_inserted_before_add_to_world,
                ),
                creature_spawn_indexed_before_add_to_world: Some(
                    creature_spawn_indexed_before_add_to_world,
                ),
                creature_unit_add_to_world,
                creature_search_formation,
                creature_aim_initialize,
                creature_vehicle_reset,
                creature_vehicle_install,
                creature_zone_script_create,
                add_to_map_tail,
            });
        }

        if kind == AccessorObjectKind::GameObject && record.game_object().is_some() {
            record
                .object_mut()
                .set_current_cell(cell.cell_x(), cell.cell_y());
            let object_store_present_before_callback = self
                .map_object_record(guid)
                .is_some_and(|record| record.game_object().is_some());
            let spawn_index_present_before_callback =
                record.game_object().is_some_and(|game_object| {
                    let spawn_id = game_object.spawn_id();
                    spawn_id != 0
                        && self
                            .gameobject_spawn_id_store_guids_like_cpp(spawn_id)
                            .contains(&guid)
                });
            let gameobject_zone_script_create = Some(GameObjectZoneScriptCreateOutcomeLikeCpp {
                guid,
                represented_callback_boundary: true,
                script_dispatch_represented: false,
                object_store_present_before_callback,
                spawn_index_present_before_callback,
            });
            let previous = self.insert_map_object_record(record)?;

            let gameobject_store_inserted_before_add_to_world = self
                .map_object_record(guid)
                .is_some_and(|record| record.game_object().is_some());
            let gameobject_spawn_indexed_before_add_to_world = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::game_object)
                .is_some_and(|game_object| {
                    let spawn_id = game_object.spawn_id();
                    spawn_id != 0
                        && self
                            .gameobject_spawn_id_store_guids_like_cpp(spawn_id)
                            .contains(&guid)
                });
            let has_represented_model = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::game_object)
                .is_some_and(GameObject::has_represented_gameobject_model_like_cpp);
            let (gameobject_model_insert, gameobject_collision_enable) = if has_represented_model {
                let gameobject_model_insert =
                    self.insert_gameobject_model_like_cpp(RepresentedGameObjectModelKeyLikeCpp {
                        owner_guid: guid,
                    });
                let gameobject_collision_enable = self
                    .entity_world
                    .get_mut(&guid)
                    .and_then(MapObjectRecord::game_object_mut)
                    .map(|game_object| {
                        // C++ `GameObject::AddToWorld()` computes toggledState before
                        // `EnableCollision(toggledState)`: chests use `getLootState() == GO_READY`,
                        // exact non-Transport GameObjects use `GetGoState() == GO_STATE_READY`.
                        // `MapObjectRecord::Transport` is handled outside this exact-typed
                        // GameObject branch and remains a delayed-add runtime gap for this
                        // represented seam.
                        let toggled_state =
                            if game_object.data().type_id as u32 == GAMEOBJECT_TYPE_CHEST {
                                game_object.loot_state() == LootState::Ready
                            } else {
                                game_object.data().state == GoState::Ready as i8
                            };
                        let collision = game_object
                            .enable_represented_gameobject_collision_like_cpp(toggled_state);
                        GameObjectCollisionEnableOutcomeLikeCpp {
                            requested_enable: collision.requested_enable,
                            represented_model_present: collision.represented_model_present,
                            previous_collision_enabled: collision.previous_collision_enabled,
                            new_collision_enabled: collision.new_collision_enabled,
                        }
                    });
                (Some(gameobject_model_insert), gameobject_collision_enable)
            } else {
                (None, None)
            };

            if let Some(game_object) = self
                .entity_world
                .get_mut(&guid)
                .and_then(MapObjectRecord::game_object_mut)
            {
                game_object.world_mut().object_mut().add_to_world();
            }
            let add_to_map_tail = self.represent_add_to_map_post_add_to_world_tail_like_cpp(
                kind,
                guid,
                active_object,
            );

            return Ok(AddToMapOutcome {
                guid,
                cell: cell.cell_coord(),
                grid,
                inserted: previous.is_none(),
                already_in_world: false,
                grid_created,
                grid_loaded,
                inserted_into_cell: true,
                gameobject_model_insert,
                gameobject_collision_enable,
                gameobject_zone_script_create,
                gameobject_store_inserted_before_add_to_world: Some(
                    gameobject_store_inserted_before_add_to_world,
                ),
                gameobject_spawn_indexed_before_add_to_world: Some(
                    gameobject_spawn_indexed_before_add_to_world,
                ),
                creature_store_inserted_before_add_to_world: None,
                creature_spawn_indexed_before_add_to_world: None,
                creature_unit_add_to_world: None,
                creature_search_formation: None,
                creature_aim_initialize: None,
                creature_vehicle_reset: None,
                creature_vehicle_install: None,
                creature_zone_script_create: None,
                add_to_map_tail,
            });
        }

        let creature_unit_add_to_world = {
            record
                .object_mut()
                .set_current_cell(cell.cell_x(), cell.cell_y());
            let creature_unit_add_to_world = if let Some(creature) = record.creature_mut() {
                Some(creature.unit_mut().add_to_world_like_cpp())
            } else {
                record.object_mut().object_mut().add_to_world();
                None
            };
            record.object_mut().object_mut().set_is_new_object(true);
            // Rust does not emit visibility here yet; keep the flag lifecycle identical to
            // C++ `Map::AddToMap` after `UpdateObjectVisibilityOnCreate()` returns.
            record.object_mut().object_mut().set_is_new_object(false);
            creature_unit_add_to_world
        };

        let creature_search_formation = if kind == AccessorObjectKind::Creature {
            record.creature().map(Creature::search_formation_like_cpp)
        } else {
            None
        };
        if let Some(outcome) = creature_search_formation {
            self.apply_creature_search_formation_like_cpp(guid, outcome);
        }

        let creature_aim_initialize = if kind == AccessorObjectKind::Creature {
            record.creature().map(Creature::aim_initialize_like_cpp)
        } else {
            None
        };

        let creature_vehicle_reset = if kind == AccessorObjectKind::Creature {
            record.creature_mut().and_then(|creature| {
                let context = creature
                    .add_to_world_vehicle_reset_context_like_cpp()?
                    .clone();
                let base_is_alive = creature.is_alive();
                creature
                    .unit_mut()
                    .subsystems_mut()
                    .vehicle
                    .reset_vehicle_kit_for_creature_add_to_world_like_cpp(&context, base_is_alive)
            })
        } else {
            None
        };

        let creature_vehicle_install = if kind == AccessorObjectKind::Creature {
            record.creature_mut().and_then(|creature| {
                let install = creature
                    .unit_mut()
                    .subsystems_mut()
                    .vehicle
                    .install_vehicle_kit_like_cpp();
                install.had_kit.then_some(install)
            })
        } else {
            None
        };
        let creature_zone_script_create = if kind == AccessorObjectKind::Creature {
            record
                .creature()
                .is_some()
                .then_some(CreatureZoneScriptCreateOutcomeLikeCpp {
                    guid,
                    represented_callback: true,
                    script_dispatch_represented: false,
                })
        } else {
            None
        };

        let (gameobject_model_insert, gameobject_collision_enable) =
            if kind == AccessorObjectKind::GameObject {
                if let Some(game_object) = record
                    .game_object_mut()
                    .filter(|game_object| game_object.has_represented_gameobject_model_like_cpp())
                {
                    let gameobject_model_insert = self.insert_gameobject_model_like_cpp(
                        RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid },
                    );
                    // C++ `GameObject::AddToWorld()` computes toggledState before
                    // `EnableCollision(toggledState)`: chests use `getLootState() == GO_READY`,
                    // exact non-Transport GameObjects use `GetGoState() == GO_STATE_READY`.
                    // `MapObjectRecord::Transport` is handled above by the kind gate and remains
                    // a delayed-add runtime gap for this represented seam.
                    let toggled_state =
                        if game_object.data().type_id as u32 == GAMEOBJECT_TYPE_CHEST {
                            game_object.loot_state() == LootState::Ready
                        } else {
                            game_object.data().state == GoState::Ready as i8
                        };
                    let collision =
                        game_object.enable_represented_gameobject_collision_like_cpp(toggled_state);
                    let gameobject_collision_enable = GameObjectCollisionEnableOutcomeLikeCpp {
                        requested_enable: collision.requested_enable,
                        represented_model_present: collision.represented_model_present,
                        previous_collision_enabled: collision.previous_collision_enabled,
                        new_collision_enabled: collision.new_collision_enabled,
                    };
                    (
                        Some(gameobject_model_insert),
                        Some(gameobject_collision_enable),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

        let previous = self.insert_map_object_record(record)?;
        let add_to_map_tail =
            self.represent_add_to_map_post_add_to_world_tail_like_cpp(kind, guid, active_object);
        Ok(AddToMapOutcome {
            guid,
            cell: cell.cell_coord(),
            grid,
            inserted: previous.is_none(),
            already_in_world: false,
            grid_created,
            grid_loaded,
            inserted_into_cell: true,
            gameobject_model_insert,
            gameobject_collision_enable,
            gameobject_zone_script_create: None,
            gameobject_store_inserted_before_add_to_world: None,
            gameobject_spawn_indexed_before_add_to_world: None,
            creature_store_inserted_before_add_to_world: None,
            creature_spawn_indexed_before_add_to_world: None,
            creature_unit_add_to_world,
            creature_search_formation,
            creature_aim_initialize,
            creature_vehicle_reset,
            creature_vehicle_install,
            creature_zone_script_create,
            add_to_map_tail,
        })
    }

    pub(super) fn player_viewpoint_guid_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let record = self.map_object_record(player_guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        let Some(player) = record.player() else {
            return Some(player_guid);
        };
        let farsight = player.active_data().farsight_object;
        Some(if farsight.is_empty() {
            player_guid
        } else {
            farsight
        })
    }

    pub(super) fn exact_cell_guids_like_cpp(&self, cell_coord: CellCoord) -> NearbyCellGuids {
        let mut nearby = NearbyCellGuids::default();
        let cell = Cell::from_cell_coord(cell_coord);
        let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y())) else {
            return nearby;
        };
        let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
            return nearby;
        };

        nearby.visited_cells = 1;
        nearby.merge_world(&local_cell.world_objects);
        nearby.merge_grid(&local_cell.grid_objects);
        nearby
    }

    pub fn get_creature(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Creature])
    }

    pub(crate) fn get_typed_creature(&self, guid: ObjectGuid) -> Option<&Creature> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Creature {
            return None;
        }
        record.creature()
    }

    /// Return an owned transform/vitals view of an exact canonical Creature.
    /// This is the preferred external read seam for systems that do not need the
    /// complete entity and remains compatible with the selected private ECS
    /// backend.
    pub fn creature_transform_vitals_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<CreatureTransformVitalsSnapshotLikeCpp> {
        self.entity_world.creature_transform_vitals_snapshot(guid)
    }

    /// Run a synchronous read against one exact canonical Creature without
    /// exposing the storage representation. `R` is owned independently of the
    /// callback borrow, so a future ECS guard cannot escape this method.
    pub fn with_creature_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Creature) -> R,
    ) -> Option<R> {
        self.entity_world.with_creature(guid, read)
    }

    /// Run one synchronous mutation inside the canonical entity owner and
    /// return only an owned result.
    pub fn with_creature_mut_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        write: impl FnOnce(&mut Creature) -> R,
    ) -> Option<R> {
        self.entity_world.with_creature_mut(guid, write)
    }

    pub fn get_typed_creature_mut(&mut self, guid: ObjectGuid) -> Option<&mut Creature> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Creature {
            return None;
        }
        record.creature_mut()
    }

    pub fn get_pet(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Pet])
    }

    pub fn get_typed_pet(&self, guid: ObjectGuid) -> Option<&Pet> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Pet {
            return None;
        }
        record.pet()
    }

    pub fn get_typed_pet_mut(&mut self, guid: ObjectGuid) -> Option<&mut Pet> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Pet {
            return None;
        }
        record.pet_mut()
    }

    /// Run one synchronous read against an exact canonical Pet without exposing
    /// the map's storage representation or allowing the borrowed entity to
    /// escape. This is the migration-safe equivalent of C++'s typed object-store
    /// lookup for adapter code that returns an owned snapshot.
    pub fn with_pet_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Pet) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::Pet {
            return None;
        }
        record.pet().map(read)
    }

    pub fn with_game_object_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&GameObject) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::GameObject {
            return None;
        }
        record.game_object().map(read)
    }

    /// Read either the exact Creature or Pet body addressed by `guid` while the
    /// callback is in scope. The optional owner is populated only for Pets.
    pub fn with_creature_or_pet_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Creature, Option<ObjectGuid>) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        match record.kind() {
            AccessorObjectKind::Creature => record.creature().map(|creature| read(creature, None)),
            AccessorObjectKind::Pet => record
                .pet()
                .map(|pet| read(pet.creature(), Some(pet.owner_guid()))),
            _ => None,
        }
    }

    /// Read the common WorldObject projection for one of the explicitly
    /// accepted canonical kinds. The callback result must be owned, so this
    /// remains compatible with a guard-based entity backend.
    pub fn with_world_object_by_kinds_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
        read: impl FnOnce(&WorldObject) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        allowed
            .contains(&record.kind())
            .then(|| read(record.object()))
    }

    pub fn contains_map_object_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.entity_world.get(&guid).is_some()
    }

    pub fn with_area_trigger_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&AreaTrigger) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::AreaTrigger {
            return None;
        }
        record.area_trigger().map(read)
    }

    pub fn with_scene_object_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&SceneObject) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::SceneObject {
            return None;
        }
        record.scene_object().map(read)
    }

    pub fn with_conversation_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Conversation) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::Conversation {
            return None;
        }
        record.conversation().map(read)
    }

    pub fn get_typed_player(&self, guid: ObjectGuid) -> Option<&Player> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        record.player()
    }

    pub fn get_typed_player_mut(&mut self, guid: ObjectGuid) -> Option<&mut Player> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        record.player_mut()
    }

    pub fn get_typed_corpse(&self, guid: ObjectGuid) -> Option<&Corpse> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Corpse {
            return None;
        }
        record.corpse()
    }

    pub fn get_typed_corpse_mut(&mut self, guid: ObjectGuid) -> Option<&mut Corpse> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Corpse {
            return None;
        }
        record.corpse_mut()
    }

    pub fn get_typed_dynamic_object(&self, guid: ObjectGuid) -> Option<&DynamicObject> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::DynamicObject {
            return None;
        }
        record.dynamic_object()
    }

    pub fn get_typed_dynamic_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut DynamicObject> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::DynamicObject {
            return None;
        }
        record.dynamic_object_mut()
    }

    pub fn typed_combat_unit_guids_like_cpp(&self) -> Vec<ObjectGuid> {
        self.entity_world
            .iter()
            .filter_map(|(guid, record)| {
                matches!(
                    record.kind(),
                    AccessorObjectKind::Player | AccessorObjectKind::Creature
                )
                .then_some(*guid)
            })
            .collect()
    }

    pub fn get_dynamic_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::DynamicObject])
    }

    pub fn get_corpse(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Corpse])
    }

    pub fn mark_active_cell(&mut self, cell: CellCoord) {
        assert!(cell.is_coord_valid());
        self.active_cells.insert(cell);
    }

    pub fn unmark_active_cell(&mut self, cell: CellCoord) {
        self.active_cells.remove(&cell);
    }

    pub fn get_ngrid(&self, coord: GridCoord) -> Option<&NGrid> {
        let index = grid_index(coord)?;
        self.grids[index].as_deref()
    }

    pub fn get_ngrid_mut(&mut self, coord: GridCoord) -> Option<&mut NGrid> {
        let index = grid_index(coord)?;
        self.grids[index].as_deref_mut()
    }

    pub fn set_ngrid(&mut self, coord: GridCoord, grid: Option<NGrid>) {
        let index = checked_grid_index(coord);
        self.grids[index] = grid.map(Box::new);
    }

    pub fn is_grid_loaded(&self, coord: GridCoord) -> bool {
        self.get_ngrid(coord)
            .is_some_and(NGrid::grid_object_data_loaded)
    }

    pub fn loaded_grid_coords_like_cpp(&self) -> Vec<GridCoord> {
        self.grids
            .iter()
            .enumerate()
            .filter_map(|(index, grid)| {
                grid.as_ref()
                    .filter(|grid| grid.grid_object_data_loaded())
                    .map(|_| {
                        GridCoord::new(
                            (index as u32) / MAX_NUMBER_OF_GRIDS,
                            (index as u32) % MAX_NUMBER_OF_GRIDS,
                        )
                    })
            })
            .collect()
    }

    pub fn ensure_grid_created(&mut self, coord: GridCoord) -> bool {
        let index = checked_grid_index(coord);
        if self.grids[index].is_some() {
            return false;
        }

        let mut grid = NGrid::from_coords(
            coord.x_coord as i32,
            coord.y_coord as i32,
            self.grid_expiry_ms,
            self.grid_unload,
        );
        grid.set_state(GridStateKind::Idle);
        self.grids[index] = Some(Box::new(grid));

        let (terrain_x, terrain_y) = terrain_grid_coords(coord);
        self.terrain.load_map_and_vmap(terrain_x, terrain_y);
        true
    }

    pub fn ensure_grid_loaded(&mut self, cell: &Cell) -> bool {
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        self.ensure_grid_created(coord);
        let index = checked_grid_index(coord);
        {
            let grid = self.grids[index].as_mut().expect("grid was just created");
            if grid.grid_object_data_loaded() {
                return false;
            }

            grid.set_grid_object_data_loaded(true);
            self.lifecycle.load_grid_objects(grid, cell);
        }
        self.activate_registered_corpses_for_grid_like_cpp(coord);
        true
    }

    pub fn ensure_grid_loaded_for_active_object(
        &mut self,
        cell: &Cell,
        kind: ActiveObjectKind,
    ) -> bool {
        let loaded_now = self.ensure_grid_loaded(cell);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        self.mark_active_cell(cell.cell_coord());

        if matches!(kind, ActiveObjectKind::Player) {
            // Use `ensure_grid_loaded_for_player_phase` when phase-shift state
            // is available; this entry point only has the object kind.
        }

        let active_expiry_ms = (self.grid_expiry_ms as f32 * 0.1) as i64;
        let grid = self.get_ngrid_mut(coord).expect("grid was just loaded");
        if grid.state() != GridStateKind::Active {
            grid.info_mut().reset_time_tracker(active_expiry_ms);
            grid.set_state(GridStateKind::Active);
        }

        loaded_now
    }

    pub fn load_grid(&mut self, x: f32, y: f32) -> bool {
        self.ensure_grid_loaded(&Cell::from_world(x, y))
    }

    pub fn load_grid_for_active_object(&mut self, x: f32, y: f32, kind: ActiveObjectKind) -> bool {
        self.ensure_grid_loaded_for_active_object(&Cell::from_world(x, y), kind)
    }

    pub fn reset_grid_expiry(&self, grid: &mut NGrid, factor: f32) {
        grid.info_mut()
            .reset_time_tracker((self.grid_expiry_ms as f32 * factor) as i64);
    }

    pub fn active_objects_near_grid(&self, grid: &NGrid) -> bool {
        if active_cells_near_grid(&self.active_cells, self.visible_distance, grid) {
            return true;
        }

        let active_non_player_cells: HashSet<_> = self
            .active_non_players_like_cpp
            .iter()
            .filter_map(|guid| {
                let record = self.map_object_record(*guid)?;
                record.object().object().is_in_world().then(|| {
                    compute_cell_coord(record.object().position().x, record.object().position().y)
                })
            })
            .collect();
        active_cells_near_grid(&active_non_player_cells, self.visible_distance, grid)
    }

    pub fn unload_grid_at(&mut self, coord: GridCoord, unload_all: bool) -> bool {
        let index = checked_grid_index(coord);
        let Some(mut grid) = self.grids[index].take() else {
            return false;
        };

        if !self.can_unload_grid(&grid, unload_all) {
            self.grids[index] = Some(grid);
            return false;
        }

        self.run_unload_lifecycle(&mut grid, unload_all);
        true
    }

    pub(super) fn can_unload_grid(&self, grid: &NGrid, unload_all: bool) -> bool {
        unload_all
            || (grid.world_creature_count_in_ngrid() == 0 && !self.active_objects_near_grid(grid))
    }

    pub(super) fn run_unload_lifecycle(&mut self, grid: &mut NGrid, unload_all: bool) {
        // C++ `Map::UnloadGrid` drains Creature/GameObject/AreaTrigger move lists
        // only in the `!unloadAll` branch, before and after the evacuator
        // (`Map.cpp:1579-1596`). `UnloadGrid(..., true)` does not drain or
        // relocate move-lists; `Map::UnloadAll` only clears Creature/GameObject
        // delayed moves before entering that loop (`Map.cpp:1646-1651`). Rust
        // still keeps the rest of this unload lifecycle represented: no
        // DynamicObject drain in this path, no full visibility/fanout/scripts/DB.
        if !unload_all {
            self.move_all_creatures_in_move_list_like_cpp();
            self.move_all_game_objects_in_move_list_like_cpp();
            self.move_all_area_triggers_in_move_list_like_cpp();
            self.lifecycle.evacuate_grid(grid);
            self.drain_grid_unload_actions_like_cpp();
            self.move_all_creatures_in_move_list_like_cpp();
            self.move_all_game_objects_in_move_list_like_cpp();
            self.move_all_area_triggers_in_move_list_like_cpp();
        }

        self.lifecycle.clean_grid(grid);
        self.drain_grid_unload_actions_like_cpp();
        self.personal_phase_tracker.unload_grid(grid);
        self.lifecycle.unload_grid_objects(grid);
        self.drain_grid_unload_actions_like_cpp();

        let coord = GridCoord::new(grid.x() as u32, grid.y() as u32);
        self.deactivate_registered_corpses_for_grid_like_cpp(coord);
        let (terrain_x, terrain_y) = terrain_grid_coords(coord);
        self.terrain.unload_map(terrain_x, terrain_y);
    }

    pub(super) fn drain_grid_unload_actions_like_cpp(&mut self) -> Vec<GridUnloadApplyOutcome> {
        let actions = self.lifecycle.take_unload_actions_like_cpp();
        if actions.is_empty() {
            return Vec::new();
        }

        apply_grid_unload_actions(self, actions)
    }
}
