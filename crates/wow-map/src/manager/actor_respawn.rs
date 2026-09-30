//! One owned Respawn operation before ObjectUpdater, on the real admitted plan.
//! Only the existing Noop canonical runtime admits the resolved motor here.
use super::*;
use crate::map_manager::{PendingRespawn, WorldCreature};
use crate::spawn::{ActorRespawnPhaseOutcome, ActorRespawnStatus, RespawnKey};
use std::collections::VecDeque;

mod disposition;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorRespawnError {
    OriginMismatch,
    WrongEpoch,
    WrongState,
    Busy,
    ParticipantOrder,
    StaleParticipant,
    OperationMismatch,
    Disposed,
    ReservationLost,
}

/// Rejection always returns the same complete owned query/reply for recovery.
#[derive(Debug)]
pub struct ActorRespawnRejected<T> {
    pub error: ActorRespawnError,
    pub owned: T,
}

pub(super) struct RespawnOperation {
    identity: Arc<()>,
    epoch: u64,
    pub(super) participant: MapTickParticipantLikeCpp,
    disposed: bool,
}

impl RespawnOperation {
    pub(super) fn epoch(&self) -> u64 {
        self.epoch
    }
}

#[derive(Debug)]
struct RespawnContinuation {
    origin: Arc<()>,
    identity: Arc<()>,
    epoch: u64,
    participant: MapTickParticipantLikeCpp,
    remaining: VecDeque<PendingRespawn>,
    outcome: ActorRespawnPhaseOutcome,
}

#[derive(Debug)]
pub enum ActorRespawnProgress {
    Complete(ActorRespawnPhaseOutcome),
    Pending(ActorRespawnRequest),
}

/// Owned payload and batch remainder. Dropping/losing it never settles the slot.
#[derive(Debug)]
#[must_use]
pub struct ActorRespawnRequest {
    continuation: RespawnContinuation,
    pending: PendingRespawn,
}

#[derive(Debug)]
#[must_use]
pub struct ActorRespawnReply {
    continuation: RespawnContinuation,
    pending: PendingRespawn,
    actor: WorldCreature,
}

impl ActorRespawnRequest {
    pub fn key(&self) -> MapKey {
        self.continuation.participant.key
    }
    pub fn guid(&self) -> ObjectGuid {
        self.pending.create_data.guid
    }

    /// APP calls this only after releasing Map/entity guards. No live actor is
    /// borrowed: factory entropy and terrain I/O operate on this new owned motor.
    /// Both callbacks run once; resumption accepts the reply, never reruns them.
    pub fn resolve(
        self,
        factory: impl FnOnce(&PendingRespawn, u32) -> WorldCreature,
        snap: impl FnOnce(&mut WorldCreature, u16),
    ) -> ActorRespawnReply {
        let mut actor = factory(&self.pending, self.continuation.participant.key.instance_id);
        snap(&mut actor, self.pending.map_id);
        ActorRespawnReply {
            continuation: self.continuation,
            pending: self.pending,
            actor,
        }
    }
}

impl MapManager {
    pub(crate) fn actor_respawn_is_active(&self) -> bool {
        self.active_respawn.is_some()
    }

    /// Earn even an empty phase from the real plan; an empty arbitrary list
    /// must never bypass origin/state admission in APP.
    pub fn validate_actor_respawn_phase(
        &self,
        plan: &MapTickPlanLikeCpp,
    ) -> Result<(), ActorRespawnError> {
        self.validate_respawn_plan(plan)?;
        if self.active_respawn.is_some() {
            return Err(ActorRespawnError::Busy);
        }
        Ok(())
    }

    pub(super) fn respawn_ready_for_objects(&self, plan: &MapTickPlanLikeCpp) -> bool {
        self.active_respawn.is_none()
            && match self.respawn_cursor {
                Some((epoch, index)) if epoch == plan.epoch_like_cpp() => {
                    index == plan.updated_maps_like_cpp().len()
                }
                _ => true, // The compatibility path has not begun the dormant phase.
            }
    }

