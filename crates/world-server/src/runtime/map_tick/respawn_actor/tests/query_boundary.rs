//! Actual APP resolver releases canonical guards while retaining the writer fence.
use super::*;
use std::cell::RefCell;

#[test]
fn factory_and_ground_query_can_lock_canonical_and_retain_existing_writer_fence() {
    let now = Instant::now();
    let (mut manager, _) = canonical();
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut()
        .queue_actor(pending(20, now))
        .unwrap();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let canonical = Arc::new(Mutex::new(manager));
    let fence = Arc::new(Mutex::new(()));
    let _writer_guard = fence.lock().unwrap();
    let calls = RefCell::new(Vec::new());
    // A real file-backed provider, with the original missing-tile fallback.
    let terrain = LiveTerrainHeights::new("/nonexistent/rustycore-respawn-query-contract");
    let outcomes = drive_prepared_actor_respawns(
        &canonical,
        &plan,
        &wow_data::MapStore::from_entries([]),
        now,
        now,
        100,
        |incoming, instance| {
            assert!(canonical.try_lock().is_ok());
            assert!(fence.try_lock().is_err());
            calls.borrow_mut().push("factory");
            world_creature_from_pending_respawn_like_cpp(incoming, instance)
        },
        |actor, map_id| {
            assert!(canonical.try_lock().is_ok());
            assert!(fence.try_lock().is_err());
            calls.borrow_mut().push("snap");
            snap_respawn_creature_to_ground_like_cpp(actor, map_id, &terrain);
        },
    )
    .unwrap();
    assert_eq!(*calls.borrow(), vec!["factory", "snap"]);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].1.attempts[0].status,
        ActorRespawnStatus::Inserted
    );
    assert!(canonical.lock().unwrap().can_resume_tick(&plan));
}

#[test]
fn app_processes_maps_and_ordinals_serially_without_preparing_later_map_prefix() {
    let now = Instant::now();
    let (mut manager, _) = canonical();
    manager.create_world_map(572, 8);
    let first = pending(21, now);
    let first_guid = first.create_data.guid;
    let sibling = pending(22, now - Duration::from_secs(1));
    let sibling_guid = sibling.create_data.guid;
    let mut later = pending(23, now);
    later.map_id = 572;
    let later_guid = later.create_data.guid;
    for incoming in [first, sibling] {
        manager
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .respawn_store_like_cpp_mut()
            .queue_actor(incoming)
            .unwrap();
    }
    manager
        .find_map_mut(572, 8)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut()
        .queue_actor(later)
        .unwrap();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let canonical = Arc::new(Mutex::new(manager));
    let trace = RefCell::new(Vec::new());
    let outcomes = drive_prepared_actor_respawns(
        &canonical,
        &plan,
        &wow_data::MapStore::from_entries([]),
        now,
        now,
        100,
        |incoming, instance| {
            let guard = canonical.try_lock().expect("factory outside guard");
            let queued_later = guard
                .find_map(572, 8)
                .unwrap()
                .map()
                .respawn_store_like_cpp()
                .actor_queue_len();
            trace
                .borrow_mut()
                .push(("factory", incoming.create_data.guid, queued_later));
            drop(guard);
            world_creature_from_pending_respawn_like_cpp(incoming, instance)
        },
        |actor, _| {
            let guard = canonical.try_lock().expect("snap outside guard");
            trace.borrow_mut().push((
                "snap",
                actor.guid(),
                guard
                    .find_map(572, 8)
                    .unwrap()
                    .map()
                    .respawn_store_like_cpp()
                    .actor_queue_len(),
            ));
        },
    )
    .unwrap();
    assert_eq!(
        *trace.borrow(),
        vec![
            ("factory", first_guid, 1),
            ("snap", first_guid, 1),
            ("factory", sibling_guid, 1),
            ("snap", sibling_guid, 1),
            ("factory", later_guid, 0),
            ("snap", later_guid, 0),
        ]
    );
    assert_eq!(
        outcomes.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
        vec![MapKey::new(571, 7), MapKey::new(572, 8)]
    );
    assert_eq!(
        outcomes[0]
            .1
            .attempts
            .iter()
            .map(|a| a.guid)
            .collect::<Vec<_>>(),
        vec![first_guid, sibling_guid]
    );
    assert_eq!(outcomes[1].1.attempts[0].guid, later_guid);
}

#[test]
fn empty_foreign_plan_cannot_bypass_app_phase_admission() {
    let now = Instant::now();
    let (mut manager, _) = canonical();
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut()
        .queue_actor(pending(24, now))
        .unwrap();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let foreign = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200)
        .begin_tick_like_cpp(200)
        .into_started()
        .unwrap();
    let canonical = Arc::new(Mutex::new(manager));
    let mut calls = 0;
    // Wrong origin fails BEFORE the factory and leaves the original pending item.
    let rejected = drive_prepared_actor_respawns(
        &canonical,
        &foreign,
        &wow_data::MapStore::from_entries([]),
        now,
        now,
        100,
        |_, _| panic!("foreign plan must not construct a motor"),
        |_, _| {},
    )
    .unwrap_err();
    // An empty foreign plan must still be validated; no arbitrary Idle/list shortcut.
    assert_eq!(
        rejected.error,
        Some(wow_map::manager::ActorRespawnError::OriginMismatch)
    );
    let outcomes = drive_prepared_actor_respawns(
        &canonical,
        &plan,
        &wow_data::MapStore::from_entries([]),
        now,
        now,
        100,
        |incoming, instance| {
            calls += 1;
            world_creature_from_pending_respawn_like_cpp(incoming, instance)
        },
        |_, _| {},
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(outcomes[0].1.attempts.len(), 1);
}

#[test]
fn app_returns_whole_reply_on_owner_invalidation_without_repeating_factory() {
    let now = Instant::now();
    let (mut manager, _) = canonical();
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut()
        .queue_actor(pending(25, now))
        .unwrap();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let canonical = Arc::new(Mutex::new(manager));
    let mut calls = 0;
    let original = RefCell::new(None);
    let rejected = drive_prepared_actor_respawns(
        &canonical,
        &plan,
        &wow_data::MapStore::from_entries([]),
        now,
        now,
        100,
        |incoming, instance| {
            calls += 1;
            world_creature_from_pending_respawn_like_cpp(incoming, instance)
        },
        |_, _| {
            let mut guard = canonical.try_lock().expect("query outside guard");
            *original.borrow_mut() = Some(std::mem::replace(
                &mut *guard,
                wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200),
            ));
        },
    )
    .unwrap_err();
    assert_eq!(calls, 1);
    assert_eq!(
        rejected.error,
        Some(wow_map::manager::ActorRespawnError::OriginMismatch)
    );
    assert!(rejected.completed.is_empty());
    let reply = rejected
        .reply
        .expect("same owned motor and payload returned");
    let mut original = original.into_inner().unwrap();
    original.dispose_actor_respawn_reply(reply).unwrap();
    assert!(!original.can_resume_tick(&plan));
    assert!(original.begin_tick_like_cpp(200).is_busy());
    assert!(canonical.lock().unwrap().find_map(571, 7).is_none());
}
