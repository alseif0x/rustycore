//! Preparation of the first original post-commit, before-hook melee death.
//! The original token remains pending. Preparation neither executes hooks nor
//! observes a final loot source, advances the ledger, or disposes the slot.
use super::{
    CapturedMeleeKill, MeleeKillBatch, MeleeKillCapture, MeleeKillCaptureError,
    MeleeKillOccurrence,
};
use crate::manager::{ActorTickAccessError, MapManager, MapObjectTickContinuation, ObjectMapUpdateToken};
use crate::map::CreatureMeleeSwingOutcome;
use crate::map_manager::CreatureLootObservation;
use wow_core::ObjectGuid;

mod loot;
pub use loot::{MeleeLootError, PreparedMeleeLoot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeleeKillPhaseError {
    NotReserved,
    NoOccurrence,
    Unavailable(MeleeKillCaptureError),
    RootStale(ActorTickAccessError),
    TargetUnavailable { guid: ObjectGuid },
    TargetActorChanged { guid: ObjectGuid },
    AuthorityConflict { guid: ObjectGuid },
    GenerationConflict { guid: ObjectGuid, captured: u64, current: u64 },
    HealthRevisionConflict { guid: ObjectGuid, captured: u64, current: u64 },
}

/// Owns the original execution's three values without copying their buffers.
/// Cursor zero is deliberately fixed until a separately contracted driver exists.
pub struct PendingMeleeKills {
    outcome: CreatureMeleeSwingOutcome,
    token: ObjectMapUpdateToken,
    batch: MeleeKillBatch,
    cursor: usize,
}
impl PendingMeleeKills {
    pub(super) fn new(outcome: CreatureMeleeSwingOutcome, token: ObjectMapUpdateToken,
        batch: MeleeKillBatch) -> Self {
        Self { outcome, token, batch, cursor: 0 }
    }
    pub fn outcome(&self) -> &CreatureMeleeSwingOutcome { &self.outcome }
    pub fn token(&self) -> &ObjectMapUpdateToken { &self.token }
    pub(crate) fn batch(&self) -> &MeleeKillBatch { &self.batch }
    pub fn root_guid(&self) -> ObjectGuid { self.batch.root_guid }
    pub fn occurrence_count(&self) -> usize { self.batch.occurrences.len() }
    fn first_capture(&self) -> &CapturedMeleeKill {
        match &self.batch.occurrences[self.cursor].capture {
            MeleeKillCapture::Captured(captured) => captured,
            MeleeKillCapture::Unavailable(_) => unreachable!("prepared capture is available"),
        }
    }
    pub fn cursor(&self) -> usize { self.cursor }
}

/// Read-only preparation evidence, not a hook receipt or completion authority.
pub struct PreparedMeleeKill { pending: PendingMeleeKills }
impl PreparedMeleeKill {
    pub fn pending(&self) -> &PendingMeleeKills { &self.pending }
    pub(crate) fn occurrence(&self) -> &MeleeKillOccurrence {
        &self.pending.batch.occurrences[self.pending.cursor]
    }
    pub(crate) fn captured(&self) -> &CapturedMeleeKill {
        match &self.occurrence().capture {
            MeleeKillCapture::Captured(captured) => captured,
            MeleeKillCapture::Unavailable(_) => unreachable!("preparation rejected unavailable capture"),
        }
    }
    /// This source was captured after the original damage commit, BEFORE hooks.
    /// It is not a final loot-generation source (C++ Unit::Kill generates loot
    /// before KillRewarder/procs/AI/scripts, not after all those hooks).
    pub fn before_hook_source(&self) -> &CreatureLootObservation {
        &self.captured().source
    }
    pub fn into_pending(self) -> PendingMeleeKills { self.pending }
}

impl MapManager {
    pub fn prepare_next_melee_kill(
        &self,
        tick: &MapObjectTickContinuation,
        mut pending: PendingMeleeKills,
    ) -> Result<PreparedMeleeKill, (MeleeKillPhaseError, PendingMeleeKills)> {
        let result = self.validate_current_melee_kill(tick, &mut pending);
        match result {
            Ok(()) => Ok(PreparedMeleeKill { pending }),
            Err(error) => Err((error, pending)),
        }
    }

    fn validate_current_melee_kill(&self, tick: &MapObjectTickContinuation,
        pending: &mut PendingMeleeKills) -> Result<(), MeleeKillPhaseError> {
        if !pending.batch.is_reserved() {
            return Err(MeleeKillPhaseError::NotReserved);
        }
        self.resume_actor_operation(tick, &mut pending.token,
            pending.batch.root_guid, &pending.batch.root_witness)
            .map_err(MeleeKillPhaseError::RootStale)?;
        let occurrence = pending.batch.occurrences.get(pending.cursor)
            .ok_or(MeleeKillPhaseError::NoOccurrence)?;
        let captured = match &occurrence.capture {
            MeleeKillCapture::Captured(captured) => captured,
            MeleeKillCapture::Unavailable(error) =>
                return Err(MeleeKillPhaseError::Unavailable(*error)),
        };
        // The same Map borrow covers all TARGET facts; TARGET is a damage
        // participant and need not belong to the frozen update selection.
        let map = self.maps.get(&pending.token.key())
            .expect("current root token retains its map").map();
        let guid = captured.target_guid;
        let witness = map.creature_actor_witness(guid)
            .ok_or(MeleeKillPhaseError::TargetUnavailable { guid })?;
        if !witness.same_actor(&captured.target_witness) {
            return Err(MeleeKillPhaseError::TargetActorChanged { guid });
        }
        let actor = map.creature_actor(guid)
            .ok_or(MeleeKillPhaseError::TargetUnavailable { guid })?;
        let authority = actor.creature.loot_authority_like_cpp();
        if !authority.shares_storage_like_cpp(&captured.authority) {
            return Err(MeleeKillPhaseError::AuthorityConflict { guid });
        }
        let generation = authority.generation_like_cpp();
        if generation != captured.object_generation {
            return Err(MeleeKillPhaseError::GenerationConflict {
                guid, captured: captured.object_generation, current: generation,
            });
        }
        let revision = actor.creature.unit().health_state_revision_like_cpp();
        if revision != captured.health_revision {
            return Err(MeleeKillPhaseError::HealthRevisionConflict {
                guid, captured: captured.health_revision, current: revision,
            });
        }
        Ok(())
    }

}

#[cfg(test)]
mod tests;
