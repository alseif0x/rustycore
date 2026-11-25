//! Map removal cannot destroy a Player whose lifetime still belongs to a session.
//! C++ MapManager::DestroyMap (MapManager.cpp:322-339) refuses an occupied map;
//! Map::RemoveAllPlayers (Map.cpp:1629-1643) requests homebind teleports, not deletion.
use super::{ManagedMap, MapManager, MapTickParticipantLikeCpp};
use crate::MapKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapUnloadBlockedLikeCpp {
    pub occupied_maps: Vec<MapKey>,
}

impl MapManager {
    pub fn destroy_map(&mut self, map_id: u32, instance_id: u32) -> bool {
        let key = MapKey::new(map_id, instance_id);
        let Some(incarnation) = self.map_incarnation_like_cpp(key) else {
            return false;
        };
        let Some(map) = self.maps.get_mut(&key) else {
            return false;
        };
        if !Self::destroy_map_inner(map) {
            return false;
        }
        if map.kind().frees_instance_id_on_destroy() {
            self.instance_ids_pending_retirement_like_cpp
                .insert(instance_id, MapTickParticipantLikeCpp { key, incarnation });
            self.instance_ids_pending_retirement_submit_like_cpp
                .insert(instance_id);
        }
        self.maps.remove(&key);
        self.map_incarnations_like_cpp.remove(&key);
        true
    }

    /// Only removed maps can reach the DB writer. An abandoned split tick can
    /// still have its old map registered until a later tick completes removal.
    pub fn take_pending_instance_id_retirements_like_cpp(
        &mut self,
    ) -> Vec<MapTickParticipantLikeCpp> {
        std::mem::take(&mut self.instance_ids_pending_retirement_submit_like_cpp)
            .into_iter()
            .filter_map(|id| {
                self.instance_ids_pending_retirement_like_cpp
                    .get(&id)
                    .copied()
            })
            .collect()
    }

    pub fn retry_instance_id_retirement_submission_like_cpp(
        &mut self,
        retired: MapTickParticipantLikeCpp,
    ) {
        if self
            .instance_ids_pending_retirement_like_cpp
            .get(&retired.key.instance_id)
            == Some(&retired)
            && self.map_incarnation_like_cpp(retired.key) != Some(retired.incarnation)
        {
            self.instance_ids_pending_retirement_submit_like_cpp
                .insert(retired.key.instance_id);
        }
    }

    /// A delayed acknowledgement belongs to one destroyed map incarnation.
    pub fn complete_instance_id_retirement_like_cpp(
        &mut self,
        retired: MapTickParticipantLikeCpp,
    ) -> bool {
        let id = retired.key.instance_id;
        if self.instance_ids_pending_retirement_like_cpp.get(&id) != Some(&retired)
            || self.map_incarnation_like_cpp(retired.key) == Some(retired.incarnation)
        {
            return false;
        }
        self.instance_ids_pending_retirement_like_cpp.remove(&id);
        self.instance_ids_pending_retirement_submit_like_cpp
            .remove(&id);
        self.instance_ids.free_instance_id(id);
        true
    }

    pub(super) fn destroy_map_inner(map: &mut ManagedMap) -> bool {
        // The current runtime has no Map-owned evacuation delivery yet. Retain
        // the owner until its real transfer/detach completes; never fake evacuation
        // by clearing a compatibility counter. No callbacks or I/O under this owner.
        if map.have_players() {
            return false;
        }
        map.unload_all();
        true
    }

    /// Shutdown must drain active Players first, as the C++ session/logout owner
    /// does (worldserver/Main.cpp:390-391, before :345). Reject the whole request
    /// before any map is unloaded; detached Players
    /// remain owned here and are not implicitly retired by deleting map storage.
    pub fn unload_all(&mut self) -> Result<(), MapUnloadBlockedLikeCpp> {
        let occupied_maps: Vec<_> = self
            .maps
            .iter()
            .filter_map(|(key, map)| map.have_players().then_some(*key))
            .collect();
        if !occupied_maps.is_empty() {
            return Err(MapUnloadBlockedLikeCpp { occupied_maps });
        }
        for map in self.maps.values_mut() {
            map.unload_all();
        }
        self.maps.clear();
        // A key with no map has no incarnation either; leaving stale entries
        // here would let a later tick recognise a map that no longer exists.
        self.map_incarnations_like_cpp.clear();
        Ok(())
    }
}
