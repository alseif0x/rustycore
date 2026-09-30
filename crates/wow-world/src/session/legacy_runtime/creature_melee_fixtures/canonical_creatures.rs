//! Original canonical creature fixtures; admission negatives remain explicit in each test.
use super::*;

pub fn add_canonical_test_creature_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
) {
    add_canonical_test_creature_on_map_with_world_state(
        canonical,
        guid,
        entry,
        position,
        npc_flags,
        map_id,
        instance_id,
        true,
    );
}

pub fn add_canonical_test_creature_on_map_with_world_state(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
    is_in_world: bool,
) {
    add_canonical_test_creature_on_map_with_world_state_and_owner(
        canonical,
        guid,
        entry,
        position,
        npc_flags,
        map_id,
        instance_id,
        is_in_world,
        None,
    );
}

pub fn add_canonical_test_creature_on_map_with_world_state_and_owner(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
    is_in_world: bool,
    owner_guid: Option<ObjectGuid>,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_owner_guid(owner_guid);
    if is_in_world {
        creature.unit_mut().world_mut().object_mut().add_to_world();
    }

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

