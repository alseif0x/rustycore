//! Recoverable preflight and exactly-once final suffix on real admitted work.
use super::*;
use crate::map_manager::{pending_respawn_from_world_creature_like_cpp,
    world_creature_from_pending_respawn_like_cpp, WorldCreature};
use std::time::Instant;
use wow_core::{Position, guid::HighGuid};
use wow_entities::Creature;

fn setup() -> (MapManager, MapObjectTickContinuation) {
    let mut manager = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    manager.updater.activate(1);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let tick = manager.begin_object_tick(plan).unwrap();
    (manager, tick)
}

fn finish(manager: &mut MapManager, tick: &mut MapObjectTickContinuation,
    token: ObjectMapUpdateToken) {
    match manager.try_finish_object_map(tick, token, None,
        None::<&mut fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>>,
        MapCreatureUpdateOwnerLikeCpp::ExternalRuntime) {
        Ok(_) => {}
        Err((error, _token)) => panic!("actual token finish rejected: {error:?}"),
    }
}

fn drain(manager: &mut MapManager, tick: &mut MapObjectTickContinuation) {
    while let Some(token) = manager.prepare_next_object_map(tick,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap() {
        finish(manager, tick, token);
    }
}

fn close(manager: &mut MapManager, tick: MapObjectTickContinuation) {
    if let Err((error, _tick)) = manager.try_finalize_object_tick(tick) {
        panic!("ready original continuation rejected: {error:?}");
    }
}

fn unchanged_tail(manager: &MapManager, timer: i64) {
    assert_eq!(manager.timer.current(), timer);
    assert_eq!(manager.updater.wait_calls(), 0);
    for id in [1, 2] {
        assert!(manager.find_map(id, 0).unwrap().delayed_update_calls().is_empty());
    }
}

fn one_tail(manager: &MapManager) {
    assert_eq!(manager.updater.wait_calls(), 1);
    assert_eq!(manager.timer.current(), 0);
    assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Idle);
    for id in [1, 2] {
        assert_eq!(manager.find_map(id, 0).unwrap().delayed_update_calls(), [200]);
    }
}

#[test]
fn incomplete_close_returns_original_vectors_then_runs_one_tail_after_drain() {
    let (mut manager, tick) = setup();
    let participants = tick.updated_maps.as_ptr();
    let origin = Arc::as_ptr(&tick.origin);
    let timer = manager.timer.current();
    let mut tick = match manager.try_finalize_object_tick(tick) {
        Err((ObjectMapTickError::Incomplete { processed_participants: 0, total_participants: 2 }, tick)) => tick,
        _ => panic!("incomplete continuation must return before tail"),
    };
    assert_eq!(tick.updated_maps.as_ptr(), participants);
    assert_eq!(Arc::as_ptr(&tick.origin), origin);
    assert_eq!(tick.next_participant, 0);
    unchanged_tail(&manager, timer);
    drain(&mut manager, &mut tick);
    close(&mut manager, tick);
    one_tail(&manager);
    assert!(manager.begin_tick_like_cpp(199).into_started().is_none());
    assert_eq!(manager.begin_tick_like_cpp(1).into_started().unwrap().effective_diff_ms(), 200);
}

