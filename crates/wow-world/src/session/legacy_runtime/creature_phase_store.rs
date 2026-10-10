// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3a-1: where a creature combat phase finds the creatures it runs on.
//!
//! The legacy combat phases enumerate and mutate the creatures of the legacy
//! `SharedMapManager`. The admitted canonical executor runs the **same** phase
//! bodies over the admitted canonical map instead. This trait is the one seam
//! between the two: a phase body is written once, against
//! [`CreaturePhaseStoreLikeCpp`], and each owner supplies its store.
//!
//! - The legacy store is the legacy `MapManager` itself, under the write guard
//!   the legacy phase already takes. It enumerates every creature of every
//!   active legacy map, exactly as before.
//! - The admitted canonical store enumerates only the admitted selection of one
//!   canonical tick (the `MapObjectUpdateSelectionLikeCpp::NearbyCells` objects
//!   frozen by the admission), in admission order, and resolves them on the
//!   canonical map the executor holds locked. A creature outside the admitted
//!   selection is not part of the transition and is not found.
//!
//! Two operations are bridge-only and therefore explicit hooks, not hidden
//! differences: the legacy packet projection's movement-flag mirror after a
//! spline stop (the canonical store has no cached projection), and whether a
//! movement frame carries a transported representation for the legacy
//! bridge's canonical gate.
//!
//! #1263 F6-8D3a-2: the movement phase lends each creature to the movement
//! bodies as a `CreatureMovementLikeCpp` — with the legacy bridge's cached
//! projection, or the canonical incarnation alone — so the movement step and
//! `CreatureAI::TriggerAlert`'s `MoveDistract` run one body on either store.

use super::*;

/// The creature store a combat phase body runs against.
pub(in crate::session) trait CreaturePhaseStoreLikeCpp {
    /// The residences this store visits, in visit order.
    fn phase_map_keys_like_cpp(&self) -> Vec<(u16, u32)>;
    /// The creatures of one residence, in visit order.
    fn phase_creature_guids_like_cpp(&self, map_id: u16, instance_id: u32) -> Vec<ObjectGuid>;
    fn phase_creature_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Creature>;
    fn phase_creature_mut_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&mut wow_entities::Creature>;
    /// Bridge hook after a represented spline stop: the legacy packet
    /// projection mirrors the stopped spline's movement flags. A store without
    /// a cached projection has nothing to mirror.
    fn phase_after_spline_stop_like_cpp(&mut self, map_id: u16, instance_id: u32, guid: ObjectGuid);
    /// One creature lent to the movement bodies (`MotionMaster`, the spline
    /// and the generators), when this store visits it.
    fn phase_creature_movement_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<crate::map_manager::CreatureMovementLikeCpp<'_>>;
    /// Any creature of the residence's map, visited or not: the chase victim
    /// C++ `ChaseMovementGenerator` reaches through its `Unit*`.
    fn phase_map_creature_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Creature>;
    /// Whether a movement frame carries the moved creature's representation
    /// for the legacy bridge's canonical admission gate. A store that drives
    /// the canonical incarnation in place has nothing to transport.
    fn phase_movement_representation_like_cpp(&self) -> bool;
    /// C++ `CreatureAI::TriggerAlert` → `MotionMaster::MoveDistract(5s, angle)`
    /// (`CreatureAI.cpp:141-160`, `MotionMaster.cpp:1096-1104`): whether the
    /// distract generator started.
    fn phase_begin_alert_distract_like_cpp(
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

impl CreaturePhaseStoreLikeCpp for crate::map_manager::MapManager {
    fn phase_map_keys_like_cpp(&self) -> Vec<(u16, u32)> {
        self.active_map_keys()
    }

    fn phase_creature_guids_like_cpp(&self, map_id: u16, instance_id: u32) -> Vec<ObjectGuid> {
        self.creature_guids(map_id, instance_id)
    }

    fn phase_creature_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Creature> {
        self.find_creature(map_id, instance_id, guid)
            .map(|creature| &creature.creature)
    }

    fn phase_creature_mut_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&mut wow_entities::Creature> {
        self.find_creature_mut(map_id, instance_id, guid)
            .map(|creature| &mut creature.creature)
    }

    fn phase_after_spline_stop_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) {
        if let Some(creature) = self.find_creature_mut(map_id, instance_id, guid) {
            creature.sync_create_projection_movement_flags_like_cpp();
        }
    }

    fn phase_creature_movement_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<crate::map_manager::CreatureMovementLikeCpp<'_>> {
        self.find_creature_mut(map_id, instance_id, guid)
            .map(crate::map_manager::WorldCreature::movement_like_cpp)
    }

    fn phase_map_creature_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Creature> {
        self.phase_creature_like_cpp(map_id, instance_id, guid)
    }

    fn phase_movement_representation_like_cpp(&self) -> bool {
        true
    }
}

/// The admitted selection of one canonical tick, resolved on the canonical
/// manager the executor holds locked.
pub(in crate::session) struct AdmittedCanonicalCreatureStoreLikeCpp<'a> {
    manager: &'a mut wow_map::MapManager,
    /// `(residence, creatures)` in admission order.
    selection: Vec<((u16, u32), Vec<ObjectGuid>)>,
}

impl<'a> AdmittedCanonicalCreatureStoreLikeCpp<'a> {
    /// Build the store over the admitted objects. The admission is already
    /// fenced by the caller under the same guard; an object whose residence
    /// does not fit the legacy `(u16, u32)` key space is not visited.
    pub(in crate::session) fn new_like_cpp(
        manager: &'a mut wow_map::MapManager,
        objects: &[AdmittedCreatureExecutionObjectLikeCpp],
    ) -> Self {
        let mut selection: Vec<((u16, u32), Vec<ObjectGuid>)> = Vec::new();
        for object in objects {
            let Ok(map_id) = u16::try_from(object.map_id) else {
                continue;
            };
            let key = (map_id, object.instance_id);
            match selection.iter_mut().find(|(existing, _)| *existing == key) {
                Some((_, guids)) => guids.push(object.creature_guid),
                None => selection.push((key, vec![object.creature_guid])),
            }
        }
        Self { manager, selection }
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

impl CreaturePhaseStoreLikeCpp for AdmittedCanonicalCreatureStoreLikeCpp<'_> {
    fn phase_map_keys_like_cpp(&self) -> Vec<(u16, u32)> {
        self.selection.iter().map(|(key, _)| *key).collect()
    }

    fn phase_creature_guids_like_cpp(&self, map_id: u16, instance_id: u32) -> Vec<ObjectGuid> {
        self.selection
            .iter()
            .find(|(key, _)| *key == (map_id, instance_id))
            .map(|(_, guids)| guids.clone())
            .unwrap_or_default()
    }

    fn phase_creature_like_cpp(
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

    fn phase_creature_mut_like_cpp(
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

    fn phase_after_spline_stop_like_cpp(&mut self, _: u16, _: u32, _: ObjectGuid) {}

    fn phase_creature_movement_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<crate::map_manager::CreatureMovementLikeCpp<'_>> {
        self.phase_creature_mut_like_cpp(map_id, instance_id, guid)
            .map(crate::map_manager::CreatureMovementLikeCpp::canonical_like_cpp)
    }

    fn phase_map_creature_like_cpp(
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

    fn phase_movement_representation_like_cpp(&self) -> bool {
        false
    }
}
