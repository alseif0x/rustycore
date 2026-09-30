//! Explicit abandonment of I/O whose complete owned input is back with the caller.

use super::{CanonicalMovementError, CanonicalMovementOutcome, MovementFailure};
use super::super::{queries, SharedCanonicalMapManager, MapObjectTickContinuation, ObjectMapUpdateToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalMovementAbandonment {
    UnlaunchedRequestDiscarded,
    ObservedReplyDiscarded,
}

/// This is an abandoned movement pass, not a completed map or tick. Its
/// partial effects remain owned for the caller's explicit publication/fence
/// disposition; this method neither delivers nor finishes any remaining work.
pub struct CanonicalMovementAbandoned {
    pub partial: CanonicalMovementOutcome,
    pub abandonment: CanonicalMovementAbandonment,
}

impl CanonicalMovementError {
    pub fn dispose_owned_request(
        self,
        manager: &SharedCanonicalMapManager,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
    ) -> Result<CanonicalMovementAbandoned, Self> {
        match &self.failure {
            MovementFailure::MissingTerrain(_)
            | MovementFailure::Resume { .. }
            | MovementFailure::ResumePoisoned(_) => {}
            // No original continuation is owned here: neither a panic nor
            // an admission error supplies evidence that its slot is settled.
            MovementFailure::Actor(_) | MovementFailure::Poisoned
            | MovementFailure::MapId(_) | MovementFailure::QueryPanicked(_) => return Err(self),
        }
        let mut manager = match manager.lock() {
            Ok(manager) => manager,
            Err(_) => return Err(self),
        };
        let Self { partial, failure } = self;
        let (abandonment, failure) = match failure {
            MovementFailure::MissingTerrain(request) => {
                match manager.discard_movement_request(tick, token, request) {
                    Ok(()) => (CanonicalMovementAbandonment::UnlaunchedRequestDiscarded, None),
                    Err((_error, request)) => (CanonicalMovementAbandonment::UnlaunchedRequestDiscarded,
                        Some(MovementFailure::MissingTerrain(request))),
                }
            }
            MovementFailure::Resume { error, response } => {
                match queries::discard(&mut manager, tick, token, response) {
                    Ok(()) => (CanonicalMovementAbandonment::ObservedReplyDiscarded, None),
                    Err((_discard_error, response)) => (CanonicalMovementAbandonment::ObservedReplyDiscarded,
                        Some(MovementFailure::Resume { error, response })),
                }
            }
            MovementFailure::ResumePoisoned(response) => {
                match queries::discard(&mut manager, tick, token, response) {
                    Ok(()) => (CanonicalMovementAbandonment::ObservedReplyDiscarded, None),
                    Err((_error, response)) => (CanonicalMovementAbandonment::ObservedReplyDiscarded,
                        Some(MovementFailure::ResumePoisoned(response))),
                }
            }
            // Kept exhaustive so adding another error never makes it
            // implicitly disposable, even after acquiring a manager borrow.
            failure @ (MovementFailure::Actor(_) | MovementFailure::Poisoned
                | MovementFailure::MapId(_) | MovementFailure::QueryPanicked(_)) =>
                return Err(Self { partial, failure }),
        };
        match failure {
            Some(failure) => Err(Self { partial, failure }),
            None => Ok(CanonicalMovementAbandoned { partial, abandonment }),
        }
    }
}
