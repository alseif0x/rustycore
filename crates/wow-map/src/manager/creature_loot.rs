//! Concrete Actor loot access; no owner transport or runtime producer is activated.
use super::{ActorTickAccessError, MapManager, MapObjectTickContinuation,
    MapTickCoordinationStateLikeCpp, ObjectMapUpdateToken};
use crate::map::CreatureActorWitness;
use crate::map_manager::{CreatureLootObservation, CreatureLootReleaseOutcome, CreatureLootReleasePhase};
use crate::MapKey;
use std::{collections::HashMap, sync::Arc};
use wow_core::ObjectGuid;
use wow_entities::{CreatureLoot, OwnedLootAuthority, UnitValuesUpdate};

mod reserved;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureLootAccessError {
    WrongOrigin, MissingMap, StaleIncarnation, WitnessMismatch, Busy,
    PendingActorOperation, ActorPresent, Tick(ActorTickAccessError),
}

/// NoActor is the ONLY result that permits the existing compatibility owner.
#[derive(Debug)]
pub enum CreatureLootAccess<T> { NoActor, Rejected(CreatureLootAccessError), Ready(T) }

/// Existing map origin/incarnation and Actor witness, never the Actor or its runtime.
#[derive(Debug)]
pub struct CreatureLootActorHandle {
    origin: Arc<()>, key: MapKey, incarnation: u64, guid: ObjectGuid,
    witness: CreatureActorWitness,
    // Immutable provenance only; the actual operation slot remains on the token.
    reserved_epoch: Option<u64>,
}
impl CreatureLootActorHandle {
    pub fn key(&self) -> MapKey { self.key }
    pub fn guid(&self) -> ObjectGuid { self.guid }
}

#[derive(Debug)]
pub struct CreatureLootActorObservation {
    handle: CreatureLootActorHandle,
    source: CreatureLootObservation,
}
impl CreatureLootActorObservation {
    pub fn into_parts(self) -> (CreatureLootActorHandle, CreatureLootObservation) {
        (self.handle, self.source)
    }
}

impl MapManager {
    /// Presence classification reads only the private entry identity. Loot facts
    /// are read only AFTER Idle/respawn-slot validation.
    pub fn observe_idle_creature_loot(&self, key: MapKey, guid: ObjectGuid)
        -> CreatureLootAccess<CreatureLootActorObservation>
    {
        match self.idle_creature_loot_handle(key, guid) {
            CreatureLootAccess::NoActor => CreatureLootAccess::NoActor,
            CreatureLootAccess::Rejected(error) => CreatureLootAccess::Rejected(error),
            CreatureLootAccess::Ready(handle) => {
                let source = self.maps.get(&key).expect("validated map").map()
                    .observe_loot_actor(guid).expect("the same guard retains the Actor");
                CreatureLootAccess::Ready(CreatureLootActorObservation { handle, source })
            }
        }
    }

    pub fn idle_creature_loot_handle(&self, key: MapKey, guid: ObjectGuid)
        -> CreatureLootAccess<CreatureLootActorHandle>
    {
        let Some(map) = self.maps.get(&key) else { return CreatureLootAccess::NoActor; };
        let Some(witness) = map.map().creature_actor_witness(guid) else { return CreatureLootAccess::NoActor; };
        if self.tick_coordination_like_cpp() != MapTickCoordinationStateLikeCpp::Idle
            || self.active_respawn.is_some()
        { return CreatureLootAccess::Rejected(CreatureLootAccessError::Busy); }
        let Some(incarnation) = self.map_incarnation_like_cpp(key) else {
            return CreatureLootAccess::Rejected(CreatureLootAccessError::MissingMap);
        };
        CreatureLootAccess::Ready(CreatureLootActorHandle {
            origin: Arc::clone(&self.tick_origin), key, incarnation, guid, witness,
            reserved_epoch: None,
        })
    }

