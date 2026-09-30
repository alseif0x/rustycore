//! Owned I/O evidence authorizes explicit abandonment, never implicit success.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalSpellAbandonment {
    UnlaunchedRequest,
    ObservedLosReply,
    /// START/GO were appended to the retained partial plan. This does not
    /// claim recipient delivery, completion of the Spell family or map tail.
    PublishedCast,
}
pub struct CanonicalSpellAbandoned {
    pub partial: LegacyCreatureSpellTickOutcomeLikeCpp,
    pub abandonment: CanonicalSpellAbandonment,
}
impl CanonicalSpellError {
    pub fn dispose_owned_request(self, manager: &SharedCanonicalMapManager,
        tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
    ) -> Result<CanonicalSpellAbandoned, Self> {
        let Self { mut partial, failure } = self;
        if !matches!(&failure, CanonicalSpellFailure::MissingTerrain(_)
            | CanonicalSpellFailure::ResumePoisoned { .. } | CanonicalSpellFailure::Resume(_)
            | CanonicalSpellFailure::PublicationPoisoned(_) | CanonicalSpellFailure::Publication(_))
        { return Err(Self { partial, failure }); }
        let Ok(mut guard) = manager.lock() else { return Err(Self { partial, failure }); };
        let (outcome, abandonment) = match failure {
            CanonicalSpellFailure::MissingTerrain(request) => match guard.discard_spell_request(tick, token, request) {
                Ok(outcome) => (outcome, CanonicalSpellAbandonment::UnlaunchedRequest),
                Err((_, request)) => return Err(Self { partial, failure: CanonicalSpellFailure::MissingTerrain(request) }),
            },
            CanonicalSpellFailure::ResumePoisoned { continuation, response } =>
                match guard.discard_spell_los_reply(tick, token, continuation, response) {
                    Ok(outcome) => (outcome, CanonicalSpellAbandonment::ObservedLosReply),
                    Err(failure) => return Err(Self { partial, failure: CanonicalSpellFailure::ResumePoisoned {
                        continuation: failure.continuation, response: failure.response } }),
                },
            CanonicalSpellFailure::Resume(failure) => {
                let error = failure.error;
                match guard.discard_spell_los_reply(tick, token, failure.continuation, failure.response) {
                    Ok(outcome) => (outcome, CanonicalSpellAbandonment::ObservedLosReply),
                    Err(mut failure) => {
                        // A rejected disposition retains the original failure,
                        // as well as its exact continuation/observed response.
                        failure.error = error;
                        return Err(Self { partial, failure: CanonicalSpellFailure::Resume(failure) });
                    }
                }
            }
            CanonicalSpellFailure::PublicationPoisoned(continuation) =>
                match guard.discard_spell_publication(tick, token, continuation) {
                    Ok(outcome) => (outcome, CanonicalSpellAbandonment::PublishedCast),
                    Err(failure) => return Err(Self { partial,
                        failure: CanonicalSpellFailure::PublicationPoisoned(failure.continuation) }),
                },
            CanonicalSpellFailure::Publication(failure) => {
                let error = failure.error;
                match guard.discard_spell_publication(tick, token, failure.continuation) {
                    Ok(outcome) => (outcome, CanonicalSpellAbandonment::PublishedCast),
                    Err(mut failure) => {
                        failure.error = error;
                        return Err(Self { partial, failure: CanonicalSpellFailure::Publication(failure) });
                    }
                }
            }
            failure => return Err(Self { partial, failure }),
        };
        drop(guard);
        packets::replace_outcome(&mut partial, outcome);
        Ok(CanonicalSpellAbandoned { partial, abandonment })
    }
}
