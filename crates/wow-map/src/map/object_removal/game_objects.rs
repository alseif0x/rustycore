// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject owner cleanup, spell-owned removals and linked-trap scheduling.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
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
            .is_some_and(|record| record.is_unit_owner());
        let owned_guids_before = self
            .map_object_record(owner_guid)
            .and_then(|record| record.unit())
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
                    .and_then(|record| record.game_object())
                    .is_some_and(|game_object| game_object.spell_id() == spell_id)
            })
            .collect();

        let mut owner_guid_cleared = 0;
        let mut respawn_time_cleared = 0;
        for guid in &matched_guids {
            if let Some(game_object) = self
                .entity_world
                .get_mut(guid)
                .and_then(ObjectMut::game_object_mut)
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
            .and_then(|record| record.unit_mut())
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
    pub(in crate::map) fn gameobject_remove_from_owner_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<GameObjectRemoveFromOwnerOutcomeLikeCpp> {
        let (owner_guid_before, spell_id) = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(|record| record.game_object())
            .filter(|game_object| game_object.world().object().is_in_world())
            .map(|game_object| (game_object.owner_guid(), game_object.spell_id()))?;

        let owner_found_as_unit_like = !owner_guid_before.is_empty()
            && self
                .map_object_record(owner_guid_before)
                .is_some_and(|record| record.is_unit_owner());
        let cleared_owner = !owner_guid_before.is_empty();

        if cleared_owner {
            if let Some(game_object) = self
                .entity_world
                .get_mut(&guid)
                .and_then(ObjectMut::game_object_mut)
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
                .map(|mut record| {
                    let creature_ai_callback_represented = match record.kind() {
                        AccessorObjectKind::Creature => record
                            .reborrow()
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
                            .reborrow()
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
                    let Some(owner) = record.unit_mut() else {
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
    pub(super) fn gameobject_remove_linked_trap_like_cpp(
        &mut self,
        guid: ObjectGuid,
        remove_from_map_in_progress: &mut HashSet<ObjectGuid>,
    ) -> Option<GameObjectRemoveLinkedTrapOutcomeLikeCpp> {
        let linked_trap_guid = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(|record| record.game_object())
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
}
