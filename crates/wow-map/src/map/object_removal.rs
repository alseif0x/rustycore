// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map object removal and lifecycle cleanup.

use super::*;
use crate::map_rules::{
    map_record_is_unit_like_gameobject_owner_like_cpp, map_record_unit_like_cpp,
    map_record_unit_mut_like_cpp, player_set_viewpoint_outcome_like_cpp,
};

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
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
        let record = self.entity_world.remove(&guid)?;
        self.unindex_map_object_record_by_spawn_id_like_cpp(&record);
        self.unlink_map_reference_like_cpp(guid);
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
            .is_some_and(map_record_is_unit_like_gameobject_owner_like_cpp);
        let owned_guids_before = self
            .map_object_record(owner_guid)
            .and_then(map_record_unit_like_cpp)
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
                .entity_world
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
            .entity_world
            .get_mut(&owner_guid)
            .and_then(map_record_unit_mut_like_cpp)
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
                .is_some_and(map_record_is_unit_like_gameobject_owner_like_cpp);
        let cleared_owner = !owner_guid_before.is_empty();

        if cleared_owner {
            if let Some(game_object) = self
                .entity_world
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
            self.entity_world
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
                    let Some(owner) = map_record_unit_mut_like_cpp(record) else {
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
    /// Ownership: only canonical same-map `Map::entity_world` typed records are
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
                        player_set_viewpoint_outcome_like_cpp(
                            player_guid,
                            viewpoint_guid,
                            false,
                            PlayerSetViewpointStatusLikeCpp::Removed,
                            None,
                            false,
                            true,
                        )
                    }
                    Some(_) => player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                        None,
                        false,
                        false,
                    ),
                    None => player_set_viewpoint_outcome_like_cpp(
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
                        player_set_viewpoint_outcome_like_cpp(
                            player_guid,
                            viewpoint_guid,
                            false,
                            PlayerSetViewpointStatusLikeCpp::Removed,
                            None,
                            false,
                            true,
                        )
                    }
                    _ => player_set_viewpoint_outcome_like_cpp(
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
                .entity_world
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
            let source_kind = self.map_object_record(guid).map(MapObjectRecord::kind);
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
                .and_then(MapObjectRecord::creature)
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

}
