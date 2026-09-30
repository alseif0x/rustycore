//! One Pool planner and ordered recursive action engine for owned/Record loads.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn spawn_pool_materialized<L>(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        pool_id: u32,
        spawn_store: &SpawnStore,
        explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        load_record: L,
    ) -> Result<LoadedGridPoolOutcome, PoolMgrPlanErrorLikeCpp>
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        self.spawn_pool_materialized_core(
            pool_mgr, pool_id, spawn_store, explicit_roll_for, choose_equal,
            load_record, LoadedGridReceipts::Owned(Vec::new()),
        )
    }

    pub(super) fn spawn_pool_materialized_core<L>(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        pool_id: u32,
        spawn_store: &SpawnStore,
        explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        mut load_record: L,
        mut receipts: LoadedGridReceipts,
    ) -> Result<LoadedGridPoolOutcome, PoolMgrPlanErrorLikeCpp>
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        let plan = pool_mgr.spawn_pool_plan_like_cpp(
            &mut self.pool_data,
            pool_id,
            explicit_roll_for,
            choose_equal,
        )?;
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
        self.apply_pool_spawn_pool_materialized(
            &plan,
            spawn_store,
            &mut summary,
            Some(&mut load_record),
            &mut receipts,
        );
        Ok(receipts.finish_pool(summary))
    }

    pub(in crate::map) fn apply_pool_typed_materialized<L>(
        &mut self,
        plan: &PoolTypedSpawnPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
        receipts: &mut LoadedGridReceipts,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        if let Some(object_plan) = plan.object_plan.as_ref() {
            self.apply_pool_spawn_object_materialized(
                object_plan,
                spawn_store,
                summary,
                load_record.as_deref_mut(),
                receipts,
            );
        }
    }

    fn apply_pool_spawn_pool_materialized<L>(
        &mut self,
        plan: &PoolSpawnPoolPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
        receipts: &mut LoadedGridReceipts,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        for subplan in &plan.subplans {
            self.apply_pool_typed_materialized(
                subplan,
                spawn_store,
                summary,
                load_record.as_deref_mut(),
                receipts,
            );
        }
    }

    fn apply_pool_spawn_object_materialized<L>(
        &mut self,
        plan: &PoolSpawnObjectPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
        receipts: &mut LoadedGridReceipts,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
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
                        self.apply_pool_spawn_pool_materialized(
                            child_plan,
                            spawn_store,
                            summary,
                            load_record.as_deref_mut(),
                            receipts,
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
                other => self.apply_pool_spawn_action_materialized(
                    other,
                    spawn_store,
                    summary,
                    load_record.as_deref_mut(),
                    receipts,
                ),
            }
        }
    }

    fn apply_pool_spawn_action_materialized<L>(
        &mut self,
        action: PoolSpawnObjectActionLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        load_record: Option<&mut L>,
        receipts: &mut LoadedGridReceipts,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        match action {
            PoolSpawnObjectActionLikeCpp::DespawnOne { kind, guid } => {
                self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
            }
            PoolSpawnObjectActionLikeCpp::RespawnOne { kind, guid } => {
                self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
                self.report_pool_spawn_materialized(
                    kind,
                    guid,
                    true,
                    spawn_store,
                    summary,
                    load_record,
                    receipts,
                );
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
                self.report_pool_spawn_materialized(
                    kind,
                    guid,
                    false,
                    spawn_store,
                    summary,
                    load_record,
                    receipts,
                );
            }
        }
    }

    fn report_pool_spawn_materialized<L>(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        spawn_id: u64,
        respawn: bool,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        load_record: Option<&mut L>,
        receipts: &mut LoadedGridReceipts,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
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
            receipts.pool_no_loader(PoolSpawnActionLoadPlanLikeCpp { object_type, spawn_id, respawn });
            return;
        };

        let materialization = match load_record(self, object_type, spawn_id) {
            Ok(Some(materialization)) => materialization,
            failure => {
                summary.pool_spawn_actions_blocked_loaded_grid += 1;
                summary
                    .pool_spawn_action_load_plans
                    .push(PoolSpawnActionLoadPlanLikeCpp {
                        object_type,
                        spawn_id,
                        respawn,
                    });
                receipts.pool_load_failed(
                    PoolSpawnActionLoadPlanLikeCpp { object_type, spawn_id, respawn }, failure,
                );
                return;
            }
        };

        let admission = self.admit_loaded_grid_materialization(materialization);
        receipts.pool_admitted(
            PoolSpawnActionLoadPlanLikeCpp { object_type, spawn_id, respawn }, admission, summary,
        );
    }

}

#[cfg(test)]
mod tests;
