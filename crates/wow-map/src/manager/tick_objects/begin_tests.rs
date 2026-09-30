//! Original admitted plans survive each BEGIN gate without any first write.
use super::*;
use crate::map_manager::{
    WorldCreature, pending_respawn_from_world_creature_like_cpp,
    world_creature_from_pending_respawn_like_cpp,
};
use std::time::Instant;
use wow_core::{Position, guid::HighGuid};
use wow_entities::Creature;

fn admitted() -> (MapManager, MapTickPlanLikeCpp) {
    let mut manager = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    manager.create_map_entry(
        33,
        7,
        1,
        ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    manager.find_map_mut(33, 7).unwrap().set_can_unload(true);
    manager.updater.activate(1);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    (manager, plan)
}

fn begin(manager: &mut MapManager, plan: MapTickPlanLikeCpp) -> MapObjectTickContinuation {
    match manager.try_begin_object_tick(plan) {
        Ok(tick) => tick,
        Err((error, _plan)) => panic!("actual ready plan must begin: {error:?}"),
    }
}

#[test]
fn foreign_begin_returns_original_origin_and_both_nonempty_identity_vectors() {
    let (mut owner, plan) = admitted();
    let (mut recipient, _recipient_plan) = admitted();
    let origin = Arc::as_ptr(&plan.origin);
    let updated = plan.updated.as_ptr();
    let destroyed = plan.destroyed.as_ptr();
    let epoch = plan.epoch;
    let timer = recipient.timer.current();
    // Actual admission precedes this test-only state fault. Origin must win
    // over the state mismatch; no origin, plan or continuation is fabricated.
    recipient.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Idle;
    let plan = match recipient.try_begin_object_tick(plan) {
        Err((ObjectMapTickError::OriginMismatch { plan_epoch }, plan)) => {
            assert_eq!(plan_epoch, epoch);
            plan
        }
        _ => panic!("origin must precede state"),
    };
    assert_eq!(Arc::as_ptr(&plan.origin), origin);
    assert_eq!(plan.updated.as_ptr(), updated);
    assert_eq!(plan.destroyed.as_ptr(), destroyed);
    assert_eq!(plan.updated.len(), 2);
    assert_eq!(plan.destroyed.len(), 1);
    assert_eq!(plan.updated[0].key, MapKey::new(1, 0));
    assert_eq!(plan.updated[1].key, MapKey::new(2, 0));
    assert_eq!(plan.destroyed[0].key, MapKey::new(33, 7));
    assert_eq!(plan.effective_diff_ms, 200);
    assert_eq!(recipient.timer.current(), timer);
    assert_eq!(
        recipient.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(recipient.updater.wait_calls(), 0);
    assert!(recipient.find_map(33, 7).is_some());
    assert!(owner.can_resume_tick(&plan));
    let tick = begin(&mut owner, plan);
    assert_eq!(Arc::as_ptr(&tick.origin), origin);
    assert_eq!(tick.updated_maps.len(), 2);
    assert_eq!(tick.destroyed_maps.len(), 1);
}

#[test]
fn wrong_epoch_then_state_return_same_plan_before_allocations_or_state_write() {
    let (mut manager, plan) = admitted();
    let epoch = plan.epoch;
    let origin = Arc::as_ptr(&plan.origin);
    let updated = plan.updated.as_ptr();
    let destroyed = plan.destroyed.as_ptr();
    let timer = manager.timer.current();
    manager.tick_coordination_like_cpp =
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch + 1);
    let plan = match manager.try_begin_object_tick(plan) {
        Err((
            ObjectMapTickError::WrongEpoch {
                expected_epoch,
                actual_epoch,
            },
            plan,
        )) => {
            assert_eq!(expected_epoch, epoch);
            assert_eq!(actual_epoch, epoch + 1);
            plan
        }
        _ => panic!("same-origin wrong epoch must return the original plan"),
    };
    assert_eq!(Arc::as_ptr(&plan.origin), origin);
    assert_eq!(plan.updated.as_ptr(), updated);
    assert_eq!(plan.destroyed.as_ptr(), destroyed);
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch + 1)
    );
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(epoch);
    let plan = match manager.try_begin_object_tick(plan) {
        Err((
            ObjectMapTickError::WrongState {
                expected_epoch,
                state,
            },
            plan,
        )) => {
            assert_eq!(expected_epoch, epoch);
            assert_eq!(state, MapTickCoordinationStateLikeCpp::Resuming(epoch));
            plan
        }
        _ => panic!("wrong state must retain the same admitted plan"),
    };
    assert_eq!(Arc::as_ptr(&plan.origin), origin);
    assert_eq!(plan.updated.as_ptr(), updated);
    assert_eq!(plan.destroyed.as_ptr(), destroyed);
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(epoch)
    );
    assert_eq!(manager.timer.current(), timer);
    assert_eq!(manager.updater.wait_calls(), 0);
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch);
    let tick = begin(&mut manager, plan);
    assert_eq!(tick.epoch, epoch);
    assert_eq!(tick.effective_diff_ms, 200);
}

