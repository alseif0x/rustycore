//! Concrete loot operations select one legitimate owner; Actor rejection never falls back.
use super::*;
use wow_map::manager::{CreatureLootAccess, CreatureLootAccessError, CreatureLootActorHandle};
use wow_map::map_manager::{CreatureLootObservation, CreatureLootReleasePhase};
use wow_entities::CreatureLoot;

impl WorldSession {
    fn creature_loot_owner_key(&self) -> wow_map::MapKey {
        self.current_canonical_player_map_key_like_cpp()
            .or_else(|| self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp())))
            .unwrap_or_else(|| {
                let (map, instance) = self.current_legacy_runtime_map_key_like_cpp();
                wow_map::MapKey::new(u32::from(map), instance)
            })
    }

    pub(crate) fn capture_creature_loot_owner(&self, guid: ObjectGuid)
        -> CreatureLootAccess<CreatureLootActorHandle>
    {
        let Some(manager) = self.canonical_map_manager.as_ref() else { return CreatureLootAccess::NoActor; };
        let key = self.creature_loot_owner_key();
        let Ok(manager) = manager.lock() else { return CreatureLootAccess::Rejected(CreatureLootAccessError::Busy); };
        manager.idle_creature_loot_handle(key, guid)
    }

    pub(crate) fn observe_creature_loot_owner(&mut self, guid: ObjectGuid)
        -> (CreatureLootAccess<CreatureLootActorHandle>, Option<CreatureLootObservation>)
    {
        let canonical = if let Some(manager) = self.canonical_map_manager.as_ref() {
            let key = self.creature_loot_owner_key();
            match manager.lock() {
                Ok(manager) => manager.observe_idle_creature_loot(key, guid),
                Err(_) => CreatureLootAccess::Rejected(CreatureLootAccessError::Busy),
            }
        } else { CreatureLootAccess::NoActor };
        match canonical {
            CreatureLootAccess::Ready(observation) => {
                let (handle, source) = observation.into_parts();
                (CreatureLootAccess::Ready(handle), Some(source))
            }
            CreatureLootAccess::Rejected(error) => (CreatureLootAccess::Rejected(error), None),
            CreatureLootAccess::NoActor => {
                // Preserve the old compatibility read's snapshot sync while it owns the body.
                let source = self.mutate_world_creature(guid, |actor| actor.observe_loot());
                (CreatureLootAccess::NoActor, source)
            }
        }
    }

    fn creature_loot_handle_is_here(&self, handle: &CreatureLootActorHandle, guid: ObjectGuid) -> bool {
        self.creature_loot_owner_key() == handle.key() && handle.guid() == guid
    }

    pub(crate) fn install_creature_loot_for_owner(
        &mut self, owner: &CreatureLootAccess<CreatureLootActorHandle>, guid: ObjectGuid,
        authority: &OwnedLootAuthority, generation: u64, lifetime: u64,
        shared: Option<CreatureLoot>, personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        match owner {
            CreatureLootAccess::Rejected(_) => false,
            CreatureLootAccess::Ready(handle) => {
                if !self.creature_loot_handle_is_here(handle, guid) { return false; }
                let Some(manager) = self.canonical_map_manager.as_ref() else { return false; };
                let Ok(mut manager) = manager.lock() else { return false; };
                matches!(manager.install_idle_creature_kill_loot(handle, authority, generation,
                    lifetime, shared, personal), CreatureLootAccess::Ready(true))
            }
            CreatureLootAccess::NoActor => {
                if !matches!(self.capture_creature_loot_owner(guid), CreatureLootAccess::NoActor) { return false; }
                let expected = authority.clone(); // Original install compatibility clone point.
                self.mutate_world_creature(guid, move |actor| {
                    actor.install_kill_loot(&expected, generation, lifetime, shared, personal)
                }).unwrap_or(false)
            }
        }
    }

    pub(crate) fn force_creature_loot_flags(
        &mut self, owner: &CreatureLootAccess<CreatureLootActorHandle>, guid: ObjectGuid,
    ) -> Option<wow_entities::UnitValuesUpdate> {
        match owner {
            CreatureLootAccess::Rejected(_) => None,
            CreatureLootAccess::Ready(handle) => {
                if !self.creature_loot_handle_is_here(handle, guid) { return None; }
                let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
                match manager.force_idle_creature_loot_flags(handle) {
                    CreatureLootAccess::Ready(values) => Some(values), _ => None,
                }
            }
            CreatureLootAccess::NoActor => {
                if !matches!(self.capture_creature_loot_owner(guid), CreatureLootAccess::NoActor) { return None; }
                self.mutate_world_creature(guid, |actor| actor.force_loot_flags())
            }
        }
    }

    pub(crate) fn release_creature_loot_corpse(
        &mut self, owner: &CreatureLootAccess<CreatureLootActorHandle>, guid: ObjectGuid,
        authority: Option<&OwnedLootAuthority>, generation: u64, revision: u64,
        fully_skinned: bool, decay_rate: f32, phase: CreatureLootReleasePhase,
    ) -> Option<(Option<(u32, u32)>, wow_entities::UnitValuesUpdate)> {
        match owner {
            CreatureLootAccess::Rejected(_) => None,
            CreatureLootAccess::Ready(handle) => {
                if !self.creature_loot_handle_is_here(handle, guid) { return None; }
                let authority = authority?;
                let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
                match manager.release_idle_creature_loot(handle, authority, generation, revision,
                    fully_skinned, decay_rate, phase) {
                    CreatureLootAccess::Ready(Some(result)) => Some(result.into_parts()), _ => None,
                }
            }
            CreatureLootAccess::NoActor => {
                if let Some(authority) = authority {
                    if !matches!(self.capture_creature_loot_owner(guid), CreatureLootAccess::NoActor) { return None; }
                    let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
                    let manager = self.map_manager.as_ref().cloned()?;
                    let (result, snapshot) = {
                        let mut manager = manager.write().unwrap_or_else(|poisoned| poisoned.into_inner());
                        let actor = manager.find_creature_mut(map_id, instance_id, guid)?;
                        actor.release_legacy_looted_corpse(authority, generation, revision,
                            fully_skinned, decay_rate, phase)?
                    };
                    self.sync_canonical_creature_entity_like_cpp(snapshot);
                    Some(result.into_parts())
                } else {
                    if !matches!(self.capture_creature_loot_owner(guid), CreatureLootAccess::NoActor) { return None; }
                    self.mutate_world_creature(guid, |actor| {
                        actor.release_local_looted_corpse(fully_skinned, decay_rate).into_parts()
                    })
                }
            }
        }
    }

    pub(crate) fn creature_loot_fully_consumed(
        &mut self, owner: &CreatureLootAccess<CreatureLootActorHandle>, guid: ObjectGuid,
    ) -> Option<bool> {
        match owner {
            CreatureLootAccess::Rejected(_) => None,
            CreatureLootAccess::Ready(handle) => {
                if !self.creature_loot_handle_is_here(handle, guid) { return None; }
                let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
                match manager.idle_creature_loot_fully_consumed(handle) {
                    CreatureLootAccess::Ready(value) => Some(value), _ => None,
                }
            }
            CreatureLootAccess::NoActor => {
                let key = self.canonical_object_lookup_map_key_like_cpp(
                    u32::from(self.player_map_id_like_cpp()))?;
                let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
                match manager.creature_record_loot_fully_consumed(key, guid) {
                    CreatureLootAccess::Ready(value) => value, _ => None,
                }
            }
        }
    }
}

impl WorldSession {
    /// Prepared AE reader only; no general world_creature_guids route is changed.
    pub(crate) fn canonical_creature_loot_candidates(&self) -> Option<Vec<ObjectGuid>> {
        let key = self.creature_loot_owner_key();
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        match manager.idle_creature_loot_candidates(key) {
            CreatureLootAccess::Ready(guids) => Some(guids), _ => None,
        }
    }
}
