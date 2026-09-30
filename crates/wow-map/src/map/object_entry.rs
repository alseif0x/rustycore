// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owned transport for the single canonical map object table.
//!
//! Moves preserve the complete stored value, including a live creature motor.
//! Actor admission remains private until production ownership is transferred.

use wow_entities::MapObjectRecord;
use std::sync::Arc;

use crate::map_manager::WorldCreature;
use super::{
    AccessorObjectKind, GridLifecycle, Map, MapObjectStoreError, ObjectMut, ObjectRef,
    TerrainGridLoader,
};

/// Complete displaced or removed map-owned value, including its execution state.
pub struct OwnedMapObject {
    entry: ObjectEntry,
}

impl std::fmt::Debug for OwnedMapObject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("OwnedMapObject").field("entry", &self.entry).finish()
    }
}

impl OwnedMapObject {
    pub(in crate::map) fn new(entry: ObjectEntry) -> Self {
        Self { entry }
    }

    #[cfg(test)]
    pub(crate) fn record(&self) -> Option<&MapObjectRecord> {
        match &self.entry {
            ObjectEntry::Record(record) => Some(record),
            ObjectEntry::CreatureActor(_) => None,
        }
    }
}

#[derive(Debug)]
pub(in crate::map) enum ObjectEntry {
    Record(MapObjectRecord),
    CreatureActor(CreatureActorEntry),
}

/// Identity of one admitted actor, independent of GUID and entity revisions.
/// Retaining this token across removal prevents same-GUID readmission ABA.
#[derive(Debug, Clone)]
pub(crate) struct CreatureActorWitness(Arc<()>);

impl CreatureActorWitness {
    pub(crate) fn same_actor(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// The complete motor and its immutable identity move as one stored value.
#[derive(Debug)]
pub(in crate::map) struct CreatureActorEntry {
    actor: Box<WorldCreature>,
    witness: CreatureActorWitness,
}

impl CreatureActorEntry {
    fn new(actor: WorldCreature) -> Self {
        Self {
            actor: Box::new(actor),
            witness: CreatureActorWitness(Arc::new(())),
        }
    }

    pub(in crate::map) fn actor(&self) -> &WorldCreature {
        &self.actor
    }

    pub(in crate::map) fn actor_mut(&mut self) -> &mut WorldCreature {
        &mut self.actor
    }

    pub(in crate::map) fn witness(&self) -> CreatureActorWitness {
        self.witness.clone()
    }
}

impl ObjectEntry {
    pub(in crate::map) fn from_creature_actor(actor: WorldCreature) -> Self {
        Self::CreatureActor(CreatureActorEntry::new(actor))
    }

    pub(in crate::map) fn as_ref(&self) -> ObjectRef<'_> {
        match self {
            Self::Record(record) => ObjectRef::new(record),
            Self::CreatureActor(actor) => ObjectRef::from_creature(&actor.actor().creature),
        }
    }

    pub(in crate::map) fn as_mut(&mut self) -> ObjectMut<'_> {
        match self {
            Self::Record(record) => ObjectMut::new(record),
            Self::CreatureActor(actor) => ObjectMut::from_creature(&mut actor.actor_mut().creature),
        }
    }

    #[cfg(test)]
    fn into_record_fixture(self) -> Result<MapObjectRecord, Self> {
        match self {
            Self::Record(record) => Ok(record),
            actor @ Self::CreatureActor(_) => Err(actor),
        }
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub(super) fn creature_actor_entry(
        &self,
        actor: WorldCreature,
    ) -> Result<ObjectEntry, (MapObjectStoreError, WorldCreature)> {
        if let Err(error) = self.validate_creature_actor(&actor) {
            return Err((error, actor));
        }
        Ok(ObjectEntry::from_creature_actor(actor))
    }

    pub(super) fn validate_creature_actor(
        &self,
        actor: &WorldCreature,
    ) -> Result<(), MapObjectStoreError> {
        let object = actor.creature.unit().world();
        MapObjectRecord::validate_world_object(AccessorObjectKind::Creature, object)
            .map_err(MapObjectStoreError::from)?;
        self.validate_map_object(object)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod actor_tests;
