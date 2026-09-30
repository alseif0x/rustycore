//! Concrete I/O requests keep one immutable Actor admission and Step continuation.

use super::*;

pub enum ActorMovementProgress {
    Complete(ActorMovementCompletion),
    Pending(ActorMovementPending),
}

pub enum ActorMovementPending {
    Path(ActorPathRequest),
    StaticHeight(ActorStaticHeightRequest),
    GridHeight(ActorGridHeightRequest),
}

impl ActorMovementPending {
    pub(super) fn identity(&self) -> &ActorStepIdentity {
        match self {
            Self::Path(request) => &request.continuation.identity,
            Self::StaticHeight(request) => &request.continuation.identity,
            Self::GridHeight(request) => &request.continuation.identity,
        }
    }
}

pub struct ActorPathRequest {
    query: CreaturePathQueryLikeCpp,
    continuation: ActorPathContinuation,
}

pub struct ActorPathContinuation {
    pub(super) identity: ActorStepIdentity,
    pub(super) continuation: StepPathContinuation,
}

pub struct ActorStaticHeightRequest {
    query: ActorStaticHeightQuery,
    continuation: ActorStaticHeightContinuation,
}

pub struct ActorStaticHeightContinuation {
    pub(super) identity: ActorStepIdentity,
    pub(super) continuation: StepStaticHeightContinuation,
}

pub struct ActorGridHeightRequest {
    query: ActorGridHeightQuery,
    continuation: ActorGridHeightContinuation,
}

pub struct ActorGridHeightContinuation {
    pub(super) identity: ActorStepIdentity,
    pub(super) continuation: StepGridHeightContinuation,
}

impl ActorPathRequest {
    pub fn into_parts(self) -> (CreaturePathQueryLikeCpp, ActorPathContinuation) {
        (self.query, self.continuation)
    }
}

impl ActorPathContinuation {
    pub fn map_id(&self) -> u32 {
        self.continuation.map_id()
    }

    pub fn instance_id(&self) -> u32 {
        self.continuation.instance_id()
    }

    pub fn phase_shift(&self) -> &PhaseShift {
        self.continuation.phase_shift()
    }
}

impl ActorStaticHeightRequest {
    pub fn into_parts(self) -> (ActorStaticHeightQuery, ActorStaticHeightContinuation) {
        (self.query, self.continuation)
    }
}

impl ActorGridHeightRequest {
    pub fn into_parts(self) -> (ActorGridHeightQuery, ActorGridHeightContinuation) {
        (self.query, self.continuation)
    }
}

pub struct ActorStaticHeightQuery {
    pub map_id: u32,
    pub point: wow_core::Position,
    pub probe_z: f32,
}

pub struct ActorGridHeightQuery {
    pub map_id: u32,
    pub point: wow_core::Position,
}

pub(super) fn wrap(
    identity: ActorStepIdentity,
    progress: StepProgress,
    actor: &mut crate::map_manager::WorldCreature,
    key: crate::MapKey,
    incarnation: u64,
) -> ActorMovementProgress {
    match progress {
        // The manager releases the slot only for this explicit terminal Step.
        StepProgress::Complete(effect) => ActorMovementProgress::Complete(
            completion::capture(actor, identity.guid, key, incarnation, effect),
        ),
        StepProgress::Pending(StepPending::Path(request)) => {
            let (query, continuation) = request.into_parts();
            ActorMovementProgress::Pending(ActorMovementPending::Path(ActorPathRequest {
                query, continuation: ActorPathContinuation { identity, continuation },
            }))
        }
        StepProgress::Pending(StepPending::StaticHeight(request)) => {
            let (query, continuation) = request.into_parts();
            ActorMovementProgress::Pending(ActorMovementPending::StaticHeight(ActorStaticHeightRequest {
                query: ActorStaticHeightQuery { map_id: query.map_id, point: query.point, probe_z: query.probe_z },
                continuation: ActorStaticHeightContinuation { identity, continuation },
            }))
        }
        StepProgress::Pending(StepPending::GridHeight(request)) => {
            let (query, continuation) = request.into_parts();
            ActorMovementProgress::Pending(ActorMovementPending::GridHeight(ActorGridHeightRequest {
                query: ActorGridHeightQuery { map_id: query.map_id, point: query.point },
                continuation: ActorGridHeightContinuation { identity, continuation },
            }))
        }
    }
}