#[test]
fn actual_pending_respawn_precedes_foreign_origin_and_preserves_both_owners() {
    let (mut owner, foreign_plan) = admitted();
    let original_origin = Arc::as_ptr(&foreign_plan.origin);
    let original_updated = foreign_plan.updated.as_ptr();
    let original_destroyed = foreign_plan.destroyed.as_ptr();
    let mut recipient = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    recipient.create_world_map(571, 7);
    recipient.create_world_map(572, 0);
    let now = Instant::now();
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
            96001,
        ));
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(96001);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let pending = pending_respawn_from_world_creature_like_cpp(
        &WorldCreature::from_canonical(creature, data),
        now,
        571,
    );
    let store = recipient
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut();
    store.save_actor_row(&pending, 571, 7, now, 100);
    store.queue_actor(pending).unwrap();
    let own_plan = recipient.begin_tick_like_cpp(200).into_started().unwrap();
    let request = match recipient
        .begin_actor_respawn_map(&own_plan, own_plan.updated[0], now, now, 100, true)
        .unwrap()
    {
        ActorRespawnProgress::Pending(request) => request,
        _ => panic!("actual queued actor must reserve its respawn operation"),
    };
    let guid = request.guid();
    let timer = recipient.timer.current();
    let foreign_plan = match recipient.try_begin_object_tick(foreign_plan) {
        Err((ObjectMapTickError::RespawnOperationInFlight, plan)) => plan,
        _ => panic!("actual respawn gate must precede foreign origin"),
    };
    assert_eq!(Arc::as_ptr(&foreign_plan.origin), original_origin);
    assert_eq!(foreign_plan.updated.as_ptr(), original_updated);
    assert_eq!(foreign_plan.destroyed.as_ptr(), original_destroyed);
    assert_eq!(request.guid(), guid);
    assert!(recipient.active_respawn.is_some());
    assert_eq!(recipient.timer.current(), timer);
    assert_eq!(recipient.updater.wait_calls(), 0);
    assert_eq!(
        recipient.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(own_plan.epoch)
    );
    let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
    assert!(matches!(
        recipient.resume_actor_respawn(&own_plan, reply).unwrap(),
        ActorRespawnProgress::Complete(_)
    ));
    assert!(matches!(
        recipient
            .begin_actor_respawn_map(&own_plan, own_plan.updated[1], now, now, 100, true)
            .unwrap(),
        ActorRespawnProgress::Complete(_)
    ));
    let foreign_plan = match recipient.try_begin_object_tick(foreign_plan) {
        Err((ObjectMapTickError::OriginMismatch { .. }, plan)) => plan,
        _ => panic!("settling respawn must expose the next existing origin gate"),
    };
    assert_eq!(foreign_plan.updated.as_ptr(), original_updated);
    assert_eq!(foreign_plan.destroyed.as_ptr(), original_destroyed);
    assert!(owner.can_resume_tick(&foreign_plan));
    let _owner_tick = begin(&mut owner, foreign_plan);
    let _recipient_tick = begin(&mut recipient, own_plan);
}

#[test]
fn actual_partial_respawn_cursor_returns_plan_until_all_admitted_maps_are_done() {
    let (mut manager, plan) = admitted();
    let now = Instant::now();
    let updated = plan.updated.as_ptr();
    let destroyed = plan.destroyed.as_ptr();
    assert!(matches!(
        manager
            .begin_actor_respawn_map(&plan, plan.updated[0], now, now, 100, true)
            .unwrap(),
        ActorRespawnProgress::Complete(_)
    ));
    assert!(manager.active_respawn.is_none());
    let plan = match manager.try_begin_object_tick(plan) {
        Err((ObjectMapTickError::RespawnOperationInFlight, plan)) => plan,
        _ => panic!("unfinished real respawn cursor must reject BEGIN"),
    };
    assert_eq!(plan.updated.as_ptr(), updated);
    assert_eq!(plan.destroyed.as_ptr(), destroyed);
    assert_eq!(manager.respawn_cursor, Some((plan.epoch, 1)));
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch)
    );
    assert!(matches!(
        manager
            .begin_actor_respawn_map(&plan, plan.updated[1], now, now, 100, true)
            .unwrap(),
        ActorRespawnProgress::Complete(_)
    ));
    let tick = begin(&mut manager, plan);
    assert_eq!(tick.updated_maps.len(), 2);
    assert_eq!(tick.destroyed_maps.len(), 1);
    assert_eq!(manager.updater.wait_calls(), 0);
}

#[test]
fn successful_begin_preserves_original_phase_fields_and_existing_allocations() {
    let (mut manager, plan) = admitted();
    let origin = Arc::as_ptr(&plan.origin);
    let epoch = plan.epoch;
    let timer = manager.timer.current();
    let tick = begin(&mut manager, plan);
    assert_eq!(Arc::as_ptr(&tick.origin), origin);
    assert_eq!(tick.epoch, epoch);
    assert_eq!(tick.effective_diff_ms, 200);
    assert_eq!(tick.updated_maps[0].key, MapKey::new(1, 0));
    assert_eq!(tick.updated_maps[1].key, MapKey::new(2, 0));
    assert_eq!(tick.destroyed_maps[0].key, MapKey::new(33, 7));
    assert_eq!(tick.next_participant, 0);
    assert!(tick.in_flight.is_none());
    assert!(tick.resumed_keys.is_empty());
    assert!(tick.resumed_keys.capacity() >= tick.updated_maps.len());
    assert_eq!(manager.timer.current(), timer);
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(epoch)
    );
    assert_eq!(manager.updater.wait_calls(), 0);
    assert!(manager.find_map(33, 7).is_some());
}

#[test]
fn old_begin_wrapper_keeps_error_and_discards_plan_without_releasing_admission() {
    let (mut owner, plan) = admitted();
    let mut foreign = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    let epoch = plan.epoch;
    assert!(matches!(foreign.begin_object_tick(plan),
        Err(ObjectMapTickError::OriginMismatch { plan_epoch }) if plan_epoch == epoch));
    assert!(owner.begin_tick_like_cpp(1).is_busy());
    assert_eq!(
        foreign.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(foreign.updater.wait_calls(), 0);
}
