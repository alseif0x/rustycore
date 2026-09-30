//! Consumable movement operations for one exclusively borrowed, unchanged actor.
//!
//! Requests own only query/launch data. They do not own an actor, generator or
//! RNG, and dropping a request does not complete its transition. The synchronous
//! callers below resolve the same requests immediately. A future map caller must
//! supply its own lifetime, write-admission and cancellation contract.

use super::*;

mod chase;
mod home;
mod random;
mod terrain;
mod waypoint;

// Step adapters need private height request data, without opening family fields.
#[path = "step/pending.rs"]
pub(super) mod step_pending;

pub(super) use chase::prepare as prepare_chase;
pub(super) use home::prepare as prepare_home;
pub(super) use random::prepare as prepare_random;
pub(super) use waypoint::prepare as prepare_waypoint;

pub(super) enum MovementProgress {
    Complete(MovementCompletion),
    Pending(PendingMovement),
}

pub(super) enum MovementCompletion {
    Home(ChaseTickOutcomeLikeCpp),
    Random(Option<(Position, MoveSpline)>),
    Waypoint(WaypointMovementAction, Option<(Position, MoveSpline)>),
    Chase(ChaseTickOutcomeLikeCpp),
}

pub(super) enum PendingMovement {
    Path(PathRequest),
    StaticHeight(terrain::StaticHeightRequest),
    GridHeight(terrain::GridHeightRequest),
}

pub(super) struct PathRequest {
    query: CreaturePathQueryLikeCpp,
    continuation: PathContinuation,
}

pub(super) struct PathContinuation {
    purpose: PathPurpose,
    terrain_enabled: bool,
}

enum PathPurpose {
    Home(home::Launch),
    Random(random::Query),
    Waypoint(waypoint::Query),
    Chase(chase::Launch),
}

enum TerrainPurpose {
    Home(home::Destination),
    Chase(chase::Destination),
    Path(terrain::PathNormalization),
}

impl PathRequest {
    pub(super) fn into_parts(self) -> (CreaturePathQueryLikeCpp, PathContinuation) {
        (self.query, self.continuation)
    }
}

impl PathContinuation {
    pub(super) fn resume(
        self,
        actor: &mut WorldCreature,
        response: Option<DetourPolyPath>,
    ) -> MovementProgress {
        match self.purpose {
            PathPurpose::Home(launch) => {
                home::resolved(actor, launch, response, self.terrain_enabled)
            }
            PathPurpose::Random(query) => {
                random::resolved(actor, query, response, self.terrain_enabled)
            }
            PathPurpose::Waypoint(query) => {
                waypoint::resolved(actor, query, response, self.terrain_enabled)
            }
            PathPurpose::Chase(launch) => {
                chase::resolved(actor, launch, response, true, self.terrain_enabled)
            }
        }
    }
}

fn request_path(
    query: CreaturePathQueryLikeCpp,
    purpose: PathPurpose,
    terrain_enabled: bool,
) -> MovementProgress {
    MovementProgress::Pending(PendingMovement::Path(PathRequest {
        query,
        continuation: PathContinuation {
            purpose,
            terrain_enabled,
        },
    }))
}

pub(super) fn run(
    actor: &mut WorldCreature,
    mut progress: MovementProgress,
    terrain: Option<&LiveTerrainHeights>,
    resolve_path: &mut impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
) -> MovementCompletion {
    loop {
        progress = match progress {
            MovementProgress::Complete(result) => return result,
            MovementProgress::Pending(PendingMovement::Path(request)) => {
                // Moving the query out leaves the non-Clone continuation owned
                // by this call; no actor borrow crosses the resolver call.
                let (query, continuation) = request.into_parts();
                let response = resolve_path(query);
                continuation.resume(actor, response)
            }
            MovementProgress::Pending(PendingMovement::StaticHeight(request)) => {
                let height = terrain
                    .expect("height requests require terrain")
                    .static_height_like_cpp(
                        request.request.map_id,
                        request.request.point.x,
                        request.request.point.y,
                        request.request.probe_z,
                    );
                request.resume(actor, height)
            }
            MovementProgress::Pending(PendingMovement::GridHeight(request)) => {
                let height = terrain
                    .expect("height requests require terrain")
                    .grid_height_like_cpp(
                        request.request.map_id,
                        request.request.point.x,
                        request.request.point.y,
                    );
                request.resume(actor, height)
            }
        };
    }
}

pub(super) fn normalize_position_sync(
    actor: &WorldCreature,
    point: Position,
    heights: Option<&LiveTerrainHeights>,
) -> Position {
    terrain::normalize_sync(actor, point, heights)
}

#[cfg(test)]
mod tests;
