//! Fresh admission, ownership on rejection, and the existing lifecycle boundary.

use super::*;
use crate::{Cell, GridCoord, GridStateKind, NGrid};
use rand::rngs::StdRng;
use wow_core::{Position, guid::HighGuid};
use wow_entities::{Creature, MapObjectRecord, ObjectAccessorError, OwnedLootAuthorityLifecycle};

#[derive(Default)]
struct Terrain {
    loads: usize,
}
impl TerrainGridLoader for Terrain {
    fn load_map_and_vmap(&mut self, _: u32, _: u32) {
        self.loads += 1;
    }
    fn unload_map(&mut self, _: u32, _: u32) {}
}

#[derive(Default)]
struct Lifecycle {
    loads: usize,
}
impl GridLifecycle for Lifecycle {
    fn load_grid_objects(&mut self, _: &mut NGrid, _: &Cell) {
        self.loads += 1;
    }
    fn stop_grid_objects(&mut self, _: &NGrid) {}
    fn evacuate_grid(&mut self, _: &mut NGrid) {}
    fn clean_grid(&mut self, _: &mut NGrid) {}
    fn unload_grid_objects(&mut self, _: &mut NGrid) {}
}

fn map() -> Map<Terrain, Lifecycle> {
    Map::with_hooks(
        571,
        7,
        1,
        1000,
        true,
        100.0,
        Terrain::default(),
        Lifecycle::default(),
    )
}

fn creature(counter: i64, spawn_id: u64) -> Creature {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(spawn_id);
    creature
}

fn actor(counter: i64, spawn_id: u64, point: bool) -> (WorldCreature, StdRng) {
    let creature = creature(counter, spawn_id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let mut incoming = WorldCreature::from_canonical(creature, data);
    incoming.create_data.npc_flags = 0x1234;
    let rng = incoming.seed_actor_storage_runtime(point);
    assert!(!incoming.creature.unit().world().object().is_in_world());
    (incoming, rng)
}

fn inserted(map: &mut Map<Terrain, Lifecycle>, incoming: WorldCreature) -> AddToMapOutcome {
    match map.admit_fresh_creature_actor(incoming).unwrap() {
        FreshCreatureActorAdmission::Inserted { outcome } => outcome,
        other => panic!("fresh empty slot must insert: {other:?}"),
    }
}

fn assert_no_admission_effects(map: &Map<Terrain, Lifecycle>) {
    assert_eq!(map.map_object_count(), 0);
    assert!(map.grids.iter().all(Option::is_none));
    assert_eq!(map.terrain.loads, 0);
    assert_eq!(map.lifecycle.loads, 0);
    assert!(map.creatures_by_spawn_id.is_empty());
    assert!(map.active_cells.is_empty());
    assert!(map.active_non_players_like_cpp.is_empty());
    assert!(map.creature_group_holder_like_cpp.is_empty());
}

#[test]
fn fresh_rejection_returns_motor_and_loot_without_grid_cell_or_store_effects() {
    // Identity priority precedes fresh/coordinate checks. Invalid coordinates
    // include both axes, NaN and infinity, rather than only a finite boundary.
    for case in 0..9 {
        let mut map = map();
        let (mut incoming, mut rng) = actor(301, 3010, false);
        incoming.creature.unit_mut().world_mut().set_active(true);
        match case {
            0 => {
                incoming
                    .creature
                    .unit_mut()
                    .world_mut()
                    .reset_map()
                    .unwrap();
                incoming
                    .creature
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .create(ObjectGuid::EMPTY);
            }
            1 => incoming
                .creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(ObjectGuid::EMPTY),
            2 => incoming
                .creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(ObjectGuid::create_world_object(
                    HighGuid::Pet,
                    0,
                    1,
                    571,
                    7,
                    42,
                    301,
                )),
            3 => {
                incoming
                    .creature
                    .unit_mut()
                    .world_mut()
                    .reset_map()
                    .unwrap();
                incoming
                    .creature
                    .unit_mut()
                    .world_mut()
                    .set_map(530, 7)
                    .unwrap();
            }
            4 => incoming
                .creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .add_to_world(),
            5 => incoming
                .creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(20_000.0, 2.0, 3.0)),
            6 => incoming
                .creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(1.0, -20_000.0, 3.0)),
            7 => incoming
                .creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(f32::NAN, 2.0, 3.0)),
            8 => incoming
                .creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(1.0, f32::INFINITY, 3.0)),
            _ => unreachable!(),
        }
        let guid = incoming.guid();
        let loot = incoming.creature.loot_authority_like_cpp().clone();
        let (error, mut returned) = map.admit_fresh_creature_actor(incoming).unwrap_err();
        match case {
            0 => assert!(matches!(
                error,
                FreshCreatureActorAdmissionError::Store(MapObjectStoreError::InvalidRecord(
                    ObjectAccessorError::ObjectHasNoMap { .. }
                ))
            )),
            1 => assert!(matches!(
                error,
                FreshCreatureActorAdmissionError::Store(MapObjectStoreError::InvalidRecord(
                    ObjectAccessorError::UnsupportedGuidKind { .. }
                ))
            )),
            2 => assert!(matches!(
                error,
                FreshCreatureActorAdmissionError::Store(MapObjectStoreError::InvalidRecord(
                    ObjectAccessorError::WrongGuidKind { .. }
                ))
            )),
            3 => assert!(matches!(
                error,
                FreshCreatureActorAdmissionError::Store(MapObjectStoreError::WrongMap { .. })
            )),
            4 => assert_eq!(
                error,
                FreshCreatureActorAdmissionError::AlreadyInWorld { guid }
            ),
            _ => assert!(matches!(
                error,
                FreshCreatureActorAdmissionError::InvalidCoordinates { .. }
            )),
        }
        assert_eq!(returned.guid(), guid);
        assert_eq!(returned.create_data.npc_flags, 0x1234);
        assert!(
            returned
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&loot)
        );
        assert_ne!(
            loot.lifecycle_like_cpp(),
            OwnedLootAuthorityLifecycle::Detached
        );
        assert_eq!(
            returned.creature.unit().world().object().is_in_world(),
            case == 4
        );
        returned.assert_actor_storage_runtime(&mut rng, false);
        assert_no_admission_effects(&map);
    }
}

