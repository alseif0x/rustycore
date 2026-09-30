//! Map and item GUID allocation through the original owners.

use super::recovery_support::*;
use std::sync::Barrier;
use wow_core::guid::ObjectGuidGenerator;
use wow_world::test_fixtures::loot::{
    allocate_loot_guid_for_test, allocate_loot_item_guids_for_test,
    attach_loot_allocator_for_test as attach_loot_guid_allocator_for_owner,
    materialize_loot_pools_for_test, next_map_loot_guid_for_test,
};

#[test]
fn loot_guid_allocator_refuses_different_owner_map_without_advancing_like_cpp() {
    let mut session = make_session();
    let canonical_owner =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 9, 571, 7, 404, 19_702);
    let other_map_owner =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 9, 0, 7, 505, 19_703);
    attach_loot_guid_allocator_for_owner(&mut session, canonical_owner);

    assert!(allocate_loot_guid_for_test(&mut session, other_map_owner).is_none());

    let first_allocated = allocate_loot_guid_for_test(&mut session, canonical_owner)
        .expect("the rejected owner must not consume the canonical map sequence");
    assert_eq!(first_allocated.counter(), 1);
}

#[test]
fn loot_guid_allocator_without_canonical_map_fails_without_advancing_like_cpp() {
    let mut session = make_session();
    let owner_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 9, 571, 7, 303, 19_701);

    assert!(allocate_loot_guid_for_test(&mut session, owner_guid).is_none());

    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    let first_allocated = allocate_loot_guid_for_test(&mut session, owner_guid)
        .expect("the first allocation after attaching the map should succeed");
    assert_eq!(first_allocated.counter(), 1);
}

#[test]
fn map_owned_loot_guid_sequence_is_shared_across_owner_kinds_like_cpp() {
    let mut session = make_session();
    let map_id = 571;
    let realm_id = 7;
    session.set_realm_id(realm_id);
    let shared_owner_counter = 19_700;
    let creature_owner = ObjectGuid::create_world_object(
        HighGuid::Creature,
        0,
        11,
        map_id,
        17,
        101,
        shared_owner_counter,
    );
    let gameobject_owner = ObjectGuid::create_world_object(
        HighGuid::GameObject,
        0,
        12,
        map_id,
        18,
        202,
        shared_owner_counter,
    );
    attach_loot_guid_allocator_for_owner(&mut session, creature_owner);

    let creature_loot = allocate_loot_guid_for_test(&mut session, creature_owner)
        .expect("the creature owner map should allocate a LootObject");
    let gameobject_loot = allocate_loot_guid_for_test(&mut session, gameobject_owner)
        .expect("the gameobject owner map should share the LootObject sequence");

    assert_ne!(creature_loot, gameobject_loot);
    assert_eq!(creature_loot.counter(), 1);
    assert_eq!(gameobject_loot.counter(), 2);
    for loot_guid in [creature_loot, gameobject_loot] {
        assert_eq!(loot_guid.high_type(), HighGuid::LootObject);
        assert_eq!(loot_guid.sub_type(), 0);
        assert_eq!(loot_guid.realm_id(), realm_id);
        assert_eq!(loot_guid.map_id(), map_id);
        assert_eq!(loot_guid.server_id(), 0);
        assert_eq!(loot_guid.entry(), 0);
    }

    assert_eq!(
        next_map_loot_guid_for_test(&session, u32::from(map_id), 0).unwrap(),
        3
    );
}

#[test]
fn personal_loot_pools_receive_distinct_map_owned_guids_like_cpp() {
    let mut session = make_session();
    session.set_realm_id(7);
    let owner_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 9, 571, 7, 606, 19_704);
    let first_player = ObjectGuid::create_player(1, 42);
    let second_player = ObjectGuid::create_player(1, 77);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = allocate_loot_guid_for_test(&mut session, owner_guid)
        .expect("the base personal pool should receive a map-owned LootObject");
    loot.allowed_looters = vec![second_player, first_player];

    let (shared, personal) =
        materialize_loot_pools_for_test(&mut session, owner_guid, first_player, loot, true)
            .expect("both personal pools should materialize");

    assert!(shared.is_none());
    assert_eq!(personal.len(), 2);
    let first_pool = personal.get(&first_player).unwrap();
    let second_pool = personal.get(&second_player).unwrap();
    assert_ne!(first_pool.loot_guid, second_pool.loot_guid);
    assert_eq!(first_pool.loot_guid.counter(), 1);
    assert_eq!(second_pool.loot_guid.counter(), 2);
    for (player_guid, pool) in [(first_player, first_pool), (second_player, second_pool)] {
        assert_eq!(pool.loot_guid.high_type(), HighGuid::LootObject);
        assert_eq!(pool.loot_guid.realm_id(), 7);
        assert_eq!(pool.loot_guid.map_id(), 571);
        assert_eq!(pool.loot_guid.server_id(), 0);
        assert_eq!(pool.loot_guid.entry(), 0);
        assert_eq!(pool.allowed_looters, vec![player_guid]);
        assert_eq!(pool.items[0].allowed_looters, vec![player_guid]);
    }
}

#[test]
fn item_instance_guid_allocator_fails_closed_and_never_reuses_failed_grant_like_cpp() {
    let mut session = make_session();
    assert_eq!(allocate_loot_item_guids_for_test(&session, 1), None);

    let generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 91_000));
    session.set_item_guid_generator_like_cpp(Arc::clone(&generator));

    // C++ consumes a GUID when Item::CreateItem runs.  A later storage or
    // transaction failure may leave a gap, but must never make that GUID
    // available to a competing durable grant.
    let abandoned_after_persistence_failure =
        allocate_loot_item_guids_for_test(&session, 1).expect("allocator must be installed");
    assert_eq!(abandoned_after_persistence_failure[0].0, 91_000);
    drop(abandoned_after_persistence_failure);

    let next_grant =
        allocate_loot_item_guids_for_test(&session, 1).expect("allocator must remain installed");
    assert_eq!(next_grant[0].0, 91_001);
    assert_eq!(generator.next_after_max_used(), 91_002);
}

#[test]
fn item_instance_guid_allocator_is_shared_across_concurrent_loot_sessions_like_cpp() {
    const WORKERS: usize = 8;
    const GUIDS_PER_WORKER: usize = 128;
    const FIRST_GUID: i64 = 40_000;

    let generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Item, FIRST_GUID));
    let start = Arc::new(Barrier::new(WORKERS));
    let handles = (0..WORKERS)
        .map(|_| {
            let generator = Arc::clone(&generator);
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                let mut session = make_session();
                session.set_realm_id(7);
                session.set_item_guid_generator_like_cpp(generator);
                start.wait();
                allocate_loot_item_guids_for_test(&session, GUIDS_PER_WORKER)
                    .expect("shared item allocator must be installed")
            })
        })
        .collect::<Vec<_>>();

    let mut allocated = handles
        .into_iter()
        .flat_map(|handle| handle.join().expect("allocation worker must finish"))
        .collect::<Vec<_>>();
    allocated.sort_unstable_by_key(|(db_guid, _)| *db_guid);

    assert_eq!(allocated.len(), WORKERS * GUIDS_PER_WORKER);
    for (offset, (db_guid, object_guid)) in allocated.iter().enumerate() {
        let expected = FIRST_GUID as u64 + offset as u64;
        assert_eq!(*db_guid, expected);
        assert!(object_guid.is_item());
        assert_eq!(object_guid.counter() as u64, expected);
    }
    assert_eq!(
        generator.next_after_max_used(),
        FIRST_GUID + (WORKERS * GUIDS_PER_WORKER) as i64
    );
}
