//! Failure preserves every owned input and any already-committed domain effects.
use super::*;
use wow_map::{ActorAggroContinuation, ActorAggroLosRequest, ActorAggroPrepareFailure, ActorAggroResumeFailure};

pub struct CanonicalAggroError {
    pub partial: LegacyCreatureAggroTickOutcomeLikeCpp,
    pub failure: CanonicalAggroFailure,
}

pub enum CanonicalAggroFailure {
    PreparePoisoned { candidates: Vec<AggroCandidate> },
    Prepare(ActorAggroPrepareFailure),
    MissingTerrain(ActorAggroLosRequest),
    ResumePoisoned { continuation: ActorAggroContinuation, response: bool },
    Resume(ActorAggroResumeFailure),
    QueryPanicked,
}

impl CanonicalAggroError {
    /// Effects inside an owned pending operation remain inspectable, without
    /// cloning or consuming its continuation. Publication is the caller's job.
    pub fn pending_effects(&self) -> Option<&AggroOutcome> {
        match &self.failure {
            CanonicalAggroFailure::MissingTerrain(request) => Some(request.partial()),
            CanonicalAggroFailure::ResumePoisoned { continuation, .. } => Some(continuation.partial()),
            CanonicalAggroFailure::Resume(failure) => Some(failure.continuation.partial()),
            _ => None,
        }
    }
}

impl std::fmt::Debug for CanonicalAggroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanonicalAggroError").finish_non_exhaustive()
    }
}
