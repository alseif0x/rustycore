// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map object removal and lifecycle cleanup.

use super::*;

mod game_objects;
mod membership;
mod viewpoints;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn remove_map_object(&mut self, guid: ObjectGuid) -> Option<OwnedMapObject> {
        self.take_object_entry(guid).map(OwnedMapObject::new)
    }

    pub(super) fn take_object_entry(&mut self, guid: ObjectGuid) -> Option<ObjectEntry> {
        let entry = self.entity_world.take(&guid)?;
        self.unindex_map_object_record_by_spawn_id_like_cpp(entry.as_ref());
        self.unlink_map_reference_like_cpp(guid);
        Some(entry)
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
                .and_then(|record| record.dynamic_object())
                .is_some_and(|dynamic_object| {
                    dynamic_object.world().object().is_in_world()
                        && dynamic_object.is_caster_viewpoint()
                });
            let dynamic_object_caster_viewpoint = should_cleanup_dynamic_object_caster_viewpoint
                .then(|| self.apply_dynamic_object_caster_viewpoint_like_cpp(guid, false));
            let dynamic_object_remove_cleanup = self
                .entity_world
                .get_mut(&guid)
                .and_then(ObjectMut::dynamic_object_mut)
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
                .and_then(|record| record.game_object())
                .filter(|game_object| game_object.world().object().is_in_world())
                .filter(|game_object| game_object.has_represented_gameobject_model_like_cpp())
                .map(|_| RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid });
            let gameobject_model_remove_pending_before_callback = gameobject_model_key
                .is_some_and(|key| self.contains_gameobject_model_like_cpp(key));
            let gameobject_zone_script_remove = self
                .map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                .and_then(|record| record.game_object())
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
            let source_kind = self.map_object_record(guid).map(|record| record.kind());
            let creature_destroy_recipient_guids = if remove_from_map_was_in_world
                && (guid.is_creature_or_pet() || guid.is_corpse())
            {
                self.capture_object_visibility_destroy_recipients_like_cpp(guid)
            } else if remove_from_map_was_in_world {
                if source_kind == Some(AccessorObjectKind::Transport) {
                    self.mark_transport_players_for_visibility_like_cpp(guid);
                } else {
                    self.mark_nearby_players_for_visibility_like_cpp(guid);
                }
                Vec::new()
            } else {
                Vec::new()
            };
            // C++ `Map::RemoveFromMap` performs the destroy visibility walk
            // while the source is still attached. Mark recipients and retain
            // the object subset before erasing the canonical record;
            // packet delivery remains outside this map mutation.
            let creature_zone_script_remove = self
                .map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::Creature)
                .and_then(|record| record.creature())
                .filter(|creature| creature.unit().world().object().is_in_world())
                .map(|_| CreatureZoneScriptRemoveOutcomeLikeCpp {
                    guid,
                    represented_callback: true,
                    script_dispatch_represented: false,
                });
            let creature_remove_formation = self.remove_creature_from_formation_like_cpp(guid);
            let creature_unit_remove_from_world = self
                .entity_world
                .get_mut(&guid)
                .and_then(ObjectMut::creature_mut)
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
            let mut entry = self
                .take_object_entry(guid)
                .ok_or(RemoveFromMapError::ObjectNotFound { guid })?;
            // Rust's non-delete outcome retains only the erased
            // `WorldObject`; the typed entity and any actor motor still die.
            // Until this API retains the owned entry like C++ retains the pointer,
            // both paths must terminally detach that otherwise orphaned
            // authority. A stale lease may finish only if it already crossed
            // the protected durable boundary.
            detach_typed_loot_authority_like_cpp(entry.as_mut());
            let was_world_object_like_cpp = map_record_is_world_object_like_cpp(entry.as_ref());
            let was_in_world = remove_from_map_was_in_world;
            let cxx_in_world =
                was_in_world && remove_from_map_in_world_eligible_type_like_cpp(kind);
            let personal_phase_owner = entry
                .as_ref()
                .object()
                .phase_shift()
                .personal_guid_like_cpp();
            let cell = Cell::from_world(
                entry.as_ref().object().position().x,
                entry.as_ref().object().position().y,
            );
            let grid = GridCoord::new(cell.grid_x(), cell.grid_y());

            entry.as_mut().object_mut().object_mut().remove_from_world();
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

            entry.as_mut().object_mut().clear_current_cell();
            entry
                .as_mut()
                .object_mut()
                .reset_map()
                .map_err(RemoveFromMapError::ResetMap)?;

            if (cxx_in_world && guid.is_creature_or_pet()) || (was_in_world && guid.is_corpse()) {
                self.pending_object_visibility_destroy_recipients_like_cpp
                    .push(ObjectVisibilityDestroyRecipientsLikeCpp {
                        object_guid: guid,
                        recipient_guids: creature_destroy_recipient_guids,
                    });
            }

            // Preserve the typed Player for MapManager's detached/far-teleport
            // owner. The `WorldObject` is only an immutable compatibility
            // projection in the outcome; no second mutable Player is created.
            let object = entry.as_ref().object().clone();
            let (player, terminal_entry) =
                if !delete_from_world && kind == AccessorObjectKind::Player {
                    match entry {
                        ObjectEntry::Record(record) => (record.into_player().ok(), None),
                        actor @ ObjectEntry::CreatureActor(_) => (None, Some(actor)),
                    }
                } else {
                    (None, Some(entry))
                };

            let outcome = RemoveFromMapOutcome {
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
            };
            // Preserve the existing end-of-operation body-erasure gap,
            // including non-delete removal. No actor is degraded to Record;
            // the complete wrapper retains its witness until this disposition.
            match terminal_entry {
                Some(ObjectEntry::Record(record)) => drop(record),
                Some(ObjectEntry::CreatureActor(actor_entry)) => drop(actor_entry),
                None => {}
            }
            Ok(outcome)
        })();
        remove_from_map_in_progress.remove(&guid);
        outcome
    }

    /// C++ `Map::DespawnAll` represented over map-local by-spawn indexes.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2034-2055` snapshots Creature/GameObject by-spawn stores and
    ///   queues each object through `AddObjectToRemoveList`.
    /// - `Map.cpp:2547-2555` marks each queued object destroyed and runs cleanup
    ///   before insertion into the map-owned remove-list.
    /// - `Map.cpp:2574-2646` later physically drains the list.
    pub fn despawn_all_by_spawn_id_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> DespawnAllBySpawnIdOutcomeLikeCpp {
        let mut outcome = DespawnAllBySpawnIdOutcomeLikeCpp {
            object_type,
            spawn_id,
            queued: 0,
            removed: 0,
            duplicates: 0,
            stale_index_entries: 0,
            remove_errors: 0,
            unsupported_live_despawn_type: 0,
        };

        let guids = match object_type {
            SpawnObjectType::Creature => self.creature_spawn_id_store_guids_like_cpp(spawn_id),
            SpawnObjectType::GameObject => self.gameobject_spawn_id_store_guids_like_cpp(spawn_id),
            SpawnObjectType::AreaTrigger => {
                outcome.unsupported_live_despawn_type = 1;
                return outcome;
            }
        };

        for guid in guids {
            let still_matches = match object_type {
                SpawnObjectType::Creature => self
                    .map_object_record(guid)
                    .and_then(|record| record.creature())
                    .is_some_and(|creature| creature.spawn_id() == spawn_id),
                SpawnObjectType::GameObject => self
                    .map_object_record(guid)
                    .and_then(|record| record.game_object())
                    .is_some_and(|gameobject| gameobject.spawn_id() == spawn_id),
                SpawnObjectType::AreaTrigger => false,
            };
            if !still_matches {
                outcome.stale_index_entries += 1;
                continue;
            }

            let queued = self.add_object_to_remove_list_like_cpp(guid);
            if queued.missing_or_stale {
                outcome.stale_index_entries += 1;
            } else if queued.unsupported_kind.is_some() {
                outcome.unsupported_live_despawn_type += 1;
            } else if queued.duplicate {
                outcome.duplicates += 1;
            } else if queued.queued {
                outcome.queued += 1;
            }
        }

        outcome
    }
}
