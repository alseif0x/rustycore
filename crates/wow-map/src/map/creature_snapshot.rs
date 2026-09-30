// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Complete Creature snapshot replacement inside the existing Map borrow.

use super::{
    AccessorObjectKind, GridLifecycle, Map, MapObjectRecord, MapObjectStoreError, ObjectEntry,
    ObjectRef, TerrainGridLoader, detach_typed_loot_authority_like_cpp,
    typed_loot_authorities_share_storage_like_cpp,
};
use wow_core::ObjectGuid;
use wow_entities::Creature;

pub(super) enum PreparedCreatureSnapshot {
    Record(MapObjectRecord),
    Actor(Creature),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureSnapshotReplaceError {
    Store(MapObjectStoreError),
    NotExactCreature { guid: ObjectGuid },
}

impl From<MapObjectStoreError> for CreatureSnapshotReplaceError {
    fn from(error: MapObjectStoreError) -> Self {
        Self::Store(error)
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Replace an exact existing Creature with a complete transport snapshot.
    /// The caller retains its current synchronous guard and revision/CAS gates.
    pub fn replace_creature_snapshot(
        &mut self,
        record: MapObjectRecord,
    ) -> Result<(), CreatureSnapshotReplaceError> {
        let guid = record.object().guid();
        if record.kind() != AccessorObjectKind::Creature || record.creature().is_none() {
            return Err(CreatureSnapshotReplaceError::NotExactCreature { guid });
        }
        self.validate_map_object(record.object())?;
        let exact_destination = self.entity_world.get(&guid).is_some_and(|current| {
            current.kind() == AccessorObjectKind::Creature && current.creature().is_some()
        });
        if !exact_destination {
            return Err(CreatureSnapshotReplaceError::NotExactCreature { guid });
        }

        // Preparation moves the incoming Creature out of its record before
        // any take, authority detach or index mutation. The same exclusive
        // Map borrow protects the target variant through the following take.
        match self.entity_world.prepare_creature_snapshot(record)? {
            PreparedCreatureSnapshot::Record(record) => {
                let previous = self.insert_object_entry(ObjectEntry::Record(record))?;
                drop(previous);
            }
            PreparedCreatureSnapshot::Actor(incoming) => {
                let mut entry = self
                    .entity_world
                    .take(&guid)
                    .ok_or(CreatureSnapshotReplaceError::NotExactCreature { guid })?;
                if !typed_loot_authorities_share_storage_like_cpp(
                    entry.as_ref(),
                    ObjectRef::from_creature(&incoming),
                ) {
                    detach_typed_loot_authority_like_cpp(entry.as_mut());
                }
                self.unindex_map_object_record_by_spawn_id_like_cpp(entry.as_ref());
                let old_creature = match &mut entry {
                    ObjectEntry::CreatureActor(actor) => {
                        std::mem::replace(&mut actor.actor_mut().creature, incoming)
                    }
                    ObjectEntry::Record(_) => {
                        unreachable!("snapshot target variant cannot change inside the Map borrow")
                    }
                };
                self.index_map_object_record_by_spawn_id_like_cpp(entry.as_ref());
                let displaced = self.entity_world.insert(entry);
                debug_assert!(displaced.is_none());
                // The old entity dies only after the actor has been reinserted.
                drop(old_creature);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
