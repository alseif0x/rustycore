//! Ordinary object phases keep their original work and token on rejection.
use super::*;
use super::super::build_loaded_grid_gameobject_respawn_record_like_cpp;

/// Every variant owns the actual rejected stage, without cloning it.
pub(crate) enum CanonicalObjectResumeFailure {
    BeginRejected {
        failure: ObjectWorkBeginFailure,
    },
    PrepareRejected {
        error: wow_map::ObjectMapTickError,
        work: CanonicalObjectWork,
    },
    FinishRejected {
        error: wow_map::ObjectMapTickError,
        work: CanonicalObjectWork,
        token: wow_map::ObjectMapUpdateToken,
    },
    FinalizeRejected {
        error: wow_map::ObjectMapTickError,
        work: CanonicalObjectWork,
    },
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_resume(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    plan: wow_map::MapTickPlanLikeCpp,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Result<Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp>, CanonicalObjectResumeFailure> {
    let work = CanonicalObjectWork::try_begin(
        manager,
        legacy_manager,
        plan,
        scheduler,
        canonical_spawn_metadata,
        condition_store,
        map_store,
        loaded_grid_creature_respawn_caches,
    ).map_err(|failure| CanonicalObjectResumeFailure::BeginRejected { failure })?;
    resume_objects(manager, work, canonical_spawn_metadata, map_store,
        loaded_grid_creature_respawn_caches)
}

fn resume_objects(
    manager: &mut wow_map::MapManager,
    mut work: CanonicalObjectWork,
    canonical_spawn_metadata: &CanonicalSpawnMetadataLikeCpp,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Result<Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp>, CanonicalObjectResumeFailure> {
    loop {
        let token = match work.try_prepare_next(manager) {
            Ok(Some(token)) => token,
            Ok(None) => break,
            Err(error) => return Err(CanonicalObjectResumeFailure::PrepareRejected { error, work }),
        };
        work = finish_prepared_map(manager, work, token, canonical_spawn_metadata,
            loaded_grid_creature_respawn_caches)?;
    }
    finalize_work(manager, work, map_store)
}

fn finish_prepared_map(
    manager: &mut wow_map::MapManager,
    mut work: CanonicalObjectWork,
    token: wow_map::ObjectMapUpdateToken,
    canonical_spawn_metadata: &CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Result<CanonicalObjectWork, CanonicalObjectResumeFailure> {
    let mut load_record = |map, object_type, spawn_id| match object_type {
        wow_map::SpawnObjectType::GameObject => {
            build_loaded_grid_gameobject_respawn_record_like_cpp(
                map,
                object_type,
                spawn_id,
                canonical_spawn_metadata,
                loaded_grid_creature_respawn_caches,
            )
        }
        wow_map::SpawnObjectType::Creature | wow_map::SpawnObjectType::AreaTrigger => None,
    };
    match work.try_finish_map(
        manager,
        token,
        Some((
            canonical_spawn_metadata.spawn_store(),
            canonical_spawn_metadata.pool_mgr_like_cpp(),
        )),
        &mut load_record,
    ) {
        Ok(_) => Ok(work),
        Err((error, token)) => Err(CanonicalObjectResumeFailure::FinishRejected { error, work, token }),
    }
}

fn finalize_work(
    manager: &mut wow_map::MapManager,
    work: CanonicalObjectWork,
    map_store: &wow_data::MapStore,
) -> Result<Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp>, CanonicalObjectResumeFailure> {
    work.try_complete(manager, map_store)
        .map_err(|(error, work)| CanonicalObjectResumeFailure::FinalizeRejected { error, work })
}

impl CanonicalObjectResumeFailure {
    /// Explicit retry starts at the rejected stage, never at BEGIN/respawns.
    /// The ordinary producer retains failures and does not call this method.
    pub(crate) fn retry(
        self,
        manager: &mut wow_map::MapManager,
        canonical_spawn_metadata: &CanonicalSpawnMetadataLikeCpp,
        map_store: &wow_data::MapStore,
        loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    ) -> Result<Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp>, Self> {
        match self {
            failure @ Self::BeginRejected { .. } => Err(failure),
            Self::PrepareRejected { work, .. } => resume_objects(manager, work,
                canonical_spawn_metadata, map_store, loaded_grid_creature_respawn_caches),
            Self::FinishRejected { work, token, .. } => {
                let work = finish_prepared_map(manager, work, token,
                    canonical_spawn_metadata, loaded_grid_creature_respawn_caches)?;
                resume_objects(manager, work, canonical_spawn_metadata, map_store,
                    loaded_grid_creature_respawn_caches)
            }
            Self::FinalizeRejected { work, .. } => finalize_work(manager, work, map_store),
        }
    }
}

#[cfg(test)]
mod tests;
