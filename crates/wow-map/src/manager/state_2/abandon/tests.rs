//! Real admitted plans and respawn ownership; no synthetic producer failure.
use super::*;
use crate::map_manager::{pending_respawn_from_world_creature_like_cpp,
    world_creature_from_pending_respawn_like_cpp, WorldCreature};
use crate::manager::ActorRespawnProgress;
use std::time::Instant;
use wow_core::{Position, guid::HighGuid};
use wow_entities::Creature;

fn admitted(diff: u32) -> (MapManager, MapTickPlanLikeCpp) {
    let mut manager = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    manager.create_map_entry(33, 7, 1,
        ManagedMapKind::Dungeon { has_reset_schedule: false });
    manager.find_map_mut(33, 7).unwrap().set_can_unload(true);
    manager.updater.activate(1);
    let plan = manager.begin_tick_like_cpp(diff).into_started().unwrap();
    (manager, plan)
}

#[test]
fn foreign_abandon_returns_original_buffers_then_only_original_owner_becomes_idle() {
    let (mut owner, plan) = admitted(200);
    let (mut foreign, foreign_plan) = admitted(300);
    let origin = Arc::as_ptr(&plan.origin);
    let updated = plan.updated.as_ptr();
    let destroyed = plan.destroyed.as_ptr();
    let epoch = plan.epoch;
    assert_eq!(plan.epoch, foreign_plan.epoch);
    assert_eq!(plan.updated, foreign_plan.updated);
    assert_eq!(plan.destroyed, foreign_plan.destroyed);
    assert!(!Arc::ptr_eq(&plan.origin, &foreign_plan.origin));
    let plan = match foreign.try_abandon_tick(plan) {
        Err((MapTickResumeLikeCpp::Rejected { state, plan_epoch }, plan)) => {
            assert_eq!(state, MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch));
            assert_eq!(plan_epoch, epoch);
            plan
        }
        _ => panic!("foreign manager must return the original plan"),
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
    assert_eq!(owner.timer.current(), 200);
    assert_eq!(foreign.timer.current(), 300);
    assert_eq!(owner.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch));
    assert!(foreign.can_resume_tick(&foreign_plan));
    assert!(owner.try_abandon_tick(plan).is_ok());
    assert_eq!(owner.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Idle);
    assert_eq!(owner.timer.current(), 200);
    assert_eq!(foreign.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch));
    assert_eq!(foreign.timer.current(), 300);
    assert!(foreign.can_resume_tick(&foreign_plan));
    assert_eq!(owner.updater.wait_calls(), 0);
    assert_eq!(foreign.updater.wait_calls(), 0);
    assert!(owner.find_map(33, 7).is_some());
    assert!(owner.find_map(1, 0).unwrap().delayed_update_calls().is_empty());
    assert!(foreign.find_map(1, 0).unwrap().delayed_update_calls().is_empty());
}

#[test]
fn wrong_epoch_and_state_rejections_preserve_original_plan_and_timer() {
    let (mut manager, plan) = admitted(200);
    let origin = Arc::as_ptr(&plan.origin);
    let updated = plan.updated.as_ptr();
    let destroyed = plan.destroyed.as_ptr();
    let epoch = plan.epoch;
    // These state faults follow real admission, never manufacture a plan.
    for state in [MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch + 1),
        MapTickCoordinationStateLikeCpp::Resuming(epoch)] {
        manager.tick_coordination_like_cpp = state;
        assert!(!manager.can_resume_tick(&plan));
    }
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch + 1);
    let (status, plan) = manager.try_abandon_tick(plan).unwrap_err();
    assert_eq!(status, MapTickResumeLikeCpp::Rejected {
        state: MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch + 1), plan_epoch: epoch });
    assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch + 1));
    assert_eq!(Arc::as_ptr(&plan.origin), origin);
    assert_eq!(plan.updated.as_ptr(), updated);
    assert_eq!(plan.destroyed.as_ptr(), destroyed);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(epoch);
    let (status, plan) = manager.try_abandon_tick(plan).unwrap_err();
    assert_eq!(status, MapTickResumeLikeCpp::Rejected {
        state: MapTickCoordinationStateLikeCpp::Resuming(epoch), plan_epoch: epoch });
    assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Resuming(epoch));
    assert_eq!(Arc::as_ptr(&plan.origin), origin);
    assert_eq!(plan.updated.as_ptr(), updated);
    assert_eq!(plan.destroyed.as_ptr(), destroyed);
    assert_eq!(plan.effective_diff_ms, 200);
    assert_eq!(manager.timer.current(), 200);
    assert_eq!(manager.updater.wait_calls(), 0);
    assert!(manager.find_map(1, 0).unwrap().delayed_update_calls().is_empty());
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch);
    assert!(manager.try_abandon_tick(plan).is_ok());
    assert_eq!(manager.timer.current(), 200);
    assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Idle);
}

