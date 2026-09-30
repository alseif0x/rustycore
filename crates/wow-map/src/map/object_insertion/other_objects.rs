// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Generic and other typed admission stages; inherited kind gates stay intact.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{

    // Preserve the coordinator's captured admission facts; no new reads or context.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_other_object_entry(
        &mut self,
        mut entry: ObjectEntry,
        kind: AccessorObjectKind,
        guid: ObjectGuid,
        cell: Cell,
        grid: GridCoord,
        active_object: bool,
        grid_loaded: bool,
        grid_created: bool,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        let creature_unit_add_to_world = {
            entry
                .as_mut().object_mut()
                .set_current_cell(cell.cell_x(), cell.cell_y());
            let creature_unit_add_to_world = if let Some(creature) = entry.as_mut().creature_mut() {
                Some(creature.unit_mut().add_to_world_like_cpp())
            } else {
                entry.as_mut().object_mut().object_mut().add_to_world();
                None
            };
            entry.as_mut().object_mut().object_mut().set_is_new_object(true);
            entry.as_mut().object_mut().object_mut().set_is_new_object(false);
            creature_unit_add_to_world
        };

        let creature_search_formation = if kind == AccessorObjectKind::Creature {
            entry.as_ref().creature().map(Creature::search_formation_like_cpp)
        } else {
            None
        };
        if let Some(outcome) = creature_search_formation {
            self.apply_creature_search_formation_like_cpp(guid, outcome);
        }

        let creature_aim_initialize = if kind == AccessorObjectKind::Creature {
            entry.as_ref().creature().map(Creature::aim_initialize_like_cpp)
        } else {
            None
        };

        let creature_vehicle_reset = if kind == AccessorObjectKind::Creature {
            entry.as_mut().creature_mut().and_then(|creature| {
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
            entry.as_mut().creature_mut().and_then(|creature| {
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
            entry
                .as_ref().creature()
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
                if let Some(game_object) = entry
                    .as_mut().game_object_mut()
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

        let previous = self.insert_object_entry(entry)?;
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
