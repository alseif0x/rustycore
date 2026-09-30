//! Preserve the original plan and distinguish whether the respawn prefix ran.
use super::*;

mod abandon;

pub(crate) enum ObjectWorkBeginFailure {
    BeforePrefix {
        plan: wow_map::MapTickPlanLikeCpp,
    },
    AfterPrefix {
        error: wow_map::ObjectMapTickError,
        plan: wow_map::MapTickPlanLikeCpp,
        respawn_summary: CanonicalSpawnGroupConditionTickSummaryLikeCpp,
    },
}

impl CanonicalObjectWork {
    pub(crate) fn try_begin(
        manager: &mut wow_map::MapManager,
        legacy_manager: Option<&SharedMapManager>,
        plan: wow_map::MapTickPlanLikeCpp,
        scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
        canonical_spawn_metadata: &CanonicalSpawnMetadataLikeCpp,
        condition_store: &wow_data::ConditionEntriesByTypeStore,
        map_store: &wow_data::MapStore,
        loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    ) -> Result<Self, ObjectWorkBeginFailure> {
        let effective_diff_ms = plan.effective_diff_ms();
        if !manager.can_resume_tick(&plan) {
            // Respawn work is part of the same admitted tick.  Refuse it before
            // touching any map when this manager does not own that admitted plan.
            return Err(ObjectWorkBeginFailure::BeforePrefix { plan });
        }
        // TrinityCore runs ProcessRespawns and UpdateSpawnGroupConditions after
        // the map's session pass and before ObjectUpdater (`Map.cpp:682-693`).
        // Keep that phase inside the admitted map incarnations, before the resume
        // method visits objects and captures SendObjectUpdates values.
        let respawn_summary = canonical_map_tick_respawn_phase_like_cpp(
            manager,
            plan.updated_maps_like_cpp(),
            legacy_manager,
            effective_diff_ms,
            scheduler,
            canonical_spawn_metadata,
            condition_store,
            map_store,
            loaded_grid_creature_respawn_caches,
        );
        match manager.try_begin_object_tick(plan) {
            Ok(object_tick) => Ok(Self { object_tick, respawn_summary }),
            Err((error, plan)) => Err(ObjectWorkBeginFailure::AfterPrefix {
                error, plan, respawn_summary,
            }),
        }
    }
}

#[cfg(test)]
mod tests;
