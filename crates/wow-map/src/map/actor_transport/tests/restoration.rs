use super::*;

#[test]
fn raw_canonical_record_take_restore_retains_body_address_and_spawn_membership() {
    let (mut map, source, guid, _) = fixtures::pair(831, true);
    let pointer = map.get_typed_creature(guid).unwrap() as *const Creature;
    let loot = map.get_typed_creature(guid).unwrap().loot_authority_like_cpp().clone();
    let entry = map.entity_world.take(&guid).unwrap();
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(8310), vec![guid]);
    map.entity_world.restore_transport_entry(guid, entry).unwrap();
    assert_rejected_owners(&map, &source, guid, pointer);
    assert!(map.get_typed_creature(guid).unwrap().loot_authority_like_cpp().shares_storage_like_cpp(&loot));
}

#[test]
fn raw_actor_round_trip_keeps_box_witness_and_nondefault_motor_without_replay() {
    let (mut map, mut source, guid, mut rng) = fixtures::pair(832, false);
    map.transport_legacy_creature_ownership(&mut source).unwrap();
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let witness = map.creature_actor_witness(guid).unwrap();
    let entry = map.entity_world.take(&guid).unwrap();
    map.entity_world.restore_transport_entry(guid, entry).unwrap();
    assert_eq!(map.creature_actor(guid).unwrap() as *const WorldCreature, pointer);
    assert!(map.creature_actor_witness(guid).unwrap().same_actor(&witness));
    map.creature_actor_mut(guid).unwrap().assert_actor_storage_runtime(&mut rng, false);
}

#[test]
fn occupied_or_wrong_guid_canonical_restore_returns_the_whole_entry() {
    let (mut map, mut source, guid, mut rng) = fixtures::pair(833, true);
    map.transport_legacy_creature_ownership(&mut source).unwrap();
    let original_pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let witness = map.creature_actor_witness(guid).unwrap();
    let entry = map.entity_world.take(&guid).unwrap();
    let wrong_guid = ObjectGuid::create_player(1, 1);
    let entry = map.entity_world.restore_transport_entry(wrong_guid, entry).unwrap_err();
    let other_record = fixtures::pair(833, false).0.entity_world.take(&guid).unwrap();
    map.entity_world.restore_transport_entry(guid, other_record).unwrap();
    let occupied_pointer = map.get_typed_creature(guid).unwrap() as *const Creature;
    let entry = map.entity_world.restore_transport_entry(guid, entry).unwrap_err();
    assert_eq!(map.get_typed_creature(guid).unwrap() as *const Creature, occupied_pointer);
    let occupied = map.entity_world.take(&guid).unwrap();
    map.entity_world.restore_transport_entry(guid, entry).unwrap();
    assert_eq!(map.creature_actor(guid).unwrap() as *const WorldCreature, original_pointer);
    assert!(map.creature_actor_witness(guid).unwrap().same_actor(&witness));
    map.creature_actor_mut(guid).unwrap().assert_actor_storage_runtime(&mut rng, true);
    drop(occupied);
}
