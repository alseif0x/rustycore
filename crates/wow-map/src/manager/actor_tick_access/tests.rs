//! Token gates, recoverable finish and outstanding actor-operation ownership.

use super::*;
use crate::manager::ActorTickAccessError;
use crate::manager::actor_tick_access::fixtures::*;
use std::cell::Cell as CounterCell;
use wow_core::Position;
use wow_entities::{AccessorObjectKind, MapObjectRecord};

type LoadRecord = fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>;

fn finish(
    manager: &mut MapManager,
    tick: &mut MapObjectTickContinuation,
    token: ObjectMapUpdateToken,
) -> ObjectMapFinishOutcome {
    match manager.try_finish_object_map::<LoadRecord>(
        tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
    ) {
        Ok(outcome) => outcome,
        Err((error, _token)) => panic!("fixture must finish after settlement: {error:?}"),
    }
}

fn assert_no_tail(manager: &MapManager) {
    let map = manager.find_map(1, 0).unwrap();
    assert_eq!(map.last_creatures_update_summary().visited, 0);
    assert!(!map.last_map_update_tail_summary_like_cpp().script_hook.invoked);
    assert!(map.delayed_update_calls().is_empty());
    assert_eq!(manager.updater.pending_requests, 1);
    assert_eq!(manager.updater.scheduled_updates, 1);
    assert!(matches!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Resuming(_)));
}

#[test]
fn actor_access_rejects_foreign_tick_and_token_before_callback_and_returns_finish_ownership() {
    let (mut first, guid) = manager_with_actor(401);
    let (mut second, _) = manager_with_actor(401);
    let (mut first_tick, mut first_token) = start(&mut first, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let (mut second_tick, mut second_token) = start(&mut second, 300, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let calls = CounterCell::new(0);
    for tick in [&first_tick, &second_tick] {
        assert!(matches!(second.with_selected_actor(tick, &mut first_token, guid, None, |_| {
            calls.set(calls.get() + 1);
        }), Err(ActorTickAccessError::Tick(ObjectMapTickError::OriginMismatch { plan_epoch: 1 }))));
    }
    assert_eq!(calls.get(), 0);
    let (error, returned) = second.try_finish_object_map::<LoadRecord>(
        &mut second_tick, first_token, None, None, MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
    ).unwrap_err();
    assert_eq!(error, ObjectMapTickError::OriginMismatch { plan_epoch: 1 });
    first_token = returned;
    assert_no_tail(&first);
    assert_no_tail(&second);
    let (health, witness) = first.with_selected_actor(&first_tick, &mut first_token, guid, None, |actor| {
        actor.creature.unit_mut().set_health(31);
        actor.creature.current_health()
    }).unwrap();
    assert_eq!(health, 31);
    assert!(witness.same_actor(&first.find_map(1, 0).unwrap().map().creature_actor_witness(guid).unwrap()));
    assert_eq!(second.selected_actor_guids(&second_tick, &mut second_token).unwrap(), vec![guid]);
    assert_eq!(finish(&mut first, &mut first_tick, first_token), ObjectMapFinishOutcome::Completed);
    assert_eq!(finish(&mut second, &mut second_tick, second_token), ObjectMapFinishOutcome::Completed);
}

#[test]
fn actor_token_gates_preserve_error_priority_and_recoverable_accounting_before_any_callback() {
    for case in 0..11 {
        let (mut manager, guid) = manager_with_actor(402);
        let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
        let token_origin = Arc::clone(&token.origin);
        let tick_origin = Arc::clone(&tick.origin);
        let epoch = token.epoch;
        let participant = token.participant;
        let participant_index = token.participant_index;
        let accounting_started = token.accounting_started;
        let in_flight = tick.in_flight;
        let diff_ms = token.effective_diff_ms;
        let expected = match case {
            0 => {
                token.origin = Arc::new(());
                ObjectMapTickError::OriginMismatch { plan_epoch: epoch }
            }
            1 => {
                token.epoch += 1;
                ObjectMapTickError::WrongEpoch { expected_epoch: epoch, actual_epoch: epoch + 1 }
            }
            2 => {
                tick.in_flight = None;
                ObjectMapTickError::NoMapInFlight
            }
            3 => {
                token.participant_index += 1;
                ObjectMapTickError::TokenMismatch { key: participant.key }
            }
            4 => {
                token.participant.key = MapKey::new(2, 0);
                ObjectMapTickError::TokenMismatch { key: token.key() }
            }
            5 => {
                token.accounting_started = !accounting_started;
                ObjectMapTickError::TokenMismatch { key: participant.key }
            }
            6 => {
                token.effective_diff_ms += 1;
                ObjectMapTickError::TokenMismatch { key: participant.key }
            }
            7 => {
                tick.origin = Arc::new(());
                token.epoch += 1;
                ObjectMapTickError::OriginMismatch { plan_epoch: epoch }
            }
            8 => {
                tick.epoch += 1;
                ObjectMapTickError::WrongEpoch { expected_epoch: epoch + 1, actual_epoch: epoch }
            }
            9 => {
                manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Idle;
                token.origin = Arc::new(());
                ObjectMapTickError::WrongState { expected_epoch: epoch, state: MapTickCoordinationStateLikeCpp::Idle }
            }
            10 => {
                manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(epoch + 7);
                ObjectMapTickError::WrongEpoch { expected_epoch: epoch, actual_epoch: epoch + 7 }
            }
            _ => unreachable!(),
        };
        let calls = CounterCell::new(0);
        assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, None, |_| {
            calls.set(calls.get() + 1);
        }).unwrap_err(), ActorTickAccessError::Tick(expected));
        let (error, returned) = manager.try_finish_object_map::<LoadRecord>(
            &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
        ).unwrap_err();
        assert_eq!(error, expected);
        assert_eq!(calls.get(), 0);
        token = returned;
        assert!(token.actor_operation.is_none());
        assert_eq!(manager.find_map(1, 0).unwrap().map().creature_actor(guid).unwrap().creature.current_health(), 75);
        assert_eq!(manager.updater.pending_requests, 1);
        assert!(!manager.find_map(1, 0).unwrap().last_map_update_tail_summary_like_cpp().script_hook.invoked);
        token.origin = token_origin;
        tick.origin = tick_origin;
        tick.epoch = epoch;
        token.epoch = epoch;
        token.participant = participant;
        token.participant_index = participant_index;
        token.accounting_started = accounting_started;
        token.effective_diff_ms = diff_ms;
        tick.in_flight = in_flight;
        manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(epoch);
        assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
        assert_eq!(manager.updater.pending_requests, 0);
        assert_eq!(manager.updater.scheduled_updates, 1);
    }
}

