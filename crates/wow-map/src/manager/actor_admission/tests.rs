//! The manager seam neither creates maps nor redirects an incoming actor.

use super::*;
use crate::MIN_GRID_DELAY_MS;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{Creature, MapObjectRecord};

fn actor(counter: i64) -> WorldCreature {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_world_object(
            HighGuid::Creature,
            0,
            1,
            571,
            7,
            42,
            counter,
        ));
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(counter as u64);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    WorldCreature::from_canonical(creature, data)
}

#[test]
fn missing_map_returns_complete_incoming_without_implicit_map_creation() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    let mut incoming = actor(401);
    let mut rng = incoming.seed_actor_storage_runtime(true);
    let key = MapKey::new(571, 7);
    let (error, mut returned) = manager
        .admit_fresh_creature_actor(key, incoming)
        .unwrap_err();
    assert_eq!(
        error,
        FreshCreatureActorMapAdmissionError::MissingMap { key }
    );
    assert!(manager.find_map(571, 7).is_none());
    returned.assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn manager_admission_keeps_requested_map_identity_and_owned_rejection() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(571, 7);
    manager.create_world_map(530, 7);
    let mut incoming = actor(402);
    let mut rng = incoming.seed_actor_storage_runtime(false);
    let (error, mut returned) = manager
        .admit_fresh_creature_actor(MapKey::new(530, 7), incoming)
        .unwrap_err();
    assert!(matches!(
        error,
        FreshCreatureActorMapAdmissionError::Admission(FreshCreatureActorAdmissionError::Store(
            crate::MapObjectStoreError::WrongMap { .. }
        ))
    ));
    returned.assert_actor_storage_runtime(&mut rng, false);
    for map_id in [530, 571] {
        assert_eq!(
            manager
                .find_map(map_id, 7)
                .unwrap()
                .map()
                .map_object_count(),
            0
        );
    }
    let guid = returned.guid();
    let admitted = manager
        .admit_fresh_creature_actor(MapKey::new(571, 7), returned)
        .unwrap();
    let FreshCreatureActorAdmission::Inserted { outcome } = admitted else {
        panic!("owned retry must insert into the correct existing map");
    };
    assert!(outcome.inserted && outcome.inserted_into_cell);
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .object_is_in_world(guid)
    );
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .creature_actor_mut(guid)
        .unwrap()
        .assert_actor_storage_runtime(&mut rng, false);
    assert_eq!(
        manager.find_map(530, 7).unwrap().map().map_object_count(),
        0
    );
}

#[test]
fn manager_forwards_duplicate_actor_and_record_without_promoting_or_displacing() {
    for record in [false, true] {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
        let key = MapKey::new(571, 7);
        let map = manager.create_world_map(571, 7).map_mut();
        let current = actor(403);
        let guid = current.guid();
        if record {
            map.insert_map_object_record(MapObjectRecord::new_creature(current.creature).unwrap())
                .unwrap();
        } else {
            assert!(matches!(
                map.admit_fresh_creature_actor(current).unwrap(),
                FreshCreatureActorAdmission::Inserted { .. }
            ));
        }
        let mut incoming = actor(403);
        let mut rng = incoming.seed_actor_storage_runtime(true);
        let mut returned = match manager.admit_fresh_creature_actor(key, incoming).unwrap() {
            FreshCreatureActorAdmission::ExistingRecord { incoming } if record => incoming,
            FreshCreatureActorAdmission::ExistingActor { incoming } if !record => incoming,
            other => panic!("duplicate classification must pass through: {other:?}"),
        };
        returned.assert_actor_storage_runtime(&mut rng, true);
        let map = manager.find_map(571, 7).unwrap().map();
        assert_eq!(map.map_object_count(), 1);
        assert_eq!(map.creature_spawn_id_store_guids_like_cpp(403), vec![guid]);
        assert_eq!(map.creature_actor(guid).is_some(), !record);
        assert_eq!(map.object_is_in_world(guid), !record);
    }
}
