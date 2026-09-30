//! Admission classification and identity at the dormant canonical actor seam.

use super::*;
use rand::rngs::StdRng;
use wow_core::{Position, guid::HighGuid};
use wow_entities::{Creature, MapObjectRecord, ObjectAccessorError, OwnedLootAuthorityLifecycle};

fn creature(counter: i64, spawn_id: u64) -> Creature {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(spawn_id);
    creature
}

fn actor(counter: i64, spawn_id: u64, point: bool) -> (WorldCreature, StdRng) {
    let creature = creature(counter, spawn_id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let mut actor = WorldCreature::from_canonical(creature, data);
    actor.create_data.npc_flags = 0x1234;
    let rng = actor.seed_actor_storage_runtime(point);
    (actor, rng)
}

fn insert(map: &mut Map, actor: WorldCreature) -> CreatureActorWitness {
    match map.admit_creature_actor(actor).unwrap() {
        CreatureActorAdmission::Inserted { witness } => witness,
        other => panic!("empty fixture must admit exactly once: {other:?}"),
    }
}

#[test]
fn admission_failure_preserves_validation_priority_incoming_motor_and_existing_actor() {
    for case in 0..4 {
        let (current, _) = actor(201, 2010, true);
        let guid = current.guid();
        let mut map = Map::new(571, 7, 1, 1000);
        let witness = insert(&mut map, current);
        let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
        let (mut incoming, mut rng) = actor(201, 2011, false);
        match case {
            0 => {
                incoming.creature.unit_mut().world_mut().reset_map().unwrap();
                incoming.creature.unit_mut().world_mut().object_mut().create(ObjectGuid::EMPTY);
            }
            1 => incoming.creature.unit_mut().world_mut().object_mut().create(ObjectGuid::EMPTY),
            2 => {
                incoming.creature.unit_mut().world_mut().object_mut()
                    .create(ObjectGuid::create_global(HighGuid::Player, 0, 201));
                incoming.creature.unit_mut().world_mut().reset_map().unwrap();
                incoming.creature.unit_mut().world_mut().set_map(530, 7).unwrap();
            }
            3 => {
                incoming.creature.unit_mut().world_mut().reset_map().unwrap();
                incoming.creature.unit_mut().world_mut().set_map(530, 7).unwrap();
            }
            _ => unreachable!(),
        }
        let loot = incoming.creature.loot_authority_like_cpp().clone();
        let (error, mut returned) = map.admit_creature_actor(incoming).unwrap_err();
        match case {
            0 => assert!(matches!(error, CreatureActorAdmissionError::Store(
                MapObjectStoreError::InvalidRecord(ObjectAccessorError::ObjectHasNoMap { .. })
            ))),
            1 => assert!(matches!(error, CreatureActorAdmissionError::Store(
                MapObjectStoreError::InvalidRecord(ObjectAccessorError::UnsupportedGuidKind { .. })
            ))),
            2 => assert!(matches!(error, CreatureActorAdmissionError::Store(
                MapObjectStoreError::InvalidRecord(ObjectAccessorError::WrongGuidKind {
                    expected: AccessorObjectKind::Creature, ..
                })
            ))),
            3 => assert!(matches!(error, CreatureActorAdmissionError::Store(
                MapObjectStoreError::WrongMap { .. }
            ))),
            _ => unreachable!(),
        }
        returned.assert_actor_storage_runtime(&mut rng, false);
        assert_eq!(returned.create_data.npc_flags, 0x1234);
        assert!(returned.creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
        assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
        assert_eq!(map.map_object_count(), 1);
        assert_eq!(map.creature_actor(guid).unwrap() as *const WorldCreature, pointer);
        assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
        assert_eq!(map.creature_spawn_id_store_guids_like_cpp(2010), vec![guid]);
        assert!(map.creature_spawn_id_store_guids_like_cpp(2011).is_empty());
    }
}

#[test]
fn inserted_actor_is_the_only_entity_and_raw_access_keeps_its_witness() {
    let (incoming, mut rng) = actor(202, 2020, true);
    let guid = incoming.guid();
    let loot = incoming.creature.loot_authority_like_cpp().clone();
    let mut map = Map::new(571, 7, 1, 1000);
    let witness = insert(&mut map, incoming);
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let stored = map.creature_actor_mut(guid).unwrap();
    stored.creature.unit_mut().set_health(33);
    stored.assert_actor_storage_runtime(&mut rng, true);
    assert_eq!(map.with_creature_like_cpp(guid, Creature::current_health), Some(33));
    assert_eq!(map.creature_actor(guid).unwrap() as *const WorldCreature, pointer);
    assert!(map.with_creature_like_cpp(guid, |creature|
        creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot)).unwrap());
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(2020), vec![guid]);
    assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
    assert!(witness.same_actor(&witness.clone()));
    let missing = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, 999);
    assert!(map.creature_actor(missing).is_none());
    assert!(map.creature_actor_mut(missing).is_none());
    assert!(map.creature_actor_witness(missing).is_none());
}

