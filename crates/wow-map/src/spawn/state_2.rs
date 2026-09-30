//! Spawn object and group model state definitions, part 2 of 2.
//!
//! Separated from the spawn.rs root under #644. Behaviour is preserved.

use super::*;

impl Ord for RespawnQueueKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.respawn_time
            .cmp(&other.respawn_time)
            // Trinity's heap comparator makes larger spawn ids win equal-time ties.
            .then_with(|| other.spawn_id.cmp(&self.spawn_id))
            // Same spawn id can exist for different types; C++ then orders larger type first.
            .then_with(|| other.object_type.cmp(&self.object_type))
    }
}

impl PartialOrd for RespawnQueueKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

mod actors;
mod saved;
mod phase;
pub use phase::{ActorRespawnAttempt, ActorRespawnPhaseOutcome, ActorRespawnStatus};
mod transport;
mod reservations;
pub use reservations::RespawnReserved;
pub use actors::OwnedRespawn;
pub use transport::{RespawnTransfer, RespawnTransferError};
#[cfg(test)]
mod tests;

use crate::map_manager::{PendingRespawn, PersistedRespawnRowLikeCpp};
use std::time::Instant;

/// Persistent identities and transient GUID-low identities never alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RespawnKey {
    Persistent(SpawnObjectType, SpawnId),
    TransientCreature(u64),
}

#[derive(Debug)]
enum RespawnSlot {
    SavedOnly(PersistedRespawnRowLikeCpp),
    QueuedCatalog {
        info: RespawnInfoLikeCpp,
        row: Option<PersistedRespawnRowLikeCpp>,
    },
    QueuedActor {
        // Captured runtime metadata is moved, never reconstructed from INFO.
        // Its exact Instant is retained inside this immutable owned payload.
        payload: Box<PendingRespawn>,
        info: Option<RespawnInfoLikeCpp>,
        row: Option<PersistedRespawnRowLikeCpp>,
    },
}

impl RespawnSlot {
    fn info(&self) -> Option<&RespawnInfoLikeCpp> {
        match self {
            Self::SavedOnly(_) => None,
            Self::QueuedCatalog { info, .. } => Some(info),
            Self::QueuedActor { info, .. } => info.as_ref(),
        }
    }

    fn row(&self) -> Option<&PersistedRespawnRowLikeCpp> {
        match self {
            Self::SavedOnly(row) => Some(row),
            Self::QueuedCatalog { row, .. } | Self::QueuedActor { row, .. } => row.as_ref(),
        }
    }
}

/// One slot and one executor per identity. Both temporary production owners
/// (legacy MapInstance and canonical Map) use this definition; production has
/// not become single-authority until quiescent transport retires the legacy rail.
#[derive(Debug, Default)]
pub struct RespawnStoreLikeCpp {
    slots: BTreeMap<RespawnKey, RespawnSlot>,
    // Derived indexes contain keys only, never payloads or second mutable rows.
    pub(super) respawn_times: BTreeSet<RespawnQueueKey>,
    // Position is the ordinal; replacement removes then appends, like the old Vec.
    actor_order: Vec<RespawnKey>,
    // Operational keys only: payloads move into one owned continuation.
    reserved: BTreeSet<RespawnKey>,
}

