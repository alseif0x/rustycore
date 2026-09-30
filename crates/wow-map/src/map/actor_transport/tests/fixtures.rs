//! One initial snapshot represents the existing two-owner bridge; never clone
//! WorldCreature or its motor to simulate an ownership move.

use crate::Map;
use crate::map_manager::{Grid, GridCoord, MapInstance, WorldCreature};
use rand::rngs::StdRng;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{Creature, CurrentSpellRef, CurrentSpellSlot, MapObjectRecord, ReactState};

pub(super) fn add_pair(map: &mut Map, source: &mut MapInstance, counter: i64, point: bool) -> (ObjectGuid, StdRng) {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(counter as u64 * 10);
    // Only setup mirrors the old bridge. Both following owners move into
    // their stores; the operation under test contains no Creature clone.
    let canonical = creature.clone();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(canonical).unwrap()).unwrap();
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let mut actor = WorldCreature::from_canonical(creature, data);
    actor.create_data.npc_flags = 0x1234;
    actor.create_data.scale = 1.75;
    let rng = actor.seed_actor_storage_runtime(point);
    actor.creature.set_respawn_time(12345);
    actor.creature.set_respawn_delay(73);
    actor.creature.set_react_state(ReactState::Passive);
    actor.creature.unit_mut().subsystems_mut().auras.apply_transform_aura_like_cpp(118, false, None);
    actor.creature.unit_mut().subsystems_mut().spells.set_cooldown(133, 500, 1500);
    actor.creature.unit_mut().set_current_cast_spell(
        CurrentSpellSlot::Generic,
        CurrentSpellRef::new(133, Some(guid), None).with_cast_time_ms(1500),
    );
    actor.creature.unit_mut().world_mut().object_mut().add_to_world();
    let coord = GridCoord::new(0, 0);
    source.grids.entry(coord).or_insert_with(|| Grid::new(0, 0)).creatures.insert(guid, actor);
    (guid, rng)
}

pub(super) fn pair(counter: i64, point: bool) -> (Map, MapInstance, ObjectGuid, StdRng) {
    let mut map = Map::new(571, 7, 1, 1000);
    let mut source = MapInstance::new(571, 7);
    let (guid, rng) = add_pair(&mut map, &mut source, counter, point);
    (map, source, guid, rng)
}
