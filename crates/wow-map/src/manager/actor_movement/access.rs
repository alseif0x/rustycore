//! Bind the owned movement inputs to the token owner's admission and slot gates.

use super::*;
use crate::map_manager::WorldCreature;

impl MapManager {
    pub(super) fn begin_actor_step(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        prepare: impl FnOnce(&mut WorldCreature, u32) -> StepProgress,
    ) -> Result<ActorMovementProgress, ActorTickAccessError> {
        let witness = self.begin_actor_operation(tick, token, guid, None)?;
        let identity = ActorStepIdentity::new(token, guid, witness.clone());
        let diff_ms = token.effective_diff_ms();
        let key = token.key();
        let incarnation = token.incarnation();
        let (progress, _witness) = self.with_selected_actor(
            tick, token, guid, Some(&witness),
            |actor| {
                let progress = prepare(actor, diff_ms);
                requests::wrap(identity, progress, actor, key, incarnation)
            },
        )?;
        if matches!(&progress, ActorMovementProgress::Complete(_)) {
            // The callback holds only the actor borrow: it cannot change the
            // manager admission or slot validated immediately before it ran.
            token.actor_operation = None;
        }
        Ok(progress)
    }

    pub(super) fn resume_actor_step<Input>(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        identity: ActorStepIdentity,
        input: Input,
        resume: impl FnOnce(&mut WorldCreature, Input) -> StepProgress,
    ) -> Result<ActorMovementProgress, (ActorTickAccessError, ActorStepIdentity, Input)> {
        if !identity.matches_token(token) {
            return Err((ActorTickAccessError::OperationMismatch { guid: identity.guid }, identity, input));
        }
        if let Err(error) = self.resume_actor_operation(
            tick, token, identity.guid, &identity.witness,
        ) {
            return Err((error, identity, input));
        }
        // Keep ownership of input until the already-validated actor borrow is
        // obtained. Every rejection returns the exact input, without invoking
        // a closure that captured and could otherwise discard its response.
        let Some(map) = self.maps.get_mut(&token.key()) else {
            return Err((ActorTickAccessError::ActorUnavailable { guid: identity.guid }, identity, input));
        };
        let Some(actor) = map.map_mut().creature_actor_mut(identity.guid) else {
            return Err((ActorTickAccessError::ActorUnavailable { guid: identity.guid }, identity, input));
        };
        let progress = resume(actor, input);
        let progress = requests::wrap(identity, progress, actor, token.key(), token.incarnation());
        if matches!(&progress, ActorMovementProgress::Complete(_)) {
            token.actor_operation = None;
        }
        Ok(progress)
    }
}
