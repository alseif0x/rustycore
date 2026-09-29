// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map object insertion and activation lifecycle.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
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
            if kind == AccessorObjectKind::Transport {
                self.mark_transport_players_for_visibility_like_cpp(guid);
            } else {
                self.mark_nearby_players_for_visibility_like_cpp(guid);
            }

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
            if kind == AccessorObjectKind::Transport {
                self.mark_transport_players_for_visibility_like_cpp(guid);
            } else {
                self.mark_nearby_players_for_visibility_like_cpp(guid);
            }

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
        if kind == AccessorObjectKind::Transport {
            self.mark_transport_players_for_visibility_like_cpp(guid);
        } else {
            self.mark_nearby_players_for_visibility_like_cpp(guid);
        }
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
}
