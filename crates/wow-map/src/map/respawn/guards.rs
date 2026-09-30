use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Linked-respawn branch from C++ `Map::CheckRespawn`.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp:2004-2020`.
    /// This implements only the linked-time guard after earlier live-object
    /// blockers have already cleared. It never runs PoolMgr, DoRespawn, DB
    /// save/delete, entity creation, fanout, or RNG; the caller supplies the
    /// explicit jitter that represents C++ `urand(5, 15)`.
    pub fn check_respawn_linked_respawn_guard_like_cpp(
        &self,
        info: &mut RespawnInfoLikeCpp,
        linked_store: &LinkedRespawnStoreLikeCpp,
        now: i64,
        jitter_secs: u32,
    ) -> CheckRespawnLinkedRespawnGuardOutcomeLikeCpp {
        let Some(guid_high) = (match info.object_type {
            SpawnObjectType::Creature => Some(HighGuid::Creature),
            SpawnObjectType::GameObject => Some(HighGuid::GameObject),
            SpawnObjectType::AreaTrigger => None,
        }) else {
            return CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::UnsupportedSpawnType;
        };

        let this_guid = ObjectGuid::create_world_object(
            guid_high,
            0,
            0,
            self.map_id as u16,
            0,
            info.entry,
            info.spawn_id as i64,
        );
        let linked_time = self.get_linked_respawn_time_like_cpp(this_guid, linked_store);
        if linked_time == 0 {
            return CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::Allowed;
        }

        if linked_time == i64::MAX {
            info.respawn_time = linked_time;
            return CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedInfinite;
        }

        if linked_store.get_linked_respawn_guid_like_cpp(this_guid) == this_guid {
            info.respawn_time = now + WEEK_SECS_LIKE_CPP;
            return CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedSelfNeverRespawn;
        }

        info.respawn_time = now.max(linked_time) + i64::from(jitter_secs);
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedDelayed
    }

    /// Live object existence guard from C++ `Map::CheckRespawn`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:1966-2002` checks whether an already-live creature/gameobject
    ///   with the same spawn id blocks respawn, clears `respawnTime`, and returns
    ///   false when blocked.
    /// - `Map.cpp:1972-1983` allows dynamic escort NPC respawn only when the
    ///   matching live creature is already escorting.
    ///
    /// Source of truth for this slice is canonical map-owned `entity_world`, with
    /// typed map-local by-spawn-id indexes mirroring Trinity's multimap stores.
    /// Callers must provide the `CONFIG_RESPAWN_DYNAMIC_ESCORTNPC` value and the
    /// real escort runtime predicate; this helper does not invent
    /// `Creature::IsEscorted`, PoolMgr, linked respawn, `DoRespawn`, DB writes, or
    /// fanout side effects.
    pub fn check_respawn_live_object_guard_like_cpp<F>(
        &self,
        info: &mut RespawnInfoLikeCpp,
        spawn_store: &SpawnStore,
        respawn_dynamic_escortnpc: bool,
        mut is_creature_escorted: F,
    ) -> CheckRespawnLiveObjectGuardOutcomeLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
    {
        let Some(spawn_data) = spawn_store.spawn_data(info.object_type, info.spawn_id) else {
            return CheckRespawnLiveObjectGuardOutcomeLikeCpp::MissingSpawnData;
        };

        match info.object_type {
            SpawnObjectType::Creature => {
                let is_escort = respawn_dynamic_escortnpc
                    && spawn_data
                        .spawn_group
                        .flags
                        .contains(SpawnGroupFlags::ESCORTQUESTNPC);

                let Some(creature_guids) = self.creatures_by_spawn_id.get(&info.spawn_id) else {
                    return CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed;
                };

                for guid in creature_guids {
                    let Some(record) = self.entity_world.get(guid) else {
                        continue;
                    };
                    let Some(creature) = record.creature() else {
                        continue;
                    };
                    if creature.spawn_id() != info.spawn_id || !creature.is_alive() {
                        continue;
                    }
                    if is_escort && is_creature_escorted(creature.guid(), creature) {
                        continue;
                    }

                    info.respawn_time = 0;
                    return CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn;
                }

                CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed
            }
            SpawnObjectType::GameObject => {
                if self
                    .gameobjects_by_spawn_id
                    .get(&info.spawn_id)
                    .is_some_and(|gameobject_guids| {
                        gameobject_guids.iter().any(|guid| {
                            self.entity_world.get(guid).is_some_and(|record| {
                                record.game_object().is_some_and(|gameobject| {
                                    gameobject.spawn_id() == info.spawn_id
                                })
                            })
                        })
                    })
                {
                    info.respawn_time = 0;
                    return CheckRespawnLiveObjectGuardOutcomeLikeCpp::GameObjectBlocksRespawn;
                }

                CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed
            }
            SpawnObjectType::AreaTrigger => {
                CheckRespawnLiveObjectGuardOutcomeLikeCpp::UnsupportedSpawnType
            }
        }
    }

    /// Composite helper preserving represented C++ `Map::CheckRespawn` guard order.
    ///
    /// C++ anchors:
    /// - `Map.cpp:1950-2023` defines the full return/mutate contract.
    /// - `Map.cpp:1956-1964` checks spawn-group activity first.
    /// - `Map.cpp:1966-2002` checks live object blockers second.
    /// - `Map.cpp:2004-2020` checks linked respawn only after earlier guards allow.
    ///
    /// Runtime timer source of truth is this map-owned `RespawnStoreLikeCpp` via
    /// `RespawnInfoLikeCpp`; metadata stays caller-supplied `SpawnStore` until
    /// ObjectMgr ownership moves into `Map`; live blockers come from `entity_world`;
    /// linked metadata is read-only. This helper deliberately does not execute
    /// PoolMgr, `DoRespawn`, DB save/delete, entity creation, fanout, or RNG.
    pub fn check_respawn_like_cpp<F>(
        &self,
        info: &mut RespawnInfoLikeCpp,
        spawn_store: &SpawnStore,
        linked_store: &LinkedRespawnStoreLikeCpp,
        now: i64,
        jitter_secs: u32,
        respawn_dynamic_escortnpc: bool,
        mut is_creature_escorted: F,
    ) -> CheckRespawnCompositeOutcomeLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
    {
        if matches!(info.object_type, SpawnObjectType::AreaTrigger) {
            return CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType;
        }

        match self.check_respawn_spawn_group_guard_like_cpp(info, spawn_store) {
            CheckRespawnSpawnGroupGuardOutcomeLikeCpp::Allowed => {}
            CheckRespawnSpawnGroupGuardOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer => {
                return CheckRespawnCompositeOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer;
            }
            CheckRespawnSpawnGroupGuardOutcomeLikeCpp::MissingSpawnData => {
                return CheckRespawnCompositeOutcomeLikeCpp::MissingSpawnData;
            }
        }

        match self.check_respawn_live_object_guard_like_cpp(
            info,
            spawn_store,
            respawn_dynamic_escortnpc,
            &mut is_creature_escorted,
        ) {
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed => {}
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn => {
                return CheckRespawnCompositeOutcomeLikeCpp::AliveCreatureBlocksRespawn;
            }
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::GameObjectBlocksRespawn => {
                return CheckRespawnCompositeOutcomeLikeCpp::GameObjectBlocksRespawn;
            }
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::MissingSpawnData => {
                return CheckRespawnCompositeOutcomeLikeCpp::MissingSpawnData;
            }
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::UnsupportedSpawnType => {
                return CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType;
            }
        }

        match self.check_respawn_linked_respawn_guard_like_cpp(info, linked_store, now, jitter_secs)
        {
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::Allowed => {
                CheckRespawnCompositeOutcomeLikeCpp::Allowed
            }
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedInfinite => {
                CheckRespawnCompositeOutcomeLikeCpp::LinkedInfinite
            }
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedSelfNeverRespawn => {
                CheckRespawnCompositeOutcomeLikeCpp::LinkedSelfNeverRespawn
            }
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedDelayed => {
                CheckRespawnCompositeOutcomeLikeCpp::LinkedDelayed
            }
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::UnsupportedSpawnType => {
                CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType
            }
        }
    }
}
