//! Explicit ownership disposal is fail-stop, never successful phase completion.
use super::*;

impl MapManager {
    fn dispose_respawn_owned(
        &mut self,
        continuation: &RespawnContinuation,
        pending: &PendingRespawn,
    ) -> Result<(), ActorRespawnError> {
        self.validate_respawn_slot(continuation)?;
        // A stale incarnation can be disposed without touching its replacement.
        if self.map_incarnation_like_cpp(continuation.participant.key)
            == Some(continuation.participant.incarnation)
        {
            let map = self
                .maps
                .get_mut(&continuation.participant.key)
                .ok_or(ActorRespawnError::StaleParticipant)?
                .map_mut();
            let keys: Vec<_> = std::iter::once(pending)
                .chain(continuation.remaining.iter())
                .map(RespawnKey::for_actor)
                .collect();
            if keys
                .iter()
                .any(|key| !map.respawn_store_like_cpp().is_reserved(*key))
            {
                return Err(ActorRespawnError::ReservationLost);
            }
            for key in keys {
                map.respawn_store_like_cpp_mut().release_respawn_key(key);
            }
        }
        // Keep the manager operation: dropping/disposing work cannot authorize
        // can_resume, abandon, tail, transport or destruction after a partial prefix.
        self.active_respawn
            .as_mut()
            .expect("validated operation")
            .disposed = true;
        Ok(())
    }

    pub fn dispose_actor_respawn_request(
        &mut self,
        request: ActorRespawnRequest,
    ) -> Result<ActorRespawnPhaseOutcome, ActorRespawnRejected<ActorRespawnRequest>> {
        match self.dispose_respawn_owned(&request.continuation, &request.pending) {
            Ok(()) => Ok(request.continuation.outcome),
            Err(error) => Err(ActorRespawnRejected {
                error,
                owned: request,
            }),
        }
    }

    /// Return already-produced persistence evidence for explicit recovery.
    /// This is not Complete: the disposed manager slot remains fail-stop.
    pub fn dispose_actor_respawn_reply(
        &mut self,
        reply: ActorRespawnReply,
    ) -> Result<ActorRespawnPhaseOutcome, ActorRespawnRejected<ActorRespawnReply>> {
        match self.dispose_respawn_owned(&reply.continuation, &reply.pending) {
            Ok(()) => Ok(reply.continuation.outcome),
            Err(error) => Err(ActorRespawnRejected {
                error,
                owned: reply,
            }),
        }
    }
}
