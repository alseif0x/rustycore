//! Owned LOS transport. No Drop settlement, actor snapshot or runtime clone.
use super::*;
use wow_entities::LineOfSightEndpoint;

pub enum ActorSpellProgress {
    Complete(SpellOutcome), Pending(ActorSpellLosRequest),
    Publication { completion: crate::map_manager::SpellCompletion,
        continuation: ActorSpellPublicationContinuation },
}
pub struct ActorSpellContinuation {
    pub(super) identity: ActorStepIdentity,
    pub(super) pending: SpellLosPending,
    pub(super) witnesses: HashMap<ObjectGuid, CreatureActorWitness>,
    pub(super) players: HashMap<ObjectGuid, (PlayerHandle, u64)>,
    pub(super) terrain_enabled: bool,
}
pub struct ActorSpellPublicationContinuation {
    pub(super) identity: ActorStepIdentity,
    pub(super) pending: SpellPublicationPending,
    pub(super) witnesses: HashMap<ObjectGuid, CreatureActorWitness>,
    pub(super) players: HashMap<ObjectGuid, (PlayerHandle, u64)>,
    pub(super) terrain_enabled: bool,
}
impl ActorSpellPublicationContinuation {
    pub fn partial(&self) -> &SpellOutcome { self.pending.partial() }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorSpellLosQuery {
    pub map_id: u32, pub instance_id: u32,
    pub from: LineOfSightEndpoint, pub to: LineOfSightEndpoint,
}
pub struct ActorSpellLosRequest { pub(super) continuation: ActorSpellContinuation }
impl ActorSpellLosRequest {
    pub fn query(&self) -> ActorSpellLosQuery {
        let key = self.continuation.pending.key();
        let (from, to) = self.continuation.pending.endpoints();
        ActorSpellLosQuery { map_id: key.map_id, instance_id: key.instance_id, from, to }
    }
    pub fn partial(&self) -> &SpellOutcome { self.continuation.partial() }
    pub fn into_continuation(self) -> ActorSpellContinuation { self.continuation }
    pub fn resolve(self, query: impl FnOnce(ActorSpellLosQuery) -> bool) -> (ActorSpellContinuation, bool) {
        let response = query(self.query());
        (self.continuation, response)
    }
}
impl ActorSpellContinuation {
    pub fn partial(&self) -> &SpellOutcome { self.pending.partial() }
}
pub(super) fn wrap(token: &mut ObjectMapUpdateToken, identity: ActorStepIdentity,
    witnesses: HashMap<ObjectGuid, CreatureActorWitness>, players: HashMap<ObjectGuid, (PlayerHandle, u64)>,
    terrain_enabled: bool, progress: SpellProgress) -> ActorSpellProgress {
    match progress {
        SpellProgress::Complete(outcome) => {
            // The current slot was validated before this synchronous operation.
            token.actor_operation = None;
            ActorSpellProgress::Complete(outcome)
        }
        SpellProgress::Pending(pending) => ActorSpellProgress::Pending(ActorSpellLosRequest {
            continuation: ActorSpellContinuation { identity, pending, witnesses, players, terrain_enabled },
        }),
        SpellProgress::Publication { completion, continuation: pending } => ActorSpellProgress::Publication {
            completion, continuation: ActorSpellPublicationContinuation {
                identity, pending, witnesses, players, terrain_enabled },
        },
    }
}
