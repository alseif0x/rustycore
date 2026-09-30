//! Scalar results of the prepared Actor lifecycle/respawn phase. No payload copy.
use super::*;
use wow_core::ObjectGuid;
use wow_persistence::RespawnPersistenceMutationLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorRespawnStatus {
    GuidOccupied,
    SpawnOccupied,
    Inserted,
    AdmissionRejected,
}

#[derive(Debug)]
pub struct ActorRespawnAttempt {
    pub key: RespawnKey,
    pub guid: ObjectGuid,
    pub status: ActorRespawnStatus,
}

#[derive(Debug, Default)]
pub struct ActorRespawnPhaseOutcome {
    pub creatures_seen: usize,
    pub corpses_removed: usize,
    pub removal_failures: usize,
    pub invalid_map_id: bool,
    pub attempts: Vec<ActorRespawnAttempt>,
    pub respawn_db_mutations: Vec<RespawnPersistenceMutationLikeCpp>,
}

