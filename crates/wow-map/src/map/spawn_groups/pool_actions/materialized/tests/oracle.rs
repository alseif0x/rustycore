//! Full frozen pre-Pool-owned bodies, used only as compatibility test oracles.

use super::*;

impl Map {
    pub(super) fn original_typed<L>(
        &mut self,
        plan: &PoolTypedSpawnPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        if let Some(object_plan) = plan.object_plan.as_ref() {
            self.original_object(
                object_plan,
                spawn_store,
                summary,
                load_record.as_deref_mut(),
            );
        }
    }

    fn original_pool<L>(
        &mut self,
        plan: &PoolSpawnPoolPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        for subplan in &plan.subplans {
            self.original_typed(subplan, spawn_store, summary, load_record.as_deref_mut());
        }
    }

    fn original_object<L>(
        &mut self,
        plan: &PoolSpawnObjectPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let mut child_spawn_plans = plan.child_pool_spawn_plans.iter();
        let mut child_despawn_plans = plan.child_pool_despawn_plans.iter();
        for action in &plan.actions {
            match *action {
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_spawn_plans.next() {
                        self.original_pool(
                            child_plan,
                            spawn_store,
                            summary,
                            load_record.as_deref_mut(),
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                PoolSpawnObjectActionLikeCpp::DespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_despawn_plans.next() {
                        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
                            child_plan, summary,
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                PoolSpawnObjectActionLikeCpp::RespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                }
                | PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {}
                other => {
                    self.original_action(other, spawn_store, summary, load_record.as_deref_mut())
                }
            }
        }
    }

    fn original_action<L>(
        &mut self,
        action: PoolSpawnObjectActionLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        match action {
            PoolSpawnObjectActionLikeCpp::DespawnOne { kind, guid } => {
                self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
            }
            PoolSpawnObjectActionLikeCpp::RespawnOne { kind, guid } => {
                self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
                self.original_spawn(kind, guid, true, spawn_store, summary, load_record);
            }
            PoolSpawnObjectActionLikeCpp::RemoveRespawnTime { kind, guid } => {
                let Some(object_type) = pool_member_kind_to_spawn_object_type_like_cpp(kind) else {
                    return;
                };
                if self
                    .remove_respawn_time_like_cpp(object_type, guid as SpawnId)
                    .is_some()
                {
                    summary.pool_respawn_timers_removed += 1;
                } else {
                    summary.pool_respawn_timers_missing += 1;
                }
            }
            PoolSpawnObjectActionLikeCpp::SpawnOne { kind, guid } => {
                self.original_spawn(kind, guid, false, spawn_store, summary, load_record);
            }
        }
    }

    fn original_spawn<L>(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        spawn_id: u64,
        respawn: bool,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(object_type) = pool_member_kind_to_spawn_object_type_like_cpp(kind) else {
            summary.pool_unsupported_action_kind += 1;
            return;
        };
        let spawn_id = spawn_id as SpawnId;
        let Some(spawn_data) = spawn_store.spawn_data(object_type, spawn_id) else {
            summary.pool_spawn_actions_missing_spawn_data += 1;
            return;
        };
        let cell = cell_from_world(spawn_data.spawn_point.x, spawn_data.spawn_point.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        if !self.is_grid_loaded(grid) {
            summary.pool_spawn_actions_skipped_unloaded_grid += 1;
            return;
        }

        let Some(load_record) = load_record else {
            summary.pool_spawn_actions_blocked_loaded_grid += 1;
            summary
                .pool_spawn_action_load_plans
                .push(PoolSpawnActionLoadPlanLikeCpp {
                    object_type,
                    spawn_id,
                    respawn,
                });
            return;
        };

        let Some(records) = load_record(self, object_type, spawn_id) else {
            summary.pool_spawn_actions_blocked_loaded_grid += 1;
            summary
                .pool_spawn_action_load_plans
                .push(PoolSpawnActionLoadPlanLikeCpp {
                    object_type,
                    spawn_id,
                    respawn,
                });
            return;
        };

        let (_, loaded_grid_primary_record, primary_result) = self
            .admit_loaded_grid_materialization(LoadedGridMaterialization::records(records))
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
    }

    pub(super) fn original_facade<L>(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        pool_id: u32,
        spawn_store: &SpawnStore,
        explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        mut load_record: L,
    ) -> Result<ProcessRespawnsSafeSideEffectsSummaryLikeCpp, PoolMgrPlanErrorLikeCpp>
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let plan = pool_mgr.spawn_pool_plan_like_cpp(
            &mut self.pool_data,
            pool_id,
            explicit_roll_for,
            choose_equal,
        )?;
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
        self.original_pool(&plan, spawn_store, &mut summary, Some(&mut load_record));
        Ok(summary)
    }
}
