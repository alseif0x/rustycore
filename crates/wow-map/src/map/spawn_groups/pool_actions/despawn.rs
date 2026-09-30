//! Existing Pool despawn operations; bodies and action ordering preserved.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub(super) fn apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolDespawnPoolPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        for subplan in &plan.subplans {
            self.apply_pool_typed_despawn_plan_safe_map_actions_like_cpp(subplan, summary);
        }
    }

    pub(super) fn apply_pool_typed_despawn_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolTypedDespawnPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        if let Some(object_plan) = plan.object_plan.as_ref() {
            self.apply_pool_despawn_object_plan_safe_map_actions_like_cpp(object_plan, summary);
        }
    }

    pub(super) fn apply_pool_despawn_object_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolDespawnObjectPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        let mut child_pool_plans = plan.child_pool_plans.iter();
        for action in &plan.actions {
            match *action {
                PoolSpawnObjectActionLikeCpp::DespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_pool_plans.next() {
                        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
                            child_plan, summary,
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                other => match other {
                    PoolSpawnObjectActionLikeCpp::DespawnOne { kind, guid } => {
                        self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
                    }
                    PoolSpawnObjectActionLikeCpp::RemoveRespawnTime { kind, guid } => {
                        let Some(object_type) =
                            pool_member_kind_to_spawn_object_type_like_cpp(kind)
                        else {
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
                    PoolSpawnObjectActionLikeCpp::SpawnOne { .. }
                    | PoolSpawnObjectActionLikeCpp::RespawnOne { .. } => {}
                },
            }
        }
    }

    pub(super) fn apply_pool_despawn_one_safe_map_action_like_cpp(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        spawn_id: u64,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        let spawn_id = spawn_id as SpawnId;
        let guids = match kind {
            PoolMemberKindLikeCpp::Creature => {
                self.creature_spawn_id_store_guids_like_cpp(spawn_id)
            }
            PoolMemberKindLikeCpp::GameObject => {
                self.gameobject_spawn_id_store_guids_like_cpp(spawn_id)
            }
            PoolMemberKindLikeCpp::Pool => {
                summary.pool_unsupported_action_kind += 1;
                return;
            }
        };

        for guid in guids {
            if self.map_object_record(guid).is_none() {
                summary.pool_stale_index_entries += 1;
                continue;
            }
            match self.remove_from_map_like_cpp(guid, true) {
                Ok(_removed) => {
                    summary.pool_objects_removed += 1;
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => {
                    summary.pool_stale_index_entries += 1;
                }
                Err(_error) => {
                    summary.pool_remove_errors += 1;
                }
            }
        }
    }

    /// Map-owned facade for a direct C++ `PoolMgr::DespawnPool(spawns, pool_id,
    /// alwaysDeleteRespawnTime)` call.
    ///
    /// Ownership stays one-way: `PoolMgrLikeCpp` plans and mutates only this
    /// map's canonical `SpawnedPoolDataLikeCpp`; `Map` then applies only safe
    /// map-local Creature/GameObject removal and respawn-timer deletion actions
    /// already represented by the plan. It does not fabricate live records,
    /// persist DB state, or fan out packets/scripts/AI.
    pub fn despawn_pool_safe_map_actions_like_cpp(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        pool_id: u32,
        always_delete_respawn_time: bool,
    ) -> Result<ProcessRespawnsSafeSideEffectsSummaryLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let plan = pool_mgr.despawn_pool_plan_like_cpp(
            &mut self.pool_data,
            pool_id,
            always_delete_respawn_time,
        )?;
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(&plan, &mut summary);
        Ok(summary)
    }

}
