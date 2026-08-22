// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Respawn scheduling and its records.

use super::*;

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

    /// Safe side-effect seam for represented C++ `Map::ProcessRespawns` branches.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2191-2198` processes only due respawn timers in queue order.
    /// - `Map.cpp:2200-2211` detects `PoolMgr::IsPartOfAPool` before
    ///   `CheckRespawn`, updates map-owned `SpawnedPoolData` through
    ///   `PoolMgr::UpdatePool`, then removes the respawn timer with DB-delete
    ///   ownership left to the caller bridge.
    /// - `Map.cpp:2213-2224` allowed respawn removes+calls `DoRespawn`; blocked here.
    /// - `Map.cpp:2226-2231` removes a timer when `CheckRespawn` set respawnTime=0.
    /// - `Map.cpp:2233-2238` updates the heap position and persists a future
    ///   `respawnTime` when `CheckRespawn` rescheduled the timer.
    ///
    /// This helper executes only safe map-owned in-memory effects represented so
    /// far: pooled timer -> deterministic `UpdatePool` plan + map-owned
    /// `SpawnedPoolDataLikeCpp` mutation + timer removal, `DoRespawn`'s unloaded-grid
    /// early return after timer removal, loaded-grid non-pooled `DoRespawn` via a
    /// caller-supplied typed `MapObjectRecord` loader, zero-delete for inactive
    /// spawn-groups/live-object blockers, and linked-respawn future reschedule by
    /// replacing the same map-owned respawn timer. DB effects, live record
    /// construction, grid/session fanout, and scripts stay outside this lock-owned
    /// helper.
    /// `consume_due_timer_on_load_failure_like_cpp` selects the live C++ path,
    /// where `ProcessRespawns` has already popped the timer before a failed
    /// `LoadFromDB`, versus the older represented safe wrapper, which has no
    /// loader at all and must leave the timer intact rather than discard work.
    pub fn process_due_respawns_composite_loaded_grid_respawns_like_cpp<F, R, C, L>(
        &mut self,
        now: i64,
        spawn_store: &SpawnStore,
        linked_store: &LinkedRespawnStoreLikeCpp,
        pool_mgr: &PoolMgrLikeCpp,
        jitter_secs: u32,
        respawn_dynamic_escortnpc: bool,
        mut is_creature_escorted: F,
        mut explicit_roll_for: R,
        mut choose_equal: C,
        consume_due_timer_on_load_failure_like_cpp: bool,
        mut load_record: L,
    ) -> ProcessRespawnsSafeSideEffectsSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

        loop {
            let next_key = { self.respawn_timer_keys_like_cpp().next() };
            let Some((object_type, spawn_id)) = next_key else {
                break;
            };
            let Some(info) = self
                .get_respawn_info_like_cpp(object_type, spawn_id)
                .cloned()
            else {
                summary.blocked_missing_spawn_data += 1;
                break;
            };
            if now < info.respawn_time {
                break;
            }

            match pool_mgr.is_part_of_a_pool_like_cpp(object_type, spawn_id) {
                Ok(0) => {}
                Ok(pool_id) => match pool_mgr.update_pool_plan_like_cpp(
                    &mut self.pool_data,
                    pool_id,
                    object_type,
                    spawn_id,
                    &mut explicit_roll_for,
                    &mut choose_equal,
                ) {
                    Ok(plan) => {
                        self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp(
                            &plan,
                            spawn_store,
                            &mut summary,
                            Some(&mut load_record),
                        );
                        self.remove_respawn_time_like_cpp(object_type, spawn_id);
                        summary.processed_pool_timers += 1;
                        summary.pool_update_plans.push(plan);
                        continue;
                    }
                    Err(error) => {
                        summary.blocked_pool_plan_errors.push(error);
                        break;
                    }
                },
                Err(error) => {
                    summary.blocked_pool_plan_errors.push(error);
                    break;
                }
            }

            if spawn_store.spawn_data(object_type, spawn_id).is_none() {
                summary.blocked_missing_spawn_data += 1;
                if consume_due_timer_on_load_failure_like_cpp {
                    // C++ pops the due timer before `DoRespawn`; a stale DB
                    // spawn makes `LoadFromDB` fail, but it must not pin the
                    // queue head and starve every later respawn on the map.
                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                    continue;
                }
                break;
            }

            let mut checked_info = info;
            match self.check_respawn_like_cpp(
                &mut checked_info,
                spawn_store,
                linked_store,
                now,
                jitter_secs,
                respawn_dynamic_escortnpc,
                &mut is_creature_escorted,
            ) {
                CheckRespawnCompositeOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer
                    if checked_info.respawn_time == 0 =>
                {
                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                    summary.deleted_inactive_spawn_group += 1;
                }
                CheckRespawnCompositeOutcomeLikeCpp::AliveCreatureBlocksRespawn
                | CheckRespawnCompositeOutcomeLikeCpp::GameObjectBlocksRespawn
                    if checked_info.respawn_time == 0 =>
                {
                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                    summary.deleted_live_object_blocker += 1;
                }
                CheckRespawnCompositeOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer
                | CheckRespawnCompositeOutcomeLikeCpp::AliveCreatureBlocksRespawn
                | CheckRespawnCompositeOutcomeLikeCpp::GameObjectBlocksRespawn => {
                    summary.blocked_do_respawn_runtime += 1;
                    break;
                }
                CheckRespawnCompositeOutcomeLikeCpp::Allowed => {
                    if is_grid_id_loaded(self, checked_info.grid_id) {
                        let Some(records) = load_record(self, object_type, spawn_id) else {
                            summary.blocked_loaded_grid_respawn_loads += 1;
                            summary.blocked_do_respawn_runtime += 1;
                            // `Map::ProcessRespawns` erases the timer before
                            // `DoRespawn`; `Creature/GameObject::LoadFromDB`
                            // failure deletes the temporary object and the loop
                            // continues with the next due timer.
                            if consume_due_timer_on_load_failure_like_cpp {
                                self.remove_respawn_time_like_cpp(object_type, spawn_id);
                                continue;
                            }
                            break;
                        };

                        // C++ `ProcessRespawns` pops/erases the timer before
                        // calling `DoRespawn`. For DB-backed GameObjects,
                        // `GameObject::Create` may also create and AddToMap a
                        // linked trap first; that AddToMap failure only deletes
                        // the trap and does not block the owner. The primary
                        // `AddToMap` result remains determinant as in C++.
                        self.remove_respawn_time_like_cpp(object_type, spawn_id);
                        for pre_add_record in records.pre_add_records {
                            let _ = self.add_map_object_record_to_map_like_cpp(pre_add_record);
                        }
                        let primary_record = records.primary_record;
                        let loaded_grid_primary_record = primary_record.clone();
                        match self.add_map_object_record_to_map_like_cpp(primary_record) {
                            Ok(_outcome) => {
                                summary.executed_loaded_grid_respawns += 1;
                                summary
                                    .loaded_grid_primary_records
                                    .push(loaded_grid_primary_record);
                            }
                            Err(_error) => {
                                summary.blocked_loaded_grid_respawn_add_to_map += 1;
                            }
                        }
                        continue;
                    }

                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                    summary.processed_unloaded_grid_respawns += 1;
                    continue;
                }
                CheckRespawnCompositeOutcomeLikeCpp::LinkedInfinite
                | CheckRespawnCompositeOutcomeLikeCpp::LinkedSelfNeverRespawn
                | CheckRespawnCompositeOutcomeLikeCpp::LinkedDelayed => {
                    if checked_info.respawn_time == i64::MAX || checked_info.respawn_time > now {
                        let rescheduled_info = checked_info.clone();
                        self.remove_respawn_time_like_cpp(object_type, spawn_id);
                        self.add_respawn_info_like_cpp(checked_info);
                        summary.rescheduled_linked_respawns.push(rescheduled_info);
                    } else {
                        summary.blocked_linked_respawn_non_future += 1;
                        break;
                    }
                }
                CheckRespawnCompositeOutcomeLikeCpp::MissingSpawnData => {
                    summary.blocked_missing_spawn_data += 1;
                    break;
                }
                CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType => {
                    summary.blocked_unsupported_spawn_type += 1;
                    break;
                }
            }
        }

        summary
    }

    /// Compatibility wrapper that preserves the old safe-side-effects API by
    /// keeping loaded-grid non-pooled `DoRespawn` blocked through a loader that
    /// returns no typed record.
    pub fn process_due_respawns_composite_safe_side_effects_like_cpp<F, R, C>(
        &mut self,
        now: i64,
        spawn_store: &SpawnStore,
        linked_store: &LinkedRespawnStoreLikeCpp,
        pool_mgr: &PoolMgrLikeCpp,
        jitter_secs: u32,
        respawn_dynamic_escortnpc: bool,
        is_creature_escorted: F,
        explicit_roll_for: R,
        choose_equal: C,
    ) -> ProcessRespawnsSafeSideEffectsSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    {
        self.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
            now,
            spawn_store,
            linked_store,
            pool_mgr,
            jitter_secs,
            respawn_dynamic_escortnpc,
            is_creature_escorted,
            explicit_roll_for,
            choose_equal,
            false,
            |_map, _object_type, _spawn_id| None,
        )
    }

    /// Compatibility wrapper for callers that still use the old delete-only name.
    pub fn process_due_respawns_composite_delete_only_like_cpp<F>(
        &mut self,
        now: i64,
        spawn_store: &SpawnStore,
        linked_store: &LinkedRespawnStoreLikeCpp,
        jitter_secs: u32,
        respawn_dynamic_escortnpc: bool,
        is_creature_escorted: F,
    ) -> ProcessRespawnsDeleteOnlySummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
    {
        let pool_mgr = PoolMgrLikeCpp::new();
        self.process_due_respawns_composite_safe_side_effects_like_cpp(
            now,
            spawn_store,
            linked_store,
            &pool_mgr,
            jitter_secs,
            respawn_dynamic_escortnpc,
            is_creature_escorted,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
        )
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
    /// Source of truth for this slice is canonical map-owned `map_objects`, with
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
                    let Some(record) = self.map_objects.get(guid) else {
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
                            self.map_objects.get(guid).is_some_and(|record| {
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
    /// ObjectMgr ownership moves into `Map`; live blockers come from `map_objects`;
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

    pub(super) fn active_respawn_location_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<ActiveNonPlayerRespawnLocationLikeCpp> {
        let record = self.map_object_record(guid)?;
        match record.kind() {
            AccessorObjectKind::Creature => {
                let creature = record.creature()?;
                let spawn_id = creature.spawn_id();
                (spawn_id != 0).then_some(ActiveNonPlayerRespawnLocationLikeCpp {
                    spawn_id,
                    position: creature.ai_home_position(),
                })
            }
            AccessorObjectKind::GameObject => {
                let game_object = record.game_object()?;
                let spawn_id = game_object.spawn_id();
                (spawn_id != 0).then_some(ActiveNonPlayerRespawnLocationLikeCpp {
                    spawn_id,
                    position: game_object.stationary_position(),
                })
            }
            _ => None,
        }
    }

    pub(super) fn mutate_unload_active_lock_for_respawn_location_like_cpp(
        &mut self,
        location: ActiveNonPlayerRespawnLocationLikeCpp,
        increment: bool,
    ) -> ActiveNonPlayerUnloadLockOutcomeLikeCpp {
        if !is_valid_map_coord_2d(location.position.x, location.position.y) {
            return ActiveNonPlayerUnloadLockOutcomeLikeCpp {
                spawn_id: location.spawn_id,
                respawn_grid: None,
                respawn_grid_missing: true,
                invalid_respawn_position: true,
                lock_incremented: false,
                lock_decremented: false,
            };
        }

        let cell = Cell::from_world(location.position.x, location.position.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        let Some(ngrid) = self.get_ngrid_mut(grid) else {
            return ActiveNonPlayerUnloadLockOutcomeLikeCpp {
                spawn_id: location.spawn_id,
                respawn_grid: Some(grid),
                respawn_grid_missing: true,
                invalid_respawn_position: false,
                lock_incremented: false,
                lock_decremented: false,
            };
        };

        if increment {
            ngrid.info_mut().inc_unload_active_lock();
        } else {
            ngrid.info_mut().dec_unload_active_lock();
        }

        ActiveNonPlayerUnloadLockOutcomeLikeCpp {
            spawn_id: location.spawn_id,
            respawn_grid: Some(grid),
            respawn_grid_missing: false,
            invalid_respawn_position: false,
            lock_incremented: increment,
            lock_decremented: !increment,
        }
    }
}
