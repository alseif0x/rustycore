//! Dormant loot continuation over the token's existing actor-operation slot.
//! C++ Unit::Kill / Creature::AllLootRemovedFromCorpse ordering stays in the
//! shared motor. This child adds only Rust admission/resume/disposal gates.
use super::*;

impl MapManager {
    pub fn begin_actor_loot_operation(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
    ) -> CreatureLootAccess<CreatureLootActorObservation> {
        let witness = match self.begin_actor_operation(tick, token, guid, None) {
            Ok(witness) => witness,
            Err(error) => {
                return CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(error));
            }
        };
        // Reservation precedes all loot facts. No await or externally supplied
        // callback intervenes between these accesses to the same guarded map.
        let (source, _) = self
            .with_selected_actor(tick, token, guid, Some(&witness), |actor| {
                actor.observe_loot()
            })
            .expect("the just-reserved Actor remains admitted");
        CreatureLootAccess::Ready(CreatureLootActorObservation {
            handle: CreatureLootActorHandle {
                origin: Arc::clone(&self.tick_origin),
                key: token.key(),
                incarnation: token.incarnation(),
                guid,
                witness,
                reserved_epoch: Some(token.actor_loot_epoch()),
            },
            source,
        })
    }

    fn require_reserved_loot_identity(
        &self,
        token: &ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle,
    ) -> Result<(), CreatureLootAccessError> {
        if !Arc::ptr_eq(&self.tick_origin, &handle.origin) {
            return Err(CreatureLootAccessError::WrongOrigin);
        }
        if token.key() != handle.key || token.incarnation() != handle.incarnation {
            return Err(CreatureLootAccessError::StaleIncarnation);
        }
        if handle.reserved_epoch != Some(token.actor_loot_epoch()) {
            return Err(CreatureLootAccessError::Tick(
                ActorTickAccessError::OperationMismatch { guid: handle.guid },
            ));
        }
        Ok(())
    }

    fn with_reserved_actor_loot<R>(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle,
        apply: impl FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    ) -> CreatureLootAccess<R> {
        if let Err(error) = self.require_reserved_loot_identity(token, handle) {
            return CreatureLootAccess::Rejected(error);
        }
        if let Err(error) = self.resume_actor_operation(tick, token, handle.guid, &handle.witness) {
            return CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(error));
        }
        match self.with_selected_actor(tick, token, handle.guid, Some(&handle.witness), apply) {
            Ok((result, _)) => CreatureLootAccess::Ready(result),
            Err(error) => CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(error)),
        }
    }

    pub fn install_reserved_actor_kill_loot(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle,
        authority: &OwnedLootAuthority,
        generation: u64,
        lifetime: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> CreatureLootAccess<bool> {
        self.with_reserved_actor_loot(tick, token, handle, |actor| {
            actor.install_kill_loot(authority, generation, lifetime, shared, personal)
        })
    }

    pub fn force_reserved_actor_loot_flags(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle,
    ) -> CreatureLootAccess<UnitValuesUpdate> {
        self.with_reserved_actor_loot(tick, token, handle, |actor| actor.force_loot_flags())
    }

    pub fn release_reserved_actor_loot(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle,
        authority: &OwnedLootAuthority,
        generation: u64,
        revision: u64,
        fully_skinned: bool,
        decay_rate: f32,
        phase: CreatureLootReleasePhase,
    ) -> CreatureLootAccess<Option<CreatureLootReleaseOutcome>> {
        self.with_reserved_actor_loot(tick, token, handle, |actor| {
            actor.release_looted_corpse(
                authority,
                generation,
                revision,
                fully_skinned,
                decay_rate,
                phase,
            )
        })
    }

    /// Call only once work has settled or been disposed. This deliberately
    /// uses the original disposal gate, which also permits stale-map/ABA cleanup.
    pub fn complete_reserved_actor_loot_operation(
        &self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        handle: CreatureLootActorHandle,
    ) -> Result<(), (CreatureLootAccessError, CreatureLootActorHandle)> {
        if let Err(error) = self.require_reserved_loot_identity(token, &handle) {
            return Err((error, handle));
        }
        match self.complete_actor_operation(tick, token, handle.guid, &handle.witness) {
            Ok(()) => Ok(()),
            Err(error) => Err((CreatureLootAccessError::Tick(error), handle)),
        }
    }
}
#[cfg(test)]
mod tests;