    pub fn observe_tick_creature_loot(
        &mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
    ) -> CreatureLootAccess<CreatureLootActorObservation> {
        if let Err(error) = self.require_current_actor_token(tick, token) {
            return CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(error));
        }
        if token.actor_operation.is_some() {
            return CreatureLootAccess::Rejected(CreatureLootAccessError::PendingActorOperation);
        }
        let map = self.maps.get(&token.key()).expect("current token retains its map").map();
        let admitted = token.continuation.actor_workset(map).iter()
            .any(|(selected, _)| *selected == guid);
        if map.creature_actor(guid).is_none() {
            if admitted {
                return CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
                    ActorTickAccessError::ActorUnavailable { guid },
                ));
            }
            return CreatureLootAccess::NoActor;
        }
        match self.with_selected_actor(tick, token, guid, None, |actor| actor.observe_loot()) {
            Ok((source, witness)) => CreatureLootAccess::Ready(CreatureLootActorObservation {
                handle: CreatureLootActorHandle { origin: Arc::clone(&self.tick_origin),
                    key: token.key(), incarnation: token.incarnation(), guid, witness,
                    reserved_epoch: None },
                source,
            }),
            Err(error) => CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(error)),
        }
    }

    fn require_idle_loot_handle(&self, handle: &CreatureLootActorHandle)
        -> Result<(), CreatureLootAccessError>
    {
        if !Arc::ptr_eq(&self.tick_origin, &handle.origin) { return Err(CreatureLootAccessError::WrongOrigin); }
        if self.tick_coordination_like_cpp() != MapTickCoordinationStateLikeCpp::Idle
            || self.active_respawn.is_some()
        { return Err(CreatureLootAccessError::Busy); }
        if self.map_incarnation_like_cpp(handle.key) != Some(handle.incarnation) {
            return Err(CreatureLootAccessError::StaleIncarnation);
        }
        let map = self.maps.get(&handle.key).ok_or(CreatureLootAccessError::MissingMap)?;
        let current = map.map().creature_actor_witness(handle.guid)
            .ok_or(CreatureLootAccessError::WitnessMismatch)?;
        if !current.same_actor(&handle.witness) { return Err(CreatureLootAccessError::WitnessMismatch); }
        Ok(())
    }

    fn with_idle_loot<R>(&mut self, handle: &CreatureLootActorHandle,
        apply: impl FnOnce(&mut crate::Map, ObjectGuid) -> R) -> CreatureLootAccess<R>
    {
        if let Err(error) = self.require_idle_loot_handle(handle) { return CreatureLootAccess::Rejected(error); }
        let map = self.maps.get_mut(&handle.key).expect("validated map").map_mut();
        CreatureLootAccess::Ready(apply(map, handle.guid))
    }

    fn with_tick_loot<R>(
        &mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle,
        apply: impl FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    ) -> CreatureLootAccess<R> {
        if !Arc::ptr_eq(&self.tick_origin, &handle.origin) { return CreatureLootAccess::Rejected(CreatureLootAccessError::WrongOrigin); }
        if token.key() != handle.key || token.incarnation() != handle.incarnation {
            return CreatureLootAccess::Rejected(CreatureLootAccessError::StaleIncarnation);
        }
        if let Err(error) = self.require_current_actor_token(tick, token) {
            return CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(error));
        }
        if token.actor_operation.is_some() {
            return CreatureLootAccess::Rejected(CreatureLootAccessError::PendingActorOperation);
        }
        match self.with_selected_actor(tick, token, handle.guid, Some(&handle.witness), apply) {
            Ok((result, _)) => CreatureLootAccess::Ready(result),
            Err(error) => CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(error)),
        }
    }

    pub fn observe_idle_creature_loot_source(&mut self, handle: &CreatureLootActorHandle)
        -> CreatureLootAccess<CreatureLootObservation>
    {
        self.with_idle_loot(handle, |map, guid| map.observe_loot_actor(guid)
            .expect("the same guard retains the Actor"))
    }

    pub fn install_idle_creature_kill_loot(
        &mut self, handle: &CreatureLootActorHandle, authority: &OwnedLootAuthority,
        generation: u64, lifetime: u64, shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> CreatureLootAccess<bool> {
        self.with_idle_loot(handle, |map, guid| map.install_actor_kill_loot(
            guid, authority, generation, lifetime, shared, personal,
        ))
    }

    pub fn install_tick_creature_kill_loot(
        &mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle, authority: &OwnedLootAuthority,
        generation: u64, lifetime: u64, shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> CreatureLootAccess<bool> {
        self.with_tick_loot(tick, token, handle, |actor| actor.install_kill_loot(
            authority, generation, lifetime, shared, personal,
        ))
    }

    pub fn force_idle_creature_loot_flags(&mut self, handle: &CreatureLootActorHandle)
        -> CreatureLootAccess<UnitValuesUpdate>
    {
        self.with_idle_loot(handle, |map, guid| map.force_actor_loot_flags(guid))
    }

    pub fn force_tick_creature_loot_flags(
        &mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle,
    ) -> CreatureLootAccess<UnitValuesUpdate> {
        self.with_tick_loot(tick, token, handle, |actor| actor.force_loot_flags())
    }

    pub fn release_idle_creature_loot(
        &mut self, handle: &CreatureLootActorHandle, authority: &OwnedLootAuthority,
        generation: u64, revision: u64, fully_skinned: bool, decay_rate: f32,
        phase: CreatureLootReleasePhase,
    ) -> CreatureLootAccess<Option<CreatureLootReleaseOutcome>> {
        self.with_idle_loot(handle, |map, guid| map.release_actor_loot(
            guid, authority, generation, revision, fully_skinned, decay_rate, phase,
        ))
    }

    pub fn release_tick_creature_loot(
        &mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        handle: &CreatureLootActorHandle, authority: &OwnedLootAuthority,
        generation: u64, revision: u64, fully_skinned: bool, decay_rate: f32,
        phase: CreatureLootReleasePhase,
    ) -> CreatureLootAccess<Option<CreatureLootReleaseOutcome>> {
        self.with_tick_loot(tick, token, handle, |actor| actor.release_looted_corpse(
            authority, generation, revision, fully_skinned, decay_rate, phase,
        ))
    }

    pub fn idle_creature_loot_fully_consumed(&mut self, handle: &CreatureLootActorHandle)
        -> CreatureLootAccess<bool>
    {
        self.with_idle_loot(handle, |map, guid| map.actor_loot_fully_consumed(guid))
    }

    pub fn idle_creature_has_loot_recipient(&mut self, handle: &CreatureLootActorHandle)
        -> CreatureLootAccess<bool>
    {
        self.with_idle_loot(handle, |map, guid| map.actor_has_loot_recipient(guid))
    }

    /// Compatibility read only: reject an Actor admitted since NoActor was
    /// observed, then read the exact Creature Record under the same map borrow.
    pub fn creature_record_loot_fully_consumed(&self, key: MapKey, guid: ObjectGuid)
        -> CreatureLootAccess<Option<bool>>
    {
        let Some(map) = self.maps.get(&key) else { return CreatureLootAccess::NoActor; };
        let map = map.map();
        if map.creature_actor(guid).is_some() {
            return CreatureLootAccess::Rejected(CreatureLootAccessError::ActorPresent);
        }
        CreatureLootAccess::Ready(map.with_creature_like_cpp(guid,
            |creature| creature.is_fully_looted_like_cpp()))
    }

    /// Prepared AE operation only. General world_creature_guids remains unchanged.
    pub fn idle_creature_loot_candidates(&self, key: MapKey)
        -> CreatureLootAccess<Vec<ObjectGuid>>
    {
        if self.tick_coordination_like_cpp() != MapTickCoordinationStateLikeCpp::Idle
            || self.active_respawn.is_some()
        { return CreatureLootAccess::Rejected(CreatureLootAccessError::Busy); }
        let Some(map) = self.maps.get(&key) else { return CreatureLootAccess::NoActor; };
        CreatureLootAccess::Ready(map.map().loot_actor_guids())
    }
}
#[cfg(test)]
mod tests;
