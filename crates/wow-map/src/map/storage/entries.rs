// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical entry insertion, spawn-index lookup and player map-reference membership.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn creature_spawn_id_store_count_like_cpp(&self, spawn_id: SpawnId) -> usize {
        self.creatures_by_spawn_id
            .get(&spawn_id)
            .map_or(0, HashSet::len)
    }

    pub fn creature_group_holder_contains_like_cpp(
        &self,
        leader_spawn_id: SpawnId,
        member_guid: ObjectGuid,
    ) -> bool {
        self.creature_group_holder_like_cpp
            .get(&leader_spawn_id)
            .is_some_and(|members| members.contains(&member_guid))
    }

    pub fn creature_spawn_id_store_guids_like_cpp(&self, spawn_id: SpawnId) -> Vec<ObjectGuid> {
        self.creatures_by_spawn_id
            .get(&spawn_id)
            .map(|guids| {
                let mut guids: Vec<_> = guids.iter().copied().collect();
                guids.sort();
                guids
            })
            .unwrap_or_default()
    }

    pub fn get_creature_by_spawn_id_like_cpp(&self, spawn_id: SpawnId) -> Option<&Creature> {
        let mut fallback_guid = None;
        let mut alive_guid = None;
        for guid in self.creature_spawn_id_store_guids_like_cpp(spawn_id) {
            let Some(creature) = self
                .map_object_record(guid)
                .and_then(|record| record.creature())
            else {
                continue;
            };
            if creature.spawn_id() != spawn_id {
                continue;
            }
            fallback_guid.get_or_insert(guid);
            if creature.is_alive() {
                alive_guid = Some(guid);
                break;
            }
        }

        alive_guid
            .or(fallback_guid)
            .and_then(|guid| self.map_object_record(guid)?.creature())
    }

    pub fn get_world_object_by_spawn_id_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<&WorldObject> {
        match object_type {
            SpawnObjectType::Creature => self
                .get_creature_by_spawn_id_like_cpp(spawn_id)
                .map(|creature| creature.unit().world()),
            SpawnObjectType::GameObject => self
                .get_gameobject_by_spawn_id_like_cpp(spawn_id)
                .map(GameObject::world),
            SpawnObjectType::AreaTrigger => self
                .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
                .map(AreaTrigger::world),
        }
    }

    pub fn insert_map_object(
        &mut self,
        kind: AccessorObjectKind,
        object: WorldObject,
    ) -> Result<Option<OwnedMapObject>, MapObjectStoreError> {
        let record = MapObjectRecord::new(kind, object)?;
        self.insert_map_object_record(record)
    }

    pub fn insert_map_object_record(
        &mut self,
        record: MapObjectRecord,
    ) -> Result<Option<OwnedMapObject>, MapObjectStoreError> {
        self.insert_object_entry(ObjectEntry::Record(record))
            .map(|previous| previous.map(OwnedMapObject::new))
    }

    pub(in crate::map) fn insert_object_entry(
        &mut self,
        entry: ObjectEntry,
    ) -> Result<Option<ObjectEntry>, MapObjectStoreError> {
        self.validate_map_object(entry.as_ref().object())?;
        let guid = entry.as_ref().object().guid();
        let mut previous = self.entity_world.take(&guid);
        if let Some(previous_entry) = previous.as_mut() {
            if !typed_loot_authorities_share_storage_like_cpp(
                previous_entry.as_ref(),
                entry.as_ref(),
            ) {
                detach_typed_loot_authority_like_cpp(previous_entry.as_mut());
            }
            self.unindex_map_object_record_by_spawn_id_like_cpp(previous_entry.as_ref());
        }
        self.index_map_object_record_by_spawn_id_like_cpp(entry.as_ref());
        let is_player = entry.as_ref().kind() == AccessorObjectKind::Player;
        let displaced = self.entity_world.insert(entry);
        debug_assert!(displaced.is_none());
        if is_player {
            self.link_map_reference_like_cpp(guid);
        }
        Ok(previous)
    }

    /// C++ `MapReference::targetObjectBuildLink` (`Maps/MapReference.cpp:22-28`)
    /// links a Player into `Map::m_mapRefManager` with `insertFirst`, so the
    /// most recently linked Player is visited first by `Map::Update`
    /// (`Maps/Map.cpp:669-680`). Re-linking the same Player moves it to the
    /// front, as unlink-then-link does in C++.
    fn link_map_reference_like_cpp(&mut self, guid: ObjectGuid) {
        self.map_reference_order_like_cpp
            .retain(|linked| *linked != guid);
        self.map_reference_order_like_cpp.insert(0, guid);
    }

    /// C++ `MapReference::targetObjectDestroyLink` removing the Player from
    /// `Map::m_mapRefManager`.
    pub(in crate::map) fn unlink_map_reference_like_cpp(&mut self, guid: ObjectGuid) {
        self.map_reference_order_like_cpp
            .retain(|linked| *linked != guid);
    }

    /// This map's players in C++ `m_mapRefManager` order: most recently linked
    /// first. This is the order `Map::Update` walks for the session pass; it is
    /// deliberately not a sorted GUID list.
    #[must_use]
    pub fn map_reference_order_like_cpp(&self) -> &[ObjectGuid] {
        &self.map_reference_order_like_cpp
    }
}
