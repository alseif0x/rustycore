// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private storage facade for the canonical objects owned by one [`super::Map`].
//!
//! C++ keeps the corresponding ownership in `Map::_objectsStore`
//! (`Map.h:418,793`). This facade deliberately preserves the current
//! `HashMap<ObjectGuid, MapObjectRecord>` behavior while removing that concrete
//! storage choice from `Map`. It exposes no `Deref`, so callers cannot acquire a
//! new dependency on `HashMap` while borrowed-record APIs are retired ahead of
//! the selected `hecs` backend.

use std::collections::hash_map::{Iter, Values};
use std::collections::HashMap;

use wow_core::ObjectGuid;
use wow_entities::MapObjectRecord;

#[derive(Debug, Default)]
pub(super) struct EntityWorld {
    records_by_guid: HashMap<ObjectGuid, MapObjectRecord>,
}

impl EntityWorld {
    pub(super) fn get(&self, guid: &ObjectGuid) -> Option<&MapObjectRecord> {
        self.records_by_guid.get(guid)
    }

    pub(super) fn get_mut(&mut self, guid: &ObjectGuid) -> Option<&mut MapObjectRecord> {
        self.records_by_guid.get_mut(guid)
    }

    pub(super) fn iter(&self) -> Iter<'_, ObjectGuid, MapObjectRecord> {
        self.records_by_guid.iter()
    }

    pub(super) fn values(&self) -> Values<'_, ObjectGuid, MapObjectRecord> {
        self.records_by_guid.values()
    }

    pub(super) fn len(&self) -> usize {
        self.records_by_guid.len()
    }

    pub(super) fn insert(&mut self, record: MapObjectRecord) -> Option<MapObjectRecord> {
        self.records_by_guid.insert(record.object().guid(), record)
    }

    pub(super) fn remove(&mut self, guid: &ObjectGuid) -> Option<MapObjectRecord> {
        self.records_by_guid.remove(guid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::guid::HighGuid;
    use wow_entities::Creature;

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
        MapObjectRecord::new_creature(creature)
            .expect("fixture creature should form a canonical map record")
    }

    #[test]
    fn same_guid_replacement_keeps_one_canonical_record() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 100, 42);
        let mut world = EntityWorld::default();

        assert!(world.insert(creature_record(guid, 25)).is_none());
        let displaced = world
            .insert(creature_record(guid, 75))
            .expect("same GUID should displace exactly one canonical record");

        assert_eq!(displaced.creature().unwrap().current_health(), 25);
        assert_eq!(world.len(), 1);
        assert_eq!(world.get(&guid).unwrap().creature().unwrap().current_health(), 75);
        assert_eq!(world.iter().map(|(stored, _)| *stored).collect::<Vec<_>>(), vec![guid]);
        assert_eq!(world.values().count(), 1);
        assert_eq!(world.remove(&guid).unwrap().creature().unwrap().current_health(), 75);
        assert_eq!(world.len(), 0);
    }
}