    fn validate_respawn_plan(&self, plan: &MapTickPlanLikeCpp) -> Result<(), ActorRespawnError> {
        if !self.owns_tick_plan(plan) {
            return Err(ActorRespawnError::OriginMismatch);
        }
        match self.tick_coordination_like_cpp {
            MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
                if epoch == plan.epoch_like_cpp() =>
            {
                Ok(())
            }
            MapTickCoordinationStateLikeCpp::AwaitingSessions(_) => {
                Err(ActorRespawnError::WrongEpoch)
            }
            _ => Err(ActorRespawnError::WrongState),
        }
    }

    fn require_respawn_participant(
        &self,
        plan: &MapTickPlanLikeCpp,
        participant: MapTickParticipantLikeCpp,
    ) -> Result<usize, ActorRespawnError> {
        self.validate_respawn_plan(plan)?;
        if self.active_respawn.is_some() {
            return Err(ActorRespawnError::Busy);
        }
        let index = match self.respawn_cursor {
            Some((epoch, index)) if epoch == plan.epoch_like_cpp() => index,
            _ => 0,
        };
        if plan.updated_maps_like_cpp().get(index) != Some(&participant) {
            return Err(ActorRespawnError::ParticipantOrder);
        }
        Ok(index)
    }

    /// Stale admission may be skipped before any prefix; it never satisfies a
    /// query or permits mutation of the replacement under the same MapKey.
    pub fn skip_stale_actor_respawn_map(
        &mut self,
        plan: &MapTickPlanLikeCpp,
        participant: MapTickParticipantLikeCpp,
    ) -> Result<(), ActorRespawnError> {
        let index = self.require_respawn_participant(plan, participant)?;
        if self.map_incarnation_like_cpp(participant.key) == Some(participant.incarnation)
            && self.maps.contains_key(&participant.key)
        {
            return Err(ActorRespawnError::ParticipantOrder);
        }
        self.respawn_cursor = Some((plan.epoch_like_cpp(), index + 1));
        Ok(())
    }

    /// Validate the admitted plan BEFORE saves/removals/draining. Install Busy
    /// before the prefix so even a panic cannot unlock another tick or transport.
    #[allow(clippy::too_many_arguments)]
    pub fn begin_actor_respawn_map(
        &mut self,
        plan: &MapTickPlanLikeCpp,
        participant: MapTickParticipantLikeCpp,
        now: Instant,
        conversion_now: Instant,
        conversion_now_secs: i64,
        persistent_world_map: bool,
    ) -> Result<ActorRespawnProgress, ActorRespawnError> {
        let index = self.require_respawn_participant(plan, participant)?;
        if self.map_incarnation_like_cpp(participant.key) != Some(participant.incarnation)
            || !self.maps.contains_key(&participant.key)
        {
            return Err(ActorRespawnError::StaleParticipant);
        }
        if self
            .maps
            .get(&participant.key)
            .expect("validated map")
            .map()
            .respawn_store_like_cpp()
            .has_reservations()
        {
            return Err(ActorRespawnError::Busy);
        }
        let identity = Arc::new(());
        self.active_respawn = Some(RespawnOperation {
            identity: Arc::clone(&identity),
            epoch: plan.epoch_like_cpp(),
            participant,
            disposed: false,
        });
        self.respawn_cursor = Some((plan.epoch_like_cpp(), index));
        let (outcome, remaining) = self
            .maps
            .get_mut(&participant.key)
            .expect("validated map")
            .map_mut()
            .prepare_actor_respawns(
                now,
                conversion_now,
                conversion_now_secs,
                persistent_world_map,
            );
        self.advance_actor_respawn(RespawnContinuation {
            origin: Arc::clone(&self.tick_origin),
            identity,
            epoch: plan.epoch_like_cpp(),
            participant,
            remaining,
            outcome,
        })
    }

    fn validate_respawn_slot(
        &self,
        continuation: &RespawnContinuation,
    ) -> Result<(), ActorRespawnError> {
        if !Arc::ptr_eq(&self.tick_origin, &continuation.origin) {
            return Err(ActorRespawnError::OriginMismatch);
        }
        let operation = self
            .active_respawn
            .as_ref()
            .ok_or(ActorRespawnError::OperationMismatch)?;
        if operation.epoch != continuation.epoch {
            return Err(ActorRespawnError::WrongEpoch);
        }
        if !Arc::ptr_eq(&operation.identity, &continuation.identity)
            || operation.participant != continuation.participant
        {
            return Err(ActorRespawnError::OperationMismatch);
        }
        if operation.disposed {
            return Err(ActorRespawnError::Disposed);
        }
        Ok(())
    }

