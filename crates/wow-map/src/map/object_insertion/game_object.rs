// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact GameObject AddToWorld model, collision and publication stages.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    // Preserve the coordinator's captured admission facts; no new reads or context.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_game_object_entry(
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
        entry
            .as_mut()
            .object_mut()
            .set_current_cell(cell.cell_x(), cell.cell_y());
        let object_store_present_before_callback = self
            .map_object_record(guid)
            .is_some_and(|view| view.game_object().is_some());
        let spawn_index_present_before_callback =
            entry.as_ref().game_object().is_some_and(|game_object| {
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
        let previous = self.insert_object_entry(entry)?;

        let gameobject_store_inserted_before_add_to_world = self
            .map_object_record(guid)
            .is_some_and(|view| view.game_object().is_some());
        let gameobject_spawn_indexed_before_add_to_world = self
            .map_object_record(guid)
            .and_then(|view| view.game_object())
            .is_some_and(|game_object| {
                let spawn_id = game_object.spawn_id();
                spawn_id != 0
                    && self
                        .gameobject_spawn_id_store_guids_like_cpp(spawn_id)
                        .contains(&guid)
            });
        let has_represented_model = self
            .map_object_record(guid)
            .and_then(|view| view.game_object())
            .is_some_and(GameObject::has_represented_gameobject_model_like_cpp);
        let (gameobject_model_insert, gameobject_collision_enable) = if has_represented_model {
            let gameobject_model_insert =
                self.insert_gameobject_model_like_cpp(RepresentedGameObjectModelKeyLikeCpp {
                    owner_guid: guid,
                });
            let gameobject_collision_enable = self
                .entity_world
                .get_mut(&guid)
                .and_then(ObjectMut::game_object_mut)
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
                    let collision =
                        game_object.enable_represented_gameobject_collision_like_cpp(toggled_state);
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
            .and_then(ObjectMut::game_object_mut)
        {
            game_object.world_mut().object_mut().add_to_world();
        }
        let add_to_map_tail =
            self.represent_add_to_map_post_add_to_world_tail_like_cpp(kind, guid, active_object);
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
}
