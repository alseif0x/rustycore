//! Query ownership, stale replies and explicit fail-stop disposal.
use super::*;

#[test]
fn foreign_manager_rejects_owned_reply_then_original_resumes_same_seeded_motor_once() {
    let now = Instant::now();
    let mut first = setup();
    let mut second = setup();
    queue(&mut first, pending(10, 10, now), now);
    queue(&mut second, pending(10, 10, now), now);
    let (first_plan, first_request) = start(&mut first, now);
    let (second_plan, second_request) = start(&mut second, now);
    let guid = first_request.guid();
    let mut calls = 0;
    let mut rng = None;
    let reply = first_request.resolve(
        |pending, instance| {
            calls += 1;
            let mut actor = world_creature_from_pending_respawn_like_cpp(pending, instance);
            rng = Some(actor.seed_actor_storage_runtime(true));
            actor
        },
        |_, _| {},
    );
    let rejected = second
        .resume_actor_respawn(&second_plan, reply)
        .unwrap_err();
    assert_eq!(rejected.error, ActorRespawnError::OriginMismatch);
    assert_eq!(calls, 1);
    assert!(
        first
            .find_map(571, 7)
            .unwrap()
            .map()
            .get_creature(guid)
            .is_none()
    );
    let outcome = match first
        .resume_actor_respawn(&first_plan, rejected.owned)
        .unwrap()
    {
        ActorRespawnProgress::Complete(outcome) => outcome,
        _ => panic!("one actor"),
    };
    assert_eq!(calls, 1);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    first
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .creature_actor_mut(guid)
        .unwrap()
        .assert_actor_storage_runtime(&mut rng.unwrap(), true);
    drop(second_request);
    assert!(!second.can_resume_tick(&second_plan));
}

#[test]
fn wrong_epoch_returns_reply_and_correct_epoch_resumes_without_factory_or_prefix_replay() {
    let now = Instant::now();
    let mut manager = setup();
    queue(&mut manager, pending(11, 11, now), now);
    let mut dead = world_creature_from_pending_respawn_like_cpp(&pending(111, 111, now), 7);
    dead.creature
        .unit_mut()
        .set_death_state(wow_constants::DeathState::JustDied);
    dead.creature.unit_mut().set_health(0);
    dead.creature.runtime_state_mut().save_respawn_requested = true;
    let dead_guid = dead.guid();
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .admit_fresh_creature_actor(dead)
        .unwrap();
    let (plan, request) = start(&mut manager, now);
    let mut calls = 0;
    let reply = request.resolve(
        |pending, instance| {
            calls += 1;
            world_creature_from_pending_respawn_like_cpp(pending, instance)
        },
        |_, _| {},
    );
    manager.tick_coordination_like_cpp =
        MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp() + 1);
    let rejected = manager.resume_actor_respawn(&plan, reply).unwrap_err();
    assert_eq!(rejected.error, ActorRespawnError::WrongEpoch);
    assert_eq!(calls, 1);
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .respawn_store_like_cpp()
            .saved_row(SpawnObjectType::Creature, 11)
            .is_some()
    );
    manager.tick_coordination_like_cpp =
        MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp());
    let outcome = match manager.resume_actor_respawn(&plan, rejected.owned).unwrap() {
        ActorRespawnProgress::Complete(outcome) => outcome,
        _ => panic!("one actor"),
    };
    assert_eq!(outcome.attempts.len(), 1);
    assert_eq!(outcome.respawn_db_mutations.len(), 2);
    assert!(matches!(
        outcome.respawn_db_mutations[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Save { .. }
    ));
    assert!(matches!(
        outcome.respawn_db_mutations[1],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));
    assert!(
        !manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .creature_actor(dead_guid)
            .unwrap()
            .creature
            .runtime_state()
            .save_respawn_requested
    );
    assert_eq!(calls, 1);
}

#[test]
fn stale_incarnation_reply_is_returned_and_disposal_does_not_touch_replacement_or_complete_tick() {
    let now = Instant::now();
    let mut manager = setup();
    queue(&mut manager, pending(12, 12, now), now);
    let (plan, request) = start(&mut manager, now);
    let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
    // Simulate external invalidation. Public destruction is separately gated.
    let key = MapKey::new(571, 7);
    manager.maps.remove(&key);
    manager.map_incarnations_like_cpp.remove(&key);
    manager.create_world_map(571, 7);
    queue(&mut manager, pending(121, 121, now), now);
    let rejected = manager.resume_actor_respawn(&plan, reply).unwrap_err();
    assert_eq!(rejected.error, ActorRespawnError::StaleParticipant);
    let outcome = manager.dispose_actor_respawn_reply(rejected.owned).unwrap();
    assert!(outcome.attempts.is_empty());
    let replacement = manager.find_map(571, 7).unwrap().map();
    assert_eq!(replacement.respawn_store_like_cpp().actor_queue_len(), 1);
    assert!(
        replacement
            .respawn_store_like_cpp()
            .saved_row(SpawnObjectType::Creature, 121)
            .is_some()
    );
    assert!(!replacement.respawn_store_like_cpp().has_reservations());
    assert!(manager.active_respawn.as_ref().unwrap().disposed);
    assert!(!manager.can_resume_tick(&plan));
    assert!(!manager.destroy_map(571, 7));
    assert!(matches!(
        manager.abandon_tick_like_cpp(plan),
        MapTickResumeLikeCpp::Rejected { .. }
    ));
}

