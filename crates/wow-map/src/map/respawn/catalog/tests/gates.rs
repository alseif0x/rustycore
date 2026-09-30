use super::*;

#[test]
fn empty_future_and_unloaded_catalog_paths_never_invoke_factory_or_emit_receipts() {
    let store = store(&[(SpawnObjectType::Creature, 401)]);
    for mode in 0..3 {
        let mut shared = map(false);
        if mode != 0 { timer(&mut shared, SpawnObjectType::Creature, 401, if mode == 1 { 101 } else { 100 }); }
        let outcome = run_owned(&mut shared, &store, &PoolMgrLikeCpp::new(), true, |_, _, _| panic!("no eager loader"));
        assert!(outcome.attempts.is_empty());
        assert_eq!(outcome.summary.processed_unloaded_grid_respawns, usize::from(mode == 2));
        assert_eq!(shared.get_respawn_info_like_cpp(SpawnObjectType::Creature, 401).is_some(), mode == 1);
    }
}

#[test]
fn inactive_live_guards_and_unsupported_timer_rejection_keep_original_order() {
    for mode in 0..4 {
        let kind = if mode == 3 { SpawnObjectType::AreaTrigger } else if mode == 2 { SpawnObjectType::GameObject } else { SpawnObjectType::Creature };
        let mut spawn = metadata(kind, 411);
        if mode == 0 {
            spawn.spawn_group = SpawnGroupTemplateData { group_id: 51, name: "manual".into(), map_id: 571, flags: SpawnGroupFlags::MANUAL_SPAWN };
        }
        let mut store = SpawnStore::new();
        store.add_object_spawn(&spawn, |_| false);
        let mut old = map(true);
        let mut shared = map(true);
        for map in [&mut old, &mut shared] {
            if mode == 1 || mode == 2 { map.insert_map_object_record(record(kind, 411, 571)).unwrap(); }
            if mode == 3 {
                assert_eq!(map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
                    object_type: kind, spawn_id: 411, entry: 42, respawn_time: 100,
                    grid_id: crate::compute_grid_coord(1.0, 2.0).get_id(),
                }), AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType);
                let mut info = RespawnInfoLikeCpp {
                    object_type: kind, spawn_id: 411, entry: 42, respawn_time: 100,
                    grid_id: crate::compute_grid_coord(1.0, 2.0).get_id(),
                };
                assert_eq!(map.check_respawn_like_cpp(&mut info, &store, &LinkedRespawnStoreLikeCpp::new(),
                    100, 5, false, |_, _| panic!("unsupported before escort")), CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType);
            } else {
                timer(map, kind, 411, 100);
            }
        }
        let expected = run_original(&mut old, &store, &PoolMgrLikeCpp::new(), false, |_, _, _| panic!("guard before loader"));
        let actual = run_owned(&mut shared, &store, &PoolMgrLikeCpp::new(), false, |_, _, _| panic!("guard before loader"));
        assert_eq!(actual.summary, expected);
        assert!(actual.attempts.is_empty());
        assert_eq!(actual.summary.deleted_inactive_spawn_group, usize::from(mode == 0));
        assert_eq!(actual.summary.deleted_live_object_blocker, usize::from(mode == 1 || mode == 2));
        // Unsupported types never enter the Catalog queue through addInfo.
        assert_eq!(actual.summary.blocked_unsupported_spawn_type, 0);
        assert!(shared.get_respawn_info_like_cpp(kind, 411).is_none());
    }
}

#[test]
fn linked_infinite_self_delayed_and_nonfuture_keep_original_remove_reinsert_and_break() {
    for mode in 0..4 {
        let store = store(&[(SpawnObjectType::Creature, 421)]);
        let mut linked = LinkedRespawnStoreLikeCpp::new();
        let this = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 42, 421);
        let master = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 42, 422);
        linked.insert_like_cpp(this, if mode == 1 { this } else { master });
        let mut old = map(true);
        let mut shared = map(true);
        for map in [&mut old, &mut shared] {
            timer(map, SpawnObjectType::Creature, 421, 90);
            if mode == 3 {
                saved_actor_timer(map, 422, 20);
            } else if mode != 1 {
                timer(map, SpawnObjectType::Creature, 422, if mode == 0 { i64::MAX } else { 120 });
            }
        }
        let jitter = if mode == 3 { 0 } else { 5 };
        let expected = old.original_catalog(100, &store, &linked, &PoolMgrLikeCpp::new(), jitter, false, |_, _| false,
            |_, _| 0.0, |_, count| (0..count).collect(), false, |_, _, _| panic!("linked gate"));
        let actual = shared.process_due_respawns_materialized(100, &store, &linked, &PoolMgrLikeCpp::new(), jitter, false, |_, _| false,
            |_, _| 0.0, |_, count| (0..count).collect(), false, |_, _, _| panic!("linked gate"));
        assert_eq!(actual.summary, expected);
        assert_eq!(actual.summary.rescheduled_linked_respawns.len(), usize::from(mode != 3));
        assert_eq!(actual.summary.blocked_linked_respawn_non_future, usize::from(mode == 3));
        assert!(actual.attempts.is_empty());
        assert_eq!(shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 421), if mode == 0 { i64::MAX } else if mode == 1 { 100 + 7 * 24 * 60 * 60 } else if mode == 2 { 125 } else { 90 });
        if mode == 3 { assert_eq!(shared.respawn_store.actor_queue_len(), 1); }
    }
}
