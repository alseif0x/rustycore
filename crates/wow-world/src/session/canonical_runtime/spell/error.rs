//! Owned requests/replies and prior publication inputs survive rejection.
use super::*;
use wow_map::{ActorSpellContinuation, ActorSpellLosRequest, ActorSpellPrepareFailure, ActorSpellResumeFailure,
    ActorSpellPublicationContinuation, ActorSpellPublicationFailure};

pub struct CanonicalSpellError {
    pub partial: LegacyCreatureSpellTickOutcomeLikeCpp,
    pub failure: CanonicalSpellFailure,
}
pub enum CanonicalSpellFailure {
    PreparePoisoned,
    Prepare(ActorSpellPrepareFailure),
    MissingTerrain(ActorSpellLosRequest),
    ResumePoisoned { continuation: ActorSpellContinuation, response: bool },
    Resume(ActorSpellResumeFailure),
    PublicationPoisoned(ActorSpellPublicationContinuation),
    Publication(ActorSpellPublicationFailure),
    QueryPanicked,
}
impl CanonicalSpellError {
    pub fn pending_effects(&self) -> Option<&SpellOutcome> {
        match &self.failure {
            CanonicalSpellFailure::MissingTerrain(request) => Some(request.partial()),
            CanonicalSpellFailure::ResumePoisoned { continuation, .. } => Some(continuation.partial()),
            CanonicalSpellFailure::Resume(failure) => Some(failure.continuation.partial()),
            CanonicalSpellFailure::PublicationPoisoned(continuation) => Some(continuation.partial()),
            CanonicalSpellFailure::Publication(failure) => Some(failure.continuation.partial()),
            _ => None,
        }
    }
}
impl std::fmt::Debug for CanonicalSpellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanonicalSpellError").finish_non_exhaustive()
    }
}
