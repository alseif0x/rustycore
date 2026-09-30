use super::*;
use crate::map::loaded_grid_admission::{LoadedGridReceipts, LoadedGridRespawnOutcome};

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
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
        self.process_due_respawns_materialized_core(
            now,
            spawn_store,
            linked_store,
            pool_mgr,
            jitter_secs,
            respawn_dynamic_escortnpc,
            &mut is_creature_escorted,
            &mut explicit_roll_for,
            &mut choose_equal,
            consume_due_timer_on_load_failure_like_cpp,
            |map, object_type, spawn_id| {
                Ok(load_record(map, object_type, spawn_id).map(LoadedGridMaterialization::records))
            },
            LoadedGridReceipts::RecordCompatibility,
        )
        .summary
    }

    /// Owned Catalog loads share the existing queue engine and timer order.
    /// Pooled attempts retain Pool provenance; no actor producer is activated.
    pub fn process_due_respawns_materialized<F, R, C, L>(
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
        consume_due_timer_on_load_failure: bool,
        load_record: L,
    ) -> LoadedGridRespawnOutcome
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
        )
            -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        self.process_due_respawns_materialized_core(
            now,
            spawn_store,
            linked_store,
            pool_mgr,
            jitter_secs,
            respawn_dynamic_escortnpc,
            is_creature_escorted,
            explicit_roll_for,
            choose_equal,
            consume_due_timer_on_load_failure,
            load_record,
            LoadedGridReceipts::Owned(Vec::new()),
        )
    }

    fn process_due_respawns_materialized_core<F, R, C, L>(
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
        mut receipts: LoadedGridReceipts,
    ) -> LoadedGridRespawnOutcome
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
        )
            -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

        loop {
            let next_key = { self.respawn_store.catalog_timer_keys().next() };
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
                        self.apply_pool_typed_materialized(
                            &plan,
                            spawn_store,
                            &mut summary,
                            Some(&mut load_record),
                            &mut receipts,
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
                        let materialization = match load_record(self, object_type, spawn_id) {
                            Ok(Some(materialization)) => materialization,
                            failure => {
                                summary.blocked_loaded_grid_respawn_loads += 1;
                                summary.blocked_do_respawn_runtime += 1;
                                receipts.catalog_load_failed(object_type, spawn_id, failure);
                                // `Map::ProcessRespawns` erases the timer before
                                // `DoRespawn`; `Creature/GameObject::LoadFromDB`
                                // failure deletes the temporary object and the loop
                                // continues with the next due timer.
                                if consume_due_timer_on_load_failure_like_cpp {
                                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                                    continue;
                                }
                                break;
                            }
                        };

                        // C++ `ProcessRespawns` pops/erases the timer before
                        // calling `DoRespawn`. For DB-backed GameObjects,
                        // `GameObject::Create` may also create and AddToMap a
                        // linked trap first; that AddToMap failure only deletes
                        // the trap and does not block the owner. The primary
                        // `AddToMap` result remains determinant as in C++.
                        self.remove_respawn_time_like_cpp(object_type, spawn_id);
                        let admission = self.admit_loaded_grid_materialization(materialization);
                        receipts.catalog_admitted(object_type, spawn_id, admission, &mut summary);
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

        receipts.finish_respawns(summary)
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
}

#[cfg(test)]
mod tests;