#[test]
fn fresh_inactive_and_active_admission_preserve_motor_and_use_original_cell_lifecycle() {
    for active in [false, true] {
        for point in [false, true] {
            let mut map = map();
            let (mut incoming, mut rng) = actor(302, 3020, point);
            incoming.creature.unit_mut().world_mut().set_active(active);
            let guid = incoming.guid();
            let loot = incoming.creature.loot_authority_like_cpp().clone();
            let outcome = inserted(&mut map, incoming);
            assert!(outcome.inserted);
            assert!(!outcome.already_in_world);
            assert_eq!(outcome.grid_created, !active);
            assert_eq!(outcome.grid_loaded, active);
            assert!(outcome.inserted_into_cell);
            assert_eq!(map.terrain.loads, 1);
            assert_eq!(map.lifecycle.loads, usize::from(active));
            let cell = Cell::from_world(1.0, 2.0);
            let grid = map.get_ngrid(outcome.grid).unwrap();
            assert_eq!(grid.grid_object_data_loaded(), active);
            assert_eq!(
                grid.state(),
                if active {
                    GridStateKind::Active
                } else {
                    GridStateKind::Idle
                }
            );
            let local = grid.get_grid_type(cell.cell_x(), cell.cell_y()).unwrap();
            assert!(local.grid_objects.creatures.contains(&guid));
            assert!(!local.world_objects.creatures.contains(&guid));
            assert_eq!(map.creature_spawn_id_store_guids_like_cpp(3020), vec![guid]);
            assert_eq!(
                outcome.creature_store_inserted_before_add_to_world,
                Some(true)
            );
            assert_eq!(
                outcome.creature_spawn_indexed_before_add_to_world,
                Some(true)
            );
            let unit = outcome.creature_unit_add_to_world.unwrap();
            assert!(unit.world_object_added && unit.is_in_world_after);
            assert!(outcome.creature_search_formation.is_some());
            assert!(outcome.creature_aim_initialize.is_some());
            assert!(outcome.creature_vehicle_reset.is_none());
            assert!(outcome.creature_vehicle_install.is_none());
            assert!(
                !outcome
                    .creature_zone_script_create
                    .unwrap()
                    .script_dispatch_represented
            );
            let tail = outcome.add_to_map_tail.unwrap();
            assert!(tail.initialize_object_represented);
            assert!(tail.no_pending_move_state);
            assert_eq!(tail.add_to_active_represented, active);
            assert!(tail.set_is_new_object_true && tail.set_is_new_object_false);
            assert!(tail.update_object_visibility_on_create_represented);
            assert!(tail.update_object_visibility_on_create_runtime_gap);
            assert!(!tail.final_is_new_object);
            assert_eq!(map.is_active_non_player_like_cpp(guid), active);
            assert_eq!(map.map_object_count(), 1);
            let stored = map.creature_actor_mut(guid).unwrap();
            assert_eq!(stored.create_data.npc_flags, 0x1234);
            assert!(
                stored
                    .creature
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(&loot)
            );
            stored.assert_actor_storage_runtime(&mut rng, point);
        }
    }
}

