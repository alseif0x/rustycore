// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3a-1: where a creature phase finds the creatures it runs on.
//!
//! The admitted canonical executor runs every phase body over the admitted
//! selection of one canonical tick (the `MapObjectUpdateSelectionLikeCpp::
//! NearbyCells` objects frozen by the admission), in admission order, resolved
//! on the canonical map the executor holds locked. A creature outside the
//! admitted selection is not part of the transition and is not found.
//!
//! #1263 F6-8E: this is the only creature store; the legacy `MapManager` store
//! and the bridge-only hooks it needed (the projection's movement-flag mirror
//! after a spline stop, and transporting a movement frame's representation to
//! the legacy bridge's canonical gate) are gone.

use super::*;

/// The admitted selection of one canonical tick, resolved on the canonical
/// manager the executor holds locked.
pub(in crate::session) struct AdmittedCanonicalCreatureStoreLikeCpp<'a> {
    manager: &'a mut wow_map::MapManager,
    /// `(residence, creatures)` in admission order.
    selection: Vec<((u16, u32), Vec<ObjectGuid>)>,
}

impl<'a> AdmittedCanonicalCreatureStoreLikeCpp<'a> {
    /// Build the store over the admitted maps and objects. The admission is
    /// already fenced by the caller under the same guard; an object whose
    /// residence does not fit the legacy `(u16, u32)` key space is not visited.
    ///
    /// #1263 F6-8D3a-3: every admitted map is a residence, with or without
    /// selected objects, as C++ `Map::Update` runs every active map —
    /// the lifecycle phase drains a map's respawn queue after its last creature
    /// despawned.
    pub(in crate::session) fn new_like_cpp(
        manager: &'a mut wow_map::MapManager,
        maps: &[AdmittedCreatureExecutionMapLikeCpp],
        objects: &[AdmittedCreatureExecutionObjectLikeCpp],
    ) -> Self {
        let mut selection: Vec<((u16, u32), Vec<ObjectGuid>)> = Vec::new();
        let residences = maps
            .iter()
            .map(|map| (map.map_id, map.instance_id, None))
            .chain(objects.iter().map(|object| {
                (
                    object.map_id,
                    object.instance_id,
                    Some(object.creature_guid),
                )
            }));
        for (map_id, instance_id, guid) in residences {
            let Ok(map_id) = u16::try_from(map_id) else {
                continue;
            };
            let key = (map_id, instance_id);
            let index = match selection.iter().position(|(existing, _)| *existing == key) {
                Some(index) => index,
                None => {
                    selection.push((key, Vec::new()));
                    selection.len() - 1
                }
            };
            selection[index].1.extend(guid);
        }
        Self { manager, selection }
    }

    /// The canonical manager, read-only.
    pub(in crate::session) fn manager_like_cpp(&self) -> &wow_map::MapManager {
        self.manager
    }

    /// The canonical manager itself, for the map-owned operations a phase runs
    /// beside its creatures (victims, commands).
    pub(in crate::session) fn manager_mut_like_cpp(&mut self) -> &mut wow_map::MapManager {
        self.manager
    }

    fn admits_like_cpp(&self, map_id: u16, instance_id: u32, guid: ObjectGuid) -> bool {
        self.selection
            .iter()
            .any(|(key, guids)| *key == (map_id, instance_id) && guids.contains(&guid))
    }
}

impl AdmittedCanonicalCreatureStoreLikeCpp<'_> {
    pub(in crate::session) fn phase_map_keys_like_cpp(&self) -> Vec<(u16, u32)> {
        self.selection.iter().map(|(key, _)| *key).collect()
    }

    pub(in crate::session) fn phase_creature_guids_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> Vec<ObjectGuid> {
        self.selection
            .iter()
            .find(|(key, _)| *key == (map_id, instance_id))
            .map(|(_, guids)| guids.clone())
            .unwrap_or_default()
    }

    pub(in crate::session) fn phase_creature_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Creature> {
        if !self.admits_like_cpp(map_id, instance_id, guid) {
            return None;
        }
        self.manager
            .find_map(u32::from(map_id), instance_id)?
            .map()
            .get_typed_creature(guid)
    }

    pub(in crate::session) fn phase_creature_mut_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&mut wow_entities::Creature> {
        if !self.admits_like_cpp(map_id, instance_id, guid) {
            return None;
        }
        self.manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_creature_mut(guid)
    }

    pub(in crate::session) fn phase_creature_movement_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<crate::map_manager::CreatureMovementLikeCpp<'_>> {
        self.phase_creature_mut_like_cpp(map_id, instance_id, guid)
            .map(crate::map_manager::CreatureMovementLikeCpp::canonical_like_cpp)
    }

    pub(in crate::session) fn phase_map_creature_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Creature> {
        self.manager
            .find_map(u32::from(map_id), instance_id)?
            .map()
            .get_typed_creature(guid)
    }

    /// C++ `CreatureAI::TriggerAlert` → `MotionMaster::MoveDistract(5s, angle)`
    /// (`CreatureAI.cpp:141-160`, `MotionMaster.cpp:1096-1104`): whether the
    /// distract generator started.
    pub(in crate::session) fn phase_begin_alert_distract_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
        orientation: f32,
    ) -> bool {
        self.phase_creature_movement_like_cpp(map_id, instance_id, guid)
            .is_some_and(|mut creature| {
                creature
                    .begin_distract_movement_like_cpp(5_000, orientation)
                    .is_some()
            })
    }
}