    fn validate_respawn_identity(
        &self,
        continuation: &RespawnContinuation,
    ) -> Result<(), ActorRespawnError> {
        self.validate_respawn_slot(continuation)?;
        if self.tick_coordination_like_cpp
            != MapTickCoordinationStateLikeCpp::AwaitingSessions(continuation.epoch)
        {
            return Err(ActorRespawnError::WrongState);
        }
        Ok(())
    }

    fn validate_respawn_current(
        &self,
        continuation: &RespawnContinuation,
    ) -> Result<(), ActorRespawnError> {
        self.validate_respawn_identity(continuation)?;
        if self.map_incarnation_like_cpp(continuation.participant.key)
            != Some(continuation.participant.incarnation)
            || !self.maps.contains_key(&continuation.participant.key)
        {
            return Err(ActorRespawnError::StaleParticipant);
        }
        Ok(())
    }

    fn advance_actor_respawn(
        &mut self,
        mut continuation: RespawnContinuation,
    ) -> Result<ActorRespawnProgress, ActorRespawnError> {
        // Called only under an already-validated exclusive manager borrow.
        let map = self
            .maps
            .get_mut(&continuation.participant.key)
            .expect("current operation map")
            .map_mut();
        while let Some(pending) = continuation.remaining.pop_front() {
            if let Some(status) = map.initial_actor_respawn_guard(&pending) {
                map.settle_actor_respawn(&pending, status, &mut continuation.outcome);
            } else {
                return Ok(ActorRespawnProgress::Pending(ActorRespawnRequest {
                    continuation,
                    pending,
                }));
            }
        }
        let (_, index) = self
            .respawn_cursor
            .expect("operation has an admitted cursor");
        self.respawn_cursor = Some((continuation.epoch, index + 1));
        self.active_respawn = None;
        Ok(ActorRespawnProgress::Complete(continuation.outcome))
    }

    pub fn resume_actor_respawn(
        &mut self,
        plan: &MapTickPlanLikeCpp,
        reply: ActorRespawnReply,
    ) -> Result<ActorRespawnProgress, ActorRespawnRejected<ActorRespawnReply>> {
        let validation = self
            .validate_respawn_plan(plan)
            .and_then(|_| {
                if plan.epoch_like_cpp() == reply.continuation.epoch {
                    Ok(())
                } else {
                    Err(ActorRespawnError::WrongEpoch)
                }
            })
            .and_then(|_| self.validate_respawn_current(&reply.continuation));
        if let Err(error) = validation {
            return Err(ActorRespawnRejected {
                error,
                owned: reply,
            });
        }
        let map = self
            .maps
            .get_mut(&reply.continuation.participant.key)
            .expect("validated map")
            .map_mut();
        let key = RespawnKey::for_actor(&reply.pending);
        if !map.respawn_store_like_cpp().is_reserved(key)
            || reply.continuation.remaining.iter().any(|pending| {
                !map.respawn_store_like_cpp()
                    .is_reserved(RespawnKey::for_actor(pending))
            })
        {
            return Err(ActorRespawnRejected {
                error: ActorRespawnError::ReservationLost,
                owned: reply,
            });
        }
        let ActorRespawnReply {
            mut continuation,
            pending,
            actor,
        } = reply;
        // A newly occupied GUID/spawn is admission failure, not an initial
        // guard rejection: consume the attempt WITHOUT deleting its saved row.
        let status = if map.initial_actor_respawn_guard(&pending).is_some() {
            ActorRespawnStatus::AdmissionRejected
        } else if matches!(
            map.admit_fresh_creature_actor(actor),
            Ok(crate::map::FreshCreatureActorAdmission::Inserted { .. })
        ) {
            ActorRespawnStatus::Inserted
        } else {
            ActorRespawnStatus::AdmissionRejected
        };
        map.settle_actor_respawn(&pending, status, &mut continuation.outcome);
        Ok(self
            .advance_actor_respawn(continuation)
            .expect("same exclusive admitted operation"))
    }
}