#[test]
fn existing_actor_returns_incoming_and_current_witness_without_overwrite_or_detach() {
    let (current, mut current_rng) = actor(203, 2030, true);
    let guid = current.guid();
    let current_loot = current.creature.loot_authority_like_cpp().clone();
    let mut map = Map::new(571, 7, 1, 1000);
    let witness = insert(&mut map, current);
    map.creature_actor_mut(guid).unwrap().creature.unit_mut().set_health(25);
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let (incoming, mut incoming_rng) = actor(203, 2031, false);
    let incoming_loot = incoming.creature.loot_authority_like_cpp().clone();
    let (mut returned, observed) = match map.admit_creature_actor(incoming).unwrap() {
        CreatureActorAdmission::ExistingActor { incoming, witness } => (incoming, witness),
        other => panic!("duplicate actor must remain explicit: {other:?}"),
    };
    assert!(witness.same_actor(&observed));
    assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
    assert_eq!(map.creature_actor(guid).unwrap() as *const WorldCreature, pointer);
    assert_eq!(map.get_typed_creature(guid).unwrap().current_health(), 25);
    assert_eq!(returned.creature.current_health(), 75);
    assert_eq!(returned.creature.spawn_id(), 2031);
    returned.assert_actor_storage_runtime(&mut incoming_rng, false);
    map.creature_actor_mut(guid).unwrap().assert_actor_storage_runtime(&mut current_rng, true);
    assert!(returned.creature.loot_authority_like_cpp().shares_storage_like_cpp(&incoming_loot));
    assert_ne!(incoming_loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_ne!(current_loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(2030), vec![guid]);
    assert!(map.creature_spawn_id_store_guids_like_cpp(2031).is_empty());
    assert_eq!(map.map_object_count(), 1);
}

#[test]
fn existing_exact_record_is_returned_as_a_migration_boundary_without_promotion() {
    let current = creature(204, 2040);
    let guid = current.guid();
    let loot = current.loot_authority_like_cpp().clone();
    let mut map = Map::new(571, 7, 1, 1000);
    map.insert_map_object_record(MapObjectRecord::new_creature(current).unwrap()).unwrap();
    let pointer = map.get_typed_creature(guid).unwrap() as *const Creature;
    let (incoming, mut rng) = actor(204, 2041, true);
    let mut returned = match map.admit_creature_actor(incoming).unwrap() {
        CreatureActorAdmission::ExistingRecord { incoming } => incoming,
        other => panic!("typed Record must not become an actor implicitly: {other:?}"),
    };
    returned.assert_actor_storage_runtime(&mut rng, true);
    assert_eq!(returned.creature.spawn_id(), 2041);
    assert_eq!(map.get_typed_creature(guid).unwrap() as *const Creature, pointer);
    assert!(map.get_typed_creature(guid).unwrap().loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert!(map.creature_actor(guid).is_none());
    assert!(map.creature_actor_mut(guid).is_none());
    assert!(map.creature_actor_witness(guid).is_none());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(2040), vec![guid]);
    assert!(map.creature_spawn_id_store_guids_like_cpp(2041).is_empty());
}

#[test]
fn generic_and_wrong_kind_records_reject_same_guid_admission_without_store_mutation() {
    for kind in [AccessorObjectKind::Creature, AccessorObjectKind::GameObject] {
        let (incoming, mut rng) = actor(205, 2050, false);
        let guid = incoming.guid();
        let loot = incoming.creature.loot_authority_like_cpp().clone();
        let mut object = incoming.creature.unit().world().clone();
        if kind == AccessorObjectKind::GameObject {
            object.object_mut().create(ObjectGuid::create_world_object(
                HighGuid::GameObject, 0, 1, 571, 7, 42, 205,
            ));
        }
        let mut record = MapObjectRecord::new(kind, object).unwrap();
        // Model an existing wrong-kind entry using the current record mutator;
        // do not widen fields or introduce a production test-only admission API.
        record.object_mut().object_mut().create(guid);
        let mut map = Map::new(571, 7, 1, 1000);
        map.insert_map_object_record(record).unwrap();
        let (error, mut returned) = map.admit_creature_actor(incoming).unwrap_err();
        assert_eq!(error, CreatureActorAdmissionError::NotExactCreature { guid, actual_kind: kind });
        returned.assert_actor_storage_runtime(&mut rng, false);
        assert!(returned.creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
        assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
        assert_eq!(map.entity_world.kind(guid), Some(kind));
        assert_eq!(map.map_object_count(), 1);
        assert!(map.creature_actor(guid).is_none());
        assert!(map.creature_actor_witness(guid).is_none());
        assert!(map.creature_spawn_id_store_guids_like_cpp(2050).is_empty());
    }
}

#[test]
fn snapshot_replacement_retains_box_and_witness_even_when_health_and_loot_change() {
    let (incoming, mut rng) = actor(206, 2060, true);
    let guid = incoming.guid();
    let old_loot = incoming.creature.loot_authority_like_cpp().clone();
    let mut map = Map::new(571, 7, 1, 1000);
    let witness = insert(&mut map, incoming);
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let spline = format!("{:?}", map.creature_actor(guid).unwrap().active_move_spline_like_cpp());
    let mut shared = map.with_creature_like_cpp(guid, Creature::clone).unwrap();
    shared.unit_mut().set_health(10);
    map.replace_creature_snapshot(MapObjectRecord::new_creature(shared).unwrap()).unwrap();
    assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
    assert_eq!(map.creature_actor(guid).unwrap() as *const WorldCreature, pointer);
    assert_ne!(old_loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);

    let mut distinct = creature(206, 2061);
    distinct.unit_mut().set_health(90);
    let new_loot = distinct.loot_authority_like_cpp().clone();
    map.replace_creature_snapshot(MapObjectRecord::new_creature(distinct).unwrap()).unwrap();
    assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
    assert_eq!(map.creature_actor(guid).unwrap() as *const WorldCreature, pointer);
    assert_eq!(map.get_typed_creature(guid).unwrap().current_health(), 90);
    assert_eq!(old_loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert!(map.get_typed_creature(guid).unwrap().loot_authority_like_cpp().shares_storage_like_cpp(&new_loot));
    assert_eq!(map.creature_actor(guid).unwrap().create_data.npc_flags, 0x1234);
    assert_eq!(format!("{:?}", map.creature_actor(guid).unwrap().active_move_spline_like_cpp()), spline);
    map.creature_actor_mut(guid).unwrap().assert_actor_storage_runtime(&mut rng, true);
    assert!(map.creature_spawn_id_store_guids_like_cpp(2060).is_empty());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(2061), vec![guid]);
}

#[test]
fn relocation_retains_witness_and_box_through_cell_moves_and_grid_load() {
    let (mut incoming, mut rng) = actor(207, 2070, true);
    incoming.creature.unit_mut().world_mut().set_active(true);
    let guid = incoming.guid();
    let mut map = Map::new(571, 7, 1, 1000);
    let witness = insert(&mut map, incoming);
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    // Cell admission belongs to the later producer migration. This fixture
    // uses the existing private path without changing production registration.
    let entry = map.take_object_entry(guid).unwrap();
    map.add_object_entry_to_map(entry).unwrap();
    for (position, moves_cell, loads_grid) in [
        (Position::xyz(2.0, 3.0, 4.0), false, false),
        (Position::xyz(90.0, 20.0, 5.0), true, false),
        (Position::xyz(700.0, 20.0, 5.0), true, true),
    ] {
        let outcome = map.relocate_map_object_like_cpp(guid, position).unwrap();
        assert!(outcome.relocated);
        assert_eq!(outcome.moved_between_cells, moves_cell);
        assert_eq!(outcome.loaded_grid, loads_grid);
        assert_eq!(map.creature_actor(guid).unwrap() as *const WorldCreature, pointer);
        assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
        assert_eq!(map.creature_actor(guid).unwrap().position(), position);
        assert_eq!(map.creature_spawn_id_store_guids_like_cpp(2070), vec![guid]);
    }
    map.creature_actor_mut(guid).unwrap().assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn same_guid_readmission_gets_new_identity_while_the_old_witness_is_retained() {
    let (first, _) = actor(208, 2080, false);
    let guid = first.guid();
    let mut map = Map::new(571, 7, 1, 1000);
    let old_witness = insert(&mut map, first);
    let old_pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let removed = map.remove_map_object(guid).unwrap();
    assert!(map.creature_actor_witness(guid).is_none());
    let (second, _) = actor(208, 2081, true);
    let new_witness = insert(&mut map, second);
    assert!(!old_witness.same_actor(&new_witness));
    // The old Box is still retained by owned transport, so this comparison
    // does not assume an allocator address cannot be reused after destruction.
    assert_ne!(map.creature_actor(guid).unwrap() as *const WorldCreature, old_pointer);
    drop(removed);
    assert!(!old_witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
    let (other, _) = actor(209, 2090, false);
    let other_witness = insert(&mut map, other);
    assert!(!new_witness.same_actor(&other_witness));
    assert_eq!(map.map_object_count(), 2);
    assert!(map.creature_spawn_id_store_guids_like_cpp(2080).is_empty());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(2081), vec![guid]);
}