#[test]
fn duplicate_actor_returns_incoming_without_reentry_displacement_or_authority_detach() {
    let mut map = map();
    let (current, mut current_rng) = actor(303, 3030, true);
    let guid = current.guid();
    let current_loot = current.creature.loot_authority_like_cpp().clone();
    inserted(&mut map, current);
    let witness = map.creature_actor_witness(guid).unwrap();
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    map.creature_actor_mut(guid)
        .unwrap()
        .creature
        .unit_mut()
        .set_health(25);
    let (mut incoming, mut incoming_rng) = actor(303, 3031, false);
    // An active incoming must not load the existing inactive actor's grid.
    incoming.creature.unit_mut().world_mut().set_active(true);
    let incoming_loot = incoming.creature.loot_authority_like_cpp().clone();
    let mut returned = match map.admit_fresh_creature_actor(incoming).unwrap() {
        FreshCreatureActorAdmission::ExistingActor { incoming } => incoming,
        other => panic!("expected actor duplicate: {other:?}"),
    };
    assert!(!returned.creature.unit().world().object().is_in_world());
    returned.assert_actor_storage_runtime(&mut incoming_rng, false);
    assert!(
        returned
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&incoming_loot)
    );
    assert_ne!(
        incoming_loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert_eq!(
        map.creature_actor(guid).unwrap() as *const WorldCreature,
        pointer
    );
    assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
    let stored = map.creature_actor_mut(guid).unwrap();
    assert_eq!(stored.creature.current_health(), 25);
    assert!(
        stored
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&current_loot)
    );
    stored.assert_actor_storage_runtime(&mut current_rng, true);
    assert_ne!(
        current_loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(3030), vec![guid]);
    assert!(map.creature_spawn_id_store_guids_like_cpp(3031).is_empty());
    assert_eq!(map.terrain.loads, 1);
    assert_eq!(map.lifecycle.loads, 0);
    assert!(!map.is_active_non_player_like_cpp(guid));
}

#[test]
fn duplicate_record_returns_motor_without_promotion_grid_or_lifecycle() {
    let mut map = map();
    let mut current = creature(304, 3040);
    current.unit_mut().set_health(18);
    let guid = current.guid();
    let loot = current.loot_authority_like_cpp().clone();
    map.insert_map_object_record(MapObjectRecord::new_creature(current).unwrap())
        .unwrap();
    let pointer = map.get_typed_creature(guid).unwrap() as *const Creature;
    let (mut incoming, mut rng) = actor(304, 3041, true);
    incoming.creature.unit_mut().world_mut().set_active(true);
    let mut returned = match map.admit_fresh_creature_actor(incoming).unwrap() {
        FreshCreatureActorAdmission::ExistingRecord { incoming } => incoming,
        other => panic!("record must not be promoted: {other:?}"),
    };
    returned.assert_actor_storage_runtime(&mut rng, true);
    assert!(map.creature_actor(guid).is_none());
    assert!(map.creature_actor_witness(guid).is_none());
    assert_eq!(map.get_typed_creature(guid).unwrap().current_health(), 18);
    assert_eq!(
        map.get_typed_creature(guid).unwrap() as *const Creature,
        pointer
    );
    assert!(
        map.get_typed_creature(guid)
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&loot)
    );
    assert_ne!(
        loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(3040), vec![guid]);
    assert!(map.creature_spawn_id_store_guids_like_cpp(3041).is_empty());
    assert!(!map.object_is_in_world(guid));
    assert!(map.grids.iter().all(Option::is_none));
    assert_eq!(map.terrain.loads, 0);
    assert_eq!(map.lifecycle.loads, 0);
}

