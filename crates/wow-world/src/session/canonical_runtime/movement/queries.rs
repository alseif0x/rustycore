//! Resolve owned queries off-thread; return the original continuation with its response.

use super::{Arc, LiveTerrainHeights, SharedCanonicalMapManager, WorldMMapPathfinderWorkerLikeCpp};
use super::{MapObjectTickContinuation, ObjectMapUpdateToken};
use super::error::MovementFailure;
use wow_core::ObjectGuid;
use wow_map::{
    ActorGridHeightContinuation, ActorMovementPending, ActorMovementProgress,
    ActorPathContinuation, ActorStaticHeightContinuation,
};
use wow_recastdetour::DetourPolyPath;
use crate::session::creature_movement_adapter::{
    creature_path_request_like_cpp, resolve_creature_detour_path_like_cpp,
};

pub(super) enum ResolvedMovementQuery {
    Path(ActorPathContinuation, Option<DetourPolyPath>),
    StaticHeight(ActorStaticHeightContinuation, f32),
    GridHeight(ActorGridHeightContinuation, f32),
}

/// Called only with the original continuation and a reply already observed
/// after the worker join. Every rejected discard reconstructs the same input.
pub(super) fn discard(
    manager: &mut wow_map::MapManager,
    tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken,
    response: ResolvedMovementQuery,
) -> Result<(), (wow_map::ActorMovementError, ResolvedMovementQuery)> {
    match response {
        ResolvedMovementQuery::Path(continuation, response) => manager
            .discard_movement_path_reply(tick, token, continuation, response)
            .map_err(|failure| (failure.error,
                ResolvedMovementQuery::Path(failure.continuation, failure.response))),
        ResolvedMovementQuery::StaticHeight(continuation, response) => manager
            .discard_movement_static_height_reply(tick, token, continuation, response)
            .map_err(|failure| (failure.error,
                ResolvedMovementQuery::StaticHeight(failure.continuation, failure.response))),
        ResolvedMovementQuery::GridHeight(continuation, response) => manager
            .discard_movement_grid_height_reply(tick, token, continuation, response)
            .map_err(|failure| (failure.error,
                ResolvedMovementQuery::GridHeight(failure.continuation, failure.response))),
    }
}

pub(super) async fn resolve(
    guid: ObjectGuid,
    pending: ActorMovementPending,
    pathfinder: Option<&Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    terrain: Option<&Arc<LiveTerrainHeights>>,
) -> Result<ResolvedMovementQuery, MovementFailure> {
    let work = match pending {
        ActorMovementPending::Path(request) => {
            let worker = pathfinder.map(Arc::clone);
            tokio::task::spawn_blocking(move || {
                let (query, continuation) = request.into_parts();
                // Move the original continuation/phase into the closure. The
                // existing worker-request adapter retains its original phase
                // representation; no extra clone transports phase across await.
                let request = creature_path_request_like_cpp(
                    query, continuation.map_id(), continuation.instance_id(), continuation.phase_shift(),
                );
                let response = resolve_creature_detour_path_like_cpp(worker.as_deref(), guid, request);
                ResolvedMovementQuery::Path(continuation, response)
            })
        }
        ActorMovementPending::StaticHeight(request) => {
            let Some(terrain) = terrain.map(Arc::clone) else {
                return Err(MovementFailure::MissingTerrain(ActorMovementPending::StaticHeight(request)));
            };
            tokio::task::spawn_blocking(move || {
                let (query, continuation) = request.into_parts();
                let response = terrain.static_height_like_cpp(query.map_id, query.point.x, query.point.y, query.probe_z);
                ResolvedMovementQuery::StaticHeight(continuation, response)
            })
        }
        ActorMovementPending::GridHeight(request) => {
            let Some(terrain) = terrain.map(Arc::clone) else {
                return Err(MovementFailure::MissingTerrain(ActorMovementPending::GridHeight(request)));
            };
            tokio::task::spawn_blocking(move || {
                let (query, continuation) = request.into_parts();
                let response = terrain.grid_height_like_cpp(query.map_id, query.point.x, query.point.y);
                ResolvedMovementQuery::GridHeight(continuation, response)
            })
        }
    };
    // A panic/cancelled JoinHandle never becomes a synthetic None/height. The
    // token retains its operation slot even if the closure lost its request.
    work.await.map_err(MovementFailure::QueryPanicked)
}

pub(super) fn resume(
    manager: &SharedCanonicalMapManager,
    tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken,
    response: ResolvedMovementQuery,
) -> Result<ActorMovementProgress, MovementFailure> {
    let mut manager = match manager.lock() {
        Ok(manager) => manager,
        Err(_) => return Err(MovementFailure::ResumePoisoned(response)),
    };
    match response {
        ResolvedMovementQuery::Path(continuation, response) => manager
            .resume_movement_path(tick, token, continuation, response)
            .map_err(|failure| MovementFailure::Resume {
                error: failure.error,
                response: ResolvedMovementQuery::Path(failure.continuation, failure.response),
            }),
        ResolvedMovementQuery::StaticHeight(continuation, response) => manager
            .resume_movement_static_height(tick, token, continuation, response)
            .map_err(|failure| MovementFailure::Resume {
                error: failure.error,
                response: ResolvedMovementQuery::StaticHeight(failure.continuation, failure.response),
            }),
        ResolvedMovementQuery::GridHeight(continuation, response) => manager
            .resume_movement_grid_height(tick, token, continuation, response)
            .map_err(|failure| MovementFailure::Resume {
                error: failure.error,
                response: ResolvedMovementQuery::GridHeight(failure.continuation, failure.response),
            }),
    }
}
