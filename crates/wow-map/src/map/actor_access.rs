// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Internal admission and access for the single map-owned Creature motor.
//! Production registration and the runtime producer are not switched here.

use super::{
    AccessorObjectKind, CreatureActorWitness, GridLifecycle, Map, MapObjectStoreError,
    ObjectEntry, TerrainGridLoader,
};
use crate::map_manager::WorldCreature;
use wow_core::ObjectGuid;

mod fresh;
pub use fresh::{FreshCreatureActorAdmission, FreshCreatureActorAdmissionError};

#[derive(Debug)]
pub(crate) enum CreatureActorAdmission {
    Inserted { witness: CreatureActorWitness },
    ExistingActor { incoming: WorldCreature, witness: CreatureActorWitness },
    ExistingRecord { incoming: WorldCreature },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreatureActorAdmissionError {
    Store(MapObjectStoreError),
    NotExactCreature { guid: ObjectGuid, actual_kind: AccessorObjectKind },
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Validate before classifying the existing entry. Admission never replaces
    /// or promotes that entry, detaches loot, or reconstructs its motor.
    /// C++ Map.cpp:530-577 and Creature.cpp:333-351 register one map-owned object;
    /// the explicit Record result preserves the unresolved Rust migration seam.
    pub(crate) fn admit_creature_actor(
        &mut self,
        incoming: WorldCreature,
    ) -> Result<CreatureActorAdmission, (CreatureActorAdmissionError, WorldCreature)> {
        if let Err(error) = self.validate_creature_actor(&incoming) {
            return Err((CreatureActorAdmissionError::Store(error), incoming));
        }
        let guid = incoming.guid();
        if let Some(current) = self.entity_world.get(&guid) {
            if current.kind() != AccessorObjectKind::Creature || current.creature().is_none() {
                return Err((CreatureActorAdmissionError::NotExactCreature {
                    guid,
                    actual_kind: current.kind(),
                }, incoming));
            }
            return Ok(match self.creature_actor_witness(guid) {
                Some(witness) => CreatureActorAdmission::ExistingActor { incoming, witness },
                None => CreatureActorAdmission::ExistingRecord { incoming },
            });
        }

        // Only an empty slot receives a new Box and a fresh witness. Reuse the
        // existing indexing path under this same exclusive Map borrow.
        let entry = ObjectEntry::from_creature_actor(incoming);
        let witness = match &entry {
            ObjectEntry::CreatureActor(actor) => actor.witness(),
            ObjectEntry::Record(_) => unreachable!("actor constructor cannot produce a Record"),
        };
        let displaced = self.insert_object_entry(entry)
            .expect("the actor's map identity was validated before constructing its entry");
        debug_assert!(displaced.is_none());
        Ok(CreatureActorAdmission::Inserted { witness })
    }

    pub(crate) fn creature_actor(&self, guid: ObjectGuid) -> Option<&WorldCreature> {
        self.entity_world.creature_actor(guid).map(|entry| entry.actor())
    }

    /// Move a real actor into an empty fixture slot through normal admission.
    /// Existing actors/records and admission errors are fixture setup failures.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn test_fixture_admit_creature_actor(&mut self, incoming: WorldCreature) {
        match self.admit_creature_actor(incoming) {
            Ok(CreatureActorAdmission::Inserted { .. }) => {}
            Ok(CreatureActorAdmission::ExistingActor { .. }) => {
                panic!("actor fixture requires a new admission, not an existing actor");
            }
            Ok(CreatureActorAdmission::ExistingRecord { .. }) => {
                panic!("actor fixture cannot promote or displace an existing record");
            }
            Err((error, _incoming)) => panic!("actor fixture admission failed: {error:?}"),
        }
    }

    pub(crate) fn creature_actor_mut(&mut self, guid: ObjectGuid) -> Option<&mut WorldCreature> {
        self.entity_world.creature_actor_mut(guid).map(|entry| entry.actor_mut())
    }

    pub(crate) fn creature_actor_witness(&self, guid: ObjectGuid) -> Option<CreatureActorWitness> {
        self.entity_world.creature_actor(guid).map(|entry| entry.witness())
    }
}

#[cfg(test)]
mod tests;