#[test]
fn generic_same_guid_record_is_rejected_with_owned_incoming_and_no_displacement() {
    let mut map = map();
    let (incoming, mut rng) = actor(305, 3050, false);
    let guid = incoming.guid();
    let mut object = incoming.creature.unit().world().clone();
    object.relocate(Position::xyz(9.0, 8.0, 7.0));
    map.insert_map_object(AccessorObjectKind::Creature, object)
        .unwrap();
    let (error, mut returned) = map.admit_fresh_creature_actor(incoming).unwrap_err();
    assert_eq!(
        error,
        FreshCreatureActorAdmissionError::NotExactCreature {
            guid,
            actual_kind: AccessorObjectKind::Creature,
        }
    );
    returned.assert_actor_storage_runtime(&mut rng, false);
    assert_eq!(
        map.map_object(guid).unwrap().position(),
        Position::xyz(9.0, 8.0, 7.0)
    );
    assert!(map.creature_actor(guid).is_none());
    assert_eq!(map.map_object_count(), 1);
    assert!(map.creature_spawn_id_store_guids_like_cpp(3050).is_empty());
    assert!(map.grids.iter().all(Option::is_none));
    assert_eq!(map.terrain.loads, 0);
    assert_eq!(map.lifecycle.loads, 0);
}

#[test]
fn in_world_incoming_is_rejected_before_duplicate_classification_or_lifecycle() {
    let mut map = map();
    let (current, _) = actor(306, 3060, true);
    let guid = current.guid();
    inserted(&mut map, current);
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let (mut incoming, mut rng) = actor(306, 3061, false);
    incoming
        .creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    incoming.creature.unit_mut().world_mut().set_active(true);
    let (error, mut returned) = map.admit_fresh_creature_actor(incoming).unwrap_err();
    assert_eq!(
        error,
        FreshCreatureActorAdmissionError::AlreadyInWorld { guid }
    );
    assert!(returned.creature.unit().world().object().is_in_world());
    returned.assert_actor_storage_runtime(&mut rng, false);
    assert_eq!(
        map.creature_actor(guid).unwrap() as *const WorldCreature,
        pointer
    );
    assert_eq!(map.lifecycle.loads, 0);
    assert!(map.creature_spawn_id_store_guids_like_cpp(3061).is_empty());
}

#[test]
fn transient_fresh_admission_does_not_invent_a_persistent_spawn_index() {
    let mut map = map();
    let (mut incoming, mut rng) = actor(307, 0, true);
    let guid = incoming.guid();
    incoming.creature.unit_mut().world_mut().set_active(true);
    let outcome = inserted(&mut map, incoming);
    assert_eq!(
        outcome.creature_spawn_indexed_before_add_to_world,
        Some(false)
    );
    assert!(map.creatures_by_spawn_id.is_empty());
    assert!(map.is_active_non_player_like_cpp(guid));
    assert!(map.creature_spawn_id_store_guids_like_cpp(0).is_empty());
    map.creature_actor_mut(guid)
        .unwrap()
        .assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn fresh_admission_reuses_loaded_grid_without_an_extra_load() {
    let mut map = map();
    let cell = Cell::from_world(1.0, 2.0);
    assert!(map.ensure_grid_loaded(&cell));
    let (mut incoming, mut rng) = actor(308, 3080, false);
    let guid = incoming.guid();
    incoming.creature.unit_mut().world_mut().set_active(true);
    let outcome = inserted(&mut map, incoming);
    assert!(!outcome.grid_loaded);
    assert!(!outcome.grid_created);
    assert_eq!(outcome.grid, GridCoord::new(cell.grid_x(), cell.grid_y()));
    assert_eq!(map.terrain.loads, 1);
    assert_eq!(map.lifecycle.loads, 1);
    assert!(map.is_active_non_player_like_cpp(guid));
    map.creature_actor_mut(guid)
        .unwrap()
        .assert_actor_storage_runtime(&mut rng, false);
}

mod lifecycle;
