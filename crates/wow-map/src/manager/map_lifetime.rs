//! Map removal cannot destroy a Player whose lifetime still belongs to a session.
//! C++ MapManager::DestroyMap (MapManager.cpp:322-339) refuses an occupied map;
//! Map::RemoveAllPlayers (Map.cpp:1629-1643) requests homebind teleports, not deletion.
use super::{InstanceIdAllocator, ManagedMap, MapManager};
use crate::MapKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapUnloadBlockedLikeCpp {
    pub occupied_maps: Vec<MapKey>,
}

impl MapManager {
    pub fn destroy_map(&mut self, map_id: u32, instance_id: u32) -> bool {
        let key = MapKey::new(map_id, instance_id);
        let Some(map) = self.maps.get_mut(&key) else {
            return false;
        };
        if !Self::destroy_map_inner(map, &mut self.instance_ids) {
            return false;
        }
        self.maps.remove(&key);
        true
    }

    pub(super) fn destroy_map_inner(
        map: &mut ManagedMap,
        instance_ids: &mut InstanceIdAllocator,
    ) -> bool {
        // The current runtime has no Map-owned evacuation delivery yet. Retain
        // the owner until its real transfer/detach completes; never fake evacuation
        // by clearing a compatibility counter. No callbacks or I/O under this owner.
        if map.have_players() {
            return false;
        }
        map.unload_all();
        if map.kind().frees_instance_id_on_destroy() {
            instance_ids.free_instance_id(map.instance_id());
        }
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
        Ok(())
    }
}
