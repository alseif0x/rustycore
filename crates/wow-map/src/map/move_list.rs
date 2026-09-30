// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map-object move-list queues and draining.

use super::*;

/// C++ `MapObjectCellMoveState` (`MapObject.h:28-33`) represented for
/// map-owned delayed cell/grid move-list state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectCellMoveStateLikeCpp {
    None,
    Active,
    Inactive,
}

pub type MapObjectCellMoveState = MapObjectCellMoveStateLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectMoveListFamilyLikeCpp {
    Creature,
    GameObject,
    DynamicObject,
    AreaTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingCellMoveLikeCpp {
    pub state: MapObjectCellMoveStateLikeCpp,
    pub new_position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddObjectToMoveListOutcomeLikeCpp {
    Queued,
    UpdatedExisting,
    LockedIgnored,
    MissingOrStale,
    WrongKind { actual: AccessorObjectKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveObjectFromMoveListOutcomeLikeCpp {
    MarkedInactive,
    AlreadyInactive,
    NotQueued,
    LockedIgnored,
    MissingOrStale,
    WrongKind { actual: AccessorObjectKind },
}

/// Drain summary for C++ `Map::MoveAll*InMoveList` (`Map.cpp:1239-1416`).
/// This is a map-owned seam only: it does not claim UpdatePositionData,
/// visibility fanout, AfterRelocation, respawn relocation, Pet::Remove,
/// dynamic tree, scripts/AI, ObjectAccessor, or session packet runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveListDrainSummaryLikeCpp {
    pub family: Option<MapObjectMoveListFamilyLikeCpp>,
    pub processed: usize,
    pub relocated: usize,
    pub inactive_reset: usize,
    pub not_in_world: usize,
    pub missing_or_stale: usize,
    pub wrong_kind: usize,
    pub blocked_by_unloaded_grid: usize,
    pub remove_list_queued: usize,
    pub pet_remove_requested: usize,
    pub respawn_relocation_unsupported: usize,
    pub failed_invalid_position: usize,
    pub failed_store: usize,
    pub locked_ignored: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapObjectMoveListEntry {
    pub guid: ObjectGuid,
    pub kind: AccessorObjectKind,
    pub move_state: MapObjectCellMoveState,
    pub new_position: Position,
    pub respawn_position: Option<Position>,
    pub is_pet: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapObjectMoveListPlan {
    pub relocated: Vec<ObjectGuid>,
    pub respawn_relocated: Vec<ObjectGuid>,
    pub remove_from_world: Vec<ObjectGuid>,
    pub pet_removed: Vec<ObjectGuid>,
    pub blocked_unloaded_grid: Vec<ObjectGuid>,
    pub reset_inactive_or_none: Vec<ObjectGuid>,
    pub skipped_not_in_world: Vec<ObjectGuid>,
    pub skipped_other_map_or_missing: Vec<ObjectGuid>,
    pub skipped_kind_mismatch: Vec<ObjectGuid>,
    pub failed_invalid_position: Vec<ObjectGuid>,
    pub failed_store: Vec<ObjectGuid>,
    pub unsupported_kind: Vec<ObjectGuid>,
}

fn move_list_family_accepts_kind_like_cpp(
    family: MapObjectMoveListFamilyLikeCpp,
    kind: AccessorObjectKind,
) -> bool {
    match family {
        MapObjectMoveListFamilyLikeCpp::Creature => {
            matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet)
        }
        MapObjectMoveListFamilyLikeCpp::GameObject => {
            matches!(
                kind,
                AccessorObjectKind::GameObject | AccessorObjectKind::Transport
            )
        }
        MapObjectMoveListFamilyLikeCpp::DynamicObject => kind == AccessorObjectKind::DynamicObject,
        MapObjectMoveListFamilyLikeCpp::AreaTrigger => kind == AccessorObjectKind::AreaTrigger,
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
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