#[test]
fn actor_workset_rejects_readmission_and_new_guid_without_an_expected_witness() {
    let (mut manager, guid) = manager_with_actor(403);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    assert_eq!(manager.selected_actor_guids(&tick, &mut token).unwrap(), vec![guid]);
    let old_witness = manager.find_map(1, 0).unwrap().map().creature_actor_witness(guid).unwrap();
    let removed = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    assert!(matches!(map.admit_creature_actor(new_actor(403, Position::xyz(10.0, 20.0, 30.0), true)).unwrap(),
        crate::map::CreatureActorAdmission::Inserted { .. }));
    let current = map.creature_actor_witness(guid).unwrap();
    assert!(!old_witness.same_actor(&current));
    let newly_admitted = insert_actor(&mut manager, 404, Position::xyz(11.0, 20.0, 30.0), false);
    let calls = CounterCell::new(0);
    assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, None, |_| {
        calls.set(calls.get() + 1);
    }).unwrap_err(), ActorTickAccessError::WitnessMismatch { guid });
    assert_eq!(manager.with_selected_actor(&tick, &mut token, newly_admitted, None, |_| {
        calls.set(calls.get() + 1);
    }).unwrap_err(), ActorTickAccessError::OutsideSelection { guid: newly_admitted });
    assert_eq!(calls.get(), 0);
    assert_eq!(manager.selected_actor_guids(&tick, &mut token).unwrap(), vec![guid]);
    assert_no_tail(&manager);
    drop(removed);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
}

#[test]
fn actor_access_rejects_record_and_wrong_kind_replacement_without_invoking_callback() {
    for wrong_kind in [false, true] {
        let (mut manager, guid) = manager_with_actor(405);
        let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
        manager.selected_actor_guids(&tick, &mut token).unwrap();
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        let record = if wrong_kind {
            let mut object = new_actor(406, Position::xyz(10.0, 20.0, 30.0), false).creature.unit().world().clone();
            object.object_mut().create(ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::GameObject, 0, 1, 1, 0, 42, 405,
            ));
            let mut record = MapObjectRecord::new(AccessorObjectKind::GameObject, object).unwrap();
            record.object_mut().object_mut().create(guid);
            record
        } else {
            MapObjectRecord::new_creature(new_actor(405, Position::xyz(10.0, 20.0, 30.0), false).creature).unwrap()
        };
        let displaced = map.insert_map_object_record(record).unwrap();
        let calls = CounterCell::new(0);
        assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, None, |_| {
            calls.set(calls.get() + 1);
        }).unwrap_err(), ActorTickAccessError::ActorUnavailable { guid });
        assert_eq!(calls.get(), 0);
        assert_no_tail(&manager);
        drop(displaced);
        assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
    }
}

