//! No mutation before invalid token/identity gates; owned inputs survive failure.
use super::*;
use crate::manager::MapTickCoordinationStateLikeCpp;

#[test]
fn foreign_origin_wrong_epoch_and_selected_record_reject_before_any_catalog() {
    let (mut manager, guid, _) = setup(821_001);
    let (tick, mut token) = fixtures::start(&mut manager, 77, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let (mut other, _, _) = setup(821_001);
    let (foreign, _) = fixtures::start(&mut other, 99, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let trace = Trace::default(); let before = prefix(&manager, guid);
    assert!(policy(Settings::default(), &trace, |policies| manager.prepare_spell(&foreign, &mut token, true, policies)).is_err());
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
    assert!(policy(Settings::default(), &trace, |policies| manager.prepare_spell(&tick, &mut token, true, policies)).is_err());
    assert_eq!(prefix(&manager, guid), before);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(1);
    // Freeze the actual Actor admission before replacing it with a Record.
    // A Record never supplies an Actor witness during initial workset capture.
    assert_eq!(manager.selected_actor_guids(&tick, &mut token).unwrap(), vec![guid]);
    let owned = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid).unwrap();
    let record = fixtures::new_actor(821_001, Position::xyz(10.0, 20.0, 30.0), true).creature;
    manager.find_map_mut(1, 0).unwrap().map_mut().insert_map_object_record(MapObjectRecord::new_creature(record).unwrap()).unwrap();
    assert!(policy(Settings::default(), &trace, |policies| manager.prepare_spell(&tick, &mut token, true, policies)).is_err());
    assert!(trace.borrow().is_empty()); assert!(token.actor_operation.is_none());
    drop(owned);
}

#[test]
fn wrong_origin_and_epoch_resume_keep_same_continuation_and_reply() {
    let (mut manager, tick, mut token, guid, _, request) = pending(821_010);
    let original = request.partial().completions.as_ptr(); let before = prefix(&manager, guid);
    let (mut other, _, _) = setup(821_010);
    let (foreign_tick, mut foreign_token) = fixtures::start(&mut other, 99, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let trace = Trace::default();
    let failure = policy(Settings::default(), &trace, |policies| other.resume_spell_los(&foreign_tick,
        &mut foreign_token, request.into_continuation(), false, policies)).err().unwrap();
    assert!(!failure.response); assert_eq!(failure.continuation.partial().completions.as_ptr(), original);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
    let failure = policy(Settings::default(), &trace, |policies| manager.resume_spell_los(&tick,
        &mut token, failure.continuation, failure.response, policies)).err().unwrap();
    assert!(!failure.response); assert_eq!(prefix(&manager, guid), before);
    assert!(trace.borrow().is_empty()); assert!(token.actor_operation.is_some());
    drop(failure);
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn actor_aba_resume_rejects_then_owned_reply_disposes_only_original_slot() {
    let (mut manager, tick, mut token, guid, _, request) = pending(821_020);
    let owned = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid).unwrap();
    assert_eq!(fixtures::insert_actor(&mut manager, 821_020, Position::xyz(10.0, 20.0, 30.0), true), guid);
    let before = prefix(&manager, guid); let trace = Trace::default();
    let failure = policy(Settings::default(), &trace, |policies| manager.resume_spell_los(&tick,
        &mut token, request.into_continuation(), true, policies)).err().unwrap();
    assert!(failure.response); assert!(trace.borrow().is_empty());
    assert_eq!(prefix(&manager, guid), before); assert!(token.actor_operation.is_some());
    let partial = manager.discard_spell_los_reply(&tick, &mut token, failure.continuation, failure.response).unwrap();
    assert_eq!(partial.casts_ready, 1); assert_eq!(partial.canonical_cast_preconditions_passed, 0);
    assert_eq!(prefix(&manager, guid), before); assert!(token.actor_operation.is_none());
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    drop(owned);
}

#[test]
fn player_readmission_is_checked_before_post_los_catalogs_or_rng() {
    let (mut manager, tick, mut token, guid, victim, request) = pending(821_030);
    let old_handle = manager.current_player_admission_like_cpp(victim).unwrap().0;
    let retired = manager.retire_player_like_cpp(old_handle).unwrap();
    assert_eq!(add_player(&mut manager, 821_031), victim);
    let before = prefix(&manager, guid); let trace = Trace::default();
    let failure = policy(Settings::default(), &trace, |policies| manager.resume_spell_los(&tick,
        &mut token, request.into_continuation(), true, policies)).err().unwrap();
    assert_eq!(failure.error, ActorSpellError::PlayerIdentityMismatch { guid: victim });
    assert!(trace.borrow().is_empty()); assert_eq!(prefix(&manager, guid), before);
    manager.discard_spell_los_reply(&tick, &mut token, failure.continuation, failure.response).unwrap();
    drop(retired);
}

#[test]
fn stale_map_unlaunched_disposition_preserves_replacement_and_manager_busy() {
    let (mut manager, tick, mut token, guid, _, request) = pending(821_040);
    let old = manager.maps.remove(&crate::MapKey::new(1, 0)).unwrap();
    manager.create_world_map(1, 0);
    fixtures::insert_actor(&mut manager, 821_040, Position::xyz(10.0, 20.0, 30.0), true);
    let before = prefix(&manager, guid);
    let partial = manager.discard_spell_request(&tick, &mut token, request).ok().unwrap();
    assert_eq!(partial.canonical_cast_preconditions_passed, 0);
    assert_eq!(prefix(&manager, guid), before); assert!(token.actor_operation.is_none());
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    drop(old);
}

#[test]
fn second_prepare_and_tail_are_blocked_drop_never_settles() {
    let (mut manager, mut tick, mut token, guid, _, request) = pending(821_050);
    let trace = Trace::default(); let before = prefix(&manager, guid);
    assert!(policy(Settings::default(), &trace, |policies| manager.prepare_spell(&tick, &mut token, false, policies)).is_err());
    assert!(trace.borrow().is_empty());
    let result = manager.try_finish_object_map::<fn(&mut crate::map::Map, crate::SpawnObjectType, crate::SpawnId)
        -> Option<crate::map::LoadedGridRespawnRecordsLikeCpp>>(&mut tick, token, None, None,
            crate::manager::MapCreatureUpdateOwnerLikeCpp::ExternalRuntime);
    let (error, recovered) = result.err().unwrap(); token = recovered;
    assert!(matches!(error, ObjectMapTickError::ActorOperationInFlight { .. }));
    assert_eq!(prefix(&manager, guid), before);
    drop(request);
    assert!(token.actor_operation.is_some()); assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn foreign_disposition_returns_original_request_and_does_not_clear_any_slot() {
    let (mut manager, tick, mut token, guid, _, request) = pending(821_060);
    let (mut other, other_tick, mut other_token, _, _, other_request) = pending(821_060);
    let original = request.query();
    let (error, request) = other.discard_spell_request(&other_tick, &mut other_token, request).err().unwrap();
    assert!(matches!(error, ActorSpellError::Access(ActorTickAccessError::OperationMismatch { .. })));
    assert_eq!(request.query(), original); assert!(other_token.actor_operation.is_some());
    assert!(token.actor_operation.is_some()); let before = prefix(&manager, guid);
    manager.discard_spell_request(&tick, &mut token, request).ok().unwrap();
    assert_eq!(prefix(&manager, guid), before);
    drop(other_request);
}

#[test]
fn legitimate_full_snapshot_keeps_actor_witness_motor_and_resume() {
    let (mut manager, tick, mut token, guid, _, request) = pending(821_070);
    let before = prefix(&manager, guid);
    let map = manager.find_map(1, 0).unwrap().map();
    let witness = map.creature_actor_witness(guid).unwrap();
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    // The historical entity snapshot clones its authorities. It never clones
    // WorldCreature/MotionMaster/RNG or substitutes a cloned motor for a move.
    let snapshot = actor(&manager, guid).creature.clone();
    manager.find_map_mut(1, 0).unwrap().map_mut().replace_creature_snapshot(MapObjectRecord::new_creature(snapshot).unwrap()).unwrap();
    assert!(witness.same_actor(&manager.find_map(1, 0).unwrap().map().creature_actor_witness(guid).unwrap()));
    assert_eq!(actor(&manager, guid) as *const WorldCreature, pointer);
    let outcome = complete!(&mut manager, &tick, &mut token, Settings::default(), &Trace::default(), policy(Settings::default(), &Trace::default(), |policies| manager.resume_spell_los(
        &tick, &mut token, request.into_continuation(), true, policies)).unwrap());
    assert_eq!(outcome.canonical_cast_preconditions_passed, 1); assert_eq!(prefix(&manager, guid), before);
}