impl RespawnStoreLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_respawn_info_like_cpp(
        &mut self,
        info: RespawnInfoLikeCpp,
    ) -> AddRespawnInfoOutcomeLikeCpp {
        if info.spawn_id == 0 {
            return AddRespawnInfoOutcomeLikeCpp::RejectedZeroSpawnId;
        }
        if !Self::has_respawn_map_like_cpp(info.object_type) {
            return AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType;
        }
        assert!(!self.is_reserved(RespawnKey::Persistent(info.object_type, info.spawn_id)),
            "use try_add_info for a reserved respawn key");

        let existing = self
            .get_respawn_info_like_cpp(info.object_type, info.spawn_id)
            .cloned();
        let replaced_existing = if let Some(existing) = existing {
            if info.respawn_time <= existing.respawn_time {
                self.respawn_times.remove(&RespawnQueueKey::from_info(&existing));
                true
            } else {
                return AddRespawnInfoOutcomeLikeCpp::RejectedExistingSoonerOrEqual;
            }
        } else {
            false
        };

        self.respawn_times.insert(RespawnQueueKey::from_info(&info));
        let key = RespawnKey::Persistent(info.object_type, info.spawn_id);
        match self.slots.remove(&key) {
            Some(RespawnSlot::QueuedActor { payload, row, .. }) => {
                self.slots.insert(key, RespawnSlot::QueuedActor {
                    payload, row, info: Some(info),
                });
            }
            previous => {
                let row = previous.and_then(|slot| slot.row().copied());
                // Ordinary addInfo is an explicit Catalog queue request, even
                // when a JUST_DIED SavedOnly row already exists.
                self.slots.insert(key, RespawnSlot::QueuedCatalog { info, row });
            }
        }

        if replaced_existing {
            AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
        } else {
            AddRespawnInfoOutcomeLikeCpp::Inserted
        }
    }

    pub fn get_respawn_time_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> i64 {
        self.get_respawn_info_like_cpp(object_type, spawn_id)
            .map_or(0, |info| info.respawn_time)
    }

    pub fn get_respawn_info_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<&RespawnInfoLikeCpp> {
        self.slots.get(&RespawnKey::Persistent(object_type, spawn_id))
            .and_then(RespawnSlot::info)
    }

    pub fn remove_respawn_time_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<RespawnInfoLikeCpp> {
        let key = RespawnKey::Persistent(object_type, spawn_id);
        if self.is_reserved(key) { return None; }
        let info = self.slots.get(&key)?.info()?.clone();
        match self.slots.remove(&key)? {
            RespawnSlot::QueuedActor { payload, row, .. } => {
                // Compatibility INFO deletion must not cancel the Actor rail.
                self.slots.insert(key, RespawnSlot::QueuedActor { payload, row, info: None });
            }
            _ => {}
        }
        self.respawn_times
            .remove(&RespawnQueueKey::from_info(&info));
        Some(info)
    }

    pub fn unload_all_respawn_infos_like_cpp(&mut self) {
        let keys: Vec<_> = self.respawn_timer_keys_like_cpp().collect();
        for (object_type, spawn_id) in keys {
            self.remove_respawn_time_like_cpp(object_type, spawn_id);
        }
    }

    pub fn respawn_timer_keys_like_cpp(
        &self,
    ) -> impl Iterator<Item = (SpawnObjectType, SpawnId)> + '_ {
        self.respawn_times
            .iter()
            .map(|key| (key.object_type, key.spawn_id))
    }

    /// Filter tags before the Unix due-head cutoff: Actor INFO remains visible
    /// to guards/grids, but its executor uses the captured Instant and ordinal.
    pub fn catalog_timer_keys(&self) -> impl Iterator<Item = (SpawnObjectType, SpawnId)> + '_ {
        self.respawn_times.iter().filter(|key| matches!(
            self.slots.get(&RespawnKey::Persistent(key.object_type, key.spawn_id)),
            Some(RespawnSlot::QueuedCatalog { .. })
        )).map(|key| (key.object_type, key.spawn_id))
    }

    pub fn process_due_respawns_like_cpp(
        &mut self,
        now: i64,
        mut is_part_of_pool: impl FnMut(SpawnObjectType, SpawnId) -> Option<u32>,
        mut check_respawn: impl FnMut(&mut RespawnInfoLikeCpp) -> CheckRespawnOutcomeLikeCpp,
    ) -> Vec<ProcessRespawnActionLikeCpp> {
        let mut actions = Vec::new();

        loop {
            let Some((object_type, spawn_id)) = self.catalog_timer_keys().next() else { break; };
            let next_key = RespawnQueueKey::from_info(
                self.get_respawn_info_like_cpp(object_type, spawn_id).expect("indexed Catalog INFO")
            );
            if now < next_key.respawn_time {
                break;
            }

            let Some(mut info) =
                self.remove_respawn_time_like_cpp(next_key.object_type, next_key.spawn_id)
            else {
                self.respawn_times.remove(&next_key);
                continue;
            };

            if let Some(pool_id) = is_part_of_pool(info.object_type, info.spawn_id) {
                actions.push(ProcessRespawnActionLikeCpp::UpdatePool {
                    pool_id,
                    object_type: info.object_type,
                    spawn_id: info.spawn_id,
                });
                continue;
            }

            match check_respawn(&mut info) {
                CheckRespawnOutcomeLikeCpp::Allowed => {
                    actions.push(ProcessRespawnActionLikeCpp::DoRespawn {
                        object_type: info.object_type,
                        spawn_id: info.spawn_id,
                        grid_id: info.grid_id,
                    });
                }
                CheckRespawnOutcomeLikeCpp::Blocked if info.respawn_time == 0 => {
                    actions.push(ProcessRespawnActionLikeCpp::DeleteRespawn {
                        object_type: info.object_type,
                        spawn_id: info.spawn_id,
                    });
                }
                CheckRespawnOutcomeLikeCpp::Blocked if now < info.respawn_time => {
                    let stored_info = info.clone();
                    let outcome = self.add_respawn_info_like_cpp(info);
                    debug_assert!(matches!(outcome, AddRespawnInfoOutcomeLikeCpp::Inserted));
                    actions
                        .push(ProcessRespawnActionLikeCpp::RescheduleAndSave { info: stored_info });
                }
                CheckRespawnOutcomeLikeCpp::Blocked => {
                    let stored_info = info.clone();
                    let outcome = self.add_respawn_info_like_cpp(info);
                    debug_assert!(matches!(outcome, AddRespawnInfoOutcomeLikeCpp::Inserted));
                    actions.push(ProcessRespawnActionLikeCpp::InvalidRescheduleNotFuture {
                        info: stored_info,
                    });
                    break;
                }
            }
        }

        actions
    }

    pub(super) const fn has_respawn_map_like_cpp(object_type: SpawnObjectType) -> bool {
        matches!(
            object_type,
            SpawnObjectType::Creature | SpawnObjectType::GameObject
        )
    }

}