#[test]
fn real_respawn_reservation_and_partial_cursor_reject_without_consuming_any_owner() {
    let (mut owner, foreign_plan) = admitted(200);
    let (mut recipient, own_plan) = admitted(300);
    let origin = Arc::as_ptr(&foreign_plan.origin);
    let updated = foreign_plan.updated.as_ptr();
    let destroyed = foreign_plan.destroyed.as_ptr();
    let own_updated = own_plan.updated.as_ptr();
    let own_destroyed = own_plan.destroyed.as_ptr();
    let now = Instant::now();
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(ObjectGuid::create_world_object(
        HighGuid::Creature, 0, 1, 1, 0, 42, 96001));
    creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(96001);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let pending = pending_respawn_from_world_creature_like_cpp(
        &WorldCreature::from_canonical(creature, data), now, 1);
    let store = recipient.find_map_mut(1, 0).unwrap().map_mut().respawn_store_like_cpp_mut();
    store.save_actor_row(&pending, 1, 0, now, 100);
    store.queue_actor(pending).unwrap();
    let request = match recipient.begin_actor_respawn_map(&own_plan, own_plan.updated[0],
        now, now, 100, true).unwrap() {
        ActorRespawnProgress::Pending(request) => request,
        _ => panic!("queued actor must own a real respawn reservation"),
    };
    let guid = request.guid();
    let operation = recipient.active_respawn.as_ref().map(|value| value as *const _);
    let (status, foreign_plan) = recipient.try_abandon_tick(foreign_plan).unwrap_err();
    assert_eq!(status, MapTickResumeLikeCpp::Rejected {
        state: MapTickCoordinationStateLikeCpp::AwaitingSessions(own_plan.epoch),
        plan_epoch: foreign_plan.epoch });
    assert_eq!(Arc::as_ptr(&foreign_plan.origin), origin);
    assert_eq!(foreign_plan.updated.as_ptr(), updated);
    assert_eq!(foreign_plan.destroyed.as_ptr(), destroyed);
    assert_eq!(recipient.active_respawn.as_ref().map(|value| value as *const _), operation);
    assert_eq!(request.guid(), guid);
    assert!(recipient.find_map(1, 0).unwrap().map().respawn_store_like_cpp().has_reservations());
    assert!(recipient.find_map(1, 0).unwrap().map().respawn_store_like_cpp()
        .saved_row(SpawnObjectType::Creature, 96001).is_some());
    assert_eq!(recipient.timer.current(), 300);
    let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
    assert!(matches!(recipient.resume_actor_respawn(&own_plan, reply).unwrap(), ActorRespawnProgress::Complete(_)));
    assert!(recipient.active_respawn.is_none());
    assert!(!recipient.find_map(1, 0).unwrap().map().respawn_store_like_cpp().has_reservations());
    let (status, own_plan) = recipient.try_abandon_tick(own_plan).unwrap_err();
    assert_eq!(status, MapTickResumeLikeCpp::Rejected {
        state: MapTickCoordinationStateLikeCpp::AwaitingSessions(own_plan.epoch), plan_epoch: own_plan.epoch });
    assert_eq!(own_plan.updated.as_ptr(), own_updated);
    assert_eq!(own_plan.destroyed.as_ptr(), own_destroyed);
    assert_eq!(recipient.respawn_cursor, Some((own_plan.epoch, 1)));
    assert_eq!(recipient.timer.current(), 300);
    assert!(matches!(recipient.begin_actor_respawn_map(&own_plan, own_plan.updated[1],
        now, now, 100, true).unwrap(), ActorRespawnProgress::Complete(_)));
    assert!(recipient.try_abandon_tick(own_plan).is_ok());
    assert!(owner.try_abandon_tick(foreign_plan).is_ok());
    assert_eq!(recipient.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Idle);
    assert_eq!(recipient.timer.current(), 300);
    assert_eq!(recipient.updater.wait_calls(), 0);
    assert!(recipient.find_map(33, 7).is_some());
    assert!(recipient.find_map(1, 0).unwrap().delayed_update_calls().is_empty());
}

#[test]
fn compatibility_wrapper_keeps_old_success_rejection_and_effects() {
    let (mut owner, foreign_plan) = admitted(200);
    let (mut recipient, own_plan) = admitted(300);
    assert_eq!(recipient.abandon_tick_like_cpp(foreign_plan), MapTickResumeLikeCpp::Rejected {
        state: MapTickCoordinationStateLikeCpp::AwaitingSessions(own_plan.epoch), plan_epoch: own_plan.epoch });
    assert_eq!(recipient.timer.current(), 300);
    assert!(recipient.can_resume_tick(&own_plan));
    assert_eq!(recipient.abandon_tick_like_cpp(own_plan), MapTickResumeLikeCpp::Resumed);
    assert_eq!(recipient.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Idle);
    assert_eq!(recipient.timer.current(), 300);
    assert_eq!(recipient.updater.wait_calls(), 0);
    assert!(recipient.find_map(33, 7).is_some());
    assert!(recipient.find_map(1, 0).unwrap().delayed_update_calls().is_empty());
    // Legacy rejection consumed the foreign plan and did not heal its owner.
    assert_eq!(owner.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::AwaitingSessions(1));
    assert_eq!(owner.timer.current(), 200);
    assert!(owner.begin_tick_like_cpp(1).is_busy());
    assert_eq!(owner.timer.current(), 200);
}
