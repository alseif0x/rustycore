//! Failure retains already-committed effects and rejected owned I/O inputs.

use super::{ActorMovementError, CanonicalMovementOutcome};
use super::queries::ResolvedMovementQuery;
use wow_map::ActorMovementPending;

#[path = "disposition.rs"]
mod disposition;
pub use disposition::{CanonicalMovementAbandoned, CanonicalMovementAbandonment};

#[cfg(test)]
#[path = "tests/disposition.rs"]
mod disposition_tests;

pub struct CanonicalMovementError {
    pub partial: CanonicalMovementOutcome,
    pub(super) failure: MovementFailure,
}

pub(super) enum MovementFailure {
    Actor(ActorMovementError),
    Poisoned,
    MapId(u32),
    QueryPanicked(tokio::task::JoinError),
    MissingTerrain(ActorMovementPending),
    Resume { error: ActorMovementError, response: ResolvedMovementQuery },
    ResumePoisoned(ResolvedMovementQuery),
}

impl CanonicalMovementError {
    pub(super) fn new(partial: CanonicalMovementOutcome, failure: MovementFailure) -> Self {
        Self { partial, failure }
    }

    pub fn actor_error(&self) -> Option<ActorMovementError> {
        match &self.failure {
            MovementFailure::Actor(error) | MovementFailure::Resume { error, .. } => Some(*error),
            _ => None,
        }
    }
}

impl std::fmt::Debug for CanonicalMovementError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("CanonicalMovementError")
            .field("failure", &self.to_string())
            .field("creatures_seen", &self.partial.creatures_seen)
            .field("movement_packets", &self.partial.movement_packets)
            .field("events", &self.partial.plan.events.len())
            .finish()
    }
}

impl std::fmt::Display for CanonicalMovementError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.failure {
            MovementFailure::Actor(error) => write!(formatter, "canonical movement admission rejected: {error:?}"),
            MovementFailure::Poisoned | MovementFailure::ResumePoisoned(_) => write!(formatter, "canonical movement manager poisoned"),
            MovementFailure::MapId(map_id) => write!(formatter, "canonical movement map ID {map_id} exceeds the represented packet map ID"),
            MovementFailure::QueryPanicked(error) => write!(formatter, "canonical movement query did not complete: {error}"),
            MovementFailure::MissingTerrain(_) => write!(formatter, "canonical movement height request has no terrain owner"),
            MovementFailure::Resume { error, .. } => write!(formatter, "canonical movement resume rejected: {error:?}"),
        }
    }
}

impl std::error::Error for CanonicalMovementError {}
