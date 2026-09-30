//! Admission and secondary identity checks remain within the map owner.
use super::super::ObjectMapTickError;
use super::*;

pub(super) struct AggroAdmission {
    pub identity: Option<ActorStepIdentity>,
    pub primaries: Vec<ObjectGuid>,
    pub witnesses: HashMap<ObjectGuid, CreatureActorWitness>,
    pub order: Vec<ObjectGuid>,
    pub players: HashMap<ObjectGuid, (PlayerHandle, u64)>,
}

impl MapManager {
    pub(super) fn admit_aggro(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        candidates: &[AggroCandidate],
    ) -> Result<AggroAdmission, ActorAggroError> {
        self.require_current_actor_token(tick, token)
            .map_err(ActorAggroError::Access)?;
        if let Some(operation) = &token.actor_operation {
            return Err(ActorAggroError::Access(ActorTickAccessError::Tick(
                ObjectMapTickError::ActorOperationInFlight {
                    guid: operation.guid,
                },
            )));
        }
        if token.key().map_id > u32::from(u16::MAX) {
            return Err(ActorAggroError::MapIdOutOfRange {
                map_id: token.key().map_id,
            });
        }
        let primaries = self
            .selected_actor_guids(tick, token)
            .map_err(ActorAggroError::Access)?;
        for guid in &primaries {
            self.with_selected_actor(tick, token, *guid, None, |_| ())
                .map_err(ActorAggroError::Access)?;
        }
        let mut players = HashMap::new();
        for candidate in candidates.iter().filter(|candidate| {
            u32::from(candidate.map_id) == token.key().map_id
                && candidate.instance_id == token.key().instance_id
        }) {
            if self
                .maps
                .get(&token.key())
                .unwrap()
                .map()
                .get_typed_player(candidate.player_guid)
                .is_some()
            {
                let stamp = self
                    .current_player_admission_like_cpp(candidate.player_guid)
                    .ok_or(ActorAggroError::PlayerIdentityMismatch {
                        guid: candidate.player_guid,
                    })?;
                if self.player_active_residence_revision_like_cpp(stamp.0)
                    != Some((token.key(), stamp.1))
                {
                    return Err(ActorAggroError::PlayerIdentityMismatch {
                        guid: candidate.player_guid,
                    });
                }
                players.insert(candidate.player_guid, stamp);
            }
        }
        let secondary = self
            .maps
            .get(&token.key())
            .unwrap()
            .runtime
            .aggro_actor_witnesses();
        let order = secondary.iter().map(|(guid, _)| *guid).collect();
        let witnesses = secondary.into_iter().collect();
        let identity = match primaries.first() {
            Some(guid) => {
                let witness = self
                    .begin_actor_operation(tick, token, *guid, None)
                    .map_err(ActorAggroError::Access)?;
                Some(ActorStepIdentity::new(token, *guid, witness))
            }
            None => None,
        };
        Ok(AggroAdmission {
            identity,
            primaries,
            witnesses,
            order,
            players,
        })
    }

    pub(super) fn validate_aggro_resume(
        &self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        continuation: &ActorAggroContinuation,
    ) -> Result<(), ActorAggroError> {
        let identity = &continuation.identity;
        if !identity.matches_token(token) {
            return Err(ActorAggroError::Access(
                ActorTickAccessError::OperationMismatch {
                    guid: identity.guid,
                },
            ));
        }
        self.resume_actor_operation(tick, token, identity.guid, &identity.witness)
            .map_err(ActorAggroError::Access)?;
        let map = self.maps.get(&token.key()).unwrap().map();
        for guid in continuation.pending.referenced_guids() {
            if guid.is_player() {
                let expected = continuation.players.get(&guid);
                if expected.is_none()
                    || expected.copied() != self.current_player_admission_like_cpp(guid)
                    || map.get_typed_player(guid).is_none()
                    || !expected.is_some_and(|(handle, revision)| {
                        self.player_active_residence_revision_like_cpp(*handle)
                            == Some((token.key(), *revision))
                    })
                {
                    return Err(ActorAggroError::PlayerIdentityMismatch { guid });
                }
            } else {
                let expected = continuation.witnesses.get(&guid);
                let current = map.creature_actor_witness(guid);
                if !expected
                    .zip(current.as_ref())
                    .is_some_and(|(expected, current)| expected.same_actor(current))
                {
                    return Err(ActorAggroError::Access(
                        ActorTickAccessError::WitnessMismatch { guid },
                    ));
                }
            }
        }
        Ok(())
    }
}