#[test]
fn inflight_close_retains_actual_token_accounting_and_retries_after_finish() {
    let (mut manager, mut tick) = setup();
    let token = manager.prepare_next_object_map(&mut tick,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap().unwrap();
    let admitted = token.participant;
    let timer = manager.timer.current();
    tick = match manager.try_finalize_object_tick(tick) {
        Err((ObjectMapTickError::MapInFlight { participant }, tick)) => {
            assert_eq!(participant, admitted);
            tick
        }
        _ => panic!("in-flight gate must precede incomplete"),
    };
    assert_eq!(tick.in_flight.unwrap().participant, admitted);
    assert_eq!(manager.updater.pending_requests, 1);
    unchanged_tail(&manager, timer);
    finish(&mut manager, &mut tick, token);
    drain(&mut manager, &mut tick);
    close(&mut manager, tick);
    one_tail(&manager);
    assert_eq!(manager.updater.pending_requests, 0);
}

#[test]
fn foreign_manager_returns_original_ready_tick_for_its_owner() {
    let (mut owner, mut tick) = setup();
    let (mut foreign, _foreign_tick) = setup();
    drain(&mut owner, &mut tick);
    let original_origin = Arc::as_ptr(&tick.origin);
    let original_resumed = tick.resumed_keys.as_ptr();
    let foreign_timer = foreign.timer.current();
    let tick = match foreign.try_finalize_object_tick(tick) {
        Err((ObjectMapTickError::OriginMismatch { .. }, tick)) => tick,
        _ => panic!("foreign close must return original work"),
    };
    assert_eq!(Arc::as_ptr(&tick.origin), original_origin);
    assert_eq!(tick.resumed_keys.as_ptr(), original_resumed);
    unchanged_tail(&foreign, foreign_timer);
    assert!(foreign.begin_tick_like_cpp(1).is_busy());
    close(&mut owner, tick);
    one_tail(&owner);
}

#[test]
fn wrong_epoch_and_state_preflight_return_real_continuation_without_effects() {
    let (mut manager, mut tick) = setup();
    drain(&mut manager, &mut tick);
    let epoch = tick.epoch;
    let timer = manager.timer.current();
    // Fault injection only into the existing coordination gate after real
    // admission; no token, continuation, origin or reservation is fabricated.
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(epoch + 1);
    tick = match manager.try_finalize_object_tick(tick) {
        Err((ObjectMapTickError::WrongEpoch { expected_epoch, actual_epoch }, tick)) => {
            assert_eq!(expected_epoch, epoch);
            assert_eq!(actual_epoch, epoch + 1);
            tick
        }
        _ => panic!("wrong epoch must retain continuation"),
    };
    unchanged_tail(&manager, timer);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch);
    tick = match manager.try_finalize_object_tick(tick) {
        Err((ObjectMapTickError::WrongState { expected_epoch, state }, tick)) => {
            assert_eq!(expected_epoch, epoch);
            assert_eq!(state, MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch));
            tick
        }
        _ => panic!("wrong state must retain continuation"),
    };
    unchanged_tail(&manager, timer);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(epoch);
    close(&mut manager, tick);
    one_tail(&manager);
}

#[test]
fn actual_pending_respawn_precedes_foreign_tick_error_without_clearing_request() {
    let (mut owner, mut tick) = setup();
    drain(&mut owner, &mut tick);
    let mut recipient = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    recipient.create_world_map(571, 7);
    let now = Instant::now();
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(ObjectGuid::create_world_object(
        HighGuid::Creature, 0, 1, 571, 7, 42, 95001));
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(95001);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let pending = pending_respawn_from_world_creature_like_cpp(
        &WorldCreature::from_canonical(creature, data), now, 571);
    let store = recipient.find_map_mut(571, 7).unwrap().map_mut().respawn_store_like_cpp_mut();
    store.save_actor_row(&pending, 571, 7, now, 100);
    store.queue_actor(pending).unwrap();
    let plan = recipient.begin_tick_like_cpp(200).into_started().unwrap();
    let request = match recipient.begin_actor_respawn_map(&plan, plan.updated_maps_like_cpp()[0],
        now, now, 100, true).unwrap() {
        ActorRespawnProgress::Pending(request) => request,
        _ => panic!("actual queued actor must create a pending request"),
    };
    let guid = request.guid();
    let timer = recipient.timer.current();
    let tick = match recipient.try_finalize_object_tick(tick) {
        Err((ObjectMapTickError::RespawnOperationInFlight, tick)) => tick,
        _ => panic!("respawn priority must precede foreign origin"),
    };
    assert_eq!(request.guid(), guid);
    assert!(recipient.active_respawn.is_some());
    assert_eq!(recipient.timer.current(), timer);
    assert_eq!(recipient.updater.wait_calls(), 0);
    assert!(recipient.find_map(571, 7).unwrap().delayed_update_calls().is_empty());
    let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
    assert!(matches!(recipient.resume_actor_respawn(&plan, reply).unwrap(),
        ActorRespawnProgress::Complete(_)));
    assert!(recipient.active_respawn.is_none());
    close(&mut owner, tick);
    one_tail(&owner);
    let mut recipient_tick = recipient.begin_object_tick(plan).unwrap();
    drain(&mut recipient, &mut recipient_tick);
    close(&mut recipient, recipient_tick);
    assert_eq!(recipient.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Idle);
}

#[test]
fn stale_token_settlement_allows_final_close_without_replaying_object_tail() {
    let (mut manager, mut tick) = setup();
    let token = manager.prepare_next_object_map(&mut tick,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap().unwrap();
    let key = token.key();
    assert!(manager.destroy_map(key.map_id, key.instance_id));
    manager.create_world_map(key.map_id, key.instance_id);
    tick = match manager.try_finalize_object_tick(tick) {
        Err((ObjectMapTickError::MapInFlight { .. }, tick)) => tick,
        _ => panic!("stale map still owns an unresolved token"),
    };
    finish(&mut manager, &mut tick, token);
    assert!(!manager.find_map(key.map_id, key.instance_id).unwrap()
        .last_map_update_tail_summary_like_cpp().script_hook.invoked);
    drain(&mut manager, &mut tick);
    close(&mut manager, tick);
    one_tail(&manager);
}
