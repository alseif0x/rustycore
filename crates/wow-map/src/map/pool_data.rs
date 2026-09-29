use std::collections::{HashMap, HashSet};

use crate::map_rules::decrement_pool_counter_like_cpp;
use crate::spawn::{SpawnId, SpawnObjectType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnedPoolDataErrorLikeCpp {
    /// C++ `SpawnedPoolData::IsSpawnedObject(SpawnObjectType, ...)` aborts for
    /// non Creature/GameObject types (`PoolMgr.cpp:66-77`). Rust returns a typed
    /// error at the seam instead of treating AreaTrigger as pooled/spawned.
    UnsupportedSpawnObjectType(SpawnObjectType),
}

/// Map-owned parity seam for C++ `SpawnedPoolData` (`PoolMgr.h:51-83`).
///
/// This is only the map-local state shape and helpers used by C++
/// `Map::_poolData` / `Map::GetPoolData()`. It does not implement real
/// `PoolMgr::SpawnPool`, `DespawnPool`, RNG/chance, entity creation,
/// AddToMap/RemoveFromMap, DB persistence/delete, or grid/session fanout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnedPoolDataLikeCpp {
    spawned_creatures: HashSet<SpawnId>,
    spawned_gameobjects: HashSet<SpawnId>,
    spawned_pools: HashMap<u32, u32>,
}

impl SpawnedPoolDataLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_spawned_objects_like_cpp(&self, pool_id: u32) -> u32 {
        self.spawned_pools.get(&pool_id).copied().unwrap_or(0)
    }

    pub fn is_spawned_creature_like_cpp(&self, spawn_id: SpawnId) -> bool {
        self.spawned_creatures.contains(&spawn_id)
    }

    pub fn is_spawned_gameobject_like_cpp(&self, spawn_id: SpawnId) -> bool {
        self.spawned_gameobjects.contains(&spawn_id)
    }

    pub fn is_spawned_pool_like_cpp(&self, sub_pool_id: u32) -> bool {
        self.spawned_pools.contains_key(&sub_pool_id)
    }

    pub fn is_spawned_object_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Result<bool, SpawnedPoolDataErrorLikeCpp> {
        match object_type {
            SpawnObjectType::Creature => Ok(self.is_spawned_creature_like_cpp(spawn_id)),
            SpawnObjectType::GameObject => Ok(self.is_spawned_gameobject_like_cpp(spawn_id)),
            SpawnObjectType::AreaTrigger => Err(
                SpawnedPoolDataErrorLikeCpp::UnsupportedSpawnObjectType(object_type),
            ),
        }
    }

    pub fn add_spawn_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
        pool_id: u32,
    ) -> Result<(), SpawnedPoolDataErrorLikeCpp> {
        match object_type {
            SpawnObjectType::Creature => {
                self.spawned_creatures.insert(spawn_id);
                *self.spawned_pools.entry(pool_id).or_insert(0) += 1;
                Ok(())
            }
            SpawnObjectType::GameObject => {
                self.spawned_gameobjects.insert(spawn_id);
                *self.spawned_pools.entry(pool_id).or_insert(0) += 1;
                Ok(())
            }
            SpawnObjectType::AreaTrigger => Err(
                SpawnedPoolDataErrorLikeCpp::UnsupportedSpawnObjectType(object_type),
            ),
        }
    }

    pub fn remove_spawn_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
        pool_id: u32,
    ) -> Result<(), SpawnedPoolDataErrorLikeCpp> {
        match object_type {
            SpawnObjectType::Creature => {
                self.spawned_creatures.remove(&spawn_id);
                decrement_pool_counter_like_cpp(&mut self.spawned_pools, pool_id);
                Ok(())
            }
            SpawnObjectType::GameObject => {
                self.spawned_gameobjects.remove(&spawn_id);
                decrement_pool_counter_like_cpp(&mut self.spawned_pools, pool_id);
                Ok(())
            }
            SpawnObjectType::AreaTrigger => Err(
                SpawnedPoolDataErrorLikeCpp::UnsupportedSpawnObjectType(object_type),
            ),
        }
    }

    pub fn add_pool_spawn_like_cpp(&mut self, sub_pool_id: u32, pool_id: u32) {
        self.spawned_pools.insert(sub_pool_id, 0);
        *self.spawned_pools.entry(pool_id).or_insert(0) += 1;
    }

    pub fn remove_pool_spawn_like_cpp(&mut self, sub_pool_id: u32, pool_id: u32) {
        self.spawned_pools.remove(&sub_pool_id);
        decrement_pool_counter_like_cpp(&mut self.spawned_pools, pool_id);
    }

    pub fn spawned_objects_like_cpp(&self) -> Vec<(SpawnObjectType, SpawnId)> {
        let mut spawned = self
            .spawned_creatures
            .iter()
            .copied()
            .map(|spawn_id| (SpawnObjectType::Creature, spawn_id))
            .chain(
                self.spawned_gameobjects
                    .iter()
                    .copied()
                    .map(|spawn_id| (SpawnObjectType::GameObject, spawn_id)),
            )
            .collect::<Vec<_>>();
        spawned.sort_unstable();
        spawned
    }
}
