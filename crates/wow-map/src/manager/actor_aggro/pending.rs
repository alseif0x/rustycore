//! Owned query/continuation transport. Drop deliberately leaves the slot busy.
use super::*;

pub enum ActorAggroProgress {
    Complete(AggroOutcome),
    Pending(ActorAggroLosRequest),
}

pub struct ActorAggroContinuation {
    pub(super) identity: ActorStepIdentity,
    pub(super) pending: AggroLosPending,
    pub(super) witnesses: HashMap<ObjectGuid, CreatureActorWitness>,
    pub(super) players: HashMap<ObjectGuid, (PlayerHandle, u64)>,
    pub(super) order: Vec<ObjectGuid>,
    pub(super) terrain_enabled: bool,
}

/// Endpoints are already collision-height/hit-sphere adjusted. No actor facts
/// or map guard are needed by the blocking worker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorAggroLosQuery { pub map_id: u32, pub from: Position, pub to: Position }

pub struct ActorAggroLosRequest { continuation: ActorAggroContinuation }

impl ActorAggroLosRequest {
    pub fn query(&self) -> ActorAggroLosQuery {
        ActorAggroLosQuery {
            map_id: u32::from(self.continuation.pending.map_id()),
            from: self.continuation.pending.from.position, to: self.continuation.pending.to.position,
        }
    }
    pub fn into_continuation(self) -> ActorAggroContinuation { self.continuation }

    /// The worker closure owns the request throughout I/O. A panic cannot
    /// return an owned continuation or imply that the map operation settled.
    pub fn resolve(self, terrain: &LiveTerrainHeights) -> (ActorAggroContinuation, bool) {
        let response = self.continuation.pending.resolve(terrain);
        (self.continuation, response)
    }
    pub fn partial(&self) -> &AggroOutcome { self.continuation.partial() }
}

impl ActorAggroContinuation {
    pub fn partial(&self) -> &AggroOutcome { self.pending.partial() }
}

pub(super) fn wrap(
    token: &mut ObjectMapUpdateToken, identity: ActorStepIdentity,
    witnesses: HashMap<ObjectGuid, CreatureActorWitness>, players: HashMap<ObjectGuid, (PlayerHandle, u64)>,
    order: Vec<ObjectGuid>, terrain_enabled: bool, progress: AggroTailProgress,
) -> ActorAggroProgress {
    match progress {
        AggroTailProgress::Complete(outcome) => {
            // No external code ran since the validated slot/identity gate.
            token.actor_operation = None;
            ActorAggroProgress::Complete(outcome)
        }
        AggroTailProgress::Pending(pending) => ActorAggroProgress::Pending(ActorAggroLosRequest {
            continuation: ActorAggroContinuation { identity, pending, witnesses, players, order, terrain_enabled },
        }),
    }
}
