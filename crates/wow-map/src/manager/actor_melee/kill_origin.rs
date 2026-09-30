//! Original kill provenance captured by the synchronous selected melee motor.
//! No receiver lookup, request scheduling, continuation writer or Drop cleanup.
use super::{ActorTickAccessError, ObjectMapTickError, ObjectMapUpdateToken};
use crate::manager::tick_objects::ActorStepIdentity;
use crate::map::{CreatureActorWitness, CreatureMeleeSwingOutcome};
use crate::map_manager::CreatureLootObservation;
use crate::{Map, MapKey};
use wow_core::ObjectGuid;
use wow_entities::OwnedLootAuthority;

mod pending;
pub use pending::{
    MeleeKillPhaseError, MeleeLootError, PendingMeleeKills, PreparedMeleeKill, PreparedMeleeLoot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MeleeKillPhase {
    Split,
    Share,
    Primary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeleeKillCaptureError {
    NoActor,
    Reservation(ActorTickAccessError),
}

pub(crate) struct CapturedMeleeKill {
    target_guid: ObjectGuid,
    target_witness: CreatureActorWitness,
    source: CreatureLootObservation,
    authority: OwnedLootAuthority,
    object_generation: u64,
    health_revision: u64,
}
impl CapturedMeleeKill {
    pub(crate) fn target_guid(&self) -> ObjectGuid {
        self.target_guid
    }
    pub(crate) fn source(&self) -> &CreatureLootObservation {
        &self.source
    }
    pub(crate) fn authority(&self) -> &OwnedLootAuthority {
        &self.authority
    }
    pub(crate) fn object_generation(&self) -> u64 {
        self.object_generation
    }
    pub(crate) fn health_revision(&self) -> u64 {
        self.health_revision
    }
}

pub(crate) enum MeleeKillCapture {
    Captured(CapturedMeleeKill),
    Unavailable(MeleeKillCaptureError),
}
pub(crate) struct MeleeKillOccurrence {
    phase: MeleeKillPhase,
    capture: MeleeKillCapture,
}
impl MeleeKillOccurrence {
    pub(crate) fn phase(&self) -> MeleeKillPhase {
        self.phase
    }
    pub(crate) fn capture(&self) -> &MeleeKillCapture {
        &self.capture
    }
}

enum RootReservation {
    NotAttempted,
    Reserved,
    Rejected(ActorTickAccessError),
}
pub(crate) struct MeleeKillBatch {
    root_guid: ObjectGuid,
    root_witness: CreatureActorWitness,
    reservation: RootReservation,
    occurrences: Vec<MeleeKillOccurrence>,
}
impl MeleeKillBatch {
    pub(crate) fn root_guid(&self) -> ObjectGuid {
        self.root_guid
    }
    pub(crate) fn is_reserved(&self) -> bool {
        matches!(self.reservation, RootReservation::Reserved)
    }
    pub(crate) fn reservation_error(&self) -> Option<ActorTickAccessError> {
        match self.reservation {
            RootReservation::Rejected(error) => Some(error),
            _ => None,
        }
    }
    pub(crate) fn occurrences(&self) -> &[MeleeKillOccurrence] {
        &self.occurrences
    }
}

/// Owns the original token and full outcome, including partial capture failures.
/// There is deliberately no implicit settlement when this value is dropped.
pub struct SelectedMeleeExecution {
    outcome: CreatureMeleeSwingOutcome,
    token: ObjectMapUpdateToken,
    batch: MeleeKillBatch,
}
impl SelectedMeleeExecution {
    pub(super) fn new(
        outcome: CreatureMeleeSwingOutcome,
        token: ObjectMapUpdateToken,
        batch: MeleeKillBatch,
    ) -> Self {
        Self {
            outcome,
            token,
            batch,
        }
    }
    pub fn outcome(&self) -> &CreatureMeleeSwingOutcome {
        &self.outcome
    }
    pub(crate) fn batch(&self) -> &MeleeKillBatch {
        &self.batch
    }
    pub fn into_pending(self) -> PendingMeleeKills {
        PendingMeleeKills::new(self.outcome, self.token, self.batch)
    }
    pub(crate) fn into_parts(
        self,
    ) -> (
        CreatureMeleeSwingOutcome,
        ObjectMapUpdateToken,
        MeleeKillBatch,
    ) {
        (self.outcome, self.token, self.batch)
    }
}

/// The entry has already validated the token and selected root witness.
/// Only the token is borrowed; MapManager remains available to the old motor.
pub(crate) struct MeleeKillCollector<'token> {
    token: &'token mut ObjectMapUpdateToken,
    batch: MeleeKillBatch,
}
impl<'token> MeleeKillCollector<'token> {
    pub(crate) const SPLIT: MeleeKillPhase = MeleeKillPhase::Split;
    pub(crate) const SHARE: MeleeKillPhase = MeleeKillPhase::Share;
    pub(crate) const PRIMARY: MeleeKillPhase = MeleeKillPhase::Primary;
    pub(super) fn new(
        token: &'token mut ObjectMapUpdateToken,
        root_guid: ObjectGuid,
        root_witness: CreatureActorWitness,
    ) -> Self {
        Self {
            token,
            batch: MeleeKillBatch {
                root_guid,
                root_witness,
                reservation: RootReservation::NotAttempted,
                occurrences: Vec::new(),
            },
        }
    }

    fn reserve_once(&mut self, map: &Map, key: MapKey) {
        if !matches!(self.batch.reservation, RootReservation::NotAttempted) {
            return;
        }
        let rejection = if key != self.token.key() {
            Some(ActorTickAccessError::Tick(
                ObjectMapTickError::TokenMismatch { key },
            ))
        } else if let Some(operation) = &self.token.actor_operation {
            Some(ActorTickAccessError::Tick(
                ObjectMapTickError::ActorOperationInFlight {
                    guid: operation.guid,
                },
            ))
        } else {
            match map.creature_actor_witness(self.batch.root_guid) {
                None => Some(ActorTickAccessError::ActorUnavailable {
                    guid: self.batch.root_guid,
                }),
                Some(current) if !current.same_actor(&self.batch.root_witness) => {
                    Some(ActorTickAccessError::WitnessMismatch {
                        guid: self.batch.root_guid,
                    })
                }
                Some(_) => None,
            }
        };
        if let Some(error) = rejection {
            self.batch.reservation = RootReservation::Rejected(error);
        } else {
            self.token.actor_operation = Some(ActorStepIdentity::new(
                self.token,
                self.batch.root_guid,
                self.batch.root_witness.clone(),
            ));
            self.batch.reservation = RootReservation::Reserved;
        }
    }

    /// Called only after an actual killed branch, under the existing map guard.
    /// A damage TARGET need not belong to the root's frozen update selection.
    pub(crate) fn capture(
        &mut self,
        map: &Map,
        key: MapKey,
        phase: MeleeKillPhase,
        target_guid: ObjectGuid,
    ) {
        self.reserve_once(map, key);
        let capture = if let RootReservation::Rejected(error) = self.batch.reservation {
            MeleeKillCapture::Unavailable(MeleeKillCaptureError::Reservation(error))
        } else {
            match (
                map.creature_actor_witness(target_guid),
                map.creature_actor(target_guid),
            ) {
                (Some(target_witness), Some(actor)) => {
                    let source = actor.observe_loot();
                    let authority = actor.creature.loot_authority_like_cpp().clone();
                    let object_generation = authority.generation_like_cpp();
                    let health_revision = actor.creature.unit().health_state_revision_like_cpp();
                    MeleeKillCapture::Captured(CapturedMeleeKill {
                        target_guid,
                        target_witness,
                        source,
                        authority,
                        object_generation,
                        health_revision,
                    })
                }
                _ => MeleeKillCapture::Unavailable(MeleeKillCaptureError::NoActor),
            }
        };
        self.batch
            .occurrences
            .push(MeleeKillOccurrence { phase, capture });
    }

    pub(super) fn finish(self) -> MeleeKillBatch {
        self.batch
    }
}

#[cfg(test)]
mod tests;
