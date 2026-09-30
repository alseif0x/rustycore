//! Real duplicate-owner fixtures followed by move-only ownership admission.

use super::*;
use crate::map::ObjectEntry;
use crate::map_manager::{Grid, WorldCreature};
use wow_entities::{Creature, MapObjectRecord, OwnedLootAuthority};

mod fixtures;
mod rejection;
mod restoration;
mod threat;
mod winners;

fn source_actor(source: &MapInstance, guid: ObjectGuid) -> &WorldCreature {
    source
        .grids
        .values()
        .find_map(|grid| grid.creatures.get(&guid))
        .unwrap()
}

fn source_actor_mut(source: &mut MapInstance, guid: ObjectGuid) -> &mut WorldCreature {
    source
        .grids
        .values_mut()
        .find_map(|grid| grid.creatures.get_mut(&guid))
        .unwrap()
}

fn source_count(source: &MapInstance) -> usize {
    source.grids.values().map(|grid| grid.creatures.len()).sum()
}

fn assert_rejected_owners(
    map: &Map,
    source: &MapInstance,
    guid: ObjectGuid,
    pointer: *const Creature,
) {
    assert_eq!(
        map.get_typed_creature(guid).unwrap() as *const Creature,
        pointer
    );
    assert!(map.creature_actor(guid).is_none());
    assert_eq!(source_actor(source, guid).guid(), guid);
}
