//! Explicit disposal consumes owned I/O evidence, never a live actor or its tail.

use super::*;

impl MapManager {
    /// A request still owned by the caller has not been handed to a worker.
    /// Discarding it releases only its original actor-operation slot. Dropping
    /// it, or losing it inside a worker, does not authorize this operation.
    pub fn discard_movement_request(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        request: ActorMovementPending,
    ) -> Result<(), (ActorMovementError, ActorMovementPending)> {
        match self.discard_movement_identity(tick, token, request.identity()) {
            Ok(()) => Ok(()),
            Err(error) => Err((error, request)),
        }
    }

    /// The caller owns both the original continuation and the observed reply;
    /// a worker transporting that continuation is therefore no longer in flight.
    pub fn discard_movement_path_reply(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: ActorPathContinuation,
        response: Option<DetourPolyPath>,
    ) -> Result<(), ActorMovementResumeFailure<ActorPathContinuation, Option<DetourPolyPath>>> {
        match self.discard_movement_identity(tick, token, &continuation.identity) {
            Ok(()) => Ok(()),
            Err(error) => Err(ActorMovementResumeFailure {
                error,
                continuation,
                response,
            }),
        }
    }

    pub fn discard_movement_static_height_reply(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: ActorStaticHeightContinuation,
        response: f32,
    ) -> Result<(), ActorMovementResumeFailure<ActorStaticHeightContinuation, f32>> {
        match self.discard_movement_identity(tick, token, &continuation.identity) {
            Ok(()) => Ok(()),
            Err(error) => Err(ActorMovementResumeFailure {
                error,
                continuation,
                response,
            }),
        }
    }

    pub fn discard_movement_grid_height_reply(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: ActorGridHeightContinuation,
        response: f32,
    ) -> Result<(), ActorMovementResumeFailure<ActorGridHeightContinuation, f32>> {
        match self.discard_movement_identity(tick, token, &continuation.identity) {
            Ok(()) => Ok(()),
            Err(error) => Err(ActorMovementResumeFailure {
                error,
                continuation,
                response,
            }),
        }
    }

    fn discard_movement_identity(
        &self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        identity: &ActorStepIdentity,
    ) -> Result<(), ActorMovementError> {
        if !identity.matches_token(token) {
            return Err(ActorMovementError::OperationMismatch {
                guid: identity.guid,
            });
        }
        // This gate checks the original token and slot, deliberately without
        // requiring a current map incarnation or actor witness. Stale/ABA work
        // may be disposed, but may never be resumed against its replacement.
        self.complete_actor_operation(tick, token, identity.guid, &identity.witness)
            .map_err(Into::into)
    }
}
