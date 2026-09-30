//! Step-level ownership around the existing family requests.
//!
//! Mounted under the family engine to retain its private scalar-height fields.
//! The step owner reexports the crate-visible staged types. No actor borrow,
//! generator or RNG is retained; the same exclusively owned actor must resume.
//! Lifetime/admission and cancellation gates belong to the future map consumer.

use super::{MovementCompletion, MovementProgress, PathContinuation, PendingMovement, terrain};
use crate::map_manager::movement::step::{CreatureMovementSource, CreatureMovementStep};
use crate::map_manager::{ChaseTickOutcomeLikeCpp, CreaturePathQueryLikeCpp, WorldCreature};
use wow_core::Position;
use wow_entities::PhaseShift;
use wow_recastdetour::DetourPolyPath;

pub(crate) enum StepProgress {
    Complete(Option<CreatureMovementStep>),
    Pending(StepPending),
}

pub(crate) enum StepPending {
    Path(StepPathRequest),
    StaticHeight(StepStaticHeightRequest),
    GridHeight(StepGridHeightRequest),
}

pub(in crate::map_manager::movement) struct StepMetadata {
    pub(in crate::map_manager::movement) source: CreatureMovementSource,
    pub(in crate::map_manager::movement) map_id: u32,
    pub(in crate::map_manager::movement) instance_id: u32,
    pub(in crate::map_manager::movement) phase_shift: PhaseShift,
}

pub(crate) struct StepPathRequest {
    query: CreaturePathQueryLikeCpp,
    continuation: StepPathContinuation,
}

pub(crate) struct StepPathContinuation {
    metadata: StepMetadata,
    continuation: PathContinuation,
}

pub(crate) struct StepStaticHeightQuery {
    pub(crate) map_id: u32,
    pub(crate) point: Position,
    pub(crate) probe_z: f32,
}

pub(crate) struct StepStaticHeightRequest {
    query: StepStaticHeightQuery,
    continuation: StepStaticHeightContinuation,
}

pub(crate) struct StepStaticHeightContinuation {
    metadata: StepMetadata,
    continuation: terrain::StaticHeightRequest,
}

pub(crate) struct StepGridHeightQuery {
    pub(crate) map_id: u32,
    pub(crate) point: Position,
}

pub(crate) struct StepGridHeightRequest {
    query: StepGridHeightQuery,
    continuation: StepGridHeightContinuation,
}

pub(crate) struct StepGridHeightContinuation {
    metadata: StepMetadata,
    continuation: terrain::GridHeightRequest,
}

impl StepPathRequest {
    pub(crate) fn into_parts(self) -> (CreaturePathQueryLikeCpp, StepPathContinuation) {
        (self.query, self.continuation)
    }
}

impl StepPathContinuation {
    pub(crate) fn map_id(&self) -> u32 {
        self.metadata.map_id
    }

    pub(crate) fn instance_id(&self) -> u32 {
        self.metadata.instance_id
    }

    pub(crate) fn phase_shift(&self) -> &PhaseShift {
        &self.metadata.phase_shift
    }

    pub(crate) fn resume(self, actor: &mut WorldCreature, response: Option<DetourPolyPath>) -> StepProgress {
        let progress = self.continuation.resume(actor, response);
        wrap(progress, self.metadata)
    }
}

impl StepStaticHeightRequest {
    pub(crate) fn into_parts(self) -> (StepStaticHeightQuery, StepStaticHeightContinuation) {
        (self.query, self.continuation)
    }
}

impl StepStaticHeightContinuation {
    pub(crate) fn resume(self, actor: &mut WorldCreature, height: f32) -> StepProgress {
        let progress = self.continuation.resume(actor, height);
        wrap(progress, self.metadata)
    }
}

impl StepGridHeightRequest {
    pub(crate) fn into_parts(self) -> (StepGridHeightQuery, StepGridHeightContinuation) {
        (self.query, self.continuation)
    }
}

impl StepGridHeightContinuation {
    pub(crate) fn resume(self, actor: &mut WorldCreature, height: f32) -> StepProgress {
        let progress = self.continuation.resume(actor, height);
        wrap(progress, self.metadata)
    }
}

pub(in crate::map_manager::movement) fn wrap(progress: MovementProgress, metadata: StepMetadata) -> StepProgress {
    match progress {
        MovementProgress::Complete(completion) => {
            let result = match completion {
                MovementCompletion::Home(outcome) | MovementCompletion::Chase(outcome) => {
                    match outcome {
                        ChaseTickOutcomeLikeCpp::Idle => None,
                        ChaseTickOutcomeLikeCpp::Stopped(stop) => Some(CreatureMovementStep::Stop(stop)),
                        ChaseTickOutcomeLikeCpp::Launched(from, spline) => Some(CreatureMovementStep::Launch {
                            source: metadata.source, from, spline,
                        }),
                    }
                }
                MovementCompletion::Random(movement) | MovementCompletion::Waypoint(_, movement) => {
                    movement.map(|(from, spline)| CreatureMovementStep::Launch {
                        source: metadata.source, from, spline,
                    })
                }
            };
            StepProgress::Complete(result)
        }
        MovementProgress::Pending(PendingMovement::Path(request)) => {
            let (query, continuation) = request.into_parts();
            StepProgress::Pending(StepPending::Path(StepPathRequest {
                query, continuation: StepPathContinuation { metadata, continuation },
            }))
        }
        MovementProgress::Pending(PendingMovement::StaticHeight(request)) => {
            let query = StepStaticHeightQuery {
                map_id: request.request.map_id,
                point: request.request.point,
                probe_z: request.request.probe_z,
            };
            StepProgress::Pending(StepPending::StaticHeight(StepStaticHeightRequest {
                query, continuation: StepStaticHeightContinuation { metadata, continuation: request },
            }))
        }
        MovementProgress::Pending(PendingMovement::GridHeight(request)) => {
            let query = StepGridHeightQuery {
                map_id: request.request.map_id,
                point: request.request.point,
            };
            StepProgress::Pending(StepPending::GridHeight(StepGridHeightRequest {
                query, continuation: StepGridHeightContinuation { metadata, continuation: request },
            }))
        }
    }
}
