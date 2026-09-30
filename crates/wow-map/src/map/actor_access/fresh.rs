// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owned fresh admission; live transfers and Record promotion are separate operations.

use super::*;
use crate::coords::is_valid_map_coord_2d;
use crate::map::AddToMapOutcome;

/// Admission never displaces an existing entry. Duplicate outcomes retain the
/// complete incoming motor, without exposing the stored actor or its witness.
#[derive(Debug)]
pub enum FreshCreatureActorAdmission {
    Inserted { outcome: AddToMapOutcome },
    ExistingActor { incoming: WorldCreature },
    ExistingRecord { incoming: WorldCreature },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FreshCreatureActorAdmissionError {
    Store(MapObjectStoreError),
    AlreadyInWorld {
        guid: ObjectGuid,
    },
    InvalidCoordinates {
        guid: ObjectGuid,
        x: f32,
        y: f32,
    },
    NotExactCreature {
        guid: ObjectGuid,
        actual_kind: AccessorObjectKind,
    },
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Move a fresh, complete motor through the existing AddToMap lifecycle.
    /// All recoverable rejection gates precede grid, cell and store mutation.
    /// A duplicate Record is returned explicitly, never promoted or rebuilt.
    ///
    /// C++ a5f8da2e Map.cpp:530-577 and Creature.cpp:333-351 establish the
    /// grid -> store/spawn index -> UnitAddToWorld -> formation/AI/vehicle/zone
    /// -> active/visibility order. Existing represented lifecycle gaps remain.
    pub fn admit_fresh_creature_actor(
        &mut self,
        incoming: WorldCreature,
    ) -> Result<FreshCreatureActorAdmission, (FreshCreatureActorAdmissionError, WorldCreature)>
    {
        if let Err(error) = self.validate_creature_actor(&incoming) {
            return Err((FreshCreatureActorAdmissionError::Store(error), incoming));
        }
        let guid = incoming.guid();
        let object = incoming.creature.unit().world();
        if object.object().is_in_world() {
            return Err((
                FreshCreatureActorAdmissionError::AlreadyInWorld { guid },
                incoming,
            ));
        }
        let position = object.position();
        if !is_valid_map_coord_2d(position.x, position.y) {
            return Err((
                FreshCreatureActorAdmissionError::InvalidCoordinates {
                    guid,
                    x: position.x,
                    y: position.y,
                },
                incoming,
            ));
        }
        if let Some(current) = self.entity_world.get(&guid) {
            if current.kind() != AccessorObjectKind::Creature || current.creature().is_none() {
                return Err((
                    FreshCreatureActorAdmissionError::NotExactCreature {
                        guid,
                        actual_kind: current.kind(),
                    },
                    incoming,
                ));
            }
            return Ok(if self.entity_world.creature_actor(guid).is_some() {
                FreshCreatureActorAdmission::ExistingActor { incoming }
            } else {
                FreshCreatureActorAdmission::ExistingRecord { incoming }
            });
        }

        // No fallible gate remains: AddToMap checks the same map identity and
        // coordinates, and insert_object_entry checks only that map identity.
        // The exclusive Map borrow prevents intervening admissions; grid hooks
        // receive terrain/grid state, not Map or its entity store. The lifecycle
        // changes current-cell/in-world state, never GUID or map identity.
        // Reuse its sole body rather than duplicating or reinitializing a motor.
        let outcome = self
            .add_object_entry_to_map(ObjectEntry::from_creature_actor(incoming))
            .expect("fresh actor preflight covers every AddToMap rejection gate");
        debug_assert!(outcome.inserted && !outcome.already_in_world);
        Ok(FreshCreatureActorAdmission::Inserted { outcome })
    }
}

#[cfg(test)]
mod tests;
