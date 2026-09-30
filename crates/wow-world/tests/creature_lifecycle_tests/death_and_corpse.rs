use super::*;

#[test]
fn legacy_creature_lifecycle_tick_once_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let guid = test_creature_guid(90_009);
    register_test_creature(&mut session, manager.clone(), guid, 25);

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        None,
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        Instant::now(),
    );

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert_eq!(outcome.creatures_seen, 0);
    assert_eq!(outcome.corpses_despawned, 0);
    assert_eq!(outcome.respawns_processed, 0);
    assert!(outcome.refresh_map_keys.is_empty());
    assert!(
        manager.read().unwrap().find_creature(0, 0, guid).is_some(),
        "default Session owner must leave the legacy creature untouched"
    );
}

#[test]
fn legacy_creature_lifecycle_persistence_does_not_double_count_stale_tick_time_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let guid = test_creature_guid(90_020);
    register_test_creature(&mut session, Arc::clone(&manager), guid, 10);
    session
        .fixture_melee_mutate_creature(guid, |creature| {
            creature.creature.set_spawn_id(90_020);
            creature.creature.ai_ownership_mut().respawn_time_secs = 30;
            assert!(creature.take_damage(10));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let before_unix = unix_now();
    let stale_tick_now = Instant::now() - Duration::from_secs(10);
    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        stale_tick_now,
    );

    assert_eq!(outcome.corpses_despawned, 0);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    let respawn_time = match outcome.respawn_db_mutations[0] {
        wow_persistence::RespawnPersistenceMutationLikeCpp::Save { respawn_time, .. } => {
            respawn_time
        }
        _ => panic!("the death transition must emit a typed respawn save"),
    };
    assert!(respawn_time >= before_unix + 28);
    assert!(
        respawn_time <= unix_now() + 30,
        "the stale scheduler Instant must not be added again to the Unix respawn time"
    );
}

#[test]
fn legacy_creature_lifecycle_tick_once_despawns_corpse_queues_respawn_and_refresh_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let guid = test_creature_guid(90_010);
    register_test_creature(&mut session, manager.clone(), guid, 10);

    let now = Instant::now();
    let past = now - Duration::from_secs(1);
    session
        .fixture_melee_mutate_creature(guid, |creature| {
            creature.creature.set_spawn_id(90_010);
            creature.take_damage(10);
            creature.set_corpse_despawn_at(Some(past));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.corpses_despawned, 1);
    assert_eq!(outcome.respawns_processed, 0);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(
        matches!(
            outcome.respawn_db_mutations[0],
            wow_persistence::RespawnPersistenceMutationLikeCpp::Save { respawn_time, .. }
                if respawn_time > unix_now()
        ),
        "C++ persists an absolute future GameTime value, never creature-local uptime"
    );
    assert_eq!(outcome.canonical_removes, 1);
    assert_eq!(outcome.canonical_inserts, 0);
    assert_eq!(outcome.canonical_respawn_adds, 1);
    assert_eq!(outcome.canonical_respawn_removes, 0);
    assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);

    {
        let guard = manager.read().unwrap();
        assert!(
            guard.find_creature(0, 0, guid).is_none(),
            "corpse must be removed from the legacy map"
        );
        assert_eq!(
            guard.respawn_queue_len(0, 0),
            1,
            "corpse removal must enqueue exactly one map-owned respawn"
        );
    }
    {
        let guard = canonical.lock().unwrap();
        assert!(
            guard
                .find_map(0, 0)
                .unwrap()
                .map()
                .with_creature_like_cpp(guid, Clone::clone)
                .is_none(),
            "canonical map object must be removed outside the legacy lock"
        );
        assert!(
            guard
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_respawn_time_like_cpp(
                    wow_map::SpawnObjectType::Creature,
                    (guid.low_value() as u64) & 0xFF_FFFF_FFFF,
                )
                > 0,
            "canonical respawn store must retain the same timer C++ Map::SaveRespawnTime adds"
        );
    }
}

#[test]
fn legacy_creature_lifecycle_tick_once_does_not_persist_dungeon_respawn_like_cpp() {
    assert_instanceable_map_does_not_persist_respawn_like_cpp(
        33,
        77,
        wow_data::map::MAP_INSTANCE,
        0,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
        90_016,
    );
}

#[test]
fn legacy_creature_lifecycle_tick_once_does_not_persist_garrison_respawn_like_cpp() {
    assert_instanceable_map_does_not_persist_respawn_like_cpp(
        1_151,
        90_019,
        wow_data::map::MAP_SCENARIO,
        wow_data::map::MAP_FLAG_GARRISON,
        // Rust currently represents garrisons as World-kind managed maps;
        // persistence must still follow C++ MapEntry::Instanceable().
        wow_map::ManagedMapKind::World,
        90_019,
    );
}
