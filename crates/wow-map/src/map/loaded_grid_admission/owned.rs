//! Owned receipts for Group/Conditions, Pool and Catalog materializations.

use super::*;
use crate::map::{SpawnGroupActiveChange, SpawnGroupConditionActionLikeCpp,
    SpawnGroupConditionUpdateOutcomeLikeCpp, SpawnGroupDespawnOutcomeLikeCpp,
    SpawnGroupSpawnLoadPlanLikeCpp, SpawnGroupSpawnOutcomeLikeCpp,
    PoolSpawnActionLoadPlanLikeCpp, ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    SpawnObjectType, SpawnId};

#[derive(Debug)]
pub enum LoadedGridAttemptPlan {
    SpawnGroup(SpawnGroupSpawnLoadPlanLikeCpp),
    Pool(PoolSpawnActionLoadPlanLikeCpp),
    Catalog { object_type: SpawnObjectType, spawn_id: SpawnId },
}

#[derive(Debug)]
pub struct LoadedGridSpawnAttempt {
    pub plan: LoadedGridAttemptPlan,
    pub result: LoadedGridSpawnAttemptResult,
}

#[derive(Debug)]
pub enum LoadedGridSpawnAttemptResult {
    NoLoader,
    Unavailable,
    PreparationRejected(LoadedGridRespawnRecordsLikeCpp),
    Admitted(LoadedGridAdmission),
}

#[derive(Debug)]
pub struct LoadedGridSpawnOutcome {
    /// Owned receipts carry all snapshots; this summary's Record vector is empty.
    pub summary: SpawnGroupSpawnOutcomeLikeCpp,
    pub attempts: Vec<LoadedGridSpawnAttempt>,
}

#[derive(Debug)]
pub struct LoadedGridPoolOutcome {
    /// Owned receipts retain snapshots and incoming motors; no summary clone.
    pub summary: ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    pub attempts: Vec<LoadedGridSpawnAttempt>,
}

#[derive(Debug)]
pub struct LoadedGridRespawnOutcome {
    /// Full admission receipts own their snapshots and incoming motors.
    pub summary: ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    pub attempts: Vec<LoadedGridSpawnAttempt>,
}

#[derive(Debug)]
pub struct LoadedGridConditionOutcome {
    pub group_id: u32,
    pub action: SpawnGroupConditionActionLikeCpp,
    pub applied_change: Option<SpawnGroupActiveChange>,
    pub despawn_outcome: Option<SpawnGroupDespawnOutcomeLikeCpp>,
    pub spawn_outcome: Option<LoadedGridSpawnOutcome>,
}

impl LoadedGridConditionOutcome {
    pub(in crate::map) fn into_record_outcome(self) -> SpawnGroupConditionUpdateOutcomeLikeCpp {
        SpawnGroupConditionUpdateOutcomeLikeCpp {
            group_id: self.group_id,
            action: self.action,
            applied_change: self.applied_change,
            despawn_outcome: self.despawn_outcome,
            // The core already projected each successful snapshot at its old
            // push site. Record wrappers never make an owned preparation error.
            spawn_outcome: self.spawn_outcome.map(|outcome| outcome.summary),
        }
    }
}

pub(in crate::map) enum LoadedGridReceipts {
    RecordCompatibility,
    Owned(Vec<LoadedGridSpawnAttempt>),
}

impl LoadedGridReceipts {
    pub(in crate::map) fn for_operation(&self) -> Self {
        match self {
            Self::RecordCompatibility => Self::RecordCompatibility,
            Self::Owned(_) => Self::Owned(Vec::new()),
        }
    }

    pub(in crate::map) fn load_failed(
        &mut self,
        plan: SpawnGroupSpawnLoadPlanLikeCpp,
        failure: Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    ) {
        let result = match failure {
            Ok(None) => LoadedGridSpawnAttemptResult::Unavailable,
            Err(records) => LoadedGridSpawnAttemptResult::PreparationRejected(records),
            Ok(Some(_)) => unreachable!("successful materialization enters admission"),
        };
        if let Self::Owned(attempts) = self {
            attempts.push(LoadedGridSpawnAttempt {
                plan: LoadedGridAttemptPlan::SpawnGroup(plan), result,
            });
        }
    }

    pub(in crate::map) fn admitted(
        &mut self,
        plan: SpawnGroupSpawnLoadPlanLikeCpp,
        admission: LoadedGridAdmission,
        outcome: &mut SpawnGroupSpawnOutcomeLikeCpp,
    ) {
        match self {
            Self::RecordCompatibility => {
                let (_, loaded_grid_primary_record, primary_result) = admission.into_record_parts();
                match primary_result {
                    Ok(_outcome) => {
                        outcome.executed_loaded_grid_spawns += 1;
                        outcome
                            .loaded_grid_primary_records
                            .push(loaded_grid_primary_record);
                    }
                    Err(_error) => outcome.blocked_loaded_grid_spawn_add_to_map += 1,
                }
            }
            Self::Owned(attempts) => {
                // Record refresh/duplicate success stays success. Fresh Actor
                // duplicates retain their incoming motor and count as blocked
                // Add under this new dormant ownership contract.
                let inserted = primary_succeeded(&admission);
                if inserted {
                    outcome.executed_loaded_grid_spawns += 1;
                } else {
                    outcome.blocked_loaded_grid_spawn_add_to_map += 1;
                }
                attempts.push(LoadedGridSpawnAttempt {
                    plan: LoadedGridAttemptPlan::SpawnGroup(plan),
                    result: LoadedGridSpawnAttemptResult::Admitted(admission),
                });
            }
        }
    }

