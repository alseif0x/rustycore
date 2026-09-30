//! Revalidate before consuming any typed Step continuation or response.

use super::*;

impl MapManager {
    pub fn resume_movement_path(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: ActorPathContinuation,
        response: Option<DetourPolyPath>,
    ) -> Result<
        ActorMovementProgress,
        ActorMovementResumeFailure<ActorPathContinuation, Option<DetourPolyPath>>,
    > {
        let ActorPathContinuation {
            identity,
            continuation,
        } = continuation;
        match self.resume_actor_step(
            tick,
            token,
            identity,
            (continuation, response),
            |actor, (continuation, response)| continuation.resume(actor, response),
        ) {
            Ok(progress) => Ok(progress),
            Err((error, identity, (continuation, response))) => Err(ActorMovementResumeFailure {
                error: error.into(),
                continuation: ActorPathContinuation {
                    identity,
                    continuation,
                },
                response,
            }),
        }
    }

    pub fn resume_movement_static_height(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: ActorStaticHeightContinuation,
        response: f32,
    ) -> Result<ActorMovementProgress, ActorMovementResumeFailure<ActorStaticHeightContinuation, f32>>
    {
        let ActorStaticHeightContinuation {
            identity,
            continuation,
        } = continuation;
        match self.resume_actor_step(
            tick,
            token,
            identity,
            (continuation, response),
            |actor, (continuation, response)| continuation.resume(actor, response),
        ) {
            Ok(progress) => Ok(progress),
            Err((error, identity, (continuation, response))) => Err(ActorMovementResumeFailure {
                error: error.into(),
                continuation: ActorStaticHeightContinuation {
                    identity,
                    continuation,
                },
                response,
            }),
        }
    }

    pub fn resume_movement_grid_height(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: ActorGridHeightContinuation,
        response: f32,
    ) -> Result<ActorMovementProgress, ActorMovementResumeFailure<ActorGridHeightContinuation, f32>>
    {
        let ActorGridHeightContinuation {
            identity,
            continuation,
        } = continuation;
        match self.resume_actor_step(
            tick,
            token,
            identity,
            (continuation, response),
            |actor, (continuation, response)| continuation.resume(actor, response),
        ) {
            Ok(progress) => Ok(progress),
            Err((error, identity, (continuation, response))) => Err(ActorMovementResumeFailure {
                error: error.into(),
                continuation: ActorGridHeightContinuation {
                    identity,
                    continuation,
                },
                response,
            }),
        }
    }
}
