//! Owned post-session object work for one admitted canonical map tick.

use crate::runtime::map::{
    CanonicalRespawnConditionSchedulerLikeCpp, LoadedGridCreatureRespawnCachesLikeCpp,
};
use crate::runtime::tick_summary::CanonicalSpawnGroupConditionTickSummaryLikeCpp;
use crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp;
use wow_world::SharedMapManager;

use super::{canonical_map_tick_respawn_phase_like_cpp, canonical_map_tick_tail_like_cpp};

mod creature_kill_loot;
pub(crate) use creature_kill_loot::ReservedCreatureLootGeneration;
mod resume;
pub(crate) use resume::{CanonicalObjectResumeFailure, try_resume};
mod begin;
pub(crate) use begin::ObjectWorkBeginFailure;

/// Owns progress without retaining a manager or metadata borrow.
///
/// Dropping unfinished work does not finalize or release the admitted tick.
pub(crate) struct CanonicalObjectWork {
    object_tick: wow_map::MapObjectTickContinuation,
    respawn_summary: CanonicalSpawnGroupConditionTickSummaryLikeCpp,
}

impl CanonicalObjectWork {
    pub(crate) fn begin(
        manager: &mut wow_map::MapManager,
        legacy_manager: Option<&SharedMapManager>,
        plan: wow_map::MapTickPlanLikeCpp,
        scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
        canonical_spawn_metadata: &CanonicalSpawnMetadataLikeCpp,
        condition_store: &wow_data::ConditionEntriesByTypeStore,
        map_store: &wow_data::MapStore,
        loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    ) -> Option<Self> {
        Self::try_begin(
            manager,
            legacy_manager,
            plan,
            scheduler,
            canonical_spawn_metadata,
            condition_store,
            map_store,
            loaded_grid_creature_respawn_caches,
        )
        .ok()
    }

    /// The outer Option reports a rejected stage; the inner None ends the pass.
    pub(crate) fn prepare_next(
        &mut self,
        manager: &mut wow_map::MapManager,
    ) -> Option<Option<wow_map::ObjectMapUpdateToken>> {
        self.try_prepare_next(manager).ok()
    }

    pub(crate) fn try_prepare_next(
        &mut self,
        manager: &mut wow_map::MapManager,
    ) -> Result<Option<wow_map::ObjectMapUpdateToken>, wow_map::ObjectMapTickError> {
        manager.prepare_next_object_map(
            &mut self.object_tick,
            wow_map::MapObjectUpdateSelectionLikeCpp::NearbyCells,
        )
    }

    pub(crate) fn finish_map<L>(
        &mut self,
        manager: &mut wow_map::MapManager,
        token: wow_map::ObjectMapUpdateToken,
        pool_update: Option<(&wow_map::SpawnStore, &wow_map::PoolMgrLikeCpp)>,
        load_record: &mut L,
    ) -> Option<()>
    where
        L: FnMut(
            &mut wow_map::Map,
            wow_map::SpawnObjectType,
            wow_map::SpawnId,
        ) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp>,
    {
        // Compatibility retains its original Option mapping and deliberately
        // discards the rejected token. Reserved work uses the recoverable API.
        match self.try_finish_map(manager, token, pool_update, load_record) {
            Ok(
                wow_map::ObjectMapFinishOutcome::Completed
                | wow_map::ObjectMapFinishOutcome::StaleParticipant { .. },
            ) => Some(()),
            Err((_error, _original_token)) => None,
        }
    }

    pub(crate) fn try_finish_map<L>(
        &mut self,
        manager: &mut wow_map::MapManager,
        token: wow_map::ObjectMapUpdateToken,
        pool_update: Option<(&wow_map::SpawnStore, &wow_map::PoolMgrLikeCpp)>,
        load_record: &mut L,
    ) -> Result<
        wow_map::ObjectMapFinishOutcome,
        (wow_map::ObjectMapTickError, wow_map::ObjectMapUpdateToken),
    >
    where
        L: FnMut(
            &mut wow_map::Map,
            wow_map::SpawnObjectType,
            wow_map::SpawnId,
        ) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp>,
    {
        // The canonical map still carries an intentionally incomplete Creature
        // visitor. Production behaviour is owned by the legacy/session runtime;
        // declare that owner so this tick cannot mutate and discard a shadow plan.
        manager.try_finish_object_map(
            &mut self.object_tick,
            token,
            pool_update,
            Some(load_record),
            wow_map::MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
        )
    }

    pub(crate) fn complete(
        self,
        manager: &mut wow_map::MapManager,
        map_store: &wow_data::MapStore,
    ) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
        self.try_complete(manager, map_store).ok().flatten()
    }

    /// Err retains the original work before any finalization/tail effect.
    /// Ok(None) is settled: finalization and the consuming APP tail both ran.
    pub(crate) fn try_complete(
        self,
        manager: &mut wow_map::MapManager,
        map_store: &wow_data::MapStore,
    ) -> Result<
        Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp>,
        (wow_map::ObjectMapTickError, CanonicalObjectWork),
    > {
        let Self {
            object_tick,
            respawn_summary,
        } = self;
        match manager.try_finalize_object_tick(object_tick) {
            Ok(()) => Ok(canonical_map_tick_tail_like_cpp(
                manager,
                respawn_summary,
                map_store,
            )),
            Err((error, object_tick)) => Err((
                error,
                Self {
                    object_tick,
                    respawn_summary,
                },
            )),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod completion_tests;
