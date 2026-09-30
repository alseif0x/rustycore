//! Explicit disposal settles only the original operation slot, including stale/ABA work.
use super::*;
impl MapManager {
    pub fn discard_spell_request(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        request: ActorSpellLosRequest,
    ) -> Result<SpellOutcome, (ActorSpellError, ActorSpellLosRequest)> {
        if let Err(error) = self.discard_spell_identity(tick, token, &request.continuation.identity)
        {
            return Err((error, request));
        }
        Ok(request.continuation.pending.into_partial())
    }
    pub fn discard_spell_los_reply(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: ActorSpellContinuation,
        response: bool,
    ) -> Result<SpellOutcome, ActorSpellResumeFailure> {
        if let Err(error) = self.discard_spell_identity(tick, token, &continuation.identity) {
            return Err(ActorSpellResumeFailure {
                error,
                continuation,
                response,
            });
        }
        Ok(continuation.pending.into_partial())
    }
    pub fn discard_spell_publication(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: ActorSpellPublicationContinuation,
    ) -> Result<SpellOutcome, ActorSpellPublicationFailure> {
        if let Err(error) = self.discard_spell_identity(tick, token, &continuation.identity) {
            return Err(ActorSpellPublicationFailure {
                error,
                continuation,
            });
        }
        Ok(continuation.pending.into_partial())
    }
    fn discard_spell_identity(
        &self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        identity: &ActorStepIdentity,
    ) -> Result<(), ActorSpellError> {
        if !identity.matches_token(token) {
            return Err(ActorSpellError::Access(
                ActorTickAccessError::OperationMismatch {
                    guid: identity.guid,
                },
            ));
        }
        self.complete_actor_operation(tick, token, identity.guid, &identity.witness)
            .map_err(ActorSpellError::Access)
    }
}
