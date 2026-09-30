use crate::map_manager::{PendingRespawn, WorldCreature};
use crate::spawn::{ActorRespawnPhaseOutcome, RespawnInfoLikeCpp};
use std::time::Instant;
use wow_core::ObjectGuid;
use wow_persistence::RespawnPersistenceMutationLikeCpp;

/// Values needed by the existing legacy factory/commit/publication adapter.
#[derive(Debug)]
pub struct LegacyCreatureRespawnPrefix {
    pub creatures_seen: usize,
    pub respawn_db_mutations: Vec<RespawnPersistenceMutationLikeCpp>,
    pub removed_corpses: Vec<RemovedCreatureCorpse>,
    pub ready: Vec<PendingRespawn>,
}

#[derive(Debug)]
pub struct RemovedCreatureCorpse {
    pub guid: ObjectGuid,
    pub respawn_info: Option<RespawnInfoLikeCpp>,
}

#[derive(Clone, Copy)]
pub(super) struct Clocks {
    pub(super) now: Instant,
    pub(super) conversion_now: Instant,
    pub(super) conversion_now_secs: i64,
}

#[derive(Default)]
pub(super) struct MemoryPrefix {
    pub(super) outcome: ActorRespawnPhaseOutcome,
    pub(super) removed_corpses: Vec<RemovedCreatureCorpse>,
    pub(super) ready: Vec<PendingRespawn>,
}

pub(super) struct CapturedCorpse {
    pub(super) pending: PendingRespawn,
    pub(super) seconds: i64,
    pub(super) info: Option<RespawnInfoLikeCpp>,
    // Preserve the removed legacy actor's lifetime through its corpse iteration.
    // Canonical keeps borrowing the attached original and always stores None.
    pub(super) _detached_actor: Option<WorldCreature>,
}
