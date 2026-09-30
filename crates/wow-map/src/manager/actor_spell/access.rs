//! Selected primary admission and identity-checked secondary Player access.
use super::*;
use super::super::ObjectMapTickError;

pub(super) struct SpellAdmission {
    pub identity: Option<ActorStepIdentity>, pub primaries: Vec<ObjectGuid>,
    pub witnesses: HashMap<ObjectGuid, CreatureActorWitness>,
    pub players: HashMap<ObjectGuid, (PlayerHandle, u64)>,
}
impl MapManager {
    pub(super) fn admit_spell(&mut self, tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken) -> Result<SpellAdmission, ActorSpellError> {
        self.require_current_actor_token(tick, token).map_err(ActorSpellError::Access)?;
        if let Some(operation) = &token.actor_operation {
            return Err(ActorSpellError::Access(ActorTickAccessError::Tick(
                ObjectMapTickError::ActorOperationInFlight { guid: operation.guid })));
        }
        if token.key().map_id > u32::from(u16::MAX) {
            return Err(ActorSpellError::MapIdOutOfRange { map_id: token.key().map_id });
        }
        let primaries = self.selected_actor_guids(tick, token).map_err(ActorSpellError::Access)?;
        let mut witnesses = HashMap::new();
        let mut players = HashMap::new();
        for guid in &primaries {
            let (target, witness) = self.with_selected_actor(tick, token, *guid, None,
                |actor| actor.creature.ai_ownership().combat_target).map_err(ActorSpellError::Access)?;
            witnesses.insert(*guid, witness);
            if let Some(target) = target.filter(|guid| guid.is_player())
                && self.maps.get(&token.key()).unwrap().map().get_typed_player(target).is_some()
            {
                let stamp = self.current_player_admission_like_cpp(target)
                    .ok_or(ActorSpellError::PlayerIdentityMismatch { guid: target })?;
                if self.player_active_residence_revision_like_cpp(stamp.0) != Some((token.key(), stamp.1)) {
                    return Err(ActorSpellError::PlayerIdentityMismatch { guid: target });
                }
                players.insert(target, stamp);
            }
        }
        let identity = match primaries.first() {
            Some(guid) => Some(ActorStepIdentity::new(token, *guid,
                self.begin_actor_operation(tick, token, *guid, None).map_err(ActorSpellError::Access)?)),
            None => None,
        };
        Ok(SpellAdmission { identity, primaries, witnesses, players })
    }

    pub(super) fn validate_spell_resume(&self, tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken, continuation: &ActorSpellContinuation) -> Result<(), ActorSpellError> {
        let (caster, victim) = continuation.pending.referenced_guids();
        self.validate_spell_frontier(tick, token, &continuation.identity,
            &continuation.witnesses, &continuation.players, victim)?;
        let map = self.maps.get(&token.key()).unwrap().map();
        continuation.pending.validate_live_cast(map.creature_actor(caster).unwrap(),
            map.get_typed_creature(caster).unwrap()).map_err(ActorSpellError::CastChanged)
    }

    pub(super) fn validate_spell_publication(&self, tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken, continuation: &ActorSpellPublicationContinuation) -> Result<(), ActorSpellError> {
        let (caster, victim) = continuation.pending.referenced_guids();
        self.validate_spell_frontier(tick, token, &continuation.identity,
            &continuation.witnesses, &continuation.players, victim)?;
        let map = self.maps.get(&token.key()).unwrap().map();
        continuation.pending.validate_live_cast(map.creature_actor(caster).unwrap(),
            map.get_typed_creature(caster).unwrap()).map_err(ActorSpellError::CastChanged)
    }

    fn validate_spell_frontier(&self, tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken, identity: &ActorStepIdentity,
        witnesses: &HashMap<ObjectGuid, CreatureActorWitness>,
        players: &HashMap<ObjectGuid, (PlayerHandle, u64)>, victim: ObjectGuid,
    ) -> Result<(), ActorSpellError> {
        if !identity.matches_token(token) {
            return Err(ActorSpellError::Access(ActorTickAccessError::OperationMismatch { guid: identity.guid }));
        }
        self.resume_actor_operation(tick, token, identity.guid, &identity.witness).map_err(ActorSpellError::Access)?;
        let map = self.maps.get(&token.key()).unwrap().map();
        for (guid, expected) in witnesses {
            if !map.creature_actor_witness(*guid).as_ref().is_some_and(|current| expected.same_actor(current)) {
                return Err(ActorSpellError::Access(ActorTickAccessError::WitnessMismatch { guid: *guid }));
            }
        }
        let expected = players.get(&victim);
        if expected.is_none() || expected.copied() != self.current_player_admission_like_cpp(victim)
            || map.get_typed_player(victim).is_none()
            || !expected.is_some_and(|(handle, revision)|
                self.player_active_residence_revision_like_cpp(*handle) == Some((token.key(), *revision)))
        { return Err(ActorSpellError::PlayerIdentityMismatch { guid: victim }); }
        if !map.get_typed_player(victim).unwrap().unit().is_alive() {
            return Err(ActorSpellError::CastChanged(SpellValidation::MissingTarget));
        }
        Ok(())
    }
}
