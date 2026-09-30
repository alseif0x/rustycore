//! Private Map operations on the already admitted complete Creature Actor.
use super::*;
use crate::map_manager::{CreatureLootObservation, CreatureLootReleaseOutcome, CreatureLootReleasePhase};
use std::collections::HashMap;
use wow_entities::{CreatureLoot, OwnedLootAuthority, UnitValuesUpdate};

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where Terrain: TerrainGridLoader, Lifecycle: GridLifecycle {
    pub(crate) fn observe_loot_actor(&self, guid: ObjectGuid) -> Option<CreatureLootObservation> {
        self.creature_actor(guid).map(|actor| actor.observe_loot())
    }
    pub(crate) fn install_actor_kill_loot(
        &mut self, guid: ObjectGuid, authority: &OwnedLootAuthority,
        generation: u64, lifetime: u64, shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        self.creature_actor_mut(guid).expect("the manager validated the same Actor")
            .install_kill_loot(authority, generation, lifetime, shared, personal)
    }
    pub(crate) fn force_actor_loot_flags(&mut self, guid: ObjectGuid) -> UnitValuesUpdate {
        self.creature_actor_mut(guid).expect("the manager validated the same Actor").force_loot_flags()
    }
    pub(crate) fn release_actor_loot(
        &mut self, guid: ObjectGuid, authority: &OwnedLootAuthority,
        generation: u64, revision: u64, fully_skinned: bool, decay_rate: f32,
        phase: CreatureLootReleasePhase,
    ) -> Option<CreatureLootReleaseOutcome> {
        self.creature_actor_mut(guid).expect("the manager validated the same Actor")
            .release_looted_corpse(authority, generation, revision, fully_skinned, decay_rate, phase)
    }
    pub(crate) fn actor_loot_fully_consumed(&self, guid: ObjectGuid) -> bool {
        self.creature_actor(guid).expect("the manager validated the same Actor")
            .creature.is_fully_looted_like_cpp()
    }
    pub(crate) fn actor_has_loot_recipient(&self, guid: ObjectGuid) -> bool {
        self.creature_actor(guid).expect("the manager validated the same Actor")
            .creature.has_loot_recipient()
    }
    pub(crate) fn loot_actor_guids(&self) -> Vec<ObjectGuid> {
        self.entity_world.iter().filter_map(|(guid, _)| self.creature_actor(*guid).map(|_| *guid)).collect()
    }
}
