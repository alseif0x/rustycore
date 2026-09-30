//! Dormant generation-origin work over one actual post-session map token.
//!
//! This does not recover the kill identity from the current GUID-only queue.
//! No queue consumer, scheduling owner or production phase is connected here.
use super::CanonicalObjectWork;
use std::collections::HashMap;
use wow_core::ObjectGuid;
use wow_entities::{CreatureLoot, OwnedLootAuthority};
use wow_map::manager::{
    ActorTickAccessError, CreatureLootAccess, CreatureLootAccessError,
    CreatureLootActorHandle,
};
use wow_map::map_manager::CreatureLootObservation;
use wow_map::{MapManager, ObjectMapUpdateToken};

/// Owns the existing token and its reserved handle, never an Actor or a guard.
/// Dropping this value neither clears the operation nor finishes the map.
pub(crate) struct ReservedCreatureLootGeneration {
    token: ObjectMapUpdateToken,
    handle: CreatureLootActorHandle,
    source: CreatureLootObservation,
}

impl ReservedCreatureLootGeneration {
    pub(crate) fn source(&self) -> &CreatureLootObservation { &self.source }
}

impl CanonicalObjectWork {
    /// Reservation precedes all source facts. Rejection returns the same token;
    /// neither absence nor stale admission permits a legacy fallback here.
    pub(crate) fn begin_reserved_creature_loot(
        &self, manager: &mut MapManager, mut token: ObjectMapUpdateToken,
        guid: ObjectGuid,
    ) -> Result<ReservedCreatureLootGeneration, (CreatureLootAccessError, ObjectMapUpdateToken)> {
        match manager.begin_actor_loot_operation(&self.object_tick, &mut token, guid) {
            CreatureLootAccess::Ready(observation) => {
                let (handle, source) = observation.into_parts();
                Ok(ReservedCreatureLootGeneration { token, handle, source })
            }
            CreatureLootAccess::Rejected(error) => Err((error, token)),
            CreatureLootAccess::NoActor => Err((CreatureLootAccessError::Tick(
                ActorTickAccessError::ActorUnavailable { guid },
            ), token)),
        }
    }

    /// Call after the real generation request has returned its pools. The
    /// existing installer consumes these inputs, including on rejected install;
    /// all operation identity/source state stays owned by the caller for disposal.
    pub(crate) fn install_reserved_creature_loot(
        &self, manager: &mut MapManager, operation: &mut ReservedCreatureLootGeneration,
        authority: &OwnedLootAuthority, generation: u64,
        shared: Option<CreatureLoot>, personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> CreatureLootAccess<bool> {
        manager.install_reserved_actor_kill_loot(
            &self.object_tick, &mut operation.token, &operation.handle, authority,
            generation, operation.source.loot_lifecycle_revision(), shared, personal,
        )
    }

    /// Explicit acknowledgement that the request and its completion effects are
    /// settled. A timeout or dropped waiter alone is not this acknowledgement.
    pub(crate) fn settle_reserved_creature_loot(
        &self, manager: &MapManager, operation: ReservedCreatureLootGeneration,
    ) -> Result<ObjectMapUpdateToken, (CreatureLootAccessError, ReservedCreatureLootGeneration)> {
        self.complete_reserved_creature_loot(manager, operation)
    }

    /// Call only after disposing the real request/result, including joining any
    /// worker that can still act. Identity failure returns every original field.
    pub(crate) fn dispose_reserved_creature_loot(
        &self, manager: &MapManager, operation: ReservedCreatureLootGeneration,
    ) -> Result<ObjectMapUpdateToken, (CreatureLootAccessError, ReservedCreatureLootGeneration)> {
        self.complete_reserved_creature_loot(manager, operation)
    }

    fn complete_reserved_creature_loot(
        &self, manager: &MapManager, operation: ReservedCreatureLootGeneration,
    ) -> Result<ObjectMapUpdateToken, (CreatureLootAccessError, ReservedCreatureLootGeneration)> {
        let ReservedCreatureLootGeneration { mut token, handle, source } = operation;
        match manager.complete_reserved_actor_loot_operation(&self.object_tick, &mut token, handle) {
            Ok(()) => Ok(token),
            Err((error, handle)) => Err((error,
                ReservedCreatureLootGeneration { token, handle, source })),
        }
    }
}

#[cfg(test)]
mod tests;
