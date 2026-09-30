//! Original Catalog loop, copied whole from PRE with its test-only name alias.
use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub(super) fn original_catalog<F, R, C, L>(
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
                        let (_, loaded_grid_primary_record, primary_result) = self
                            .admit_loaded_grid_materialization(LoadedGridMaterialization::records(
                                records,
                            ))
                            .into_record_parts();
                        match primary_result {
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
}