#[test]
fn pending_operation_survives_request_identity_drop_and_blocks_finish_without_tail_or_accounting() {
    let (mut manager, guid) = manager_with_actor(407);
    let other = insert_actor(&mut manager, 408, Position::xyz(11.0, 20.0, 30.0), false);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let witness = manager.begin_actor_operation(&tick, &mut token, guid, None).unwrap();
    let request_identity = ActorStepIdentity::new(&token, guid, witness.clone());
    drop(request_identity);
    assert!(token.actor_operation.is_some());
    assert_eq!(manager.begin_actor_operation(&tick, &mut token, other, None).unwrap_err(),
        ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight { guid }));
    let calls = CounterCell::new(0);
    assert_eq!(manager.with_selected_actor(&tick, &mut token, other, None, |_| {
        calls.set(calls.get() + 1);
    }).unwrap_err(), ActorTickAccessError::OperationMismatch { guid: other });
    let (error, returned) = manager.try_finish_object_map::<LoadRecord>(
        &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
    ).unwrap_err();
    assert_eq!(error, ObjectMapTickError::ActorOperationInFlight { guid });
    token = returned;
    assert!(token.actor_operation.as_ref().unwrap().witness.same_actor(&witness));
    assert_eq!(calls.get(), 0);
    assert_no_tail(&manager);
    assert!(manager.begin_tick_like_cpp(17).is_busy());
    assert_eq!(manager.timer.current(), 200);
    assert!(matches!(manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores),
        Err(ObjectMapTickError::MapInFlight { .. })));
    assert!(manager.resume_actor_operation(&tick, &mut token, guid, &witness).unwrap().same_actor(&witness));
    let (health, _) = manager.with_selected_actor(&tick, &mut token, guid, Some(&witness), |actor| {
        calls.set(calls.get() + 1);
        actor.creature.unit_mut().set_health(41);
        actor.creature.current_health()
    }).unwrap();
    assert_eq!((health, calls.get()), (41, 1));
    assert_eq!(manager.complete_actor_operation(&tick, &mut token, other, &witness).unwrap_err(),
        ActorTickAccessError::OperationMismatch { guid: other });
    assert!(token.actor_operation.is_some());
    manager.complete_actor_operation(&tick, &mut token, guid, &witness).unwrap();
    assert!(token.actor_operation.is_none());
    assert_eq!(manager.resume_actor_operation(&tick, &mut token, guid, &witness).unwrap_err(),
        ActorTickAccessError::NoActorOperation);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
    assert_eq!(manager.updater.pending_requests, 0);
    assert!(manager.find_map(1, 0).unwrap().last_map_update_tail_summary_like_cpp().script_hook.invoked);
    assert!(manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap().is_none());
    manager.finalize_object_tick(tick).unwrap();
    assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Idle);
    assert_eq!(manager.updater.wait_calls(), 1);
}

#[test]
fn pending_map_replacement_rejects_resume_and_busy_finish_then_settles_stale_accounting_once() {
    let (mut manager, guid) = manager_with_actor(409);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let witness = manager.begin_actor_operation(&tick, &mut token, guid, None).unwrap();
    let admitted_incarnation = token.incarnation();
    assert!(manager.destroy_map(1, 0));
    manager.create_world_map(1, 0);
    let current_incarnation = manager.map_incarnation_like_cpp(token.key());
    let calls = CounterCell::new(0);
    let expected = ActorTickAccessError::StaleParticipant { key: token.key(), admitted_incarnation, current_incarnation };
    assert_eq!(manager.resume_actor_operation(&tick, &mut token, guid, &witness).unwrap_err(), expected);
    assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, Some(&witness), |_| {
        calls.set(calls.get() + 1);
    }).unwrap_err(), expected);
    let (error, returned) = manager.try_finish_object_map::<LoadRecord>(
        &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
    ).unwrap_err();
    assert_eq!(error, ObjectMapTickError::ActorOperationInFlight { guid });
    token = returned;
    assert_eq!(calls.get(), 0);
    assert_no_tail(&manager);
    // The response has been disposed, not applied to the replacement actor.
    manager.complete_actor_operation(&tick, &mut token, guid, &witness).unwrap();
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::StaleParticipant {
        key: MapKey::new(1, 0), admitted_incarnation, current_incarnation,
    });
    assert_eq!(manager.updater.pending_requests, 0);
    assert_eq!(manager.updater.scheduled_updates, 1);
    assert!(!manager.find_map(1, 0).unwrap().last_map_update_tail_summary_like_cpp().script_hook.invoked);
}