#[test]
fn dispose_request_releases_keys_but_retains_busy_and_never_queues_or_deletes_rows() {
    let now = Instant::now();
    let mut manager = setup();
    queue(&mut manager, pending(13, 13, now), now);
    queue(&mut manager, pending(14, 14, now), now);
    let (plan, request) = start(&mut manager, now);
    let outcome = manager.dispose_actor_respawn_request(request).unwrap();
    assert!(outcome.attempts.is_empty());
    assert!(outcome.respawn_db_mutations.is_empty());
    let store = manager
        .find_map(571, 7)
        .unwrap()
        .map()
        .respawn_store_like_cpp();
    assert!(!store.has_reservations());
    assert_eq!(store.actor_queue_len(), 0);
    assert_eq!(store.saved_rows().len(), 2);
    assert!(!manager.can_resume_tick(&plan));
    assert!(manager.unload_all().is_err());
    assert!(manager.begin_tick_like_cpp(1000).is_busy());
    assert!(matches!(
        manager.begin_object_tick(plan),
        Err(ObjectMapTickError::RespawnOperationInFlight)
    ));
}

#[test]
fn panic_inside_query_loses_owned_request_but_never_releases_manager_reservation() {
    let now = Instant::now();
    let mut manager = setup();
    queue(&mut manager, pending(15, 15, now), now);
    let (plan, request) = start(&mut manager, now);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            request.resolve(
                |_, _| panic!("factory failed"),
                |_, _| panic!("snap must not run"),
            );
        }))
        .is_err()
    );
    assert!(manager.active_respawn.is_some());
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .respawn_store_like_cpp()
            .is_reserved(RespawnKey::Persistent(SpawnObjectType::Creature, 15))
    );
    assert!(!manager.can_resume_tick(&plan));
    assert!(!manager.destroy_map(571, 7));
}

#[test]
fn late_spawn_collision_does_not_delete_row_or_requeue_attempt() {
    let now = Instant::now();
    let mut manager = setup();
    queue(&mut manager, pending(16, 16, now), now);
    let (plan, request) = start(&mut manager, now);
    let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
    let other = world_creature_from_pending_respawn_like_cpp(&pending(17, 16, now), 7);
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .admit_fresh_creature_actor(other)
        .unwrap();
    let outcome = match manager.resume_actor_respawn(&plan, reply).unwrap() {
        ActorRespawnProgress::Complete(outcome) => outcome,
        _ => panic!("one attempt"),
    };
    assert_eq!(
        outcome.attempts[0].status,
        ActorRespawnStatus::AdmissionRejected
    );
    assert!(outcome.respawn_db_mutations.is_empty());
    let store = manager
        .find_map(571, 7)
        .unwrap()
        .map()
        .respawn_store_like_cpp();
    assert!(store.saved_row(SpawnObjectType::Creature, 16).is_some());
    assert_eq!(store.actor_queue_len(), 0);
    assert!(!store.has_reservations());
}

#[test]
fn missing_reservation_returns_whole_reply_without_admission() {
    let now = Instant::now();
    let mut manager = setup();
    queue(&mut manager, pending(18, 18, now), now);
    let (plan, request) = start(&mut manager, now);
    let guid = request.guid();
    let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut()
        .release_respawn_key(RespawnKey::Persistent(SpawnObjectType::Creature, 18));
    let rejected = manager.resume_actor_respawn(&plan, reply).unwrap_err();
    assert_eq!(rejected.error, ActorRespawnError::ReservationLost);
    assert_eq!(rejected.owned.actor.guid(), guid);
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .get_creature(guid)
            .is_none()
    );
    assert!(manager.active_respawn.is_some());
}

#[test]
fn epoch_stale_disposal_uses_slot_identity_and_keeps_fail_stop_even_if_state_is_lost() {
    let now = Instant::now();
    let mut manager = setup();
    queue(&mut manager, pending(19, 19, now), now);
    let (plan, request) = start(&mut manager, now);
    let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
    manager.tick_coordination_like_cpp =
        MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp() + 1);
    let rejected = manager.resume_actor_respawn(&plan, reply).unwrap_err();
    assert_eq!(rejected.error, ActorRespawnError::WrongEpoch);
    manager.dispose_actor_respawn_reply(rejected.owned).unwrap();
    let before = manager.timer.current();
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Idle;
    assert!(manager.begin_tick_like_cpp(9999).is_busy());
    assert_eq!(manager.timer.current(), before);
    assert!(!manager.can_resume_tick(&plan));
    assert!(manager.unload_all().is_err());
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .respawn_store_like_cpp()
            .saved_row(SpawnObjectType::Creature, 19)
            .is_some()
    );
}
