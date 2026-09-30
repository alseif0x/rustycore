// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private storage facade for the canonical objects owned by one [`super::Map`].
//!
//! C++ keeps the corresponding ownership in `Map::_objectsStore`
//! (`Map.h:418,793`). This facade deliberately preserves the current
//! `HashMap<ObjectGuid, ObjectEntry>` behavior while removing that concrete
//! storage choice from `Map`. It exposes no `Deref`, so callers cannot acquire a
//! new dependency on `HashMap` while borrowed views and owned transport keep
//! the concrete entry representation inside this module.

use crate::map_rules::snapshot_from_creature;
use std::collections::HashMap;

use wow_core::ObjectGuid;
use wow_entities::{AccessorObjectKind, Creature, MapObjectRecord};

use super::{CreatureTransformVitalsSnapshotLikeCpp, ObjectEntry, ObjectMut, ObjectRef};
use super::creature_snapshot::{CreatureSnapshotReplaceError, PreparedCreatureSnapshot};
use super::object_entry::CreatureActorEntry;

#[derive(Debug, Default)]
pub(super) struct EntityWorld {
    entries_by_guid: HashMap<ObjectGuid, ObjectEntry>,
}

impl EntityWorld {
    pub(super) fn get(&self, guid: &ObjectGuid) -> Option<ObjectRef<'_>> {
        self.entries_by_guid.get(guid).map(ObjectEntry::as_ref)
    }

    pub(super) fn get_mut(&mut self, guid: &ObjectGuid) -> Option<ObjectMut<'_>> {
        self.entries_by_guid.get_mut(guid).map(ObjectEntry::as_mut)
    }

    pub(super) fn kind(&self, guid: ObjectGuid) -> Option<AccessorObjectKind> {
        self.entries_by_guid.get(&guid).map(|entry| entry.as_ref().kind())
    }

    pub(super) fn creature_actor(&self, guid: ObjectGuid) -> Option<&CreatureActorEntry> {
        match self.entries_by_guid.get(&guid)? {
            ObjectEntry::CreatureActor(actor) => Some(actor),
            ObjectEntry::Record(_) => None,
        }
    }

    pub(super) fn creature_actor_mut(&mut self, guid: ObjectGuid) -> Option<&mut CreatureActorEntry> {
        match self.entries_by_guid.get_mut(&guid)? {
            ObjectEntry::CreatureActor(actor) => Some(actor),
            ObjectEntry::Record(_) => None,
        }
    }

    pub(super) fn with_creature<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Creature) -> R,
    ) -> Option<R> {
        let record = self.entries_by_guid.get(&guid)?.as_ref();
        (record.kind() == AccessorObjectKind::Creature)
            .then(|| record.creature())
            .flatten()
            .map(read)
    }

    pub(super) fn with_creature_mut<R>(
        &mut self,
        guid: ObjectGuid,
        write: impl FnOnce(&mut Creature) -> R,
    ) -> Option<R> {
        let record = self.entries_by_guid.get_mut(&guid)?.as_mut();
        (record.kind() == AccessorObjectKind::Creature)
            .then(|| record.creature_mut())
            .flatten()
            .map(write)
    }

    pub(super) fn creature_transform_vitals_snapshot(
        &self,
        guid: ObjectGuid,
    ) -> Option<CreatureTransformVitalsSnapshotLikeCpp> {
        self.with_creature(guid, |creature| snapshot_from_creature(guid, creature))
    }

    pub(super) fn creature_transform_vitals_lookups(
        &self,
        guids: impl IntoIterator<Item = ObjectGuid>,
    ) -> Vec<(ObjectGuid, Option<CreatureTransformVitalsSnapshotLikeCpp>)> {
        let mut lookups: Vec<_> = guids
            .into_iter()
            .map(|guid| (guid, self.creature_transform_vitals_snapshot(guid)))
            .collect();
        lookups.sort_unstable_by_key(|(guid, _)| *guid);
        lookups
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&ObjectGuid, ObjectRef<'_>)> {
        self.entries_by_guid.iter().map(|(guid, record)| (guid, record.as_ref()))
    }

    pub(super) fn values(&self) -> impl Iterator<Item = ObjectRef<'_>> {
        self.entries_by_guid.values().map(ObjectEntry::as_ref)
    }

    pub(super) fn len(&self) -> usize {
        self.entries_by_guid.len()
    }

    pub(super) fn insert(&mut self, entry: ObjectEntry) -> Option<ObjectEntry> {
        self.entries_by_guid.insert(entry.as_ref().object().guid(), entry)
    }

    pub(super) fn take(&mut self, guid: &ObjectGuid) -> Option<ObjectEntry> {
        self.entries_by_guid.remove(guid)
    }

    /// Borrow-only admission classification for quiescent ownership transport.
    pub(super) fn creature_transport_counterpart(
        &self,
        guid: ObjectGuid,
    ) -> Result<&Creature, super::actor_transport::CreatureActorTransportError> {
        use super::actor_transport::CreatureActorTransportError as Error;
        let entry = self.entries_by_guid.get(&guid)
            .ok_or(Error::MissingCounterpart { guid })?;
        if entry.as_ref().object().guid() != guid {
            return Err(Error::GuidMismatch { guid });
        }
        match entry {
            ObjectEntry::CreatureActor(_) => Err(Error::ExistingActor { guid }),
            ObjectEntry::Record(record) => {
                if record.kind() != AccessorObjectKind::Creature {
                    return Err(Error::NotExactCreature { guid });
                }
                record.creature().ok_or(Error::NotExactCreature { guid })
            }
        }
    }

    /// Raw vacant restoration retains every rejected owned value. It does not
    /// detach loot or touch any Map index, membership or lifecycle hook.
    pub(super) fn restore_transport_entry(
        &mut self,
        guid: ObjectGuid,
        entry: ObjectEntry,
    ) -> Result<(), ObjectEntry> {
        use std::collections::hash_map::Entry;
        if entry.as_ref().object().guid() != guid {
            return Err(entry);
        }
        match self.entries_by_guid.entry(guid) {
            Entry::Occupied(_) => Err(entry),
            Entry::Vacant(slot) => {
                slot.insert(entry);
                Ok(())
            }
        }
    }

    pub(super) fn prepare_creature_snapshot(
        &self,
        record: MapObjectRecord,
    ) -> Result<PreparedCreatureSnapshot, CreatureSnapshotReplaceError> {
        let guid = record.object().guid();
        match self.entries_by_guid.get(&guid) {
            Some(ObjectEntry::Record(_)) => Ok(PreparedCreatureSnapshot::Record(record)),
            Some(ObjectEntry::CreatureActor(_)) => record.into_creature()
                .map(PreparedCreatureSnapshot::Actor)
                .map_err(|_| CreatureSnapshotReplaceError::NotExactCreature { guid }),
            None => Err(CreatureSnapshotReplaceError::NotExactCreature { guid }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::guid::HighGuid;
    use wow_entities::{Creature, GameObject, MapObjectRecord};

    fn creature_record(guid: ObjectGuid, health: u64) -> MapObjectRecord {
        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .set_map(571, 7)
            .expect("fixture creature should accept its map identity");
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(health);
        creature.unit_mut().set_combat_reach(1.75);
        MapObjectRecord::new_creature(creature)
            .expect("fixture creature should form a canonical map record")
    }

    #[test]
    fn same_guid_replacement_keeps_one_canonical_record() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 100, 42);
        let mut world = EntityWorld::default();

        assert!(world.insert(ObjectEntry::Record(creature_record(guid, 25))).is_none());
        let displaced = world
            .insert(ObjectEntry::Record(creature_record(guid, 75)))
            .expect("same GUID should displace exactly one canonical record");

        assert_eq!(displaced.as_ref().creature().unwrap().current_health(), 25);
        assert_eq!(world.len(), 1);
        assert_eq!(
            world
                .get(&guid)
                .unwrap()
                .creature()
                .unwrap()
                .current_health(),
            75
        );
        assert_eq!(
            world.iter().map(|(stored, _)| *stored).collect::<Vec<_>>(),
            vec![guid]
        );
        assert_eq!(world.values().count(), 1);
        assert_eq!(
            world
                .take(&guid)
                .unwrap()
                .as_ref()
                .creature()
                .unwrap()
                .current_health(),
            75
        );
        assert_eq!(world.len(), 0);
    }

    #[test]
    fn creature_transform_vitals_batch_is_owned_sorted_and_exact_typed() {
        let first = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 100, 41);
        let second = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 100, 42);
        let missing = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 100, 43);
        let wrong_kind =
            ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 7, 100, 44);
        let mut world = EntityWorld::default();

        let mut second_record = creature_record(second, 75);
        second_record
            .creature_mut()
            .unwrap()
            .unit_mut()
            .world_mut()
            .relocate(wow_core::Position::new(20.0, 21.0, 22.0, 1.5));
        let mut first_record = creature_record(first, 25);
        first_record
            .creature_mut()
            .unwrap()
            .unit_mut()
            .world_mut()
            .relocate(wow_core::Position::new(10.0, 11.0, 12.0, 0.5));
        assert!(world.insert(ObjectEntry::Record(second_record)).is_none());
        assert!(world.insert(ObjectEntry::Record(first_record)).is_none());
        let mut game_object = GameObject::new();
        game_object.world_mut().object_mut().create(wrong_kind);
        game_object.world_mut().set_map(571, 7).unwrap();
        assert!(
            world
                .insert(ObjectEntry::Record(MapObjectRecord::new_game_object(game_object).unwrap()))
                .is_none()
        );

        let lookups = world.creature_transform_vitals_lookups([second, wrong_kind, missing, first]);
        let mut expected_lookup_order = vec![second, wrong_kind, missing, first];
        expected_lookup_order.sort_unstable();
        assert_eq!(
            lookups.iter().map(|(guid, _)| *guid).collect::<Vec<_>>(),
            expected_lookup_order
        );
        assert!(
            lookups
                .iter()
                .find(|(guid, _)| *guid == missing)
                .is_some_and(|(_, snapshot)| snapshot.is_none())
        );
        assert!(
            lookups
                .iter()
                .find(|(guid, _)| *guid == wrong_kind)
                .is_some_and(|(_, snapshot)| snapshot.is_none())
        );
        let snapshots: Vec<_> = lookups
            .into_iter()
            .filter_map(|(_, snapshot)| snapshot)
            .collect();
        assert_eq!(
            snapshots
                .iter()
                .map(|snapshot| snapshot.guid)
                .collect::<Vec<_>>(),
            vec![first, second]
        );
        assert_eq!(
            snapshots[0].position,
            wow_core::Position::new(10.0, 11.0, 12.0, 0.5)
        );
        assert_eq!((snapshots[0].health, snapshots[0].max_health), (25, 100));
        assert_eq!(snapshots[0].combat_reach, 1.75);
        assert_eq!((snapshots[0].map_id, snapshots[0].instance_id), (571, 7));

        world
            .with_creature_mut(first, |creature| {
                creature.unit_mut().set_health(5);
                creature
                    .unit_mut()
                    .world_mut()
                    .relocate(wow_core::Position::xyz(30.0, 31.0, 32.0));
            })
            .expect("the canonical Creature should remain present");
        assert_eq!(snapshots[0].health, 25);
        assert_eq!(snapshots[0].position.x, 10.0);
    }
}