    pub(in crate::map) fn finish(self, summary: SpawnGroupSpawnOutcomeLikeCpp) -> LoadedGridSpawnOutcome {
        let attempts = match self {
            Self::RecordCompatibility => Vec::new(),
            Self::Owned(attempts) => attempts,
        };
        LoadedGridSpawnOutcome { summary, attempts }
    }

    pub(in crate::map) fn pool_no_loader(&mut self, plan: PoolSpawnActionLoadPlanLikeCpp) {
        if let Self::Owned(attempts) = self {
            attempts.push(LoadedGridSpawnAttempt {
                plan: LoadedGridAttemptPlan::Pool(plan),
                result: LoadedGridSpawnAttemptResult::NoLoader,
            });
        }
    }

    pub(in crate::map) fn pool_load_failed(
        &mut self,
        plan: PoolSpawnActionLoadPlanLikeCpp,
        failure: Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    ) {
        let result = match failure {
            Ok(None) => LoadedGridSpawnAttemptResult::Unavailable,
            Err(records) => LoadedGridSpawnAttemptResult::PreparationRejected(records),
            Ok(Some(_)) => unreachable!("successful materialization enters admission"),
        };
        if let Self::Owned(attempts) = self {
            attempts.push(LoadedGridSpawnAttempt { plan: LoadedGridAttemptPlan::Pool(plan), result });
        }
    }

    pub(in crate::map) fn pool_admitted(
        &mut self,
        plan: PoolSpawnActionLoadPlanLikeCpp,
        admission: LoadedGridAdmission,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.respawn_admitted(LoadedGridAttemptPlan::Pool(plan), admission, summary);
    }

    pub(in crate::map) fn catalog_load_failed(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
        failure: Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    ) {
        let result = match failure {
            Ok(None) => LoadedGridSpawnAttemptResult::Unavailable,
            Err(records) => LoadedGridSpawnAttemptResult::PreparationRejected(records),
            Ok(Some(_)) => unreachable!("successful materialization enters admission"),
        };
        if let Self::Owned(attempts) = self {
            attempts.push(LoadedGridSpawnAttempt {
                plan: LoadedGridAttemptPlan::Catalog { object_type, spawn_id }, result,
            });
        }
    }

    pub(in crate::map) fn catalog_admitted(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
        admission: LoadedGridAdmission,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.respawn_admitted(LoadedGridAttemptPlan::Catalog { object_type, spawn_id }, admission, summary);
    }

    fn respawn_admitted(
        &mut self,
        plan: LoadedGridAttemptPlan,
        admission: LoadedGridAdmission,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        match self {
            Self::RecordCompatibility => {
                let (_, loaded_grid_primary_record, primary_result) = admission.into_record_parts();
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
            Self::Owned(attempts) => {
                if primary_succeeded(&admission) {
                    summary.executed_loaded_grid_respawns += 1;
                } else {
                    summary.blocked_loaded_grid_respawn_add_to_map += 1;
                }
                attempts.push(LoadedGridSpawnAttempt {
                    plan,
                    result: LoadedGridSpawnAttemptResult::Admitted(admission),
                });
            }
        }
    }

    pub(in crate::map) fn finish_pool(self, summary: ProcessRespawnsSafeSideEffectsSummaryLikeCpp) -> LoadedGridPoolOutcome {
        let attempts = match self {
            Self::RecordCompatibility => Vec::new(),
            Self::Owned(attempts) => attempts,
        };
        LoadedGridPoolOutcome { summary, attempts }
    }

    pub(in crate::map) fn finish_respawns(self, summary: ProcessRespawnsSafeSideEffectsSummaryLikeCpp) -> LoadedGridRespawnOutcome {
        let attempts = match self {
            Self::RecordCompatibility => Vec::new(),
            Self::Owned(attempts) => attempts,
        };
        LoadedGridRespawnOutcome { summary, attempts }
    }
}

fn primary_succeeded(admission: &LoadedGridAdmission) -> bool {
    match &admission.primary {
        LoadedGridPrimaryAdmission::Record { result, .. } => result.is_ok(),
        LoadedGridPrimaryAdmission::CreatureActor(result) => matches!(
            result, Ok(FreshCreatureActorAdmission::Inserted { .. }),
        ),
    }
}