#[test]
fn pending_same_guid_readmission_rejects_response_but_can_dispose_the_original_operation() {
    let (mut manager, guid) = manager_with_actor(410);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let witness = manager.begin_actor_operation(&tick, &mut token, guid, None).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    let removed = map.remove_map_object(guid).unwrap();
    map.admit_creature_actor(new_actor(410, Position::xyz(10.0, 20.0, 30.0), true)).unwrap();
    let replacement_witness = map.creature_actor_witness(guid).unwrap();
    assert!(!replacement_witness.same_actor(&witness));
    let calls = CounterCell::new(0);
    assert_eq!(manager.resume_actor_operation(&tick, &mut token, guid, &witness).unwrap_err(),
        ActorTickAccessError::WitnessMismatch { guid });
    assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, None, |_| {
        calls.set(calls.get() + 1);
    }).unwrap_err(), ActorTickAccessError::WitnessMismatch { guid });
    assert_eq!(manager.complete_actor_operation(&tick, &mut token, guid, &replacement_witness).unwrap_err(),
        ActorTickAccessError::OperationMismatch { guid });
    assert!(token.actor_operation.is_some());
    manager.complete_actor_operation(&tick, &mut token, guid, &witness).unwrap();
    assert_eq!(calls.get(), 0);
    assert_eq!(manager.find_map(1, 0).unwrap().map().creature_actor(guid).unwrap().creature.current_health(), 75);
    drop(removed);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
}

#[test]
fn missing_map_with_retained_incarnation_is_stale_and_finishes_without_actor_or_tail_effects() {
    let (mut manager, guid) = manager_with_actor(411);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let incarnation = token.incarnation();
    manager.maps.remove(&token.key());
    assert_eq!(manager.map_incarnation_like_cpp(token.key()), Some(incarnation));
    let calls = CounterCell::new(0);
    assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, None, |_| {
        calls.set(calls.get() + 1);
    }).unwrap_err(), ActorTickAccessError::StaleParticipant {
        key: token.key(), admitted_incarnation: incarnation, current_incarnation: Some(incarnation),
    });
    assert_eq!(calls.get(), 0);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::StaleParticipant {
        key: MapKey::new(1, 0), admitted_incarnation: incarnation, current_incarnation: Some(incarnation),
    });
    assert_eq!(manager.updater.pending_requests, 0);
    assert!(tick.in_flight.is_none());
    assert!(tick.resumed_keys.is_empty());
}

#[test]
fn pending_operation_identity_rejects_another_origin_epoch_index_or_participant_without_settlement() {
    for case in 0..4 {
        let (mut manager, guid) = manager_with_actor(413);
        let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
        let witness = manager.begin_actor_operation(&tick, &mut token, guid, None).unwrap();
        let operation = token.actor_operation.as_mut().unwrap();
        match case {
            0 => operation.origin = Arc::new(()),
            1 => operation.epoch += 1,
            2 => operation.participant_index += 1,
            3 => operation.participant.incarnation += 1,
            _ => unreachable!(),
        }
        let expected = ActorTickAccessError::OperationMismatch { guid };
        let calls = CounterCell::new(0);
        assert_eq!(manager.resume_actor_operation(&tick, &mut token, guid, &witness).unwrap_err(), expected);
        assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, Some(&witness), |_| {
            calls.set(calls.get() + 1);
        }).unwrap_err(), expected);
        assert_eq!(manager.complete_actor_operation(&tick, &mut token, guid, &witness).unwrap_err(), expected);
        assert_eq!(calls.get(), 0);
        assert!(token.actor_operation.is_some());
        let (error, returned) = manager.try_finish_object_map::<LoadRecord>(
            &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
        ).unwrap_err();
        assert_eq!(error, ObjectMapTickError::ActorOperationInFlight { guid });
        token = returned;
        assert_no_tail(&manager);
        // Repair only the private fixture identity, then dispose and finish.
        token.actor_operation = Some(ActorStepIdentity::new(&token, guid, witness.clone()));
        manager.complete_actor_operation(&tick, &mut token, guid, &witness).unwrap();
        assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
    }
}

#[test]
fn compatibility_finish_of_a_busy_token_remains_fail_stop_without_implicit_slot_settlement() {
    let (mut manager, guid) = manager_with_actor(412);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    manager.begin_actor_operation(&tick, &mut token, guid, None).unwrap();
    assert_eq!(manager.finish_object_map::<LoadRecord>(
        &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
    ), Err(ObjectMapTickError::ActorOperationInFlight { guid }));
    assert_no_tail(&manager);
    assert!(tick.in_flight.is_some());
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    assert_eq!(manager.timer.current(), 200);
}
