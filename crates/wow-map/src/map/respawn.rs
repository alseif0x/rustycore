// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Respawn scheduling and its records.

use super::*;

mod actors;
pub(crate) mod prefix;
mod transport;
mod active_locations;
mod guards;
mod catalog;

const WEEK_SECS_LIKE_CPP: i64 = 7 * 24 * 60 * 60;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Map-owned bridge for C++ `Map::_respawnTimes` and the per-type respawn maps.
    ///
    /// C++ anchors:
    /// - `Map.h:472-480` returns zero when a respawn time is missing or the type has no map.
    /// - `Map.h:748-777` stores respawn queues/maps on `Map`; AreaTrigger has no respawn map.
    /// - `Map.cpp:2057-2150` adds, replaces, gets, removes, and unloads respawn info coherently.
    pub const fn respawn_store_like_cpp(&self) -> &RespawnStoreLikeCpp {
        &self.respawn_store
    }

    /// Mutable access to the map-owned respawn store for bounded tests/bridges.
    ///
    /// Future runtime callers must treat `Map` as the owner/source of truth and
    /// must not keep external respawn stores that later overwrite this state.
    pub fn respawn_store_like_cpp_mut(&mut self) -> &mut RespawnStoreLikeCpp {
        &mut self.respawn_store
    }

    pub fn add_respawn_info_like_cpp(
        &mut self,
        info: RespawnInfoLikeCpp,
    ) -> AddRespawnInfoOutcomeLikeCpp {
        self.respawn_store.add_respawn_info_like_cpp(info)
    }

    pub fn get_respawn_time_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> i64 {
        self.respawn_store
            .get_respawn_time_like_cpp(object_type, spawn_id)
    }

    pub fn get_respawn_info_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<&RespawnInfoLikeCpp> {
        self.respawn_store
            .get_respawn_info_like_cpp(object_type, spawn_id)
    }

    /// C++ `Map::GetLinkedRespawnTime` dependency slice.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp:3607-3620`.
    /// The linked respawn store is read-only ObjectMgr-style metadata; the timer
    /// source of truth remains this `Map`'s map-owned `RespawnStoreLikeCpp`.
    pub fn get_linked_respawn_time_like_cpp(
        &self,
        guid: ObjectGuid,
        linked_store: &LinkedRespawnStoreLikeCpp,
    ) -> i64 {
        let linked_guid = linked_store.get_linked_respawn_guid_like_cpp(guid);
        match linked_guid.high_type() {
            HighGuid::Creature => self.get_respawn_time_like_cpp(
                SpawnObjectType::Creature,
                linked_guid.counter() as SpawnId,
            ),
            HighGuid::GameObject => self.get_respawn_time_like_cpp(
                SpawnObjectType::GameObject,
                linked_guid.counter() as SpawnId,
            ),
            _ => 0,
        }
    }

    pub fn remove_respawn_time_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<RespawnInfoLikeCpp> {
        self.respawn_store
            .remove_respawn_time_like_cpp(object_type, spawn_id)
    }

    pub fn unload_all_respawn_infos_like_cpp(&mut self) {
        self.respawn_store.unload_all_respawn_infos_like_cpp();
    }

    pub fn respawn_timer_keys_like_cpp(
        &self,
    ) -> impl Iterator<Item = (SpawnObjectType, SpawnId)> + '_ {
        self.respawn_store.respawn_timer_keys_like_cpp()
    }

    /// Delegates the C++ `Map::ProcessRespawns` action planner to the map-owned store.
    ///
    /// This only plans side effects. It does not execute PoolMgr, DoRespawn,
    /// DB persistence/delete, linked-respawn checks, entity creation, or fanout.
    pub fn process_due_respawns_like_cpp(
        &mut self,
        now: i64,
        is_part_of_pool: impl FnMut(SpawnObjectType, SpawnId) -> Option<u32>,
        check_respawn: impl FnMut(&mut RespawnInfoLikeCpp) -> CheckRespawnOutcomeLikeCpp,
    ) -> Vec<ProcessRespawnActionLikeCpp> {
        self.respawn_store
            .process_due_respawns_like_cpp(now, is_part_of_pool, check_respawn)
    }

}
