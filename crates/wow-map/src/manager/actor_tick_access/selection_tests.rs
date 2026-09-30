//! Exact selection facts, lazy loaded-cell capture and callback admission.

use super::*;
use crate::manager::ActorTickAccessError;
use crate::manager::actor_tick_access::fixtures::*;
use std::cell::Cell as CounterCell;
use wow_core::{Position, guid::HighGuid};
use wow_entities::{MapObjectRecord, Pet, PetType};

type LoadRecord = fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>;

fn finish(
    manager: &mut MapManager,
    mut tick: MapObjectTickContinuation,
    token: ObjectMapUpdateToken,
) {
    match manager.try_finish_object_map::<LoadRecord>(
        &mut tick,
        token,
        None,
        None,
        MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
    ) {
        Ok(outcome) => assert_eq!(outcome, ObjectMapFinishOutcome::Completed),
        Err((error, _token)) => panic!("settled fixture must finish: {error:?}"),
    }
}

#[test]
fn nearby_actor_selection_retains_original_plan_and_rejects_later_nearby_admission() {
    let (mut manager, center) = manager_with_actor(501);
    let neighbor = insert_actor(&mut manager, 502, Position::xyz(11.0, 20.0, 30.0), false);
    let far = insert_actor(
        &mut manager,
        503,
        Position::xyz(4000.0, 4000.0, 30.0),
        false,
    );
    let record = new_actor(504, Position::xyz(12.0, 20.0, 30.0), false).creature;
    let record_guid = record.guid();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    map.insert_map_object_record(MapObjectRecord::new_creature(record).unwrap())
        .unwrap();
    place_in_loaded_cell(map, record_guid, Position::xyz(12.0, 20.0, 30.0));
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    let original = token
        .continuation
        .nearby_object_plan
        .as_ref()
        .unwrap()
        .update_guids
        .clone();
    assert!(original.contains(&center));
    assert!(original.contains(&neighbor));
    assert!(original.contains(&record_guid));
    assert!(!original.contains(&far));
    assert!(token.continuation.actor_workset.is_none());
    let late = insert_actor(&mut manager, 505, Position::xyz(13.0, 20.0, 30.0), false);
    let expected: Vec<_> = original
        .iter()
        .copied()
        .filter(|guid| *guid != record_guid)
        .collect();
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        expected
    );
    let mut visited = Vec::new();
    for guid in &expected {
        manager
            .with_selected_actor(&tick, &mut token, *guid, None, |actor| {
                visited.push(actor.guid());
            })
            .unwrap();
    }
    assert_eq!(visited, expected);
    let rejected = CounterCell::new(0);
    for guid in [far, late, record_guid] {
        assert_eq!(
            manager
                .with_selected_actor(&tick, &mut token, guid, None, |_| {
                    rejected.set(rejected.get() + 1);
                })
                .unwrap_err(),
            ActorTickAccessError::OutsideSelection { guid }
        );
    }
    assert_eq!(rejected.get(), 0);
    assert_eq!(
        token
            .continuation
            .nearby_object_plan
            .as_ref()
            .unwrap()
            .update_guids,
        original
    );
    assert_eq!(manager.updater.pending_requests, 1);
    assert!(
        !manager
            .find_map(1, 0)
            .unwrap()
            .last_map_update_tail_summary_like_cpp()
            .script_hook
            .invoked
    );
    finish(&mut manager, tick, token);
}

#[test]
fn actor_workset_keeps_supplied_nearby_plan_order_without_sorting_again() {
    let (mut manager, first) = manager_with_actor(506);
    let second = insert_actor(&mut manager, 507, Position::xyz(11.0, 20.0, 30.0), false);
    let third = insert_actor(&mut manager, 508, Position::xyz(12.0, 20.0, 30.0), false);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    // Supply an explicit plan order through private test access. The actor
    // family must consume the plan it owns, rather than regenerate/sort it.
    let order = vec![third, first, second];
    token
        .continuation
        .nearby_object_plan
        .as_mut()
        .unwrap()
        .update_guids = order.clone();
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        order
    );
    finish(&mut manager, tick, token);
}

#[test]
fn whole_store_actor_selection_starts_at_first_access_and_uses_only_loaded_exact_actors() {
    let (mut manager, first) = manager_with_actor(509);
    let unloaded_actor = new_actor(510, Position::xyz(-4000.0, -4000.0, 30.0), false);
    let unloaded = unloaded_actor.guid();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    assert!(matches!(
        map.admit_creature_actor(unloaded_actor).unwrap(),
        crate::map::CreatureActorAdmission::Inserted { .. }
    ));
    let record = new_actor(511, Position::xyz(12.0, 20.0, 30.0), false).creature;
    let record_guid = record.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(record).unwrap())
        .unwrap();
    place_in_loaded_cell(map, record_guid, Position::xyz(12.0, 20.0, 30.0));
    let generic = new_actor(512, Position::xyz(13.0, 20.0, 30.0), false).creature;
    let generic_guid = generic.guid();
    map.insert_map_object_record(
        MapObjectRecord::new(AccessorObjectKind::Creature, generic.unit().world().clone()).unwrap(),
    )
    .unwrap();
    place_in_loaded_cell(map, generic_guid, Position::xyz(13.0, 20.0, 30.0));
    let mut pet = Pet::new(ObjectGuid::EMPTY, PetType::Hunter);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 1, 0, 42, 513);
    let world = pet.creature_mut().unit_mut().world_mut();
    world.object_mut().create(pet_guid);
    world.set_map(1, 0).unwrap();
    world.relocate(Position::xyz(14.0, 20.0, 30.0));
    world.object_mut().add_to_world();
    map.insert_map_object_record(MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
    place_in_loaded_cell(map, pet_guid, Position::xyz(14.0, 20.0, 30.0));
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    assert!(token.continuation.nearby_object_plan.is_none());
    assert!(token.continuation.actor_workset.is_none());
    // This admission is after map prepare but before the actual actor start.
    let before_start = insert_actor(&mut manager, 514, Position::xyz(15.0, 20.0, 30.0), false);
    let loaded = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .admitted_creature_guids_like_cpp();
    assert!(loaded.contains(&record_guid));
    assert!(loaded.contains(&generic_guid));
    assert!(loaded.contains(&pet_guid));
    assert!(!loaded.contains(&unloaded));
    let selected = manager.selected_actor_guids(&tick, &mut token).unwrap();
    assert_eq!(selected, vec![first, before_start]);
    let after_start = insert_actor(&mut manager, 515, Position::xyz(16.0, 20.0, 30.0), false);
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        selected
    );
    let calls = CounterCell::new(0);
    for guid in [unloaded, record_guid, generic_guid, pet_guid, after_start] {
        assert_eq!(
            manager
                .with_selected_actor(&tick, &mut token, guid, None, |_| {
                    calls.set(calls.get() + 1);
                })
                .unwrap_err(),
            ActorTickAccessError::OutsideSelection { guid }
        );
    }
    assert_eq!(calls.get(), 0);
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(unloaded)
            .unwrap()
            .creature
            .current_health(),
        75
    );
    finish(&mut manager, tick, token);
}

#[test]
fn unrelated_expected_witness_rejects_selected_actor_and_cannot_reserve_a_slot() {
    let (mut manager, guid) = manager_with_actor(516);
    let other = insert_actor(&mut manager, 517, Position::xyz(11.0, 20.0, 30.0), false);
    let unrelated = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor_witness(other)
        .unwrap();
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let calls = CounterCell::new(0);
    assert_eq!(
        manager
            .with_selected_actor(&tick, &mut token, guid, Some(&unrelated), |_| {
                calls.set(calls.get() + 1);
            })
            .unwrap_err(),
        ActorTickAccessError::WitnessMismatch { guid }
    );
    assert_eq!(
        manager
            .begin_actor_operation(&tick, &mut token, guid, Some(&unrelated))
            .unwrap_err(),
        ActorTickAccessError::WitnessMismatch { guid }
    );
    assert!(token.actor_operation.is_none());
    assert_eq!(calls.get(), 0);
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(guid)
            .unwrap()
            .creature
            .current_health(),
        75
    );
    finish(&mut manager, tick, token);
}

#[test]
fn continuation_diff_mismatch_returns_token_before_selection_callback_or_finish_effects() {
    let (mut manager, guid) = manager_with_actor(518);
    let (mut tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    token.continuation.diff_ms = 201;
    let expected = ObjectMapTickError::TokenMismatch { key: token.key() };
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap_err(),
        ActorTickAccessError::Tick(expected)
    );
    let calls = CounterCell::new(0);
    assert_eq!(
        manager
            .with_selected_actor(&tick, &mut token, guid, None, |_| {
                calls.set(calls.get() + 1);
            })
            .unwrap_err(),
        ActorTickAccessError::Tick(expected)
    );
    let (error, returned) = manager
        .try_finish_object_map::<LoadRecord>(
            &mut tick,
            token,
            None,
            None,
            MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
        )
        .unwrap_err();
    assert_eq!(error, expected);
    token = returned;
    assert_eq!(token.continuation.diff_ms, 201);
    assert!(token.continuation.actor_workset.is_none());
    assert_eq!(calls.get(), 0);
    assert_eq!(manager.updater.pending_requests, 1);
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .last_creatures_update_summary()
            .visited,
        0
    );
    assert!(
        !manager
            .find_map(1, 0)
            .unwrap()
            .last_map_update_tail_summary_like_cpp()
            .script_hook
            .invoked
    );
    token.continuation.diff_ms = 200;
    finish(&mut manager, tick, token);
}
